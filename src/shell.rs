//! `rscat --init SHELL` 输出的 shell 集成片段。
//! 内容只有一件事:把 ~/.local/bin(即 rscat 安装位置)加入 PATH,
//! 幂等,可反复 source。rscat 本体不依赖任何包装函数/别名。
//! 片段正文在 shells/ 目录(include_str! 引入,保证 --init 输出与文件完全一致)。

/// 返回对应 shell 的片段,未知返回 None。接受的大小写/别名已归一化。
pub fn snippet(shell: &str) -> Option<&'static str> {
    match shell.trim().to_lowercase().as_str() {
        "bash" => Some(include_str!("../shells/rscat.bash")),
        "zsh" => Some(include_str!("../shells/rscat.zsh")),
        "sh" | "dash" | "ksh" => Some(include_str!("../shells/rscat.sh")),
        "fish" => Some(include_str!("../shells/rscat.fish")),
        "powershell" | "pwsh" | "ps1" => Some(include_str!("../shells/rscat.ps1")),
        "cmd" | "batch" | "bat" => Some(include_str!("../shells/rscat.cmd")),
        _ => None,
    }
}
