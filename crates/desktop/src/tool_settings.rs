//! User-initiated tool operations run off the UI thread; tasks clone Engine.
use crate::{app::Downloader, messages::Update};
use gpui_kit::{component::input::InputState, *};
use youtube_engine::*;
pub(crate) struct ToolSettings {
    pub yt_dlp: Entity<InputState>,
    pub ffmpeg: Entity<InputState>,
    pub ffprobe: Entity<InputState>,
    pub release: Option<ReleaseInfo>,
    pub busy: bool,
    pub status: String,
}
impl ToolSettings {
    pub fn new(window: &mut Window, cx: &mut Context<Downloader>) -> Self {
        let doc = crate::preferences::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .unwrap_or_default();
        let mut input = |key: &str, cx: &mut Context<Downloader>| {
            cx.new(|cx| InputState::new(window, cx).default_value(doc[key].as_str().unwrap_or("")))
        };
        Self {
            yt_dlp: input("yt_dlp", cx),
            ffmpeg: input("ffmpeg", cx),
            ffprobe: input("ffprobe", cx),
            release: None,
            busy: false,
            status: "自定义路径优先，其次托管版本，最后系统 PATH。".into(),
        }
    }
    pub fn paths(&self, cx: &App) -> CustomToolPaths {
        let value = |input: &Entity<InputState>| {
            let text = input.read(cx).value().trim().to_owned();
            (!text.is_empty()).then(|| std::path::PathBuf::from(text))
        };
        CustomToolPaths {
            yt_dlp: value(&self.yt_dlp),
            ffmpeg: value(&self.ffmpeg),
            ffprobe: value(&self.ffprobe),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Apply,
    Check,
    Install,
    Revert,
}
impl Downloader {
    pub(crate) fn manage_tools(&mut self, operation: Operation, cx: &mut Context<Self>) {
        if self.tools.busy {
            return;
        }
        let custom = self.tools.paths(cx);
        let release = self.tools.release.clone();
        if matches!(operation, Operation::Install) && release.is_none() {
            return;
        }
        self.tools.busy = true;
        self.tools.status = "正在处理工具请求…".into();
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let result=(||->std::result::Result<(Option<Engine>,Option<ReleaseInfo>,String),String>{
                let manager=ToolManager::new(ToolManager::default_root()?);
                match operation {
                    Operation::Check=>{let r=manager.check_release()?;let msg=format!("官方版本 {} 可用；点击确认安装才会下载可执行文件。",r.version);Ok((None,Some(r),msg))},
                    Operation::Install|Operation::Revert|Operation::Apply=>{
                        match operation{Operation::Install=>{manager.install(release.as_ref().unwrap())?;},Operation::Revert=>{manager.revert()?;},_=>{}}
                        let resolved=manager.resolve(&custom)?;
                        let source=format!("{:?}",resolved.yt_dlp_source);
                        let engine=Engine::new(resolved.paths,EngineConfig::default());
                        let report=engine.health_check(&CancelToken::new());
                        if !report.yt_dlp.healthy{return Err("yt-dlp健康检查失败；当前运行任务不受影响。".into());}
                        Ok((Some(engine),None,format!("yt-dlp {} · 来源 {source} · FFmpeg {}",report.yt_dlp.version.unwrap_or_default(),if report.ffmpeg.as_ref().is_some_and(|t|t.healthy){"就绪"}else{"未配置"})))
                    }
                }
            })();
            let _ = tx.send_blocking(Update::Tools(result));
        });
        Self::listen(rx, cx);
        cx.notify();
    }
    pub(crate) fn persist_tool_paths(&self, cx: &App) -> std::result::Result<(), String> {
        let path = crate::preferences::path().ok_or("无法定位设置目录")?;
        let mut doc = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let paths = self.tools.paths(cx);
        doc["yt_dlp"] = serde_json::json!(paths.yt_dlp);
        doc["ffmpeg"] = serde_json::json!(paths.ffmpeg);
        doc["ffprobe"] = serde_json::json!(paths.ffprobe);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|_| "无法创建设置目录")?;
        let temp = path.with_extension("tools.tmp");
        std::fs::write(
            &temp,
            serde_json::to_vec_pretty(&doc).map_err(|_| "工具设置编码失败")?,
        )
        .map_err(|_| "工具设置保存失败")?;
        std::fs::rename(temp, path).map_err(|_| "工具设置提交失败".into())
    }
}
