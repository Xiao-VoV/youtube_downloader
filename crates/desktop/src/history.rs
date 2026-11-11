//! Bounded JSON repository. Keep persistence separate from UI and execution so
//! a future SQLite implementation can preserve the entry/domain contract.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use youtube_engine::Artifact;

const MAX_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ENTRIES: usize = 1000;
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    pub id: String,
    pub title: String,
    pub url: String,
    /// Unix seconds; no locale-dependent text in persisted records.
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub status: HistoryStatus,
    pub artifacts: Vec<Artifact>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    entries: Vec<HistoryEntry>,
}

pub struct History {
    pub entries: Vec<HistoryEntry>,
    pub error: Option<String>,
    path: Option<PathBuf>,
    blocked: bool,
}
impl History {
    pub fn load() -> Self {
        let Some(home) = std::env::var_os("HOME").filter(|s| !s.is_empty()) else {
            return Self {
                entries: Vec::new(),
                error: Some("History unavailable: HOME is unset".into()),
                path: None,
                blocked: true,
            };
        };
        Self::load_at(PathBuf::from(home).join("Library/Application Support/Shiguang/history.json"))
    }
    fn load_at(path: PathBuf) -> Self {
        let mut history = Self {
            entries: Vec::new(),
            error: None,
            path: Some(path.clone()),
            blocked: false,
        };
        match read_entries(&path) {
            Ok(Some(entries)) => history.entries = entries,
            Ok(None) => return history,
            Err(error) => {
                history.error = Some(error);
                history.blocked = true;
                return history;
            }
        }
        let mut changed = false;
        for entry in &mut history.entries {
            if entry.status == HistoryStatus::Running {
                entry.status = HistoryStatus::Interrupted;
                changed = true;
            }
        }
        if changed {
            if let Err(error) = history.save_at(&history.entries) {
                history.error = Some(error);
                history.blocked = true;
            }
        }
        history
    }
    /// Transactional in-memory update: a failed disk write does not claim the
    /// new entry was persisted. Corrupt/unsupported input is never overwritten.
    pub fn upsert(&mut self, entry: HistoryEntry) -> Result<(), String> {
        if self.blocked {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| "History writes blocked".into()));
        }
        validate_entry(&entry)?;
        let mut entries = self.entries.clone();
        if let Some(existing) = entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry;
        } else {
            entries.push(entry);
        }
        if entries.len() > MAX_ENTRIES {
            return Err("History entry limit reached; existing history preserved".into());
        }
        if let Err(error) = self.save_at(&entries) {
            self.error = Some(error.clone());
            return Err(error);
        }
        self.entries = entries;
        self.error = None;
        Ok(())
    }
    fn save_at(&self, entries: &[HistoryEntry]) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("History path unavailable")?;
        // Recheck existing bytes on every save; another process or editor must
        // not turn damaged evidence into a silent fresh history document.
        read_entries(path)?;
        let bytes = serde_json::to_vec(&Document {
            version: 1,
            entries: entries.to_vec(),
        })
        .map_err(|_| "Cannot encode history")?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("History byte limit reached".into());
        }
        atomic_write(path, &bytes)
    }
}
fn validate_entry(entry: &HistoryEntry) -> Result<(), String> {
    if entry.id.is_empty()
        || entry.id.len() > 256
        || entry.title.len() > 16384
        || entry.url.len() > 2048
        || entry.artifacts.len() > 32
    {
        return Err("Invalid history entry limits".into());
    }
    // Persist only canonical public video URLs: no cookies, proxy credentials,
    // query secrets or tool diagnostics can enter through this field.
    if youtube_engine::normalize_url(&entry.url).ok().as_deref() != Some(entry.url.as_str()) {
        return Err("History requires a canonical video URL".into());
    }
    if entry
        .artifacts
        .iter()
        .any(|a| !a.path.is_absolute() || a.path.as_os_str().len() > 4096)
    {
        return Err("Invalid history artifact path".into());
    }
    Ok(())
}
fn read_entries(path: &Path) -> Result<Option<Vec<HistoryEntry>>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot inspect history; writes blocked".into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_BYTES {
        return Err("Unsafe or oversized history; writes blocked".into());
    }
    let file = fs::File::open(path).map_err(|_| "Cannot read history; writes blocked")?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read history; writes blocked")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Oversized history; writes blocked".into());
    }
    let doc: Document =
        serde_json::from_slice(&bytes).map_err(|_| "Damaged history; writes blocked")?;
    if doc.version != 1 || doc.entries.len() > MAX_ENTRIES {
        return Err("Unsupported history version or count; writes blocked".into());
    }
    let mut ids = std::collections::HashSet::new();
    for entry in &doc.entries {
        validate_entry(entry)?;
        if !ids.insert(&entry.id) {
            return Err("Duplicate history IDs; writes blocked".into());
        }
    }
    Ok(Some(doc.entries))
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid history path")?;
    fs::create_dir_all(parent).map_err(|_| "Cannot create history directory")?;
    if !fs::symlink_metadata(parent)
        .map_err(|_| "Cannot inspect history directory")?
        .is_dir()
    {
        return Err("Unsafe history directory".into());
    }
    let temp = parent.join(format!(
        ".history-{}-{}.tmp",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp)
        .map_err(|_| "Cannot create private history transaction")?;
    let result = (|| {
        file.write_all(bytes).map_err(|_| "Cannot write history")?;
        file.sync_all().map_err(|_| "Cannot synchronize history")?;
        fs::rename(&temp, path).map_err(|_| "Cannot commit history")?;
        Ok(())
    })();
    // temp is an exact owned sibling, never derived from a history entry.
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "shiguang-history-test-{}-{}",
                std::process::id(),
                TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn path(&self) -> PathBuf {
            self.0.join("history.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert!(
                self.0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("shiguang-history-test-")
            );
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn entry() -> HistoryEntry {
        HistoryEntry {
            id: "one".into(),
            title: "Video".into(),
            url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
            started_at: 1,
            finished_at: None,
            status: HistoryStatus::Running,
            artifacts: Vec::new(),
        }
    }
    #[test]
    fn roundtrip_upsert_and_interrupt() {
        let f = Fixture::new();
        let mut h = History::load_at(f.path());
        h.upsert(entry()).unwrap();
        let mut loaded = History::load_at(f.path());
        assert_eq!(loaded.entries[0].status, HistoryStatus::Interrupted);
        let mut e = entry();
        e.status = HistoryStatus::Succeeded;
        e.finished_at = Some(2);
        loaded.upsert(e).unwrap();
        assert_eq!(History::load_at(f.path()).entries.len(), 1);
        assert_eq!(
            History::load_at(f.path()).entries[0].status,
            HistoryStatus::Succeeded
        );
    }
    #[test]
    fn corruption_is_preserved() {
        let f = Fixture::new();
        fs::write(f.path(), b"broken").unwrap();
        let mut h = History::load_at(f.path());
        assert!(h.error.is_some());
        assert!(h.upsert(entry()).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), b"broken");
    }
    #[test]
    fn disk_corruption_after_load_is_not_overwritten() {
        let f = Fixture::new();
        let mut h = History::load_at(f.path());
        fs::write(f.path(), b"broken").unwrap();
        assert!(h.upsert(entry()).is_err());
        assert!(h.entries.is_empty());
        assert_eq!(fs::read(f.path()).unwrap(), b"broken");
    }
    #[test]
    fn limits_reject_without_mutation() {
        let f = Fixture::new();
        let mut h = History::load_at(f.path());
        let mut e = entry();
        e.title = "x".repeat(16385);
        assert!(h.upsert(e).is_err());
        assert!(!f.path().exists());
    }
    #[cfg(unix)]
    #[test]
    fn private_permissions_and_symlink_rejection() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let f = Fixture::new();
        let mut h = History::load_at(f.path());
        h.upsert(entry()).unwrap();
        assert_eq!(
            fs::metadata(f.path()).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let link = f.0.join("link.json");
        symlink(f.path(), &link).unwrap();
        assert!(History::load_at(link).error.is_some());
    }
}
