//! 彩虹引擎:与 lolcat 100.0.1(lib/lolcat/lol.rb)逐字节一致的移植。
//!   rainbow(freq, i) 的 sin 三相移公式(截断取整),
//!   Paint gem 的 rgb_to_256 灰度/立方体量化,
//!   配对模型(转义连跑+单字符,逐配对 \e[39m 复位)。

use std::collections::HashMap;
use std::f64::consts::PI;

/// 与 lol.rb#rainbow 完全一致:sin 三相移,截断取整。
/// 值域恒为 [1,255],Rust float->int 截断语义与 Python int() 相同。
#[inline]
pub fn rainbow(freq: f64, i: f64) -> (u8, u8, u8) {
    let r = (freq.mul_add(i, 0.0).sin() * 127.0 + 128.0) as u8;
    let g = (freq.mul_add(i, 2.0 * PI / 3.0).sin() * 127.0 + 128.0) as u8;
    let b = (freq.mul_add(i, 4.0 * PI / 3.0).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}

/// Paint gem rgb_to_256 的 1:1 移植。输入为 0..=255 的 f64,与 Python 侧 int 运算同序。
pub fn rgb_to_256(r: f64, g: f64, b: f64) -> u8 {
    let mut gray_possible = true;
    let mut sep = 42.5f64;
    let mut gray = true;
    while gray_possible {
        if r < sep || g < sep || b < sep {
            gray = r < sep && g < sep && b < sep;
            gray_possible = false;
        }
        sep += 42.5;
    }
    if gray {
        (232.0 + ((r + g + b) / 33.0 + 0.5).floor()) as u8
    } else {
        (16 + (6.0 * (r / 256.0)) as u32 * 36
            + (6.0 * (g / 256.0)) as u32 * 6
            + (6.0 * (b / 256.0)) as u32) as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    True,
    C256,
}

impl ColorMode {
    /// 与 lol.rb#set_mode 一致:-t 强制真彩,否则看 COLORTERM,再否则 256 色。
    pub fn detect(force_truecolor: bool) -> ColorMode {
        if force_truecolor {
            return ColorMode::True;
        }
        match std::env::var("COLORTERM") {
            Ok(v) => {
                let t = v.trim().to_lowercase();
                if t == "truecolor" || t == "24bit" {
                    ColorMode::True
                } else {
                    ColorMode::C256
                }
            }
            Err(_) => ColorMode::C256,
        }
    }
}

/// 按颜色缓存 SGR 序列,复位序列为静态常量。
pub struct Painter {
    mode: ColorMode,
    invert: bool,
    cache: HashMap<(u8, u8, u8), Vec<u8>>,
}

impl Painter {
    pub fn new(mode: ColorMode, invert: bool) -> Painter {
        Painter {
            mode,
            invert,
            cache: HashMap::new(),
        }
    }

    /// 返回 (上色 SGR, 复位 SGR)。字节内容与 Python 版完全相同。
    pub fn sgr(&mut self, r: u8, g: u8, b: u8) -> (&[u8], &'static [u8]) {
        if !self.cache.contains_key(&(r, g, b)) {
            let code = match (self.mode, self.invert) {
                (ColorMode::True, false) => format!("\x1b[38;2;{r};{g};{b}m"),
                (ColorMode::True, true) => format!("\x1b[48;2;{r};{g};{b}m"),
                (ColorMode::C256, false) => {
                    format!("\x1b[38;5;{}m", rgb_to_256(r as f64, g as f64, b as f64))
                }
                (ColorMode::C256, true) => {
                    format!("\x1b[48;5;{}m", rgb_to_256(r as f64, g as f64, b as f64))
                }
            };
            self.cache.insert((r, g, b), code.into_bytes());
        }
        let reset: &'static [u8] = if self.invert { b"\x1b[49m" } else { b"\x1b[39m" };
        (&self.cache[&(r, g, b)], reset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rainbow_matches_lolcat_vectors() {
        // 由 Python 参考实现生成的对照向量(freq=0.1)。
        let cases: &[(f64, (u8, u8, u8))] = &[
            (0.0, (128, 237, 18)),
            (1.0, (140, 231, 12)),
            (2.3333333333333335, (157, 220, 6)),
            (10.0, (234, 133, 15)),
            (100.0, (58, 70, 254)),
            (3.0, (165, 214, 4)),
            (7.0, (209, 171, 2)),
        ];
        for (i, want) in cases {
            assert_eq!(rainbow(0.1, *i), *want, "i={i}");
        }
    }

    #[test]
    fn rgb_to_256_vectors() {
        // Paint gem 行为:纯灰进 232..255,彩色进 16..231。
        assert_eq!(rgb_to_256(0.0, 0.0, 0.0), 232);
        assert_eq!(rgb_to_256(255.0, 255.0, 255.0), 255);
        assert_eq!(rgb_to_256(255.0, 0.0, 0.0), 196);
        assert_eq!(rgb_to_256(0.0, 255.0, 0.0), 46);
        assert_eq!(rgb_to_256(0.0, 0.0, 255.0), 21);
        assert_eq!(rgb_to_256(128.0, 237.0, 18.0), 154);
        assert_eq!(rgb_to_256(255.0, 182.0, 142.0), 223);
        assert_eq!(rgb_to_256(219.0, 193.0, 181.0), 224);
        assert_eq!(rgb_to_256(11.0, 142.0, 230.0), 39);
    }
}
