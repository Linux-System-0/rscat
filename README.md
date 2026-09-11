# rscat 🌈🐱

**能输出图片的 lolcat** — Rust 重写,彩虹算法与 `lolcat` 逐字节一致,另支持 kitty / iTerm2 图片直通、伪终端运行模式与彩虹会话。

**lolcat that can output images** — rewritten in Rust, byte-for-byte rainbow compatible with `lolcat`, plus kitty/iTerm2 image passthrough, PTY run mode and rainbow sessions.

> 起源:前身 `nyacat`(Python 单文件)为解决 `fastfetch | lolcat` 不能同时显示图片和彩虹字而写;本仓库是其 Rust 重写与正式工程化版本,原 Python 实现见 [`original/nyacat.py`](original/nyacat.py)。
> Origin: predecessor `nyacat` (single-file Python) was written so `fastfetch` can show images *and* rainbow text at once; this repo is its Rust rewrite and productionized version. The original Python implementation lives in [`original/nyacat.py`](original/nyacat.py).

---

## 特性 Features

| | 中文 | English |
|---|---|---|
| 彩虹过滤 | 与 lolcat 100.0.1 **逐字节一致**(8 组对照+真彩/256色全通过) | Byte-for-byte compatible with lolcat 100.0.1 |
| 图片直通 | kitty 图形协议 / sixel / OSC 原样透传,不破坏图片 | kitty graphics / sixel / OSC passthrough, images intact |
| 运行模式 | `rscat -e fastfetch`:伪终端里跑命令,**图片+彩虹兼得**,无需包装命令 | `rscat -e fastfetch`: run in a pty, images + rainbow, no wrapper needed |
| 图片模式 | `rscat logo.png`:直接显示本地图片(PNG/JPEG/GIF) | `rscat logo.png`: display local images directly |
| 彩虹会话 | `rscat -a` 之后所有命令彩虹输出,`rscat -c` 取消 | `rscat -a`: all later commands rainbow, `rscat -c` cancels |
| 多语言 | 简中/繁中/英文/日文,跟随系统 `LANG`,可 `--lang` 指定 | zh-CN/zh-TW/en/ja, follows system `LANG` |
| 多 shell | bash / zsh / sh / fish / PowerShell / cmd 集成片段 | Integration snippets for 6 shells |
| 跨平台 | Linux(已测) / FreeBSD / macOS / Windows(见平台矩阵) | Linux (tested) / FreeBSD / macOS / Windows (see matrix) |

---

## 安装 Install

```bash
# 构建(需 Rust 1.70+)
cargo build --release
# 安装到 ~/.local/bin
install -Dm755 target/release/rscat ~/.local/bin/rscat

# shell 集成(可选,仅把 ~/.local/bin 加入 PATH,幂等)
rscat --init fish >> ~/.config/fish/config.fish   # fish
rscat --init bash >> ~/.bashrc                     # bash
rscat --init zsh >> ~/.zshrc                       # zsh
rscat --init sh >> ~/.profile                      # sh
rscat --init powershell >> $PROFILE                # PowerShell
rscat --init cmd > %TEMP%\rscat-init.cmd & %TEMP%\rscat-init.cmd   # cmd(运行一次)
```

Windows:把 `target\release\rscat.exe` 放到 `%USERPROFILE%\.local\bin\`,用上面 `--init` 片段加 PATH。
macOS/FreeBSD:同 Linux 流程构建(纯 portable Unix 代码,无系统依赖)。

---

## 用法 Usage

```bash
# 1. 过滤模式(等价 lolcat)
neofetch | rscat
rscat file.txt

# 2. 运行模式:fastfetch 图片 + 彩虹字(本体 -e 旗标,不再需要 nfa 包装)
rscat -e fastfetch

# 3. 图片模式:直接显示图片
rscat ~/.config/fastfetch/nyarch-logo.png

