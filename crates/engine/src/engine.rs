//! Blocking operations and background job ownership.
use crate::{process, protocol::emit, *};
use std::{sync::mpsc, thread};
#[derive(Clone, Debug)]
pub struct Engine {
    pub tools: ToolPaths,
    pub config: EngineConfig,
}
impl Engine {
    pub fn new(tools: ToolPaths, config: EngineConfig) -> Self {
        Self { tools, config }
    }
    pub fn probe(&self, url: &str, cancel: &CancelToken) -> Result<MediaInfo> {
        let url = normalize_url(url)?;
        let mut cmd = self.command()?;
        cmd.args(["--skip-download", "--dump-single-json", "--", &url]);
        let output = process::run(
            cmd,
            &self.config,
            self.config.probe_timeout,
            cancel,
            true,
            |_| Ok(()),
        )?;
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| EngineError::new(ErrorKind::Protocol, "invalid metadata JSON"))?;
        if value.get("entries").is_some()
            || value
                .get("_type")
                .and_then(|v| v.as_str())
                .is_some_and(|v| matches!(v, "playlist" | "multi_video"))
        {
            return Err(EngineError::new(
                ErrorKind::Protocol,
                "playlist metadata is not accepted",
            ));
        }
        let info: MediaInfo = serde_json::from_value(value)
            .map_err(|_| EngineError::new(ErrorKind::Protocol, "invalid media metadata fields"))?;
        if info.id.is_empty() || info.title.is_empty() {
            return Err(EngineError::new(
                ErrorKind::Protocol,
                "missing video identity or title",
            ));
        }
        Ok(info)
    }
    pub fn download(
        &self,
        request: &DownloadRequest,
        cancel: &CancelToken,
        events: Option<&EventSender>,
    ) -> Result<DownloadResult> {
        emit(request, events, EventKind::Starting, false);
        let result = self.download_inner(request, cancel, events);
        let kind = match &result {
            Ok(result) => EventKind::Completed(result.clone()),
            Err(e) if e.kind == ErrorKind::Cancelled => EventKind::Cancelled,
            Err(e) => EventKind::Failed(e.clone()),
        };
        emit(request, events, kind, true);
        result
    }
    fn download_inner(
        &self,
        request: &DownloadRequest,
        cancel: &CancelToken,
        events: Option<&EventSender>,
    ) -> Result<DownloadResult> {
        let url = normalize_url(&request.url)?;
        if request.format_id.as_ref().is_some_and(|s| {
            s.is_empty()
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'+'))
        }) {
            return Err(EngineError::new(
                ErrorKind::InvalidRequest,
                "invalid format selector",
            ));
        }
        let attempt = crate::artifacts::prepare_attempt(request)?;
        let cmd = self.download_command(request, &attempt, &url)?;
        let mut protocol = crate::protocol::DownloadProtocol::default();
        process::run(
            cmd,
            &self.config,
            self.config.download_timeout,
            cancel,
            false,
            |line| protocol.line(line, request, events),
        )?;
        crate::artifacts::validate_artifacts(protocol.paths, &attempt)
    }
    pub fn spawn_download(&self, request: DownloadRequest) -> EngineJob<DownloadResult> {
        let (tx, events) = mpsc::sync_channel(64);
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let engine = self.clone();
        let handle = thread::spawn(move || engine.download(&request, &worker_cancel, Some(&tx)));
        EngineJob {
            events,
            cancel,
            handle,
        }
    }
    /// Probe job uses empty task/attempt ids; its result is available via join.
    pub fn spawn_probe(&self, url: String) -> EngineJob<MediaInfo> {
        let (tx, events) = mpsc::sync_channel(64);
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let engine = self.clone();
        let handle = thread::spawn(move || {
            let result = engine.probe(&url, &worker_cancel);
            drop(tx);
            result
        });
        EngineJob {
            events,
            cancel,
            handle,
        }
    }
}
