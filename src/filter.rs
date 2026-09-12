//! 转义序列感知过滤器:与 nyacat Python 版 LolcatFilter 同模型。
//! 配对模型:每行切成 (转义连跑, 单个可见字符) 配对,配对序号 i 决定颜色
//! rainbow(freq, os + i/spread);os 每行 +1(首行 seed+1)。
//! 输出顺序:转义连跑原样 → 颜色 SGR → 字符 → 复位(含空字符配对)。
//! kitty APC / sixel DCS / OSC / CSI 原样直通,图片不被破坏。

use std::io::Write;

use crate::rainbow::{Painter, rainbow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Esc,
    Csi,
    Osc,
    OscEsc,
    Str,
}

pub struct LolcatFilter {
    out: Box<dyn Write>,
    freq: f64,
    spread: f64,
    os: f64,
    painter: Painter,
    animate: bool,
    duration: usize,
    speed: f64,
    state: State,
    run: Vec<u8>,
    str_esc: bool,
    txt: Vec<u8>,
    i: usize,
    run_emitted: bool,
    line: Vec<(Vec<u8>, Option<char>)>,
    saw_nl: bool,
    /// OSC 133 shell 集成区域跟踪:true = 提示符/命令行区域(不彩虹,
    /// 子进程自带的颜色原样保留),false = 命令输出区域(上彩虹)。
    /// 标记:A=提示符开始 B=命令行开始 C=输出开始 D=命令结束。
    suppress: bool,
}

impl LolcatFilter {
    pub fn new(
        out: Box<dyn Write>,
        freq: f64,
        spread: f64,
        seed: u64,
        painter: Painter,
        animate: bool,
        duration: usize,
        speed: f64,
    ) -> LolcatFilter {
        LolcatFilter {
            out,
            freq,
            spread: spread.max(0.1),
            os: seed as f64 + 1.0,
            painter,
            animate,
            duration,
            speed,
            state: State::Ground,
            run: Vec::new(),
            str_esc: false,
            txt: Vec::new(),
            i: 0,
            run_emitted: false,
            line: Vec::new(),
            suppress: false,
            saw_nl: false,
        }
    }

    fn write_raw(&mut self, data: &[u8]) {
        let _ = self.out.write_all(data);
    }

    /// 一个配对:转义连跑(可空) + 单个字符(可空)。
    fn pair(&mut self, ch: Option<char>) {
        let mut run = Vec::new();
        if self.run_emitted {
            self.run_emitted = false; // 已即时写出,字节顺序仍与 lolcat 一致
        } else if !self.run.is_empty() {
            std::mem::swap(&mut run, &mut self.run);
        }
        if self.animate {
            self.line.push((run, ch));
            return;
        }
        self.write_pair(&run, ch, self.i, false);
        self.i += 1;
    }

    /// 转义序列完结即直通:查询/图片协议必须立刻到达终端,
    /// 否则子进程等应答、缓冲等字符,互相死锁到超时(animate 保持整行缓冲除外)。
    fn seq_done(&mut self) {
        if !self.animate && !self.run.is_empty() {
            let run = std::mem::take(&mut self.run);
            self.write_raw(&run);
            self.run_emitted = true;
        }
    }

    /// OSC 133 shell 集成标记:跟踪提示符/输出区域。
    /// A=提示符开始 B=命令行开始 C=输出开始 D=命令结束。
    /// A/B/D 区域不彩虹(保留子进程自带颜色),C 之后上彩虹。
    fn osc133_check(&mut self) {
        if self.run.len() >= 7 && self.run.starts_with(b"\x1b]133;") {
            match self.run[6] {
                b'C' => self.suppress = false,
                b'A' | b'B' | b'D' => self.suppress = true,
                _ => {}
            }
        }
    }

