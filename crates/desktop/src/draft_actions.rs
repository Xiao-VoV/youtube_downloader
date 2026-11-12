//! Independent probe scheduling. One probe slot; adding drafts never waits for it.
use crate::{
    app::Downloader,
    drafts::{Draft, DraftState},
    messages::Update,
    settings::Page,
};
use gpui_kit::*;
use youtube_engine::*;
impl Downloader {
    pub(crate) fn refresh_preset_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let names = self
            .settings
            .store
            .presets
            .iter()
            .map(|p| format!("{} · {}", p.name, p.id))
            .collect::<Vec<_>>();
        self.preset_select.update(cx, |s, cx| {
            s.set_items(
                gpui_kit::component::select::SearchableVec::new(names),
                window,
                cx,
            )
        });
    }
    pub(crate) fn add_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = match normalize_url(self.url.read(cx).value().as_ref()) {
            Ok(url) => url,
            Err(e) => {
                self.status = e.to_string();
                cx.notify();
                return;
            }
        };
        if self
            .drafts
            .entries
            .iter()
            .any(|d| d.url == url && !matches!(d.state, DraftState::Cancelled | DraftState::Failed))
        {
            self.status = "This video already has a draft".into();
            cx.notify();
            return;
        }
        let profile = self
            .settings
            .store
            .presets
            .iter()
            .find(|p| Some(&p.id) == self.settings.store.active_preset_id.as_ref())
            .map(|p| p.profile.clone())
            .unwrap_or_default();
        let id = format!(
            "draft-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let draft = Draft {
            id,
            url,
            profile,
            media: None,
            state: DraftState::Waiting,
            error: None,
        };
        if let Err(e) = self.drafts.upsert(draft) {
            self.status = e;
            cx.notify();
            return;
        }
        self.url.update(cx, |s, cx| s.set_value("", window, cx));
        self.start_next_probe(cx);
        cx.notify();
    }
    pub(crate) fn start_next_probe(&mut self, cx: &mut Context<Self>) {
        if self.probe_job.is_some() {
            return;
        }
        let Some(mut draft) = self
            .drafts
            .entries
            .iter()
            .find(|d| d.state == DraftState::Waiting)
            .cloned()
        else {
            return;
        };
        draft.state = DraftState::Probing;
        if let Err(e) = self.drafts.upsert(draft.clone()) {
            self.status = e;
            return;
        }
        let job = self
            .engine
            .spawn_probe_with_profile(draft.url, draft.profile);
        self.probe_job = Some((draft.id.clone(), job.cancel.clone()));
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            while job.events.recv().is_ok() {}
            let _ = tx.send_blocking(Update::DraftPreview(draft.id, job.join()));
        });
        Self::listen(rx, cx);
    }
    pub(crate) fn save_draft_editor(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.selected_draft.clone() else {
            return true;
        };
        let Some(mut draft) = self.drafts.entries.iter().find(|d| d.id == id).cloned() else {
            return false;
        };
        let mut profile = self.settings.snapshot(cx);
        profile.mode = self.mode;
        profile.output_dir = self.output.clone();
        let stale = crate::drafts::requires_reprobe(&draft.profile, &profile);
        draft.profile = profile;
        if stale {
            draft.media = None;
            draft.state = DraftState::Waiting;
            self.media = None;
        }
        if let Err(e) = self.drafts.upsert(draft) {
            self.status = e;
            return false;
        }
        if stale {
            self.status = "配置已改变，需要重新解析。".into();
            self.start_next_probe(cx);
            return false;
        }
        true
    }
    pub(crate) fn resume_queued(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(d) = self.drafts.entries.iter().find(|d| d.id == id).cloned() {
            if let Some(output_dir) = d.profile.output_dir.clone() {
                self.queued.push_back((
                    DownloadRequest {
                        url: d.url,
                        mode: d.profile.mode,
                        output_dir,
                        format_id: None,
                        task_id: d.id.clone(),
                        attempt_id: d.id,
                    },
                    d.profile,
                ));
                self.start_queued(cx);
                cx.notify();
            }
        }
    }
    pub(crate) fn start_queued(&mut self, cx: &mut Context<Self>) {
        while self.running.len() < 3 {
            let Some((request, profile)) = self.queued.pop_front() else {
                break;
            };
            self.launch_download(request, profile, cx);
        }
    }
    pub(crate) fn retry_draft(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(mut draft) = self.drafts.entries.iter().find(|d| d.id == id).cloned() {
            draft.state = DraftState::Waiting;
            draft.error = None;
            let _ = self.drafts.upsert(draft);
            self.start_next_probe(cx);
            cx.notify();
        }
    }
    pub(crate) fn remove_draft(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some((active, cancel)) = &self.probe_job {
            if active == id {
                cancel.cancel();
            }
        }
        if let Some(mut draft) = self.drafts.entries.iter().find(|d| d.id == id).cloned() {
            draft.state = DraftState::Cancelled;
            let _ = self.drafts.upsert(draft);
        }
        let _ = self.drafts.remove(id);
        cx.notify();
    }
    pub(crate) fn open_draft(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.drafts.entries.iter().find(|d| d.id == id).cloned() else {
            return;
        };
        self.selected_draft = Some(d.id);
        self.media = d.media;
        self.preview_url = d.url;
        self.mode = d.profile.mode;
        self.output = d.profile.output_dir.clone();
        self.settings.profile = d.profile.clone();
        self.settings.proxy.update(cx, |s, cx| {
            s.set_value(d.profile.proxy.unwrap_or_default(), window, cx)
        });
        self.settings.browser_profile.update(cx, |s, cx| {
            s.set_value(d.profile.browser_profile.unwrap_or_default(), window, cx)
        });
        self.settings.filename_template.update(cx, |s, cx| {
            s.set_value(d.profile.filename_template, window, cx)
        });
        self.settings.folder_template.update(cx, |s, cx| {
            s.set_value(d.profile.folder_template.unwrap_or_default(), window, cx)
        });
        self.settings.extra.update(cx, |s, cx| {
            s.set_value(d.profile.extra_args.join("\n"), window, cx)
        });
        self.page = Page::Detail;
        cx.notify();
    }
}