# 4. 彩虹会话:之后所有命令输出都是彩虹色
rscat -a
# ... 随便跑命令,全是彩虹 ...
rscat -c     # 或输入 exit,退出会话
```

> **为什么 `fastfetch | rscat` 看不到图片?**
> **Why no images with `fastfetch | rscat`?**
> 管道里出不来图片是 fastfetch 的限制:stdout 非 tty 时它根本不发送图片数据(**实测**:连 `--kitty`/`--kitty-direct` 强制指定也只出 ASCII,0 字节 APC;且 fastfetch 没有 `--force-tty` 之类的开关),任何过滤器都变不出来。请用 `rscat -e fastfetch`(伪终端模式,fastfetch 以为在真终端里,照常发图片)。
> That's a fastfetch limitation: it never emits image data to a non-tty (**verified**: even forced `--kitty`/`--kitty-direct` yield ASCII-only, 0 APC bytes; no `--force-tty` switch exists), so no filter can conjure it. Use `rscat -e fastfetch` (PTY mode — fastfetch believes it's on a real terminal and emits images).

> **fastfetch logo 类型建议:用 `kitty`,别用 `kitty-direct`**(血泪结论,见下)。
> **fastfetch logo type: prefer `kitty` over `kitty-direct`** (hard-won, see below).

选项与 lolcat 相同(`-p/-F/-S/--animate/-d/-s/-i/-t/-f`),另加 `-e/--exec`、`-a/--always`、`-c/--cancel`、`--proto`、`--image`、`--init`、`--lang`。完整多语言帮助见 `rscat -h`。
Same options as lolcat plus rscat extras. Full multilingual help: `rscat -h`.

---

## 平台支持矩阵 Platform matrix

| 平台 Platform | 过滤 filter | 图片 image | 运行 `-e` run | 会话 `-a` session | 说明 Notes |
|---|---|---|---|---|---|
| Linux | ✅ 已测 | ✅ 已测 | ✅ 已测 | ✅ 已测 | CI/日常主力 |
| FreeBSD | ✅ 预期 | ✅ 预期 | ✅ 预期 | ✅ 预期 | portable POSIX 代码,待实机验证 |
| macOS | ✅ 预期 | ✅ 预期 | ✅ 预期 | ✅ 预期 | 同上;`--proto iterm` 适配 iTerm2/WezTerm |
| Windows | ✅ | ✅(wezterm+iterm) | ❌ 明确报错 | ❌ 明确报错 | ConPTY 后续版本;不过滤乱码,直接多语言提示 |

“预期”=“代码仅用三平台通用 POSIX 接口(libc),但作者暂无实机验证,欢迎报 issue”。
"Expected" = code only uses POSIX APIs common to all three (via `libc`), but the author has no test machine yet — issues welcome.

---

## 与前身差异 Differences from nyacat (Python)

1. **修了 `-c`/`-e` 图片文字错位 bug**(两张截屏的症状),分两层:
   - **termios 层**:运行模式曾对子伪终端和用户终端用 `setraw()`,关掉 `ONLCR`,裸 `\n` 不回车致楼梯式散架。现子端保持 cooked 只关 `ECHO`,用户端只关 `ICANON/ECHO`。
   - **协议层**(真凶):`kitty-direct` 传的是**文件路径**,fastfetch 必须发 `\x1b[6n` 查光标才知道图片占了几行;而 fastfetch 只等 **50–100ms**(实测阈值),经代理转发的应答稍慢就超时回退,文字掉到图片下方。改用 `kitty` 类型后传的是 **RGBA+zlib 内联数据**,fastfetch 用确定性 `\x1b[15A` cursor-up 排版,**零终端查询、零竞态**,经任何代理版面都不散。
   **Fixed the image/text misalignment in two layers**: (1) termios — `setraw()` killed `ONLCR` causing staircase; now cooked-minus-ECHO / cbreak-input-only. (2) protocol (the real culprit) — `kitty-direct` transmits a *file path* so fastfetch must query the cursor (`\x1b[6n`) to learn the image height, but only waits **~50–100ms** (measured); a slightly-slow proxied answer times out and text falls below the image. The `kitty` type sends *inline RGBA+zlib* with deterministic `\x1b[15A` layout — **zero queries, zero race**, stable through any proxy.
2. **修了 `--animate` 必崩 bug**:Python 版 `self.line = bytearray()` 却 `append((run, char))`,一用就 `TypeError`。Rust 版正常实现。
   **Fixed `--animate` always crashing** (bytearray.append(tuple) TypeError). Implemented correctly in Rust.
3. `-a` 会话/`-c` 取消是新增功能(Python 版没有);`-e` 取代 Python 版 `-c`(因 `-c` 已给“取消”);`--animate` 因此只有长选项。
   `-a`/`-c` sessions are new; `-e` replaces Python's `-c`; `--animate` is long-only now.
4. 非 PNG 转码由 Pillow 换成 `image` crate(纯 Rust,无系统依赖);重采样器不同导致像素均值差约 2%(肉眼不可辨),尺寸/协议帧完全一致。
   Non-PNG transcode moved from Pillow to the `image` crate (pure Rust); resampler differs slightly (~2% mean pixel diff, invisible), dimensions/framing identical.

---

## 开发 Development

```bash
cargo test          # 单元测试:彩虹向量/base64/PNG头/魔数
cargo build --release
# 与 Python 原版逐字节对照(过滤器):
nyacat -f -S 42 < corpus.bin | cmp - <(rscat -f -S 42 < corpus.bin)
# 假终端排版 oracle(需 fastfetch):
python3 tests/emu_test.py --direct   # 对照组
python3 tests/emu_test.py --proxy    # 实验组,版面应与对照组一致
```

项目结构 Layout:

```
src/            main.rs(CLI) rainbow.rs filter.rs image.rs
                pty_unix.rs pty_windows.rs persist.rs i18n.rs shell.rs
                help_{zh_cn,zh_tw,en,ja}.txt
shells/         rscat.{bash,zsh,sh,fish,ps1,cmd}(--init 输出的正本)
original/       nyacat.py(Python 前身,已修复版,供对照)
tests/          emu_test.py(假终端排版 oracle)
```

License:待定 TBD.
