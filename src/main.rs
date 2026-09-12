//! rscat:能输出图片的 lolcat(Rust)。
//! 彩虹算法与 lolcat 100.0.1 逐字节一致;kitty/iTerm2 图片直通;
//! -e 在伪终端里跑命令(图片+彩虹兼得);-a 彩虹会话;-c 取消。

mod filter;
mod i18n;
mod image;
mod persist;
mod rainbow;
mod shell;
mod shell_detect;
#[cfg(unix)]
mod pty_unix;
#[cfg(windows)]
mod pty_windows;

use std::io::{IsTerminal, Read, Write};

use crate::filter::LolcatFilter;
use crate::i18n::{Lang, Msg, help, t};
use crate::rainbow::{ColorMode, Painter};

#[cfg(unix)]
fn pty_supported() -> bool {
    true
}
#[cfg(windows)]
fn pty_supported() -> bool {
    pty_windows::supported()
}

#[cfg(unix)]
fn pty_run(cmd: &[String], env: &[(String, String)], comm: Option<&str>, flt: &mut LolcatFilter, stop: Option<&std::path::Path>) -> i32 {
    pty_unix::run(cmd, env, comm, flt, stop)
}
#[cfg(windows)]
fn pty_run(cmd: &[String], env: &[(String, String)], _comm: Option<&str>, flt: &mut LolcatFilter, _stop: Option<&std::path::Path>) -> i32 {
    let _ = (env, flt);
    pty_windows::run(cmd, flt, _stop)
}

struct Opts {
    spread: f64,
    freq: f64,
    seed: i64,
    animate: bool,
    duration: usize,
    speed: f64,
    invert: bool,
    truecolor: bool,
    force: bool,
    exec: Vec<String>,
    always: bool,
    cancel: bool,
    proto: String,
    quiet: bool,
    image_force: bool,
    init: Option<String>,
    lang_name: Option<String>,
    help: bool,
    version: bool,
    files: Vec<String>,
}

impl Opts {
    fn new() -> Opts {
        Opts {
            spread: 3.0,
            freq: 0.1,
            seed: 0,
            animate: false,
            duration: 12,
            speed: 20.0,
            invert: false,
            truecolor: false,
            force: false,
            exec: Vec::new(),
            always: false,
            cancel: false,
            proto: "auto".to_string(),
            quiet: false,
            image_force: false,
            init: None,
            lang_name: None,
            help: false,
            version: false,
            files: Vec::new(),
        }
    }
}

fn parse_number(lang: Lang, what: &str, s: &str) -> Result<f64, i32> {
    s.parse::<f64>().map_err(|_| {
        eprintln!("{} {what} = {s}", t(lang, Msg::ErrBadNumber));
        2
    })
}

