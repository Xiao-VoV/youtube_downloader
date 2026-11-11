//! UI message contract and user-facing error mapping.
use youtube_engine::*;
// All process IO runs on engine-owned threads; this bounded bridge only carries UI data.
pub(crate) enum Update {
    Health(String),
    Tools(std::result::Result<(Option<Engine>, Option<ReleaseInfo>, String), String>),
    Preview(youtube_engine::Result<MediaInfo>),
    Event(EngineEvent),
    Finished(
        String,
        youtube_engine::Result<DownloadResult>,
        Option<MediaInspection>,
    ),
}
pub(crate) fn error_text(error: &EngineError) -> String {
    let label = match error.kind {
        ErrorKind::Cancelled => "已取消，进程已回收",
        ErrorKind::InvalidUrl => "链接无效：仅支持 YouTube 单视频链接",
        ErrorKind::ToolNotFound => "未找到本地下载工具，请配置 yt-dlp / FFmpeg",
        ErrorKind::Timeout => "任务超时，进程已停止",
        ErrorKind::ProcessFailed => "引擎执行失败：请检查网络、工具版本及站点挑战依赖",
        _ => "任务失败",
    };
    format!("{label} · {}", error.message)
}
