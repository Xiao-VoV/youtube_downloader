//! Window state and user actions. No blocking process calls on the UI thread.
use crate::i18n::tr;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use youtube_engine::*;

use crate::messages::{Update, error_text};
pub(crate) struct Downloader {
    pub(crate) page: crate::settings::Page,
    pub(crate) settings_tools_tab: bool,
    pub(crate) tool_paths_open: bool,
    pub(crate) appearance: crate::preferences::Appearance,
    pub(crate) tools: crate::tool_settings::ToolSettings,
    pub(crate) settings: crate::settings::Settings,
    pub(crate) pending: Option<DownloadRequest>,
    pub(crate) drafts: crate::drafts::DraftStore,
    pub(crate) selected_draft: Option<String>,
    pub(crate) probe_job: Option<(String, CancelToken)>,
    pub(crate) preset_select: Entity<
        gpui_kit::component::select::SelectState<
            gpui_kit::component::select::SearchableVec<crate::preset_option::PresetOption>,
        >,
    >,
    pub(crate) queued: std::collections::VecDeque<(DownloadRequest, DownloadProfile)>,
    pub(crate) shutting_down: bool,
    pub(crate) duplicate_confirmed: bool,
    pub(crate) running: std::collections::BTreeMap<String, crate::jobs::ActiveDownload>,
    pub(crate) history: crate::history::History,
    pub(crate) history_sort: crate::history_table::Sort,
    pub(crate) history_selected: std::collections::BTreeSet<String>,
    pub(crate) history_columns: Vec<crate::history_columns::Field>,
    pub(crate) history_columns_open: bool,
    pub(crate) history_delete: Option<String>,
    pub(crate) history_scroll: ScrollHandle,
    pub(crate) history_page: usize,
    pub(crate) history_page_size: usize,
    pub(crate) url: Entity<InputState>,
    pub(crate) media: Option<MediaInfo>,
    pub(crate) preview_url: String,
    pub(crate) mode: Mode,
    pub(crate) output: Option<PathBuf>,
    pub(crate) status: String,
    pub(crate) health: String,
    pub(crate) busy: bool,
    pub(crate) cancel: Option<CancelToken>,
    pub(crate) engine: Engine,
    pub(crate) _subscriptions: Vec<Subscription>,
}
impl Downloader {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        crate::i18n::set_language(crate::preferences::load_language());
        let appearance = crate::preferences::load();
        crate::preferences::apply(appearance, window, cx);
        let url = cx.new(|cx| InputState::new(window, cx).placeholder(tr("粘贴 YouTube 视频链接")));
        let sub = cx.subscribe(&url, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.media = None;
                cx.notify();
            }
        });
        let mut tools = crate::tool_settings::ToolSettings::new(window, cx);
        let discovered = ToolManager::default_root()
            .ok()
            .and_then(|root| ToolManager::new(root).resolve(&tools.paths(cx)).ok())
            .map(|r| r.paths);
        tools.effective = discovered.clone();
        let engine = Engine::new(
            discovered.unwrap_or_else(|| ToolPaths {
                yt_dlp: PathBuf::from("/nonexistent/yt-dlp"),
                ffmpeg: None,
                ffprobe: None,
                deno: None,
            }),
            EngineConfig::default(),
        );
        let (tx, rx) = async_channel::bounded(16);
        let checker = engine.clone();
        std::thread::spawn(move || {
            let report = checker.health_check(&CancelToken::new());
            let text = format!(
                "yt-dlp {} · FFmpeg {} · 本地工具，不自动安装",
                if report.yt_dlp.healthy {
                    report.yt_dlp.version.as_deref().unwrap_or("就绪")
                } else {
                    "不可用"
                },
                if report.ffmpeg.as_ref().is_some_and(|t| t.healthy) {
                    "就绪"
                } else {
                    "缺失"
                }
            );
            let _ = tx.send_blocking(Update::Health(text));
        });
        Self::listen(rx, cx);
        let settings = crate::settings::Settings::new(window, cx);
        let output = settings.profile.output_dir.clone();
        let mode = settings.profile.mode;
        let appearance_sub = cx.observe_window_appearance(window, |this, window, cx| {
            if this.appearance == crate::preferences::Appearance::System {
                crate::preferences::apply(this.appearance, window, cx);
            }
        });
        use gpui_kit::component::select::{SearchableVec, SelectEvent, SelectState};
        let names = settings
            .store
            .presets
            .iter()
            .map(|p| crate::preset_option::PresetOption {
                id: p.id.clone(),
                name: p.name.clone(),
            })
            .collect::<Vec<_>>();
        let preset_select = cx.new(|cx| {
            SelectState::new(SearchableVec::new(names), None, window, cx).searchable(true)
        });
        let preset_sub = cx.subscribe_in(
            &preset_select,
            window,
            |this,
             _,
             event: &SelectEvent<SearchableVec<crate::preset_option::PresetOption>>,
             window,
             cx| {
                if let SelectEvent::Confirm(Some(name)) = event {
                    if let Some(p) = this
                        .settings
                        .store
                        .presets
                        .iter()
                        .find(|p| p.id == *name)
                        .cloned()
                    {
                        this.settings.profile = p.profile.clone();
                        this.mode = p.profile.mode;
                        this.output = p.profile.output_dir.clone();
                        this.settings.store.active_preset_id = Some(p.id);
                    }
                    cx.notify();
                    let _ = window;
                }
            },
        );
        let extra_sub = cx.subscribe(&settings.extra, |_, _, _: &InputEvent, cx| cx.notify());
        Self {
            page: crate::settings::Page::New,
            appearance,
            settings_tools_tab: false,
            tool_paths_open: false,
            tools,
            settings,
            pending: None,
            drafts: crate::drafts::DraftStore::load(),
            selected_draft: None,
            probe_job: None,
            preset_select,
            queued: Default::default(),
            shutting_down: false,
            duplicate_confirmed: false,
            running: Default::default(),
            history: crate::history::History::load(),
            history_sort: Default::default(),
            history_selected: Default::default(),
            history_columns: crate::history_columns::load(),
            history_columns_open: false,
            history_delete: None,
            history_scroll: ScrollHandle::new(),
            history_page: 0,
            history_page_size: 50,
            url,
            media: None,
            preview_url: String::new(),
            mode,
            output,
            status: String::new(),
            health: tr("正在检查本地工具…"),
            busy: false,
            cancel: None,
            engine,
            _subscriptions: vec![sub, extra_sub, appearance_sub, preset_sub],
        }
    }
    pub(crate) fn download(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.media.is_none() {
            return;
        }
        let Some(output_dir) = self.output.clone() else {
            self.status = tr("请先选择保存目录。");
            cx.notify();
            return;
        };
        let id = self.selected_draft.clone().unwrap_or_else(|| {
            format!(
                "{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
        });
        let request = DownloadRequest {
            url: self.preview_url.clone(),
            mode: self.mode,
            output_dir,
            format_id: None,
            task_id: id.clone(),
            attempt_id: id,
        };
        match self
            .engine
            .download_plan(&request, &self.settings.snapshot(cx))
        {
            Ok(_) => {
                self.pending = Some(request);
                self.duplicate_confirmed = false;
                self.page = crate::settings::Page::Detail;
            }
            Err(error) => self.status = error_text(&error),
        }
        cx.notify();
    }
    pub(crate) fn execute_confirmed(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(request) = self.pending.as_ref() else {
            return;
        };
        if !self.duplicate_confirmed
            && self
                .history
                .entries
                .iter()
                .any(|r| crate::duplicates::same_video(&r.url, &request.url))
        {
            self.status = tr("历史已有此视频，请先确认重新下载。");
            cx.notify();
            return;
        }
        if let Some(id) = &self.selected_draft {
            if !self.drafts.entries.iter().any(|d| {
                &d.id == id && d.state == crate::drafts::DraftState::Ready && d.media.is_some()
            }) {
                self.status = tr("等待解析");
                cx.notify();
                return;
            }
        }
        let profile = self.settings.snapshot(cx);
        // Edits on the review page must pass the same planner before spawning.
        if let Err(error) = self.engine.download_plan(request, &profile) {
            self.status = error_text(&error);
            cx.notify();
            return;
        }
        if let Some(id) = self.selected_draft.clone() {
            if let Some(mut draft) = self.drafts.entries.iter().find(|d| d.id == id).cloned() {
                draft.profile = profile.clone();
                draft.profile.mode = request.mode;
                draft.profile.output_dir = Some(request.output_dir.clone());
                draft.state = crate::drafts::DraftState::Queued;
                if let Err(error) = self.drafts.upsert(draft) {
                    self.status = error;
                    cx.notify();
                    return;
                }
            }
        }
        let request = self.pending.take().expect("validated pending request");
        self.selected_draft = None;
        self.queued.push_back((request, profile));
        self.start_queued(cx);
        self.page = crate::settings::Page::Tasks;
        self.status = tr("已加入下载队列");
        cx.notify();
    }
    pub(crate) fn choose_directory(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(tr("选择下载保存目录").into()),
        });
        cx.spawn(async move |this, cx| match picker.await {
            Ok(Ok(Some(paths))) => {
                let _ = this.update(cx, |this, cx| {
                    this.output = paths.into_iter().next();
                    this.settings.profile.output_dir = this.output.clone();
                    cx.notify();
                });
            }
            Ok(Ok(None)) => {}
            _ => {
                let _ = this.update(cx, |this, cx| {
                    this.status = tr("无法打开目录选择器，请重试。");
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
impl Drop for Downloader {
    fn drop(&mut self) {
        if let Some((_, cancel)) = &self.probe_job {
            cancel.cancel();
        }
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
    }
}
