//! 运行模式(Unix):在伪终端里跑子命令,stdout 彩虹转发,stdin 原样透传。
//! 关键修正(相对旧 Python 版曾经的 bug):
//!   子伪终端保持内核默认 cooked 模式(ONLCR 等输出加工必须保留,
//!   否则子进程写的裸 \n 不会被翻译成 \r\n,图片排版直接散架),
//!   只关 ECHO(避免代理注入的查询应答被回显);
//!   用户真终端只关 ICANON/ECHO(字节级输入,查询应答直达),
//!   IFLAG/OFLAG 原样保留(OPOST/ONLCR 必须活着)。
//! portable recipe:posix_openpt+grantpt+unlockpt+ptsname+fork+setsid,
//! Linux / FreeBSD / macOS 通用。

use std::ffi::CString;
use std::io::Write;
use std::os::unix::io::RawFd;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::filter::LolcatFilter;

static GOT_SIGINT: AtomicBool = AtomicBool::new(false);
static GOT_SIGWINCH: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigint(_: libc::c_int) {
    GOT_SIGINT.store(true, Ordering::Relaxed);
}
extern "C" fn on_sigwinch(_: libc::c_int) {
    GOT_SIGWINCH.store(true, Ordering::Relaxed);
}

fn copy_winsize(dst: RawFd) {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) != 0 {
            return;
        }
        if ws.ws_row == 0 && ws.ws_col == 0 && ws.ws_xpixel == 0 && ws.ws_ypixel == 0 {
            return;
        }
        libc::ioctl(dst, libc::TIOCSWINSZ, &ws);
    }
}

/// 子伪终端:cooked 保留 + 去回显(勿 setraw,会杀 ONLCR 致排版散架)。
fn slave_noecho(sfd: RawFd) {
    unsafe {
        let mut t: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(sfd, &mut t) != 0 {
            return;
        }
        t.c_lflag &= !libc::ECHO;
        libc::tcsetattr(sfd, libc::TCSANOW, &t);
    }
}

/// 用户真终端:字节级输入但保留 OPOST/ONLCR(勿 setraw)。
/// 返回旧 termios 供恢复;非终端返回 None。
fn stdin_cbreak() -> Option<libc::termios> {
    unsafe {
        let mut saved: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &mut saved) != 0 {
            return None;
        }
        let mut t = saved;
        t.c_lflag &= !(libc::ICANON | libc::ECHO);
        t.c_cc[libc::VMIN as usize] = 1;
        t.c_cc[libc::VTIME as usize] = 0;
        if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSADRAIN, &t) != 0 {
            return None;
        }
        Some(saved)
    }
}

fn restore_stdin(saved: &libc::termios) {
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSADRAIN, saved);
    }
}

fn write_all(fd: RawFd, mut data: &[u8]) {
    while !data.is_empty() {
        let n = unsafe { libc::write(fd, data.as_ptr() as *const libc::c_void, data.len()) };
        if n <= 0 {
            break;
        }
        data = &data[n as usize..];
    }
}

/// 改进程 comm(15 字节截断)。代理期间把 rscat 的 comm 临时改成
/// 真实终端名,让 fastfetch 的终端模块(按父进程找终端)显示本尊。
/// prctl(PR_SET_NAME) 是 Linux 专属;FreeBSD/macOS 上不伪装,
/// fastfetch 会显示 "rscat"(可接受的行为差异)。
#[cfg(target_os = "linux")]
fn set_comm(name: &str) {
    let mut buf = [0u8; 16];
    let bytes = name.as_bytes();
    let n = bytes.len().min(15);
    buf[..n].copy_from_slice(&bytes[..n]);
    unsafe {
        libc::prctl(libc::PR_SET_NAME, buf.as_ptr(), 0, 0, 0);
    }
}

#[cfg(not(target_os = "linux"))]
fn set_comm(_name: &str) {}

