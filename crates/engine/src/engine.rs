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
        self.probe_with_profile(url, &DownloadProfile::default(), cancel)
    }
    pub fn probe_with_profile(
        &self,
        url: &str,
        profile: &DownloadProfile,
        cancel: &CancelToken,
    ) -> Result<MediaInfo> {
        let cmd = self.probe_plan(url, profile)?.command();
        let output = process::run(
            cmd,
            &self.config,
            self.config.probe_timeout,
            cancel,
            true,
            |_| Ok(()),
        )?;
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            output.with_diagnostics(EngineError::new(
                ErrorKind::Protocol,
                format!("invalid metadata JSON: {error}"),
            ))
        })?;
        if value.get("entries").is_some()
            || value
                .get("_type")
                .and_then(|v| v.as_str())
                .is_some_and(|v| matches!(v, "playlist" | "multi_video"))
        {
            return Err(output.with_diagnostics(EngineError::new(
                ErrorKind::Protocol,
                "playlist metadata is not accepted",
            )));
        }
        let info: MediaInfo = serde_json::from_value(value).map_err(|error| {
            output.with_diagnostics(EngineError::new(
                ErrorKind::Protocol,
                format!("invalid media metadata fields: {error}"),
            ))
        })?;
        if info.id.is_empty() || info.title.is_empty() {
            return Err(output.with_diagnostics(EngineError::new(
                ErrorKind::Protocol,
                "missing video identity or title",
            )));
        }
        Ok(info)
    }
    pub fn download(
        &self,
        request: &DownloadRequest,
        cancel: &CancelToken,
        events: Option<&EventSender>,
    ) -> Result<DownloadResult> {
        self.download_with_profile(request, &DownloadProfile::default(), cancel, events)
    }
    pub fn download_with_profile(
        &self,
        request: &DownloadRequest,
        profile: &DownloadProfile,
        cancel: &CancelToken,
        events: Option<&EventSender>,
    ) -> Result<DownloadResult> {
        emit(request, events, EventKind::Starting, false);
        let result = self.download_inner(request, profile, cancel, events);
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
        profile: &DownloadProfile,
        cancel: &CancelToken,
        events: Option<&EventSender>,
    ) -> Result<DownloadResult> {
        let plan = self.download_plan(request, profile)?;
        let mut effective = request.clone();
        if let Some(root) = &profile.output_dir {
            effective.output_dir = root.clone();
        }
        let attempt = crate::artifacts::prepare_attempt(&effective)?;
        let cmd = plan.command();
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
        self.spawn_download_with_profile(request, DownloadProfile::default())
    }
    pub fn spawn_download_with_profile(
        &self,
        request: DownloadRequest,
        profile: DownloadProfile,
    ) -> EngineJob<DownloadResult> {
        let (tx, events) = mpsc::sync_channel(64);
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let engine = self.clone();
        let handle = thread::spawn(move || {
            engine.download_with_profile(&request, &profile, &worker_cancel, Some(&tx))
        });
        EngineJob {
            events,
            cancel,
            handle,
        }
    }
    /// Probe job uses empty task/attempt ids; its result is available via join.
    pub fn spawn_probe(&self, url: String) -> EngineJob<MediaInfo> {
        self.spawn_probe_with_profile(url, DownloadProfile::default())
    }
    pub fn spawn_probe_with_profile(
        &self,
        url: String,
        profile: DownloadProfile,
    ) -> EngineJob<MediaInfo> {
        let (tx, events) = mpsc::sync_channel(64);
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let engine = self.clone();
        let handle = thread::spawn(move || {
            let result = engine.probe_with_profile(&url, &profile, &worker_cancel);
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
