use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn delete_commits_metadata_and_cascades_artifacts_without_deleting_files() {
    let f = Fixture::new();
    let mut h = History::load_at(f.db());
    let mut e = entry();
    e.status = HistoryStatus::Succeeded;
    let media = f.0.join("retained.mp4");
    fs::write(&media, b"media").unwrap();
    e.artifacts[0].path = media.clone();
    h.upsert(e.clone()).unwrap();
    let mut other = e.clone();
    other.id = "unrelated".into();
    h.upsert(other).unwrap();
    h.delete(&e.id).unwrap();
    assert_eq!(h.entries.len(), 1);
    assert_eq!(History::load_at(f.db()).entries.len(), 1);
    assert_eq!(fs::read(media).unwrap(), b"media");
    let connection = rusqlite::Connection::open(f.db()).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM artifacts WHERE record_id=?1",
            rusqlite::params![e.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}
#[test]
fn delete_running_and_missing_preserve_entries() {
    let f = Fixture::new();
    let mut h = History::load_at(f.db());
    h.upsert(entry()).unwrap();
    assert!(h.delete(&entry().id).is_err());
    assert!(h.delete("missing").is_err());
    assert_eq!(h.entries.len(), 1);
    // Also protect a Running status changed in the DB after the UI snapshot.
    h.entries[0].status = HistoryStatus::Succeeded;
    assert!(h.delete(&entry().id).is_err());
    assert_eq!(h.entries.len(), 1);
}
#[test]
fn delete_storage_failure_preserves_memory_and_corrupt_file() {
    let f = Fixture::new();
    let mut h = History::load_at(f.db());
    let mut e = entry();
    e.status = HistoryStatus::Failed;
    h.upsert(e.clone()).unwrap();
    fs::write(f.db(), b"broken").unwrap();
    assert!(h.delete(&e.id).is_err());
    assert_eq!(h.entries.len(), 1);
    assert_eq!(fs::read(f.db()).unwrap(), b"broken");
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "shiguang-sqlite-test-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn db(&self) -> PathBuf {
        self.0.join("history.sqlite3")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(
            self.0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("shiguang-sqlite-test-")
        );
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn entry() -> HistoryEntry {
    HistoryEntry {
        id: "one'; DROP TABLE records;--".into(),
        title: "Video".into(),
        url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
        started_at: 1,
        finished_at: None,
        status: HistoryStatus::Running,
        artifacts: vec![Artifact {
            path: PathBuf::from("/tmp/final.mp4"),
            size: 12,
        }],
        metadata: HistoryMetadata {
            source_site: "youtube".into(),
            media_id: "dQw4w9WgXcQ".into(),
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        },
    }
}
#[test]
fn sqlite_roundtrip_upsert_and_interruption() {
    let f = Fixture::new();
    let mut h = History::load_at(f.db());
    assert!(h.error.is_none());
    h.upsert(entry()).unwrap();
    let mut h = History::load_at(f.db());
    assert_eq!(h.entries[0].status, HistoryStatus::Interrupted);
    assert_eq!(h.entries[0].metadata.width, Some(1920));
    assert_eq!(h.entries[0].artifacts[0].size, 12);
    let mut e = entry();
    e.status = HistoryStatus::Succeeded;
    h.upsert(e).unwrap();
    assert_eq!(History::load_at(f.db()).entries.len(), 1);
}
#[test]
fn legacy_migrates_once_and_is_preserved() {
    let f = Fixture::new();
    let json = serde_json::to_vec(&Document {
        version: 1,
        entries: vec![entry()],
    })
    .unwrap();
    fs::write(f.0.join("history.json"), &json).unwrap();
    let h = History::load_at(f.db());
    assert!(h.error.is_none(), "{:?}", h.error);
    assert_eq!(h.entries.len(), 1);
    assert_eq!(fs::read(f.0.join("history.json")).unwrap(), json);
    fs::write(f.0.join("history.json"), b"broken after import").unwrap();
    assert!(History::load_at(f.db()).error.is_none());
}
#[test]
fn corrupt_json_does_not_create_database() {
    let f = Fixture::new();
    fs::write(f.0.join("history.json"), b"broken").unwrap();
    let mut h = History::load_at(f.db());
    assert!(h.error.is_some());
    assert!(h.upsert(entry()).is_err());
    assert!(!f.db().exists());
}
#[test]
fn corrupt_database_is_not_overwritten() {
    let f = Fixture::new();
    fs::write(f.db(), b"not sqlite").unwrap();
    let mut h = History::load_at(f.db());
    assert!(h.error.is_some());
    assert!(h.upsert(entry()).is_err());
    assert_eq!(fs::read(f.db()).unwrap(), b"not sqlite");
}
#[test]
fn future_schema_rejected() {
    let f = Fixture::new();
    let connection = rusqlite::Connection::open(f.db()).unwrap();
    connection.execute_batch("PRAGMA user_version=99").unwrap();
    drop(connection);
    assert!(History::load_at(f.db()).error.is_some());
}
#[test]
fn old_json_entry_defaults_metadata() {
    let mut value = serde_json::to_value(entry()).unwrap();
    value.as_object_mut().unwrap().remove("metadata");
    let old: HistoryEntry = serde_json::from_value(value).unwrap();
    assert!(old.metadata.source_site.is_empty());
}
#[cfg(unix)]
#[test]
fn private_file_and_symlink_rejected() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = Fixture::new();
    let h = History::load_at(f.db());
    assert!(h.error.is_none());
    assert_eq!(
        fs::metadata(f.db()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let link = f.0.join("link.sqlite3");
    symlink(f.db(), &link).unwrap();
    assert!(History::load_at(link).error.is_some());
}
