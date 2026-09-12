//! 会话持久化:彩虹会话标记文件。
//! 路径:Unix 上 $XDG_CONFIG_HOME/rscat/(默认 ~/.config/rscat/),
//! Windows 上 %APPDATA%/rscat/。
//! 每个会话一个独立标记文件 session-<rscat的pid>,多个会话互不干扰
//! (一个会话的清理不会误杀另一个);文件存在 = 会话开着,
//! `rscat -a` 创建,`rscat -c` 删除全部,会话主循环每轮检查
//! 自己的标记,消失即收尾退出。

use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("rscat");
            }
        }
        // 兜底:用户目录
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join("AppData").join("Roaming").join("rscat");
        }
        PathBuf::from("rscat")
    }
    #[cfg(not(windows))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("rscat");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config").join("rscat");
        }
        PathBuf::from(".config/rscat")
    }
}

/// 本会话的标记文件(session-<pid>)。
pub fn session_marker() -> PathBuf {
    config_dir().join(format!("session-{}", std::process::id()))
}

/// 是否存在任何彩虹会话(任意 session-* 标记)。
pub fn session_active() -> bool {
    session_markers().next().is_some()
}

/// 列出所有会话标记文件。
pub fn session_markers() -> impl Iterator<Item = PathBuf> {
    let dir = config_dir();
    (|| {
        let rd = std::fs::read_dir(dir).ok()?;
        Some(rd.flatten().map(|e| e.path()).filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("session-"))
                .unwrap_or(false)
        }))
    })()
    .into_iter()
    .flatten()
}

/// 创建本会话的标记(父目录一并建好)。失败返回 Err(错误信息由调用方 i18n)。
pub fn session_start() -> std::io::Result<()> {
    let m = session_marker();
    if let Some(dir) = m.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&m, std::process::id().to_string())?;
    Ok(())
}

/// 删除全部会话标记(`rscat -c`:取消所有会话)。
pub fn session_stop() {
    for m in session_markers() {
        let _ = std::fs::remove_file(m);
    }
}

/// 是否正运行在 rscat 彩虹会话的子 shell 里(防嵌套)。
pub fn inside_session() -> bool {
    std::env::var("RSCAT_SESSION").as_deref() == Ok("1")
}