/// 在伪终端里跑 cmd。
/// comm:代理期间 rscat 的进程名伪装(调用 shell 的名字)。fastfetch 等按
/// 父进程链报 SHELL/终端:shell 模块取直接父进程(包装 shell,本尊);
/// 终端模块跳过 shell 往上找第一个非 shell —— 若 rscat 保持自己的名字
/// 会被当成终端显示 "rscat";伪装成 shell 后 fastfetch 会穿过它命中
/// 链上真正的终端模拟器(kitty 等),连版本号都是真终端应答的。
/// env:需要注入子进程的环境变量(SHELL 指向真实调用 shell、zsh 的 ZDOTDIR 等)。
/// stop_file:会话标记文件;若给出且文件消失则结束(供 -a/-c 用)。
/// 返回子进程退出码。
pub fn run(
    cmd: &[String],
    env: &[(String, String)],
    comm: Option<&str>,
    flt: &mut LolcatFilter,
    stop_file: Option<&Path>,
) -> i32 {
    if cmd.is_empty() {
        return 0;
    }
    unsafe {
        let mfd = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        if mfd < 0 {
            return 127;
        }
        if libc::grantpt(mfd) != 0 || libc::unlockpt(mfd) != 0 {
            libc::close(mfd);
            return 127;
        }
        let name_ptr = libc::ptsname(mfd);
        if name_ptr.is_null() {
            libc::close(mfd);
            return 127;
        }
        let name = std::ffi::CStr::from_ptr(name_ptr).to_bytes().to_vec();
        let sfd = libc::open(
            CString::new(name).unwrap().as_ptr(),
            libc::O_RDWR | libc::O_NOCTTY,
        );
        if sfd < 0 {
            libc::close(mfd);
            return 127;
        }
        copy_winsize(sfd);
        slave_noecho(sfd);

        // argv
        let cstrs: Vec<CString> = cmd
            .iter()
            .map(|s| CString::new(s.as_str()).unwrap())
            .collect();
        let mut argv: Vec<*const libc::c_char> = cstrs.iter().map(|c| c.as_ptr()).collect();
        argv.push(std::ptr::null());

        let pid = libc::fork();
        if pid < 0 {
            libc::close(mfd);
            libc::close(sfd);
            return 127;
        }
        if pid == 0 {
            // ---- 子进程 ----
            libc::setsid();
            let _ = libc::ioctl(sfd, libc::TIOCSCTTY, 0);
            // 新 pty 的前台进程组未设置(TIOCGPGRP=0):shell 检测到
            // tcgetpgrp != pgrp 时会禁用行编辑并因 SIGTTIN 停住
            // (症状:嵌套 bash/zsh 输入不显示,回车才执行)。
            let _ = libc::tcsetpgrp(sfd, libc::getpgrp());
            for (k, v) in env {
                if let (Ok(k), Ok(v)) = (
                    CString::new(k.as_str()),
                    CString::new(v.as_str()),
                ) {
                    libc::setenv(k.as_ptr(), v.as_ptr(), 1);
                }
            }
            libc::dup2(sfd, libc::STDIN_FILENO);
            libc::dup2(sfd, libc::STDOUT_FILENO);
            libc::dup2(sfd, libc::STDERR_FILENO);
            if sfd > libc::STDERR_FILENO {
                libc::close(sfd);
            }
            libc::close(mfd);
            libc::execvp(argv[0], argv.as_ptr());
            let _ = std::io::stderr().write_all(b"rscat: exec failed\n");
            libc::_exit(127);
        }
        // ---- 父进程 ----
        libc::close(sfd);
        // 代理期间把 comm 伪装成调用 shell 的名字(见 run() 文档)
        if let Some(c) = comm {
            set_comm(c);
        }
        libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
        libc::signal(libc::SIGWINCH, on_sigwinch as *const () as libc::sighandler_t);
        GOT_SIGINT.store(false, Ordering::Relaxed);
        GOT_SIGWINCH.store(false, Ordering::Relaxed);

        let saved = stdin_cbreak();
        let mut poll_stdin = saved.is_some();
        let mut rc = 0;
        let mut child_gone = false;
        let mut stop_sent = false;
        let mut stop_at: Option<std::time::Instant> = None;
        let mut buf = [0u8; 65536];

        loop {
            if GOT_SIGWINCH.swap(false, Ordering::Relaxed) {
                // 窗口变了:把新尺寸同步给子伪终端(从真 stdout 读)
                let mut ws: libc::winsize = std::mem::zeroed();
                if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 {
                    // 通过主端设置从端尺寸:对主端用 TIOCSWINSZ 同样生效
                    libc::ioctl(mfd, libc::TIOCSWINSZ, &ws);
                }
            }
            if GOT_SIGINT.swap(false, Ordering::Relaxed) {
                // 转发给子进程(交互式 shell 取消当前行,非交互等价于终止)
                libc::kill(pid, libc::SIGINT);
            }
            if let Some(p) = stop_file {
                if !stop_sent && !p.exists() {
                    // 会话被 rscat -c 取消:先 SIGHUP(交互式 shell 会正常退出,
                    // 它们通常忽略 SIGTERM),500ms 后还活着则 SIGKILL。
                    libc::kill(pid, libc::SIGHUP);
                    stop_sent = true;
                    stop_at = Some(std::time::Instant::now());
                }
                if stop_sent && !child_gone {
                    if let Some(t0) = stop_at {
                        if t0.elapsed() > std::time::Duration::from_millis(500) {
                            libc::kill(pid, libc::SIGKILL);
                        }
                    }
                }
            }

            let mut rfds: libc::fd_set = std::mem::zeroed();
            libc::FD_ZERO(&mut rfds);
            libc::FD_SET(mfd, &mut rfds);
            if poll_stdin {
                libc::FD_SET(libc::STDIN_FILENO, &mut rfds);
            }
            let maxfd = mfd.max(libc::STDIN_FILENO);
            let mut tv = libc::timeval {
                tv_sec: 0,
                tv_usec: 20000,
            };
            let n = libc::select(maxfd + 1, &mut rfds, std::ptr::null_mut(), std::ptr::null_mut(), &mut tv);
            if n < 0 {
                continue; // EINTR 等,重试
            }
            let mut mfd_hit = false;
            if libc::FD_ISSET(mfd, &rfds) {
                mfd_hit = true;
                let r = libc::read(mfd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                if r <= 0 {
                    break; // EIO/EOF:子端已关
                }
                flt.feed(&buf[..r as usize]);
            }
            if poll_stdin && libc::FD_ISSET(libc::STDIN_FILENO, &rfds) {
                let r = libc::read(
                    libc::STDIN_FILENO,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                );
                if r > 0 {
                    write_all(mfd, &buf[..r as usize]);
                } else {
                    poll_stdin = false; // stdin EOF:不再轮询
                }
            }
            // 子进程是否已退出
            let mut status = 0;
            let w = libc::waitpid(pid, &mut status, libc::WNOHANG);
            if w == pid {
                child_gone = true;
                if libc::WIFEXITED(status) {
                    rc = libc::WEXITSTATUS(status);
                } else if libc::WIFSIGNALED(status) {
                    rc = 128 + libc::WTERMSIG(status);
                }
                if !mfd_hit {
                    // 给残余输出 50ms 排空窗口
                    let mut rfds2: libc::fd_set = std::mem::zeroed();
                    libc::FD_ZERO(&mut rfds2);
                    libc::FD_SET(mfd, &mut rfds2);
                    let mut tv2 = libc::timeval {
                        tv_sec: 0,
                        tv_usec: 50000,
                    };
                    if libc::select(
                        mfd + 1,
                        &mut rfds2,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut tv2,
                    ) <= 0
                    {
                        break;
                    }
                    // 还有数据:下轮循环读
                    continue;
                }
            }
            if child_gone {
                // 退出后多转几圈确保排空
                let mut idle = 0;
                while idle < 3 {
                    let mut rfds3: libc::fd_set = std::mem::zeroed();
                    libc::FD_ZERO(&mut rfds3);
                    libc::FD_SET(mfd, &mut rfds3);
                    let mut tv3 = libc::timeval {
                        tv_sec: 0,
                        tv_usec: 20000,
                    };
                    if libc::select(
                        mfd + 1,
                        &mut rfds3,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut tv3,
                    ) <= 0
                    {
                        idle += 1;
                        continue;
                    }
                    let r = libc::read(mfd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                    if r <= 0 {
                        break;
                    }
                    flt.feed(&buf[..r as usize]);
                    idle = 0;
                }
                break;
            }
            // stop_file 消失且子进程还活着:等它收到 SIGTERM 退出(上面的 waitpid 会收)
            if stop_file.is_some_and(|p| !p.exists()) && !child_gone {
                // 再给一点时间让 SIGTERM 生效,避免忙等
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        if let Some(s) = saved {
            restore_stdin(&s);
        }
        if comm.is_some() {
            set_comm("rscat"); // 还原进程名
        }
        libc::signal(libc::SIGINT, libc::SIG_DFL);
        libc::signal(libc::SIGWINCH, libc::SIG_DFL);
        // 确保子进程已回收
        if !child_gone {
            let mut status = 0;
            libc::waitpid(pid, &mut status, 0);
            if libc::WIFEXITED(status) {
                rc = libc::WEXITSTATUS(status);
            } else if libc::WIFSIGNALED(status) {
                rc = 128 + libc::WTERMSIG(status);
            }
        }
        libc::close(mfd);
        rc
    }
}
