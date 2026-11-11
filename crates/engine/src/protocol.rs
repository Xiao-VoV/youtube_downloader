//! Bounded structured stdout protocol and UI event delivery.
use crate::*;
use std::path::PathBuf;
pub const PROGRESS_PREFIX: &str = "__YT_ENGINE_PROGRESS__";
pub const POSTPROCESS_PREFIX: &str = "__YT_ENGINE_POSTPROCESS__";
pub const ARTIFACT_PREFIX: &str = "__YT_ENGINE_ARTIFACT__";
/// Compatible MP4 only: H.264 video at <=1080p and AAC audio. The only
/// fallback is a matching already-muxed MP4, never another codec or resolution.
pub const DEFAULT_VIDEO_SELECTOR: &str = "bv[ext=mp4][height<=1080][vcodec^=avc1]+ba[ext=m4a][acodec^=mp4a]/b[ext=mp4][height<=1080][vcodec^=avc1][acodec^=mp4a]";

/// Convert yt-dlp's unavailable sentinels to None without inventing totals.
pub fn parse_progress(json: &str) -> Result<Progress> {
    let mut value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| EngineError::new(ErrorKind::Protocol, "invalid progress JSON"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| EngineError::new(ErrorKind::Protocol, "progress must be an object"))?;
    for (_, v) in object.iter_mut() {
        if v.as_str()
            .is_some_and(|s| matches!(s, "NA" | "N/A" | "None"))
        {
            *v = serde_json::Value::Null;
        }
    }
    serde_json::from_value(value)
        .map_err(|_| EngineError::new(ErrorKind::Protocol, "invalid progress fields"))
}
pub(crate) fn emit(
    request: &DownloadRequest,
    sender: Option<&EventSender>,
    kind: EventKind,
    terminal: bool,
) {
    if let Some(sender) = sender {
        let event = EngineEvent {
            task_id: request.task_id.clone(),
            attempt_id: request.attempt_id.clone(),
            kind,
        };
        if terminal {
            let _ = sender.send(event);
        } else {
            let _ = sender.try_send(event);
        }
    }
}

#[derive(Default)]
pub(crate) struct DownloadProtocol {
    pub paths: Vec<PathBuf>,
}
impl DownloadProtocol {
    pub fn line(
        &mut self,
        line: &[u8],
        request: &DownloadRequest,
        events: Option<&EventSender>,
    ) -> Result<()> {
        // Unknown human log lines may contain locale-dependent bytes.
        // Lossy decoding is bounded by max_line_bytes; structured lines
        // still require valid JSON and typed fields below.
        let text = String::from_utf8_lossy(line);
        let line = text.trim_end_matches('\r');
        if let Some(json) = line.strip_prefix(PROGRESS_PREFIX) {
            let progress = parse_progress(json)?;
            emit(request, events, EventKind::Progress(progress), false);
        } else if let Some(json) = line.strip_prefix(POSTPROCESS_PREFIX) {
            let _: serde_json::Value = serde_json::from_str(json).map_err(|_| {
                EngineError::new(ErrorKind::Protocol, "invalid postprocessing JSON")
            })?;
            emit(request, events, EventKind::PostProcessing, false);
        } else if let Some(json) = line.strip_prefix(ARTIFACT_PREFIX) {
            let path: String = serde_json::from_str(json).map_err(|_| {
                EngineError::new(ErrorKind::Protocol, "invalid final artifact JSON")
            })?;
            if self.paths.len() >= 32 {
                return Err(EngineError::new(
                    ErrorKind::OutputLimit,
                    "too many final artifacts",
                ));
            }
            self.paths.push(PathBuf::from(path));
        }

        Ok(())
    }
}
