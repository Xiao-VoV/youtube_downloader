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
        let check = |path: &Path, arg: &str, yt_dlp: bool| {
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
                    cmd.env("PATH", subprocess_path());
                    if yt_dlp {
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
        let deno = self
            .tools
            .deno
            .as_ref()
            .map(|p| check(p, "--version", false));
        let runtime_note = match &deno {
            Some(health) if health.healthy => {
                format!("Selected Deno runtime enabled: {}", health.path.display())
            }
            Some(_) => {
                "Selected Deno runtime is unavailable; runtime-dependent plans will fail.".into()
            }
            None => "Deno runtime disabled; JavaScript challenges may be unsupported.".into(),
        };
        HealthReport {
            yt_dlp: check(&self.tools.yt_dlp, "--version", true),
            ffmpeg: self.tools.ffmpeg.as_ref().map(|p| check(p, "-version", false)),
            ffprobe: self.tools.ffprobe.as_ref().map(|p| check(p, "-version", false)),
            deno,
            notes: vec![runtime_note, "Health checks do not prove site availability. Plugins and remote EJS components are disabled. No components are downloaded automatically.".into()],
        }
    }
}
/// Finder does not inherit a terminal's PATH. Append conventional macOS tool
/// locations without running shell startup scripts or mutating process globals.
/// Explicit selections and inherited PATH entries retain priority.
fn search_directories(path: Option<OsString>, home: Option<PathBuf>, macos: bool) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = path
        .as_deref()
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .filter(|p| p.is_absolute())
        .collect();
    if macos {
        let mut fallbacks = vec![
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ];
        if let Some(home) = home.filter(|p| p.is_absolute()) {
            fallbacks.extend([home.join(".local/bin"), home.join(".deno/bin")]);
        }
        fallbacks.extend(["/usr/bin", "/bin", "/usr/sbin", "/sbin"].map(PathBuf::from));
        for dir in fallbacks {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    dirs
}
pub(crate) fn subprocess_path() -> OsString {
    let dirs = search_directories(
        std::env::var_os("PATH"),
        std::env::var_os("HOME").map(PathBuf::from),
        cfg!(target_os = "macos"),
    );
    std::env::join_paths(dirs).unwrap_or_default()
}

/// Explicit paths or executable names in the effective search path, resolved to absolute paths.
pub fn resolve_tool(path: &Path) -> Result<PathBuf> {
    let candidates = if path.components().count() > 1 || path.is_absolute() {
        vec![path.to_owned()]
    } else {
        std::env::split_paths(&subprocess_path())
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
        format!(
            "selected executable not found or not executable: {}",
            path.display()
        ),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finder_path_includes_homebrew_and_user_tools() {
        let dirs = search_directories(
            Some("/usr/bin:/bin:/usr/sbin:/sbin".into()),
            Some("/Users/test".into()),
            true,
        );
        assert_eq!(dirs[0], Path::new("/usr/bin"));
        for dir in [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/Users/test/.local/bin",
            "/Users/test/.deno/bin",
        ] {
            assert!(dirs.contains(&PathBuf::from(dir)), "missing {dir}");
        }
    }

    #[test]
    fn terminal_path_priority_and_dedup_are_preserved() {
        let dirs = search_directories(
            Some("/custom/bin:/opt/homebrew/bin:/usr/bin".into()),
            None,
            true,
        );
        assert_eq!(dirs[0], Path::new("/custom/bin"));
        assert_eq!(
            dirs.iter()
                .filter(|p| **p == Path::new("/opt/homebrew/bin"))
                .count(),
            1
        );
    }

    #[test]
    fn missing_path_is_safe_and_relative_entries_are_not_searched() {
        assert!(search_directories(None, None, true).contains(&PathBuf::from("/opt/homebrew/bin")));
        assert_eq!(
            search_directories(Some(":.:relative:/usr/bin".into()), None, false),
            vec![PathBuf::from("/usr/bin")]
        );
    }

    #[test]
    fn child_process_uses_the_same_search_path() {
        let plan = crate::CommandPlan {
            executable: "/unused".into(),
            argv: vec![],
        };
        let command = plan.command();
        assert!(
            command
                .get_envs()
                .any(|(key, value)| key == "PATH" && value == Some(subprocess_path().as_os_str()))
        );
    }
}

impl ToolPaths {
    pub fn discover() -> Result<Self> {
        Ok(Self {
            yt_dlp: resolve_tool(Path::new("yt-dlp"))?,
            ffmpeg: resolve_tool(Path::new("ffmpeg")).ok(),
            ffprobe: resolve_tool(Path::new("ffprobe")).ok(),
            deno: resolve_tool(Path::new("deno")).ok(),
        })
    }
}
