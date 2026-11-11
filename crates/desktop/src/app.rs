//! Window state and user actions. No blocking process calls on the UI thread.
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
    pub(crate) appearance: crate::preferences::Appearance,
    pub(crate) tools: crate::tool_settings::ToolSettings,
    pub(crate) settings: crate::settings::Settings,
    pub(crate) pending: Option<DownloadRequest>,
    pub(crate) duplicate_confirmed: bool,
    pub(crate) running: std::collections::BTreeMap<String, crate::jobs::ActiveDownload>,
    pub(crate) history: crate::history::History,
    pub(crate) history_sort: crate::history_table::Sort,
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
    pub(crate) artifacts: Vec<Artifact>,
    pub(crate) engine: Engine,
    pub(crate) _subscriptions: Vec<Subscription>,
}
impl Downloader {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        crate::i18n::set_language(crate::preferences::load_language());
        let appearance = crate::preferences::load();
        crate::preferences::apply(appearance, window, cx);
        let url = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("粘贴 YouTube 视频链接，例如 https://youtu.be/…")
        });
        let sub = cx.subscribe(&url, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.media = None;
                cx.notify();
            }
        });
        let tools = crate::tool_settings::ToolSettings::new(window, cx);
        let discovered = ToolManager::default_root()
            .ok()
            .and_then(|root| ToolManager::new(root).resolve(&tools.paths(cx)).ok())
            .map(|r| r.paths);
        let engine = Engine::new(
            discovered
                .or_else(|| ToolPaths::discover().ok())
                .unwrap_or_else(|| ToolPaths {
                    yt_dlp: PathBuf::from("/nonexistent/yt-dlp"),
                    ffmpeg: None,
                    ffprobe: None,
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
        let extra_sub = cx.subscribe(&settings.extra, |_, _, _: &InputEvent, cx| cx.notify());
        Self {
            page: crate::settings::Page::New,
            appearance,
            tools,
            settings,
            pending: None,
            duplicate_confirmed: false,
            running: Default::default(),
            history: crate::history::History::load(),
            history_sort: Default::default(),
            history_delete: None,
            history_scroll: ScrollHandle::new(),
            history_page: 0,
            history_page_size: 50,
            url,
            media: None,
            preview_url: String::new(),
            mode,
            output,
            status: "选择保存目录，然后粘贴视频链接。".into(),
            health: "正在检查本地工具…".into(),
            busy: false,
            cancel: None,
            artifacts: vec![],
            engine,
            _subscriptions: vec![sub, extra_sub, appearance_sub],
        }
    }
    pub(crate) fn probe(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let url = self.url.read(cx).value().to_string();
        let url = match normalize_url(&url) {
            Ok(url) => url,
            Err(error) => {
                self.status = error_text(&error);
                cx.notify();
                return;
            }
        };
        self.preview_url = url.clone();
        self.media = None;
        self.artifacts.clear();
        let job = self
            .engine
            .spawn_probe_with_profile(url, self.settings.snapshot(cx));
        self.cancel = Some(job.cancel.clone());
        self.busy = true;
        self.status = "正在解析视频信息…".into();
        let (tx, rx) = async_channel::bounded(16);
        std::thread::spawn(move || {
            while job.events.recv().is_ok() {}
            let _ = tx.send_blocking(Update::Preview(job.join()));
        });
        Self::listen(rx, cx);
        cx.notify();
    }
    pub(crate) fn download(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.media.is_none() {
            return;
        }
        let Some(output_dir) = self.output.clone() else {
            self.status = "请先选择保存目录。".into();
            cx.notify();
            return;
        };
        let id = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
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
                self.page = crate::settings::Page::Confirm;
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
            self.status = "历史已有此视频，请先确认重新下载。".into();
            cx.notify();
            return;
        }
        let profile = self.settings.snapshot(cx);
        // Edits on the review page must pass the same planner before spawning.
        if let Err(error) = self.engine.download_plan(request, &profile) {
            self.status = error_text(&error);
            cx.notify();
            return;
        }
        let request = self.pending.take().expect("validated pending request");
        if self.running.len() >= 3 {
            self.pending = Some(request);
            self.status = "已有 3 个活动下载，请等待任务结束后确认。".into();
            cx.notify();
            return;
        }
        self.launch_download(request, profile, cx);
        self.page = crate::settings::Page::Tasks;
        self.status = "任务已启动，可继续新建下载或编辑配置。".into();
        cx.notify();
    }
    pub(crate) fn choose_directory(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("选择下载保存目录".into()),
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
                    this.status = "无法打开目录选择器，请重试。".into();
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
impl Drop for Downloader {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
    }
}
