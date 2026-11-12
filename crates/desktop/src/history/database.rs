//! Parameterized SQL only. Version migration and legacy import share one transaction.
use super::*;
use rusqlite::{Connection, OpenFlags, Transaction, params};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
fn error(_: impl std::fmt::Display) -> String {
    "History database operation failed; existing files preserved".into()
}
fn safe_file(path: &Path, max: u64) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.len() <= max => Ok(true),
        Ok(_) => Err("Unsafe or oversized history file; writes blocked".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(error(e)),
    }
}
fn open(path: &Path) -> Result<Connection, String> {
    if !safe_file(path, 32 * 1024 * 1024)? {
        let parent = path.parent().ok_or("Invalid history path")?;
        fs::create_dir_all(parent).map_err(error)?;
        if !fs::symlink_metadata(parent).map_err(error)?.is_dir() {
            return Err("Unsafe history directory".into());
        }
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path).map_err(error)?;
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(error)?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(error)?;
    connection
        .execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;")
        .map_err(error)?;
    let check: String = connection
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(error)?;
    if check != "ok" {
        return Err("Damaged history database; writes blocked".into());
    }
    Ok(connection)
}
fn legacy(path: &Path) -> Result<Option<Vec<HistoryEntry>>, String> {
    if !safe_file(path, MAX_BYTES)? {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(error)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Oversized legacy history".into());
    }
    let document: Document =
        serde_json::from_slice(&bytes).map_err(|_| "Damaged legacy history; import blocked")?;
    if document.version != 1 || document.entries.len() > MAX_ENTRIES {
        return Err("Unsupported legacy history".into());
    }
    let mut ids = std::collections::HashSet::new();
    for entry in &document.entries {
        validate_entry(entry)?;
        if !ids.insert(&entry.id) {
            return Err("Duplicate legacy history IDs".into());
        }
    }
    Ok(Some(document.entries))
}
fn migrate(tx: &Transaction<'_>) -> Result<(), String> {
    let version: u32 = tx
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)?;
    match version {
        0=>tx.execute_batch("CREATE TABLE records(id TEXT PRIMARY KEY,title TEXT NOT NULL,url TEXT NOT NULL,started_at INTEGER NOT NULL,finished_at INTEGER,status TEXT NOT NULL,metadata TEXT NOT NULL); CREATE INDEX records_started ON records(started_at); CREATE TABLE artifacts(record_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,ordinal INTEGER NOT NULL,path TEXT NOT NULL,size INTEGER NOT NULL,PRIMARY KEY(record_id,ordinal)); CREATE TABLE repository_flags(key TEXT PRIMARY KEY,value TEXT NOT NULL); PRAGMA user_version=1;").map_err(error),
        1=>Ok(()),
        _=>Err("Unsupported history database version; writes blocked".into()),
    }
}
pub(super) fn load(path: &Path) -> Result<Vec<HistoryEntry>, String> {
    // Validate legacy before creating a DB: damaged JSON is never replaced by
    // an apparently empty new repository. Once imported, JSON is not reread.
    let exists = safe_file(path, 32 * 1024 * 1024)?;
    let old_path = path.with_file_name("history.json");
    let initial = if !exists { legacy(&old_path)? } else { None };
    let mut connection = open(path)?;
    let tx = connection.transaction().map_err(error)?;
    migrate(&tx)?;
    let imported: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM repository_flags WHERE key='legacy_imported')",
            [],
            |r| r.get(0),
        )
        .map_err(error)?;
    if !imported {
        let old = if exists { legacy(&old_path)? } else { initial };
        if let Some(entries) = old {
            for entry in entries {
                store(&tx, &entry)?;
            }
        }
        tx.execute(
            "INSERT INTO repository_flags(key,value) VALUES('legacy_imported','1')",
            [],
        )
        .map_err(error)?;
    }
    tx.execute(
        "UPDATE records SET status='Interrupted' WHERE status='Running'",
        [],
    )
    .map_err(error)?;
    let entries = read(&tx)?;
    tx.commit().map_err(error)?;
    Ok(entries)
}
fn store(tx: &Transaction<'_>, entry: &HistoryEntry) -> Result<(), String> {
    validate_entry(entry)?;
    let metadata = serde_json::to_string(&entry.metadata).map_err(error)?;
    tx.execute("INSERT INTO records(id,title,url,started_at,finished_at,status,metadata) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET title=excluded.title,url=excluded.url,started_at=excluded.started_at,finished_at=excluded.finished_at,status=excluded.status,metadata=excluded.metadata",params![entry.id,entry.title,entry.url,entry.started_at as i64,entry.finished_at.map(|n|n as i64),format!("{:?}",entry.status),metadata]).map_err(error)?;
    tx.execute(
        "DELETE FROM artifacts WHERE record_id=?1",
        params![entry.id],
    )
    .map_err(error)?;
    for (ordinal, artifact) in entry.artifacts.iter().enumerate() {
        let path = artifact.path.to_str().ok_or("History path must be UTF-8")?;
        tx.execute(
            "INSERT INTO artifacts(record_id,ordinal,path,size) VALUES(?1,?2,?3,?4)",
            params![entry.id, ordinal as i64, path, artifact.size as i64],
        )
        .map_err(error)?;
    }
    Ok(())
}
fn read(tx: &Transaction<'_>) -> Result<Vec<HistoryEntry>, String> {
    let count: i64 = tx
        .query_row("SELECT count(*) FROM records", [], |r| r.get(0))
        .map_err(error)?;
    if count > MAX_ENTRIES as i64 {
        return Err("History count limit reached".into());
    }
    read_query(
        tx,
        "SELECT id,title,url,started_at,finished_at,status,metadata FROM records ORDER BY started_at,id",
        [],
    )
}
fn read_query<P: rusqlite::Params>(
    tx: &Transaction<'_>,
    sql: &str,
    parameters: P,
) -> Result<Vec<HistoryEntry>, String> {
    let mut statement = tx.prepare(sql).map_err(error)?;
    let rows = statement
        .query_map(parameters, |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, u64>(3)?,
                r.get::<_, Option<u64>>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(error)?;
    let mut entries = Vec::new();
    let mut total = 0usize;
    for row in rows {
        let (id, title, url, started_at, finished_at, status, metadata) = row.map_err(error)?;
        let status = match status.as_str() {
            "Running" => HistoryStatus::Running,
            "Succeeded" => HistoryStatus::Succeeded,
            "Failed" => HistoryStatus::Failed,
            "Cancelled" => HistoryStatus::Cancelled,
            "Interrupted" => HistoryStatus::Interrupted,
            _ => return Err("Invalid history status".into()),
        };
        let mut artifacts_stmt = tx
            .prepare("SELECT path,size FROM artifacts WHERE record_id=?1 ORDER BY ordinal")
            .map_err(error)?;
        let artifacts = artifacts_stmt
            .query_map(params![id], |r| {
                Ok(Artifact {
                    path: PathBuf::from(r.get::<_, String>(0)?),
                    size: r.get(1)?,
                })
            })
            .map_err(error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(error)?;
        let entry = HistoryEntry {
            id,
            title,
            url,
            started_at,
            finished_at,
            status,
            artifacts,
            metadata: serde_json::from_str(&metadata).map_err(error)?,
        };
        validate_entry(&entry)?;
        total += serde_json::to_vec(&entry).map_err(error)?.len();
        if total as u64 > MAX_BYTES {
            return Err("History content limit reached".into());
        }
        entries.push(entry);
    }
    Ok(entries)
}
/// SQL identifiers/expressions are exclusively fixed strings. Only numeric
/// LIMIT/OFFSET values enter bind parameters; no user SQL is interpolated.
pub(super) fn page(
    path: &Path,
    sort: HistorySort,
    offset: i64,
    limit: i64,
) -> Result<HistoryPage, String> {
    let mut connection = open(path)?;
    let tx = connection.transaction().map_err(error)?;
    let version: u32 = tx
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)?;
    if version != 1 {
        return Err("History schema changed; reload required".into());
    }
    let total: usize = tx
        .query_row("SELECT count(*) FROM records", [], |r| r.get(0))
        .map_err(error)?;
    if total > MAX_ENTRIES {
        return Err("History count limit reached".into());
    }
    let order = match sort.order {
        PageOrder::Title => "title COLLATE NOCASE",
        PageOrder::Status => "status",
        PageOrder::Site => "COALESCE(json_extract(metadata,'$.source_site'),'') COLLATE NOCASE",
        PageOrder::Time => "started_at",
        PageOrder::Channel => "COALESCE(json_extract(metadata,'$.channel'),'') COLLATE NOCASE",
        PageOrder::Bitrate => "COALESCE(json_extract(metadata,'$.bitrate'),0)",
        PageOrder::VideoCodec => "COALESCE(json_extract(metadata,'$.vcodec'),'') COLLATE NOCASE",
        PageOrder::AudioCodec => "COALESCE(json_extract(metadata,'$.acodec'),'') COLLATE NOCASE",
        PageOrder::Duration => "COALESCE(json_extract(metadata,'$.duration'),0)",
        PageOrder::Finished => "COALESCE(finished_at,0)",
        PageOrder::Resolution => {
            "COALESCE(json_extract(metadata,'$.height'),json_extract(metadata,'$.requested_height'),0)"
        }
        PageOrder::Format => {
            "COALESCE(json_extract(metadata,'$.ext'),json_extract(metadata,'$.requested_format'),'') COLLATE NOCASE"
        }
        PageOrder::Size => {
            "COALESCE((SELECT sum(size) FROM artifacts WHERE record_id=records.id),0)"
        }
    };
    let direction = if sort.desc { "DESC" } else { "ASC" };
    let sql = format!(
        "SELECT id,title,url,started_at,finished_at,status,metadata FROM records ORDER BY {order} {direction},id ASC LIMIT ?1 OFFSET ?2"
    );
    let entries = read_query(&tx, &sql, params![limit, offset])?;
    tx.commit().map_err(error)?;
    Ok(HistoryPage { entries, total })
}

/// BEGIN IMMEDIATE prevents a concurrent writer changing status between the
/// guard and DELETE. Foreign-key cascade removes artifact rows, not files.
pub(super) fn delete(path: &Path, id: &str) -> Result<(), String> {
    let mut connection = open(path)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(error)?;
    let version: u32 = tx
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)?;
    if version != 1 {
        return Err("History schema changed; reload required".into());
    }
    read(&tx)?;
    let running: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM records WHERE id=?1 AND status='Running')",
            params![id],
            |r| r.get(0),
        )
        .map_err(error)?;
    if running {
        return Err("Cannot delete a running download".into());
    }
    let count = tx
        .execute("DELETE FROM records WHERE id=?1", params![id])
        .map_err(error)?;
    if count == 0 {
        return Err("History entry not found".into());
    }
    tx.commit().map_err(error)
}
pub(super) fn upsert(path: &Path, entry: &HistoryEntry) -> Result<(), String> {
    let mut connection = open(path)?;
    let tx = connection.transaction().map_err(error)?;
    let version: u32 = tx
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)?;
    if version != 1 {
        return Err("History schema changed; reload required".into());
    }
    store(&tx, entry)?;
    read(&tx)?;
    tx.commit().map_err(error)
}
