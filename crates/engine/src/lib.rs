//! Synchronous yt-dlp adapter. Run blocking methods off the UI thread, or use
//! `spawn_probe` / `spawn_download`. No shell is used to launch media tools.
mod artifacts;
mod engine;
mod pipes;
mod plan;
mod profile;
pub use plan::CommandPlan;
pub use profile::*;
mod process;
mod protocol;
mod tools;
mod types;
mod urls;

pub use engine::*;
pub use protocol::{
    parse_progress, ARTIFACT_PREFIX, DEFAULT_VIDEO_SELECTOR, POSTPROCESS_PREFIX, PROGRESS_PREFIX,
};
pub use tools::resolve_tool;
pub use types::*;
pub use urls::normalize_url;
