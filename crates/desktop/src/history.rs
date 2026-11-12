//! SQLite metadata repository; UI/execution and credential configuration stay out.
mod database;
#[cfg(test)]
mod tests;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use youtube_engine::Artifact;

pub(super) const MAX_ENTRIES: usize = 1000;
pub(super) const MAX_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryMetadata {
    pub source_site: String,
    pub media_id: String,
    pub channel: Option<String>,
    pub duration: Option<f64>,
    /// Actual format bitrate in bits/second, when ffprobe reports one.
    pub bitrate: Option<u64>,
    pub original_url: String,
    pub requested_height: Option<u16>,
    pub best_resolution: bool,
    pub requested_format: String,
    pub output_dir: Option<PathBuf>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub ext: Option<String>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    pub id: String,
    pub title: String,
    pub url: String,
    /// Unix seconds, independent of locale.
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub status: HistoryStatus,
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub metadata: HistoryMetadata,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    version: u32,
    entries: Vec<HistoryEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageOrder {
    Title,
    Status,
    Site,
    Time,
    Resolution,
    Format,
    Size,
    Channel,
    Bitrate,
    VideoCodec,
    AudioCodec,
    Duration,
    Finished,
}
#[derive(Clone, Copy, Debug)]
pub struct HistorySort {
    pub order: PageOrder,
    pub desc: bool,
}
impl Default for HistorySort {
    fn default() -> Self {
        Self {
            order: PageOrder::Time,
            desc: true,
        }
    }
}
#[derive(Clone, Debug)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub total: usize,
}

pub struct History {
    pub entries: Vec<HistoryEntry>,
    pub error: Option<String>,
    path: Option<PathBuf>,
    blocked: bool,
}
impl History {
    pub fn load() -> Self {
        match youtube_engine::app_data_dir() {
            Ok(root) => Self::load_at(root.join("history.sqlite3")),
            Err(_) => Self {
                entries: Vec::new(),
                error: Some("History unavailable: HOME is unset".into()),
                path: None,
                blocked: true,
            },
        }
    }
    fn load_at(path: PathBuf) -> Self {
        let mut history = Self {
            entries: Vec::new(),
            error: None,
            path: Some(path.clone()),
            blocked: false,
        };
        match database::load(&path) {
            Ok(entries) => history.entries = entries,
            Err(error) => {
                history.error = Some(error);
                history.blocked = true;
            }
        }
        history
    }
    /// Zero-based page, database-wide sorting before LIMIT/OFFSET. The legacy
    /// entries snapshot remains available for duplicate detection and updates.
    pub fn page(
        &self,
        sort: HistorySort,
        page: usize,
        page_size: usize,
    ) -> Result<HistoryPage, String> {
        if self.blocked {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| "History unavailable".into()));
        }
        if !(1..=100).contains(&page_size) {
            return Err("History page size must be 1..100".into());
        }
        let offset = page
            .checked_mul(page_size)
            .and_then(|n| i64::try_from(n).ok())
            .ok_or("History page offset overflow")?;
        database::page(
            self.path.as_ref().ok_or("History path unavailable")?,
            sort,
            offset,
            page_size as i64,
        )
    }
    /// Remove metadata only, never media files. Commit before changing UI state.
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        if self.blocked {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| "History writes blocked".into()));
        }
        if id.is_empty() || id.len() > 256 {
            return Err("Invalid history ID".into());
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.id == id && entry.status == HistoryStatus::Running)
        {
            return Err("Cannot delete a running download".into());
        }
        let path = self.path.as_ref().ok_or("History path unavailable")?;
        match database::delete(path, id) {
            Ok(()) => {
                self.entries.retain(|entry| entry.id != id);
                self.error = None;
                Ok(())
            }
            Err(error) => {
                self.error = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn upsert(&mut self, entry: HistoryEntry) -> Result<(), String> {
        if self.blocked {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| "History writes blocked".into()));
        }
        validate_entry(&entry)?;
        let path = self.path.as_ref().ok_or("History path unavailable")?;
        match database::upsert(path, &entry) {
            Ok(()) => {
                if let Some(old) = self.entries.iter_mut().find(|e| e.id == entry.id) {
                    *old = entry;
                } else {
                    self.entries.push(entry);
                }
                self.error = None;
                Ok(())
            }
            Err(error) => {
                self.error = Some(error.clone());
                Err(error)
            }
        }
    }
}
pub(super) fn validate_entry(entry: &HistoryEntry) -> Result<(), String> {
    if entry.id.is_empty()
        || entry.id.len() > 256
        || entry.title.len() > 16384
        || entry.url.len() > 2048
        || entry.artifacts.len() > 32
        || entry.started_at > i64::MAX as u64
        || entry.finished_at.is_some_and(|v| v > i64::MAX as u64)
    {
        return Err("Invalid history entry limits".into());
    }
    if youtube_engine::normalize_url(&entry.url).ok().as_deref() != Some(entry.url.as_str()) {
        return Err("History requires canonical video URL".into());
    }
    let m = &entry.metadata;
    if !m.original_url.is_empty()
        && youtube_engine::normalize_url(&m.original_url)
            .ok()
            .as_deref()
            != Some(entry.url.as_str())
    {
        return Err("Invalid original media URL".into());
    }
    if m.duration.is_some_and(|v| !v.is_finite() || v < 0.0)
        || m.output_dir.as_ref().is_some_and(|p| !p.is_absolute())
    {
        return Err("Invalid history metadata".into());
    }
    if entry.artifacts.iter().any(|a| {
        !a.path.is_absolute() || a.path.as_os_str().len() > 4096 || a.size > i64::MAX as u64
    }) {
        return Err("Invalid artifact metadata".into());
    }
    let bytes = serde_json::to_vec(entry).map_err(|_| "Invalid history encoding")?;
    if bytes.len() > 64 * 1024 {
        return Err("History entry too large".into());
    }
    Ok(())
}
