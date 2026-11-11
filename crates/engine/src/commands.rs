//! Download-only arguments. Selection never invokes an implicit transcoder.
use crate::*;
use std::{path::Path, process::Command};
impl Engine {
    pub(crate) fn download_command(
        &self,
        request: &DownloadRequest,
        attempt: &Path,
        url: &str,
    ) -> Result<Command> {
        let mut cmd = self.command()?;
        cmd.args([
            "--newline",
            "--progress",
            "--no-simulate",
            "--no-overwrites",
            "--restrict-filenames",
        ]);
        cmd.arg("--paths")
            .arg(&attempt)
            .arg("--output")
            .arg("%(id)s.%(ext)s");
        cmd.arg("--progress-template")
            .arg(format!("download:{PROGRESS_PREFIX}%(progress)j"));
        cmd.arg("--progress-template")
            .arg(format!("postprocess:{POSTPROCESS_PREFIX}%(progress)j"));
        cmd.arg("--print")
            .arg(format!("after_move:{ARTIFACT_PREFIX}%(filepath)j"));
        let selector = request.format_id.as_deref().unwrap_or(match request.mode {
            Mode::Video => DEFAULT_VIDEO_SELECTOR,
            Mode::Audio | Mode::MP3 => "ba/b",
        });
        cmd.args(["--format", selector]);
        if request.mode == Mode::Video {
            cmd.args(["--merge-output-format", "mp4"]);
        }
        if matches!(request.mode, Mode::Audio | Mode::MP3) {
            cmd.arg("--extract-audio");
        }
        if request.mode == Mode::MP3 {
            cmd.args(["--audio-format", "mp3"]);
        }
        if let Some(ffmpeg) = &self.tools.ffmpeg {
            cmd.arg("--ffmpeg-location").arg(resolve_tool(ffmpeg)?);
        }
        cmd.arg("--").arg(url);
        Ok(cmd)
    }
}
