//! Draft paths and effective tool snapshots are intentionally separate.
use crate::{app::Downloader, i18n::tr, messages::Update};
use gpui_kit::{component::input::InputState, *};
use std::path::{Path, PathBuf};
use youtube_engine::*;
pub(crate) struct ToolSettings {
    pub yt_dlp: Entity<InputState>,
    pub ffmpeg: Entity<InputState>,
    pub ffprobe: Entity<InputState>,
    pub deno: Entity<InputState>,
    pub release: Option<ReleaseInfo>,
    pub busy: bool,
    pub status: String,
    pub effective: Option<ToolPaths>,
    pub versions: Vec<ManagedVersion>,
    pub checking: bool,
}
pub(crate) fn discover() -> CustomToolPaths {
    let managed = ToolManager::default_root().ok().and_then(|r| {
        ToolManager::new(r)
            .resolve(&CustomToolPaths::default())
            .ok()
    });
    CustomToolPaths {
        yt_dlp: resolve_tool(Path::new("yt-dlp"))
            .ok()
            .or_else(|| managed.map(|r| r.paths.yt_dlp)),
        ffmpeg: resolve_tool(Path::new("ffmpeg")).ok(),
        ffprobe: resolve_tool(Path::new("ffprobe")).ok(),
        deno: resolve_tool(Path::new("deno")).ok(),
    }
}
impl ToolSettings {
    pub fn new(window: &mut Window, cx: &mut Context<Downloader>) -> Self {
        let doc = crate::preferences::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .unwrap_or_default();
        let found = discover();
        let mut input = |key: &str, fallback: Option<PathBuf>, cx: &mut Context<Downloader>| {
            cx.new(|cx| {
                InputState::new(window, cx).default_value(
                    doc[key]
                        .as_str()
                        .map(str::to_owned)
                        .or_else(|| fallback.map(|p| p.display().to_string()))
                        .unwrap_or_default(),
                )
            })
        };
        Self {
            yt_dlp: input("yt_dlp", found.yt_dlp, cx),
            ffmpeg: input("ffmpeg", found.ffmpeg, cx),
            ffprobe: input("ffprobe", found.ffprobe, cx),
            deno: input("deno", found.deno, cx),
            release: None,
            busy: false,
            status: tr("自定义路径优先，其次托管版本，最后系统 PATH。"),
            effective: None,
            versions: ToolManager::default_root()
                .ok()
                .and_then(|r| ToolManager::new(r).installed_versions().ok())
                .unwrap_or_default(),
            checking: false,
        }
    }
    pub fn paths(&self, cx: &App) -> CustomToolPaths {
        let value = |s: &Entity<InputState>| {
            let text = s.read(cx).value().trim().to_owned();
            (!text.is_empty()).then(|| PathBuf::from(text))
        };
        CustomToolPaths {
            yt_dlp: value(&self.yt_dlp),
            ffmpeg: value(&self.ffmpeg),
            ffprobe: value(&self.ffprobe),
            deno: value(&self.deno),
        }
    }
    pub fn fill(
        &mut self,
        paths: &CustomToolPaths,
        window: &mut Window,
        cx: &mut Context<Downloader>,
    ) {
        for (input, path) in [
            (&self.yt_dlp, &paths.yt_dlp),
            (&self.ffmpeg, &paths.ffmpeg),
            (&self.ffprobe, &paths.ffprobe),
            (&self.deno, &paths.deno),
        ] {
            input.update(cx, |s, cx| {
                s.set_value(
                    path.as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    window,
                    cx,
                )
            });
        }
    }
}
pub(crate) struct ToolOutcome {
    pub engine: Option<Engine>,
    pub paths: Option<ToolPaths>,
    pub release: Option<ReleaseInfo>,
    pub versions: Vec<ManagedVersion>,
    pub status: String,
}
#[derive(Clone)]
pub(crate) enum Operation {
    Apply,
    Validate,
    Check,
    Install,
    Activate(String),
    Remove(String),
}
impl Downloader {
    pub(crate) fn rediscover_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = discover();
        self.tools.fill(&paths, window, cx);
        self.tools.status = tr("已自动探测并填入系统优先路径。");
        cx.notify();
    }
    pub(crate) fn manage_tools(&mut self, operation: Operation, cx: &mut Context<Self>) {
        let checking = matches!(operation, Operation::Check);
        if self.tools.busy || (checking && self.tools.checking) {
            return;
        }
        let mut custom = self.tools.paths(cx);
        let release = self.tools.release.clone();
        let mut protected: Vec<PathBuf> =
            self.running.values().map(|t| t.tool_path.clone()).collect();
        if let Some(paths) = &self.tools.effective {
            protected.push(paths.yt_dlp.clone());
        }
        if matches!(operation, Operation::Install) && release.is_none() {
            return;
        }
        if checking {
            self.tools.checking = true;
        } else {
            self.tools.busy = true;
        }
        self.tools.status = tr("正在处理工具请求…");
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let result = (|| -> std::result::Result<ToolOutcome, String> {
                let manager = ToolManager::new(ToolManager::default_root()?);
                let mut out = ToolOutcome {
                    engine: None,
                    paths: None,
                    release: None,
                    versions: vec![],
                    status: String::new(),
                };
                match &operation {
                    Operation::Check => {
                        let r = manager.check_release()?;
                        out.status = format!("{} {}", tr("官方版本"), r.version);
                        out.release = Some(r);
                    }
                    Operation::Install => {
                        let path = manager.install_only(release.as_ref().unwrap())?;
                        out.status = format!("{}: {}", tr("已安装，尚未切换"), path.display());
                    }
                    Operation::Remove(version) => {
                        manager.remove_version(version, &protected)?;
                        out.status = tr("托管版本已删除");
                    }
                    Operation::Activate(version) => {
                        custom.yt_dlp = Some(manager.activate(version)?);
                    }
                    _ => {}
                }
                if matches!(
                    operation,
                    Operation::Apply | Operation::Validate | Operation::Activate(_)
                ) {
                    let resolved = manager.resolve(&custom)?;
                    let paths = resolved.paths;
                    let engine = Engine::new(paths.clone(), EngineConfig::default());
                    let report = engine.health_check(&CancelToken::new());
                    if !report.yt_dlp.healthy {
                        return Err(tr("yt-dlp健康检查失败；当前运行任务不受影响。"));
                    }
                    if custom.ffmpeg.is_some() && !report.ffmpeg.as_ref().is_some_and(|t| t.healthy)
                    {
                        return Err("FFmpeg check failed".into());
                    }
                    if custom.ffprobe.is_some()
                        && !report.ffprobe.as_ref().is_some_and(|t| t.healthy)
                    {
                        return Err("ffprobe check failed".into());
                    }
                    if custom.deno.is_some() && !report.deno.as_ref().is_some_and(|t| t.healthy) {
                        return Err("Deno check failed".into());
                    }
                    out.status = format!(
                        "yt-dlp {} · FFmpeg {} · ffprobe {} · Deno {}",
                        report.yt_dlp.version.unwrap_or_default(),
                        report.ffmpeg.and_then(|t| t.version).unwrap_or_default(),
                        report.ffprobe.and_then(|t| t.version).unwrap_or_default(),
                        report
                            .deno
                            .and_then(|t| t.version)
                            .unwrap_or_else(|| tr("未配置"))
                    );
                    if !matches!(operation, Operation::Validate) {
                        persist(&custom)?;
                        out.paths = Some(paths);
                        out.engine = Some(engine);
                    }
                }
                out.versions = manager.installed_versions()?;
                Ok(out)
            })();
            let _ = tx.send_blocking(Update::Tools(checking, result));
        });
        Self::listen(rx, cx);
        cx.notify();
    }
}
fn persist(paths: &CustomToolPaths) -> std::result::Result<(), String> {
    let path = crate::preferences::path().ok_or("Cannot locate settings")?;
    let mut doc = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    doc["yt_dlp"] = serde_json::json!(paths.yt_dlp);
    doc["ffmpeg"] = serde_json::json!(paths.ffmpeg);
    doc["ffprobe"] = serde_json::json!(paths.ffprobe);
    doc["deno"] = serde_json::json!(paths.deno);
    crate::preferences::write_document(&path, &doc)
}
