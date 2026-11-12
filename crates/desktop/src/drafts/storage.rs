//! Bounded private JSON snapshots. Damage is reported, never replaced.
use super::*;
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_DRAFTS: usize = 100;
static NEXT: AtomicU64 = AtomicU64::new(0);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    entries: Vec<Draft>,
}
pub(super) fn validate(draft: &Draft) -> Result<(), String> {
    if draft.id.is_empty()
        || draft.id.len() > 256
        || draft.url.len() > 2048
        || draft.url.chars().any(char::is_control)
    {
        return Err("Invalid draft identity or URL".into());
    }
    // Refuse rather than silently persisting authenticated proxy strings.
    if draft
        .profile
        .proxy
        .as_ref()
        .is_some_and(|p| p.contains('@'))
    {
        return Err(
            "Authenticated proxy is session-only; remove credentials before saving a draft".into(),
        );
    }
    if !draft.url.is_empty() && youtube_engine::normalize_url(&draft.url).is_err() {
        return Err("Draft URL must be a single YouTube video".into());
    }
    if draft
        .media
        .as_ref()
        .is_some_and(|m| m.id.is_empty() || m.title.is_empty() || m.formats.len() > 2000)
    {
        return Err("Invalid draft media metadata".into());
    }
    if draft.error != safe_error(draft.state, draft.error.is_some()) {
        return Err("Draft contains unsupported diagnostic text".into());
    }
    if serde_json::to_vec(draft)
        .map_err(|_| "Cannot encode draft")?
        .len()
        > 512 * 1024
    {
        return Err("Draft exceeds size limit".into());
    }
    Ok(())
}
fn checked_parent(path: &Path) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid draft path")?;
    match fs::symlink_metadata(parent) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(()),
        Ok(_) => Err("Unsafe draft directory".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Cannot inspect draft directory".into()),
    }
}
pub(super) fn read(path: &Path) -> Result<Vec<Draft>, String> {
    checked_parent(path)?;
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("Cannot inspect drafts; writes blocked".into()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_BYTES as u64 {
        return Err("Unsafe or oversized drafts; writes blocked".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "Cannot open drafts")?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read drafts")?;
    if bytes.len() > MAX_BYTES {
        return Err("Draft file exceeds size limit".into());
    }
    let document: Document =
        serde_json::from_slice(&bytes).map_err(|_| "Damaged drafts; writes blocked")?;
    if document.version != 1 || document.entries.len() > MAX_DRAFTS {
        return Err("Unsupported drafts version or count".into());
    }
    let mut ids = HashSet::new();
    for draft in &document.entries {
        validate(draft)?;
        if !ids.insert(&draft.id) {
            return Err("Duplicate draft IDs".into());
        }
    }
    Ok(document.entries)
}
pub(super) fn save(path: &Path, entries: &[Draft]) -> Result<(), String> {
    // Revalidate existing bytes every time. Do not overwrite later corruption.
    read(path)?;
    if entries.len() > MAX_DRAFTS {
        return Err("Draft count limit reached".into());
    }
    let mut ids = HashSet::new();
    for draft in entries {
        validate(draft)?;
        if !ids.insert(&draft.id) {
            return Err("Duplicate draft IDs".into());
        }
    }
    let bytes = serde_json::to_vec(&Document {
        version: 1,
        entries: entries.to_vec(),
    })
    .map_err(|_| "Cannot encode drafts")?;
    if bytes.len() > MAX_BYTES {
        return Err("Draft snapshot exceeds size limit".into());
    }
    let parent = path.parent().ok_or("Invalid draft path")?;
    fs::create_dir_all(parent).map_err(|_| "Cannot create draft directory")?;
    checked_parent(path)?;
    let temporary = parent.join(format!(
        ".drafts-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|_| "Cannot create private drafts transaction")?;
    let result = (|| {
        file.write_all(&bytes).map_err(|_| "Cannot write drafts")?;
        file.sync_all().map_err(|_| "Cannot sync drafts")?;
        fs::rename(&temporary, path).map_err(|_| "Cannot commit drafts")?;
        Ok(())
    })();
    // Exact owned temp sibling only. No media/draft-selected paths are deleted.
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
