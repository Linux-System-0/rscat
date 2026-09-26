//! 运行模式(Windows):ConPTY 需要 Win32 API 绑定,本体重写暂不内置。
//! filter 模式与 image 模式在 Windows 完整可用(字节流与 kitty/iTerm2 序列
//! 均可直写控制台);-e/-a 会返回明确的多语言错误而不是乱码。
//! 如需完整 PTY,请用 Windows Terminal + wezterm(--proto iterm)组合,
//! 或等待后续基于 ConPTY 的实现。

use std::path::Path;

use crate::filter::LolcatFilter;

pub fn run(_cmd: &[String], _flt: &mut LolcatFilter, _stop_file: Option<&Path>) -> i32 {
    127
}

/// 本平台是否支持运行模式。
pub fn supported() -> bool {
    false
}
