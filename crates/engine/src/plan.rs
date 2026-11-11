//! One argv plan for execution and redacted previews; never execute display text.
use crate::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone)]
pub struct CommandPlan {
    pub executable: PathBuf,
    pub argv: Vec<String>,
}
impl std::fmt::Debug for CommandPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display_redacted())
    }
}
impl CommandPlan {
    pub fn redacted_argv(&self) -> Vec<String> {
        let mut hide = false;
        self.argv
            .iter()
            .map(|arg| {
                let value = if hide {
                    "[REDACTED]".into()
                } else {
                    arg.clone()
                };
                hide = matches!(arg.as_str(), "--proxy" | "--cookies-from-browser");
                value
            })
            .collect()
    }
    pub fn display_redacted(&self) -> String {
        std::iter::once(self.executable.to_string_lossy().into_owned())
            .chain(self.redacted_argv())
            .map(|s| quote(&s))
            .collect::<Vec<_>>()
            .join(" ")
    }
    pub(crate) fn command(&self) -> Command {
        let mut cmd = Command::new(&self.executable);
        cmd.env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            cmd.env("PATH", path);
        }
        cmd.env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .args(&self.argv);
        cmd
    }
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
fn invalid(message: &str) -> EngineError {
    EngineError::new(ErrorKind::InvalidRequest, message)
}

