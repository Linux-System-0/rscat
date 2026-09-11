//! 会话持久化:彩虹会话标记文件。
//! 路径:Unix 上 $XDG_CONFIG_HOME/rscat/session(默认 ~/.config/rscat/session),
//! Windows 上 %APPDATA%/rscat/session。
//! 文件存在 = 会话开着;rscat -a 创建,rscat -c 删除,
//! 会话主循环每轮检查,文件消失即收尾退出。

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

pub fn session_marker() -> PathBuf {
    config_dir().join("session")
}

pub fn session_active() -> bool {
    session_marker().exists()
}

/// 创建标记(父目录一并建好)。失败返回 Err(错误信息由调用方 i18n)。
pub fn session_start() -> std::io::Result<()> {
    let m = session_marker();
    if let Some(dir) = m.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&m, b"")?;
    Ok(())
}

pub fn session_stop() {
    let _ = std::fs::remove_file(session_marker());
}

/// 是否正运行在 rscat 彩虹会话的子 shell 里(防嵌套)。
pub fn inside_session() -> bool {
    std::env::var("RSCAT_SESSION").as_deref() == Ok("1")
}
