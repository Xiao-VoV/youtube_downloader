//! Persistable user intent. Browser credential access is opt-in, never discovery.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserCookies {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Brave,
    Chromium,
}
impl BrowserCookies {
    pub(crate) fn argument(&self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Safari => "safari",
            Self::Edge => "edge",
            Self::Brave => "brave",
            Self::Chromium => "chromium",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum VideoContainer {
    #[default]
    Mp4,
    Mkv,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadProfile {
    /// Preset UI selection. Execution uses the immutable request.mode snapshot.
    pub mode: crate::Mode,
    pub cookies_browser: Option<BrowserCookies>,
    /// Plain browser profile name or path; yt-dlp cookie grammar delimiters forbidden.
    pub browser_profile: Option<String>,
    pub proxy: Option<String>,
    pub max_height: u16,
    pub best_resolution: bool,
    pub container: VideoContainer,
    pub output_dir: Option<PathBuf>,
    /// Tokenized argv, not a shell string. Unknown flags fail closed.
    pub extra_args: Vec<String>,
}
impl Default for DownloadProfile {
    fn default() -> Self {
        Self {
            mode: crate::Mode::Video,
            cookies_browser: None,
            browser_profile: None,
            proxy: None,
            max_height: 1080,
            best_resolution: false,
            container: VideoContainer::Mp4,
            output_dir: None,
            extra_args: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub profile: DownloadProfile,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProfileStore {
    pub presets: Vec<Preset>,
    pub active_preset_id: Option<String>,
}