    fn write_pair(&mut self, run: &[u8], ch: Option<char>, idx: usize, frame_strip: bool) {
        // OSC 133 提示符/命令行区域:不上彩虹,子进程自带颜色原样保留。
        if self.suppress {
            if !run.is_empty() {
                self.write_raw(run);
            }
            if let Some(c) = ch {
                let mut b = [0u8; 4];
                self.write_raw(c.encode_utf8(&mut b).as_bytes());
            }
            return;
        }
        let run_bytes: &[u8] = if frame_strip {
            // animate 重绘帧:剥掉清行类 CSI(与 lol.rb println_ani 一致)。
            // 为避免分配,仅当包含 ESC 时才走剥离路径。
            if run.contains(&0x1B) {
                let stripped = strip_animate_csi(run);
                // write_pair 需要 &self.painter 可变借用,先把 stripped 存起来
                self.write_raw(&stripped);
                let (r, g, b) = rainbow(self.freq, self.os + idx as f64 / self.spread);
                let (sgr, reset) = self.painter.sgr(r, g, b);
                let sgr = sgr.to_vec();
                self.write_raw(&sgr);
                if let Some(c) = ch {
                    let mut b = [0u8; 4];
                    self.write_raw(c.encode_utf8(&mut b).as_bytes());
                }
                self.write_raw(reset);
                return;
            }
            run
        } else {
            run
        };
        if !run_bytes.is_empty() {
            self.write_raw(run_bytes);
        }
        let (r, g, b) = rainbow(self.freq, self.os + idx as f64 / self.spread);
        let (sgr, reset) = self.painter.sgr(r, g, b);
        let sgr = sgr.to_vec();
        self.write_raw(&sgr);
        if let Some(c) = ch {
            let mut b = [0u8; 4];
            self.write_raw(c.encode_utf8(&mut b).as_bytes());
        }
        self.write_raw(reset);
    }

    /// 行收尾:每行多出一个"空尾配对",换行后 os+=1。
    fn endline(&mut self) {
        self.pair(None);
        if self.animate {
            self.animate_line();
        }
        self.write_raw(b"\n");
        self.os += 1.0;
        self.i = 0;
        self.saw_nl = true;
    }

