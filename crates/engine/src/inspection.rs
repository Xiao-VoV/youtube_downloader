//! Optional ffprobe facts. Unknown streams/fields stay None, never inferred.
use crate::*;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command, time::Duration};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MediaInspection {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub ext: Option<String>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub duration: Option<f64>,
}
impl Engine {
    pub fn inspect_media(&self, path: &Path, cancel: &CancelToken) -> Result<MediaInspection> {
        let tool = self
            .tools
            .ffprobe
            .as_ref()
            .ok_or_else(|| EngineError::new(ErrorKind::ToolNotFound, "ffprobe unavailable"))?;
        let path = std::fs::canonicalize(path)
            .map_err(|_| EngineError::new(ErrorKind::Io, "media file unavailable"))?;
        if !path.is_file() {
            return Err(EngineError::new(
                ErrorKind::InvalidRequest,
                "media must be a regular file",
            ));
        }
        let mut command = Command::new(resolve_tool(tool)?);
        command.env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }
        command
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type,codec_name,width,height:format=duration",
                "-of",
                "json",
            ])
            .arg(&path);
        let mut config = self.config.clone();
        config.max_stdout_bytes = 1024 * 1024;
        let output = crate::process::run(
            command,
            &config,
            Duration::from_secs(5),
            cancel,
            true,
            |_| Ok(()),
        )?;
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| EngineError::new(ErrorKind::Protocol, "invalid ffprobe JSON"))?;
        if !value.is_object() {
            return Err(EngineError::new(
                ErrorKind::Protocol,
                "ffprobe result must be an object",
            ));
        }
        let streams = value.get("streams").and_then(|v| v.as_array());
        let video = streams.and_then(|s| {
            s.iter()
                .find(|v| v.get("codec_type").and_then(|v| v.as_str()) == Some("video"))
        });
        let audio = streams.and_then(|s| {
            s.iter()
                .find(|v| v.get("codec_type").and_then(|v| v.as_str()) == Some("audio"))
        });
        let integer = |name: &str| {
            video
                .and_then(|v| v.get(name))
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok())
                .filter(|v| *v > 0)
        };
        let codec = |stream: Option<&serde_json::Value>| {
            stream
                .and_then(|v| v.get("codec_name"))
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
        };
        let duration = value
            .get("format")
            .and_then(|v| v.get("duration"))
            .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))
            .filter(|v| v.is_finite() && *v >= 0.0);
        // This is the actual filename suffix, not ffprobe format_name (which can be
        // a comma-separated demuxer family and is not an extension).
        let ext = path.extension().and_then(|v| v.to_str()).map(str::to_owned);
        Ok(MediaInspection {
            width: integer("width"),
            height: integer("height"),
            ext,
            vcodec: codec(video),
            acodec: codec(audio),
            duration,
        })
    }
}