impl Engine {
    fn baseline_plan(&self, profile: &DownloadProfile) -> Result<CommandPlan> {
        if !(144..=4320).contains(&profile.max_height) {
            return Err(invalid("resolution must be 144..4320"));
        }
        let mut argv = vec![
            "--ignore-config",
            "--no-playlist",
            "--no-plugin-dirs",
            "--no-remote-components",
            "--no-js-runtimes",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        if let Ok(deno) = resolve_tool(Path::new("deno")) {
            argv.extend(["--js-runtimes".into(), format!("deno:{}", deno.display())]);
        }
        let proxy = profile.proxy.as_deref().unwrap_or("");
        if !proxy.is_empty() {
            let url = url::Url::parse(proxy).map_err(|_| invalid("invalid proxy URL"))?;
            if !matches!(
                url.scheme(),
                "http" | "https" | "socks4" | "socks5" | "socks5h"
            ) || url.host_str().is_none()
                || proxy.chars().any(char::is_control)
            {
                return Err(invalid("unsupported proxy URL"));
            }
        }
        argv.extend(["--proxy".into(), proxy.into()]);
        if let Some(browser) = &profile.cookies_browser {
            argv.extend(["--cookies-from-browser".into(), browser.argument().into()]);
        }
        validate_extra(&profile.extra_args)?;
        Ok(CommandPlan {
            executable: resolve_tool(&self.tools.yt_dlp)?,
            argv,
        })
    }
    pub fn probe_plan(&self, url: &str, profile: &DownloadProfile) -> Result<CommandPlan> {
        let mut plan = self.baseline_plan(profile)?;
        // Download-only extras must not perturb the metadata protocol.
        plan.argv.extend([
            "--skip-download".into(),
            "--dump-single-json".into(),
            "--".into(),
            normalize_url(url)?,
        ]);
        Ok(plan)
    }
    pub fn download_plan(
        &self,
        request: &DownloadRequest,
        profile: &DownloadProfile,
    ) -> Result<CommandPlan> {
        let mut plan = self.baseline_plan(profile)?;
        let attempt = planned_attempt(request, profile)?;
        let selector = video_selector(request, profile)?;
        plan.argv.extend(
            [
                "--newline",
                "--progress",
                "--no-simulate",
                "--no-overwrites",
                "--restrict-filenames",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        plan.argv.extend([
            "--paths".into(),
            attempt.to_string_lossy().into_owned(),
            "--output".into(),
            "%(id)s.%(ext)s".into(),
            "--progress-template".into(),
            format!("download:{PROGRESS_PREFIX}%(progress)j"),
            "--progress-template".into(),
            format!("postprocess:{POSTPROCESS_PREFIX}%(progress)j"),
            "--print".into(),
            format!("after_move:{ARTIFACT_PREFIX}%(filepath)j"),
            "--format".into(),
            selector,
        ]);
        match request.mode {
            Mode::Video => plan.argv.extend([
                "--merge-output-format".into(),
                match profile.container {
                    VideoContainer::Mp4 => "mp4",
                    VideoContainer::Mkv => "mkv",
                }
                .into(),
            ]),
            Mode::Audio => plan.argv.push("--extract-audio".into()),
            Mode::MP3 => plan.argv.extend([
                "--extract-audio".into(),
                "--audio-format".into(),
                "mp3".into(),
            ]),
        }
        if let Some(ffmpeg) = &self.tools.ffmpeg {
            plan.argv.extend([
                "--ffmpeg-location".into(),
                resolve_tool(ffmpeg)?.to_string_lossy().into_owned(),
            ]);
        }
        plan.argv.extend(profile.extra_args.clone());
        plan.argv
            .extend(["--".into(), normalize_url(&request.url)?]);
        Ok(plan)
    }
}
pub(crate) fn planned_attempt(
    request: &DownloadRequest,
    profile: &DownloadProfile,
) -> Result<PathBuf> {
    if request.attempt_id.is_empty()
        || !request
            .attempt_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err(invalid("invalid attempt_id"));
    }
    let root = profile.output_dir.as_ref().unwrap_or(&request.output_dir);
    let root = if root.is_absolute() {
        root.clone()
    } else {
        std::env::current_dir()
            .map_err(|_| invalid("working directory unavailable"))?
            .join(root)
    };
    if root.to_str().is_none() {
        return Err(invalid("output path must be UTF-8"));
    }
    Ok(root.join(format!("youtube-{}", request.attempt_id)))
}
fn video_selector(request: &DownloadRequest, profile: &DownloadProfile) -> Result<String> {
    if let Some(id) = &request.format_id {
        if id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'+'))
        {
            return Err(invalid("invalid format selector"));
        }
        return Ok(id.clone());
    }
    if request.mode != Mode::Video {
        return Ok("ba/b".into());
    }
    let h = profile.max_height;
    Ok(match profile.container {
        VideoContainer::Mp4=>format!("bv[ext=mp4][height<={h}][vcodec^=avc1]+ba[ext=m4a][acodec^=mp4a]/b[ext=mp4][height<={h}][vcodec^=avc1][acodec^=mp4a]"),
        VideoContainer::Mkv=>format!("bv[height<={h}]+ba/b[height<={h}]"),
    })
}
/// Exact allowlist: no shell hooks, credentials, config, paths, templates,
/// runtime/plugin loading or protocol overrides can enter via advanced options.
fn validate_extra(args: &[String]) -> Result<()> {
    if args.len() > 32 {
        return Err(invalid("too many extra arguments"));
    }
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        if matches!(
            flag,
            "--write-subs"
                | "--write-auto-subs"
                | "--embed-subs"
                | "--no-write-subs"
                | "--no-write-auto-subs"
        ) {
            i += 1;
            continue;
        }
        let value = args
            .get(i + 1)
            .ok_or_else(|| invalid("extra option needs a value"))?;
        let valid = match flag {
            "--retries" | "--fragment-retries" | "--concurrent-fragments" => {
                value.parse::<u16>().is_ok_and(|n| n > 0 && n <= 100)
            }
            "--limit-rate" => {
                !value.is_empty()
                    && value.len() <= 16
                    && value.bytes().all(|b| {
                        b.is_ascii_digit()
                            || matches!(b, b'.' | b'K' | b'M' | b'G' | b'k' | b'm' | b'g')
                    })
                    && value.bytes().any(|b| b.is_ascii_digit())
            }
            "--sub-langs" => {
                !value.is_empty()
                    && value.len() <= 128
                    && value.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b',' | b'-' | b'_' | b'.')
                    })
            }
            "--sub-format" => matches!(value.as_str(), "srt" | "vtt" | "ass" | "best"),
            _ => false,
        };
        if !valid {
            return Err(invalid("unknown or invalid extra option"));
        }
        i += 2;
    }
    Ok(())
}
