//! Active downloads own their immutable request/profile; the editor is only a draft.
use crate::{app::Downloader, messages::Update};
use gpui_kit::*;
use youtube_engine::*;

pub(crate) struct ActiveDownload {
    pub title: String,
    pub status: String,
    pub cancel: CancelToken,
    pub record: crate::history::HistoryEntry,
}
impl Downloader {
    pub(crate) fn launch_download(
        &mut self,
        request: DownloadRequest,
        profile: DownloadProfile,
        cx: &mut Context<Self>,
    ) {
        let id = request.task_id.clone();
        let title = self
            .media
            .as_ref()
            .map(|m| m.title.clone())
            .unwrap_or_else(|| request.url.clone());
        let record = crate::history::HistoryEntry {
            id: id.clone(),
            title: title.clone(),
            url: request.url.clone(),
            started_at: now(),
            finished_at: None,
            status: crate::history::HistoryStatus::Running,
            artifacts: vec![],
            metadata: crate::history::HistoryMetadata {
                source_site: self
                    .media
                    .as_ref()
                    .and_then(|m| m.extractor.clone())
                    .unwrap_or("youtube".into()),
                media_id: self
                    .media
                    .as_ref()
                    .map(|m| m.id.clone())
                    .unwrap_or_default(),
                channel: self
                    .media
                    .as_ref()
                    .and_then(|m| m.channel.clone().or(m.uploader.clone())),
                duration: self.media.as_ref().and_then(|m| m.duration),
                original_url: self.url.read(cx).value().to_string(),
                requested_height: (!profile.best_resolution).then_some(profile.max_height),
                best_resolution: profile.best_resolution,
                requested_format: format!("{:?}/{:?}", request.mode, profile.container),
                output_dir: Some(request.output_dir.clone()),
                ..Default::default()
            },
        };
        if let Err(error) = self.history.upsert(record.clone()) {
            self.status = format!("历史保存失败：{error}");
        }
        let inspector = self.engine.clone();
        let job = self.engine.spawn_download_with_profile(request, profile);
        self.running.insert(
            id.clone(),
            ActiveDownload {
                title,
                record,
                status: "准备下载…".into(),
                cancel: job.cancel.clone(),
            },
        );
        let (tx, rx) = async_channel::bounded(16);
        std::thread::spawn(move || {
            while let Ok(event) = job.events.recv() {
                if tx.send_blocking(Update::Event(event)).is_err() {
                    job.cancel.cancel();
                }
            }
            let result = job.join();
            let inspection = result
                .as_ref()
                .ok()
                .and_then(|r| r.artifacts.first())
                .and_then(|a| inspector.inspect_media(&a.path, &CancelToken::new()).ok());
            let _ = tx.send_blocking(Update::Finished(id, result, inspection));
        });
        Self::listen(rx, cx);
    }
}
pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl Drop for ActiveDownload {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
