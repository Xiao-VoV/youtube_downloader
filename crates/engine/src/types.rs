use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);
impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Video,
    Audio,
    MP3,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Format {
    pub format_id: String,
    pub ext: Option<String>,
    pub format_note: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub abr: Option<f64>,
    pub tbr: Option<f64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MediaInfo {
    pub id: String,
    pub title: String,
    pub extractor: Option<String>,
    pub channel: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub webpage_url: Option<String>,
    #[serde(default)]
    pub formats: Vec<Format>,
}
#[derive(Clone, Debug)]
pub struct ToolPaths {
    pub yt_dlp: PathBuf,
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    /// Selected local runtime. None disables JavaScript runtime execution.
    pub deno: Option<PathBuf>,
}
#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub probe_timeout: Duration,
    pub download_timeout: Duration,
    pub terminate_grace: Duration,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    pub max_line_bytes: usize,
}
impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            probe_timeout: Duration::from_secs(45),
            download_timeout: Duration::from_secs(7200),
            terminate_grace: Duration::from_millis(500),
            max_stdout_bytes: 16 * 1024 * 1024,
            max_stderr_bytes: 64 * 1024,
            max_line_bytes: 256 * 1024,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub url: String,
    pub mode: Mode,
    pub output_dir: PathBuf,
    /// None selects the engine's fixed best-quality policy; no arbitrary CLI.
    pub format_id: Option<String>,
    pub task_id: String,
    pub attempt_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub path: PathBuf,
    pub size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadResult {
    pub artifacts: Vec<Artifact>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    pub status: Option<String>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub total_bytes_estimate: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
    pub format_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventKind {
    Starting,
    Progress(Progress),
    PostProcessing,
    Completed(DownloadResult),
    Failed(EngineError),
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineEvent {
    pub task_id: String,
    pub attempt_id: String,
    pub kind: EventKind,
}
pub type EventSender = SyncSender<EngineEvent>;
/// Progress is best-effort on the bounded channel. Terminal delivery blocks until
/// consumed or disconnected: keep draining events before calling join().
pub struct EngineJob<T> {
    pub events: Receiver<EngineEvent>,
    pub cancel: CancelToken,
    pub(crate) handle: JoinHandle<Result<T>>,
}
impl<T> EngineJob<T> {
    pub fn join(self) -> Result<T> {
        self.handle
            .join()
            .map_err(|_| EngineError::new(ErrorKind::Internal, "engine thread panicked"))?
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    InvalidUrl,
    InvalidRequest,
    ToolNotFound,
    Spawn,
    Io,
    Cancelled,
    Timeout,
    OutputLimit,
    Protocol,
    ProcessFailed,
    UnsafeArtifact,
    MissingArtifact,
    Internal,
    Unsupported,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineError {
    pub kind: ErrorKind,
    pub message: String,
}
impl EngineError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}
impl std::error::Error for EngineError {}
pub type Result<T> = std::result::Result<T, EngineError>;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolHealth {
    pub path: PathBuf,
    pub version: Option<String>,
    pub healthy: bool,
    pub error: Option<EngineError>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthReport {
    pub yt_dlp: ToolHealth,
    pub ffmpeg: Option<ToolHealth>,
    pub ffprobe: Option<ToolHealth>,
    #[serde(default)]
    pub deno: Option<ToolHealth>,
    pub notes: Vec<String>,
}
