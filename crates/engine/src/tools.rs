//! Executable discovery, isolated command baseline, and offline health.
use crate::{process, *};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
impl Engine {
    pub fn health_check(&self, cancel: &CancelToken) -> HealthReport {
        let check = |path: &Path, arg: &str| {
            let resolved = resolve_tool(path);
            let mut health = ToolHealth {
                path: path.to_owned(),
                version: None,
                healthy: false,
                error: None,
            };
            match resolved {
                Err(e) => health.error = Some(e),
                Ok(path) => {
                    health.path = path.clone();
                    let mut cmd = Command::new(path);
                    cmd.env_clear();
                    if let Some(path) = std::env::var_os("PATH") {
                        cmd.env("PATH", path);
                    }
                    if arg == "--version" {
                        cmd.args([
                            "--ignore-config",
                            "--no-plugin-dirs",
                            "--no-remote-components",
                            "--no-js-runtimes",
                        ]);
                    }
                    cmd.arg(arg);
                    let mut config = self.config.clone();
                    config.max_stdout_bytes = 64 * 1024;
                    match process::run(cmd, &config, Duration::from_secs(5), cancel, true, |_| {
                        Ok(())
                    }) {
                        Ok(output) => {
                            let bytes = if output.stdout.is_empty() {
                                output.stderr
                            } else {
                                output.stdout
                            };
                            health.version = String::from_utf8_lossy(&bytes)
                                .lines()
                                .next()
                                .map(str::to_owned);
                            health.healthy = health.version.as_ref().is_some_and(|v| !v.is_empty());
                        }
                        Err(e) => health.error = Some(e),
                    }
                }
            }
            health
        };
        let runtime_note = match resolve_tool(Path::new("deno")) {
            Ok(path) => format!("Local Deno runtime enabled: {}", path.display()),
            Err(_) => {
                "No local Deno runtime found; JavaScript challenges may be unsupported.".into()
            }
        };
        HealthReport { yt_dlp: check(&self.tools.yt_dlp,"--version"), ffmpeg: self.tools.ffmpeg.as_ref().map(|p|check(p,"-version")), ffprobe: self.tools.ffprobe.as_ref().map(|p|check(p,"-version")), notes: vec![runtime_note, "Health checks do not prove site availability. Plugins and remote EJS components are disabled. No components are downloaded automatically.".into()] }
    }
}
/// Explicit paths or executable names in PATH, resolved to absolute paths.
pub fn resolve_tool(path: &Path) -> Result<PathBuf> {
    let candidates = if path.components().count() > 1 || path.is_absolute() {
        vec![path.to_owned()]
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_else(OsString::new))
            .map(|p| p.join(path))
            .collect()
    };
    for candidate in candidates {
        if let Ok(real) = fs::canonicalize(candidate) {
            if let Ok(meta) = fs::metadata(&real) {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
                        return Ok(real);
                    }
                }
                #[cfg(not(unix))]
                if meta.is_file() {
                    return Ok(real);
                }
            }
        }
    }
    Err(EngineError::new(
        ErrorKind::ToolNotFound,
        "selected executable not found or not executable",
    ))
}
impl ToolPaths {
    pub fn discover() -> Result<Self> {
        Ok(Self {
            yt_dlp: resolve_tool(Path::new("yt-dlp"))?,
            ffmpeg: resolve_tool(Path::new("ffmpeg")).ok(),
            ffprobe: resolve_tool(Path::new("ffprobe")).ok(),
        })
    }
}
