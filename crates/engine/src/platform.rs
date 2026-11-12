//! Small platform policy helpers. Keep OS-specific paths and asset names out of domain code.
use std::path::PathBuf;

pub fn app_data_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var_os("APPDATA")
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "APPDATA unavailable".to_owned())?;
        return Ok(PathBuf::from(root).join("Shiguang"));
    }
    #[cfg(target_os = "macos")]
    {
        let root = std::env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "HOME unavailable".to_owned())?;
        return Ok(PathBuf::from(root).join("Library/Application Support/Shiguang"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let root = std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| PathBuf::from(h).join(".local/share").into_os_string())
            })
            .ok_or_else(|| "XDG_DATA_HOME and HOME unavailable".to_owned())?;
        return Ok(PathBuf::from(root).join("Shiguang"));
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        Err("unsupported platform data directory".to_owned())
    }
}

pub(crate) fn yt_dlp_asset() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "yt-dlp.exe"
    }
    #[cfg(target_os = "macos")]
    {
        "yt-dlp_macos"
    }
    #[cfg(target_os = "linux")]
    {
        "yt-dlp_linux"
    }
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
    {
        "yt-dlp_linux"
    }
    #[cfg(not(unix))]
    {
        "yt-dlp.exe"
    }
}