/// 手写参数解析(保持帮助文本多语言,故不用 clap)。
/// -e/--exec 与 -- 之后均为 REMAINDER,直接作为待跑命令。
fn parse_args(lang: Lang, args: &[String]) -> Result<Opts, i32> {
    let mut o = Opts::new();
    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        // --opt=value 形式
        let (name, eq_val): (&str, Option<&str>) = match a.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v)),
            _ => (a.as_str(), None),
        };
        let mut val_of = |lang: Lang, what: &str| -> Result<String, i32> {
            if let Some(v) = eq_val {
                return Ok(v.to_string());
            }
            it.next().cloned().ok_or_else(|| {
                eprintln!("{}: {what}", t(lang, Msg::ErrMissingValue));
                2
            })
        };
        match name {
            "-p" | "--spread" => o.spread = parse_number(lang, "--spread", &val_of(lang, "--spread")?)?,
            "-F" | "--freq" => o.freq = parse_number(lang, "--freq", &val_of(lang, "--freq")?)?,
            "-S" | "--seed" => {
                let v = val_of(lang, "--seed")?;
                o.seed = v.parse::<i64>().map_err(|_| {
                    eprintln!("{} --seed = {v}", t(lang, Msg::ErrBadNumber));
                    2
                })?;
            }
            "--animate" => o.animate = true,
            "-d" | "--duration" => {
                let v = parse_number(lang, "--duration", &val_of(lang, "--duration")?)?;
                o.duration = v as usize;
            }
            "-s" | "--speed" => o.speed = parse_number(lang, "--speed", &val_of(lang, "--speed")?)?,
            "-i" | "--invert" => o.invert = true,
            "-t" | "--truecolor" => o.truecolor = true,
            "-f" | "--force" => o.force = true,
            "-e" | "--exec" => {
                let mut rest: Vec<String> = it.map(|s| s.to_string()).collect();
                if rest.first().map(|s| s.as_str()) == Some("--") {
                    rest.remove(0);
                }
                o.exec = rest;
                break;
            }
            "--" => {
                o.exec = it.map(|s| s.to_string()).collect();
                break;
            }
            "-a" | "--always" => o.always = true,
            "-c" | "--cancel" => o.cancel = true,
            "--proto" => {
                let v = val_of(lang, "--proto")?;
                if !["auto", "kitty", "iterm"].contains(&v.as_str()) {
                    eprintln!("{} --proto = {v}", t(lang, Msg::ErrBadNumber));
                    return Err(2);
                }
                o.proto = v;
            }
            "-q" | "--quiet" => o.quiet = true,
            "--image" => o.image_force = true,
            "--init" => o.init = Some(val_of(lang, "--init")?),
            "--lang" => o.lang_name = Some(val_of(lang, "--lang")?),
            "-h" | "--help" => o.help = true,
            "-V" | "--version" => o.version = true,
            _ if a.starts_with('-') && a.len() > 1 => {
                eprintln!("{}: {a}", t(lang, Msg::ErrUnknownFlag));
                return Err(2);
            }
            _ => o.files.push(a.to_string()),
        }
    }
    Ok(o)
}

fn random_seed() -> u64 {
    #[cfg(unix)]
    {
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            let mut b = [0u8; 1];
            if f.read_exact(&mut b).is_ok() {
                return b[0] as u64;
            }
        }
    }
    // 兜底:纳秒时间(仅 Windows 或无 urandom 时)
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 % 256)
        .unwrap_or(42)
}

fn make_filter(o: &Opts, seed: u64, out: Box<dyn Write>) -> LolcatFilter {
    LolcatFilter::new(
        out,
        o.freq,
        o.spread,
        seed,
        Painter::new(ColorMode::detect(o.truecolor), o.invert),
        o.animate,
        o.duration,
        o.speed,
    )
}

/// 终端标题压栈(kitty/xterm 标题栈):进入 pty 模式前保存用户当前标题,
/// 否则 kitty 等会把标签页标题显示成前台进程名 "rscat"。
fn title_push() {
    if std::io::stdout().is_terminal() {
        let _ = std::io::stdout().write_all(b"\x1b[22;2t");
        let _ = std::io::stdout().flush();
    }
}

/// 退出 pty 模式后的终端状态还原:弹回标题、软复位(DECSTR,
/// 归零子进程残留的 SGR/字符集/滚动区域等)、字符集 ASCII、
/// 主字体、光标显示。session=true 时再多发 kitty 键盘协议清零
/// (内层会话 shell 若被 SIGKILL 强杀来不及弹栈,外层按键解析会残留)。
fn terminal_restore(session: bool) {
    if !std::io::stdout().is_terminal() {
        return;
    }
    let mut seq: Vec<u8> = Vec::new();
    seq.extend_from_slice(b"\x1b[23;2t\x1b[!p\x1b(B\x1b)B\x1b[10m\x1b[m\x1b[?25h");
    if session {
        seq.extend_from_slice(b"\x1b[>0u\x1b[?1;5;2004l");
    }
    let _ = std::io::stdout().write_all(&seq);
    let _ = std::io::stdout().flush();
}