    fn animate_line(&mut self) {
        // println_ani:\e7 存光标 → duration 帧(\e8 回光标,os+=spread 重绘)。
        if self.line.is_empty() {
            return;
        }
        let real_os = self.os;
        self.write_raw(b"\x1b7");
        for frame in 0..self.duration {
            self.write_raw(b"\x1b8");
            self.os += self.spread;
            let line = std::mem::take(&mut self.line);
            for (idx, (run, ch)) in line.iter().enumerate() {
                self.write_pair(run, *ch, idx, frame > 0);
                let _ = self.out.flush();
            }
            self.line = Vec::new();
            std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / self.speed));
        }
        self.os = real_os;
    }

    fn emit_char(&mut self, ch: char) {
        if ch == '\n' {
            self.endline();
        } else if ch == '\t' {
            for _ in 0..8 {
                // tab 展开成 8 个独立空格配对
                self.pair(Some(' '));
            }
        } else {
            self.pair(Some(ch));
        }
    }

    /// 增量 UTF-8 解码(语义同 Python errors="replace" 增量解码器):
    /// 完整字符落盘;非法字节变 U+FFFD;末尾残缺序列留待下批数据。
    fn flush_text(&mut self) {
        // 先把本轮可解码字符收集成 owned,再逐个 emit(避开借用冲突)。
        let mut chars: Vec<char> = Vec::new();
        let mut i = 0;
        while i < self.txt.len() {
            match std::str::from_utf8(&self.txt[i..]) {
                Ok(s) => {
                    chars.extend(s.chars());
                    i = self.txt.len();
                }
                Err(e) => {
                    let valid = e.valid_up_to();
                    if valid > 0 {
                        // valid_up_to 保证此前缀合法
                        let s = std::str::from_utf8(&self.txt[i..i + valid]).unwrap();
                        chars.extend(s.chars());
                        i += valid;
                    }
                    match e.error_len() {
                        Some(n) => {
                            chars.push('\u{FFFD}');
                            i += n;
                        }
                        None => break, // 残缺尾,等下批
                    }
                }
            }
        }
        self.txt.drain(..i);
        for ch in chars {
            self.emit_char(ch);
        }
    }

    /// 转义序列状态机(序列进 run,原样直通)。
    pub fn feed(&mut self, data: &[u8]) {
        for &byte in data {
            match self.state {
                State::Ground => {
                    if byte == 0x1B {
                        self.flush_text();
                        self.state = State::Esc;
                    } else {
                        self.txt.push(byte);
                    }
                }
                State::Esc => {
                    if byte == b'[' {
                        self.state = State::Csi;
                        self.run.extend_from_slice(b"\x1b[");
                    } else if byte == b']' {
                        self.state = State::Osc;
                        self.run.extend_from_slice(b"\x1b]");
                    } else if byte == 0x50 || byte == 0x58 || byte == 0x5E || byte == 0x5F {
                        // DCS/SOS/PM/APC(图片协议)
                        self.state = State::Str;
                        self.str_esc = false;
                        self.run.push(0x1B);
                        self.run.push(byte);
                    } else {
                        self.state = State::Ground;
                        self.run.push(0x1B);
                        self.run.push(byte);
                        self.seq_done(); // 2 字符转义并入下一个配对
                    }
                }
                State::Csi => {
                    self.run.push(byte);
                    if (0x40..=0x7E).contains(&byte) {
                        self.state = State::Ground;
                        self.seq_done();
                    }
                }
                State::Osc => {
                    self.run.push(byte);
                    if byte == 0x07 {
                        self.state = State::Ground;
                        self.osc133_check();
                        self.seq_done();
                    } else if byte == 0x1B {
                        self.state = State::OscEsc;
                    }
                }
                State::OscEsc => {
                    self.run.push(byte);
                    if byte == b'\\' {
                        self.state = State::Ground;
                        self.osc133_check();
                        self.seq_done();
                    } else {
                        self.state = State::Osc;
                    }
                }
                State::Str => {
                    self.run.push(byte);
                    if self.str_esc {
                        if byte == b'\\' {
                            // ST:图片载荷结束
                            self.state = State::Ground;
                            self.seq_done();
                        } else {
                            self.str_esc = byte == 0x1B;
                        }
                    } else if byte == 0x1B {
                        self.str_esc = true;
                    }
                }
            }
        }
        let _ = self.out.flush();
    }

    /// 原始字节直通(图片载荷用):先落盘挂起的文本保证顺序,再原样写入并刷盘。
    /// 不经过染色,图片协议不被破坏。
    pub fn feed_raw(&mut self, data: &[u8]) {
        self.flush_text();
        self.write_raw(data);
        let _ = self.out.flush();
    }

    pub fn finish(&mut self, stdout_is_tty: bool) {        self.flush_text();
        if !self.animate && self.state != State::Ground && !self.run.is_empty() {
            self.run.clear(); // 末尾残缺的转义序列:按 lolcat 行为丢弃
        }
        if !self.saw_nl {
            // 末行无换行符:补空尾配对 + os 推进
            self.pair(None);
            if self.animate {
                self.animate_line();
            }
            self.os += self.i as f64 / self.spread;
        }
        if stdout_is_tty {
            self.write_raw(b"\x1b[m\x1b[?25h\x1b[?1;5;2004l");
        }
        let _ = self.out.flush();
    }
}

/// 剥掉清行/清屏类 CSI(animate 重绘帧用,对应 lol.rb 的 gsub(/\e\[[0-?]*[@JKPX]/))。
fn strip_animate_csi(run: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(run.len());
    let mut i = 0;
    while i < run.len() {
        if run[i] == 0x1B && i + 1 < run.len() && run[i + 1] == b'[' {
            let mut j = i + 2;
            while j < run.len() && (0x30..=0x3F).contains(&run[j]) {
                j += 1;
            }
            if j < run.len() {
                let f = run[j];
                if f == b'@' || f == b'J' || f == b'K' || f == b'P' || f == b'X' {
                    i = j + 1; // 整个序列丢掉
                    continue;
                }
            }
        }
        out.push(run[i]);
        i += 1;
    }
    out
}
