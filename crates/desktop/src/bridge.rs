//! Route by task identity; a completed download never mutates a different draft.
use crate::i18n::tr;
use crate::{app::Downloader, history::HistoryStatus, messages::Update};
use gpui_kit::*;
use youtube_engine::*;
impl Downloader {
    pub(crate) fn listen(rx: async_channel::Receiver<Update>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            while let Ok(update) = rx.recv().await {
                if this
                    .update(cx, |this, cx| {
                        this.apply_update(update);
                        this.start_next_probe(cx);
                        this.start_queued(cx);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
    fn apply_update(&mut self, update: Update) {
        match update {
            Update::Health(text) => self.health = text,
            Update::Tools(checking, result) => {
                if checking {
                    self.tools.checking = false;
                } else {
                    self.tools.busy = false;
                }
                match result {
                    Ok(out) => {
                        if let Some(engine) = out.engine {
                            self.engine = engine;
                            self.health = out.status.clone();
                        }
                        if out.paths.is_some() {
                            self.tools.effective = out.paths;
                        }
                        if out.release.is_some() {
                            self.tools.release = out.release;
                        }
                        // Release checks do not own the installed-version state: a
                        // concurrent local switch/delete may already have refreshed it.
                        if !checking {
                            self.tools.versions = out.versions;
                        }
                        self.tools.status = out.status;
                    }
                    Err(error) => self.tools.status = error,
                }
            }
            Update::DraftPreview(id, result) => {
                if self
                    .probe_job
                    .as_ref()
                    .is_some_and(|(active, _)| active == &id)
                {
                    self.probe_job = None;
                }
                if self
                    .drafts
                    .entries
                    .iter()
                    .any(|d| d.id == id && d.state == crate::drafts::DraftState::Probing)
                {
                    if let Err(error) = self.drafts.apply_probe(&id, result) {
                        self.status = error;
                    }
                    if self.selected_draft.as_deref() == Some(id.as_str()) {
                        self.media = self
                            .drafts
                            .entries
                            .iter()
                            .find(|d| d.id == id)
                            .and_then(|d| d.media.clone());
                    }
                }
            }
            Update::Event(event) => {
                route_event(&mut self.running, event);
            }
            Update::Finished(id, result, inspection) => {
                if let Some(task) = self.running.remove(&id) {
                    let _ = self.drafts.remove(&id);
                    let mut record = finish_record(task.record.clone(), result);
                    if let Some(info) = inspection {
                        record.metadata.width = info.width;
                        record.metadata.height = info.height;
                        record.metadata.ext = info.ext;
                        record.metadata.vcodec = info.vcodec;
                        record.metadata.acodec = info.acodec;
                        if info.duration.is_some() {
                            record.metadata.duration = info.duration;
                        }
                    }
                    if let Err(e) = self.history.upsert(record.clone()) {
                        self.status = format!("{}: {e}", tr("历史保存失败"));
                        // Preserve the completed result in this session even if disk is unavailable.
                        self.history.entries.retain(|r| r.id != record.id);
                        self.history.entries.push(record);
                    }
                }
            }
        }
    }
}
fn route_event(
    tasks: &mut std::collections::BTreeMap<String, crate::jobs::ActiveDownload>,
    event: EngineEvent,
) {
    if let Some(task) = tasks.get_mut(&event.task_id) {
        if task.attempt_id != event.attempt_id {
            return;
        }
        task.status = match event.kind {
            EventKind::Starting => tr("正在启动…"),
            EventKind::PostProcessing => tr("正在合并或转换…"),
            EventKind::Progress(p) => progress_text(p),
            _ => return,
        };
    }
}
fn finish_record(
    mut record: crate::history::HistoryEntry,
    result: youtube_engine::Result<DownloadResult>,
) -> crate::history::HistoryEntry {
    record.finished_at = Some(crate::jobs::now());
    match result {
        Ok(output) => {
            record.status = HistoryStatus::Succeeded;
            record.artifacts = output.artifacts;
        }
        Err(e) => {
            record.status = if e.kind == ErrorKind::Cancelled {
                HistoryStatus::Cancelled
            } else {
                HistoryStatus::Failed
            }
        }
    }
    record
}
#[cfg(test)]
mod tests {
    use super::{finish_record, route_event};
    use crate::history::{HistoryEntry, HistoryStatus};
    use youtube_engine::{DownloadResult, EngineError, ErrorKind};
    #[test]
    fn concurrent_events_and_cancellation_are_isolated() {
        use crate::jobs::ActiveDownload;
        use youtube_engine::{CancelToken, EngineEvent, EventKind};
        let mut tasks = std::collections::BTreeMap::new();
        for id in ["one", "two"] {
            tasks.insert(
                id.to_owned(),
                ActiveDownload {
                    title: id.into(),
                    attempt_id: id.into(),
                    tool_path: Default::default(),
                    title_scroll: gpui_kit::ScrollHandle::new(),
                    status: "ready".into(),
                    cancel: CancelToken::new(),
                    record: HistoryEntry {
                        id: id.into(),
                        title: id.into(),
                        url: "https://youtu.be/abcdefghijk".into(),
                        started_at: 0,
                        finished_at: None,
                        status: HistoryStatus::Running,
                        artifacts: vec![],
                        metadata: Default::default(),
                    },
                },
            );
        }
        route_event(
            &mut tasks,
            EngineEvent {
                task_id: "one".into(),
                attempt_id: "one".into(),
                kind: EventKind::PostProcessing,
            },
        );
        assert_ne!(tasks["one"].status, "ready");
        assert_eq!(tasks["two"].status, "ready");
        tasks["one"].cancel.cancel();
        assert!(!tasks["two"].cancel.is_cancelled());
        let current = tasks["one"].status.clone();
        route_event(
            &mut tasks,
            EngineEvent {
                task_id: "one".into(),
                attempt_id: "old".into(),
                kind: EventKind::Starting,
            },
        );
        assert_eq!(tasks["one"].status, current);
        tasks.remove("one");
        route_event(
            &mut tasks,
            EngineEvent {
                task_id: "one".into(),
                attempt_id: "one".into(),
                kind: EventKind::Starting,
            },
        );
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks["two"].status, "ready");
    }
    #[test]
    fn completion_preserves_task_identity_and_isolated_records() {
        let entry = HistoryEntry {
            id: "first".into(),
            title: "视频一".into(),
            url: "https://youtu.be/abcdefghijk".into(),
            started_at: 1,
            finished_at: None,
            status: HistoryStatus::Running,
            artifacts: vec![],
            metadata: Default::default(),
        };
        let mut other = entry.clone();
        other.id = "second".into();
        let done = finish_record(entry.clone(), Ok(DownloadResult { artifacts: vec![] }));
        assert_eq!(done.id, "first");
        assert_eq!(done.title, "视频一");
        assert_eq!(done.status, HistoryStatus::Succeeded);
        assert!(done.finished_at.is_some());
        assert_eq!(other.id, "second");
        assert_eq!(other.status, HistoryStatus::Running);
        assert!(other.finished_at.is_none());
        assert_eq!(
            finish_record(
                entry.clone(),
                Err(EngineError::new(ErrorKind::Cancelled, "cancel"))
            )
            .status,
            HistoryStatus::Cancelled
        );
        assert_eq!(
            finish_record(
                entry,
                Err(EngineError::new(ErrorKind::ProcessFailed, "failure"))
            )
            .status,
            HistoryStatus::Failed
        );
    }
}
fn progress_text(p: Progress) -> String {
    match (
        p.downloaded_bytes,
        p.total_bytes.or(p.total_bytes_estimate).filter(|n| *n > 0),
    ) {
        (Some(done), Some(total)) => format!(
            "{} · {:.1}% · {:.1} MB",
            tr("下载中"),
            (done as f64 / total as f64 * 100.).min(100.),
            done as f64 / 1e6
        ),
        (Some(done), _) => format!("{} · {:.1} MB", tr("下载中"), done as f64 / 1e6),
        _ => tr("正在下载…"),
    }
}