fn pick_shell() -> shell_detect::Shell {
    shell_detect::detect_or_fallback()
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    // 先用环境语言解析参数(--lang 本身需要在解析后才生效,错误信息用环境语言)
    let env_lang = Lang::detect();
    let o = match parse_args(env_lang, &argv) {
        Ok(o) => o,
        Err(c) => std::process::exit(c),
    };
    // --lang 覆盖
    let lang = match &o.lang_name {
        Some(n) => match Lang::parse(n) {
            Some(l) => l,
            None => {
                eprintln!("{}: {n}", t(env_lang, Msg::ErrUnknownLang));
                std::process::exit(2);
            }
        },
        None => env_lang,
    };

    if o.help {
        print!("{}", help(lang));
        return;
    }
    if o.version {
        println!("{}", t(lang, Msg::Version));
        return;
    }
    if let Some(sh) = &o.init {
        match shell::snippet(sh) {
            Some(s) => print!("{s}"),
            None => {
                eprintln!("{}", t(lang, Msg::ErrUnknownShell));
                std::process::exit(2);
            }
        }
        return;
    }
    if o.cancel {
        // -c:取消会话(会话内/外通用;会话主循环看到标记消失即收尾)
        if persist::session_active() {
            persist::session_stop();
            println!("{}", t(lang, Msg::CancelOk));
        } else {
            println!("{}", t(lang, Msg::CancelNone));
        }
        return;
    }
    if o.always {
        // -a:彩虹会话
        if !o.exec.is_empty() {
            eprintln!("{}", t(lang, Msg::ErrConflictAlwaysExec));
            std::process::exit(2);
        }
        if persist::inside_session() {
            eprintln!("{}", t(lang, Msg::SessionNested));
            std::process::exit(1);
        }
        if persist::session_active() {
            eprintln!("{}", t(lang, Msg::SessionAlready));
            std::process::exit(1);
        }
        if !pty_supported() {
            eprintln!("{}", t(lang, Msg::ErrNeedUnixPty));
            std::process::exit(127);
        }
        if persist::session_start().is_err() {
            eprintln!("{} {:?}", t(lang, Msg::ErrReadFile), persist::session_marker());
            std::process::exit(1);
        }
        eprintln!("{}", t(lang, Msg::SessionEnter));
        std::env::set_var("RSCAT_SESSION", "1");
        let seed = if o.seed != 0 { o.seed as u64 } else { random_seed() };
        let out: Box<dyn Write> = Box::new(std::io::stdout().lock());
        let mut flt = make_filter(&o, seed, out);
        let sh = pick_shell();
        let (sargv, senv) = shell_detect::session_argv(&sh);
        title_push();
        let rc = pty_run(&sargv, &senv, Some(&sh.name()), &mut flt, Some(&persist::session_marker()));
        flt.finish(std::io::stdout().is_terminal());
        terminal_restore(true);
        persist::session_stop();
        eprintln!("{}", t(lang, Msg::SessionExit));
        std::process::exit(rc);
    }
    if !o.exec.is_empty() {
        // -e:运行模式
        let seed = if o.seed != 0 { o.seed as u64 } else { random_seed() };
        let colorize = std::io::stdout().is_terminal() || o.force;
        if !colorize {
            // 与 lolcat 一致:非 tty 且未 -f 时原样直通(直接跑,不染色)
            let mut cmd = std::process::Command::new(&o.exec[0]);
            if o.exec.len() > 1 {
                cmd.args(&o.exec[1..]);
            }
            let rc = match cmd.status() {
                Ok(s) => s.code().unwrap_or(127),
                Err(e) => {
                    eprintln!("{} {}: {e}", t(lang, Msg::ErrExecFailed), o.exec[0]);
                    127
                }
            };
            std::process::exit(rc);
        }
        if !pty_supported() {
            eprintln!("{}", t(lang, Msg::ErrNeedUnixPty));
            std::process::exit(127);
        }
        let out: Box<dyn Write> = Box::new(std::io::stdout().lock());
        let mut flt = make_filter(&o, seed, out);
        // 用真实调用 shell 包一层:fastfetch 等按父进程报 SHELL,
        // 直接 spawn 会显示 "rscat";包一层后显示 fish/zsh/bash 等本尊。
        let sh = pick_shell();
        let eargv = shell_detect::exec_argv(&sh, &o.exec);
        let eenv = shell_detect::exec_env(&sh);
        title_push();
        let rc = pty_run(&eargv, &eenv, Some(&sh.name()), &mut flt, None);
        flt.finish(std::io::stdout().is_terminal());
        terminal_restore(false);
        std::process::exit(rc);
    }

    // ---- 过滤/图片模式 ----
    let colorize = std::io::stdout().is_terminal() || o.force;
    if !colorize {
        // 原样直通(与 lolcat/Python 版一致)
        let mut stdin = std::io::stdin().lock();
        let mut stdout = std::io::stdout().lock();
        let _ = std::io::copy(&mut stdin, &mut stdout);
        return;
    }
    let seed = if o.seed != 0 { o.seed as u64 } else { random_seed() };
    let out: Box<dyn Write> = Box::new(std::io::stdout().lock());
    let mut flt = make_filter(&o, seed, out);

    // 文件分类:图片 vs 文本
    let mut images: Vec<String> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    for path in &o.files {
        if o.image_force {
            images.push(path.clone());
            continue;
        }
        let head = std::fs::File::open(path)
            .ok()
            .and_then(|mut f| {
                let mut b = [0u8; 16];
                f.read_exact(&mut b).ok().map(|_| b.to_vec())
            })
            .unwrap_or_default();
        if image::sniff_image(&head) {
            images.push(path.clone());
        } else {
            texts.push(path.clone());
        }
    }

    let is_tty = std::io::stdout().is_terminal();
    if !images.is_empty() {
        // 图片字节经 flt.feed_raw 直通(同一把 stdout 锁,无死锁、保顺序)
        let proto = image::pick_proto(&o.proto).to_string();
        for path in &images {
            let data = match std::fs::read(path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{} {path}: {e}", t(lang, Msg::ErrReadFile));
                    continue;
                }
            };
            if proto == "kitty" {
                match image::as_png(&data) {
                    Ok((png, _, _)) => {
                        let mut buf = Vec::new();
                        image::send_kitty(&mut buf, &png);
                        flt.feed_raw(&buf);
                    }
                    Err(()) => {
                        eprintln!("{} {path}", t(lang, Msg::ErrNoDecode));
                        std::process::exit(1);
                    }
                }
            } else {
                let name = std::path::Path::new(path)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                let mut buf = Vec::new();
                image::send_iterm(&mut buf, &data, &name);
                flt.feed_raw(&buf);
            }
            if !o.quiet {
                flt.feed(format!("  {path}\n").as_bytes());
            }
        }
    }
    if !texts.is_empty() {
        for path in &texts {
            match std::fs::File::open(path) {
                Ok(mut f) => {
                    let mut buf = [0u8; 65536];
                    loop {
                        match f.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => flt.feed(&buf[..n]),
                            Err(e) => {
                                eprintln!("{} {path}: {e}", t(lang, Msg::ErrReadFile));
                                break;
                            }
                        }
                    }
                }
                Err(e) => eprintln!("{} {path}: {e}", t(lang, Msg::ErrReadFile)),
            }
        }
    } else if images.is_empty() {
        let stdin = std::io::stdin();
        let mut lock = stdin.lock();
        let mut buf = [0u8; 65536];
        loop {
            match lock.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => flt.feed(&buf[..n]),
                Err(_) => break,
            }
        }
    }
    flt.finish(is_tty);
}
