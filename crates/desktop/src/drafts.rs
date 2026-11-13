//! Persisted user drafts only. Loading never spawns work or contacts a server.
mod storage;
#[cfg(test)]
mod tests;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use youtube_engine::{DownloadProfile, ErrorKind, MediaInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DraftState {
    Waiting,
    Probing,
    Ready,
    Failed,
    Cancelled,
    Queued,
    Interrupted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub id: String,
    pub url: String,
    pub profile: DownloadProfile,
    pub media: Option<MediaInfo>,
    pub state: DraftState,
    /// Full diagnostic in this session; disk snapshots contain generic summaries only.
    pub error: Option<String>,
}
pub struct DraftStore {
    pub entries: Vec<Draft>,
    pub error: Option<String>,
    path: Option<PathBuf>,
    blocked: bool,
}
impl DraftStore {
    pub fn load() -> Self {
        match std::env::var_os("HOME").filter(|v| !v.is_empty()) {
            Some(home) => Self::load_at(
                PathBuf::from(home).join("Library/Application Support/Shiguang/drafts.json"),
            ),
            None => Self {
                entries: Vec::new(),
                error: Some("Draft directory unavailable".into()),
                path: None,
                blocked: true,
            },
        }
    }
    fn load_at(path: PathBuf) -> Self {
        let mut store = Self {
            entries: Vec::new(),
            error: None,
            path: Some(path.clone()),
            blocked: false,
        };
        match storage::read(&path) {
            Ok(entries) => store.entries = entries,
            Err(error) => {
                store.error = Some(error);
                store.blocked = true;
                return store;
            }
        }
        let mut changed = false;
        for draft in &mut store.entries {
            if draft.state == DraftState::Probing {
                draft.state = DraftState::Interrupted;
                draft.error = Some("Probe interrupted".into());
                changed = true;
            }
        }
        if changed {
            if let Err(error) = store.persist(&store.entries) {
                store.error = Some(error);
                store.blocked = true;
            }
        }
        store
    }
    pub fn upsert(&mut self, mut draft: Draft) -> Result<(), String> {
        // Retain diagnostics for display/copy, but validate only the safe snapshot.
        if safe_error(draft.state, true).is_none() {
            draft.error = None;
        }
        storage::validate(&persisted_draft(&draft))?;
        let mut entries = self.entries.clone();
        if let Some(old) = entries.iter_mut().find(|d| d.id == draft.id) {
            *old = draft;
        } else {
            entries.push(draft);
        }
        self.commit(entries)
    }
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let mut entries = self.entries.clone();
        if !entries.iter().any(|d| d.id == id) {
            return Err("Draft not found".into());
        }
        entries.retain(|d| d.id != id);
        self.commit(entries)
    }
    /// Pure state/result application: never starts a probe. Unknown IDs and
    /// results arriving after cancellation/queuing do not mutate another draft.
    pub fn apply_probe(
        &mut self,
        id: &str,
        result: youtube_engine::Result<MediaInfo>,
    ) -> Result<(), String> {
        let mut draft = self
            .entries
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or("Draft not found")?;
        if draft.state != DraftState::Probing {
            return Err("Stale draft probe result".into());
        }
        match result {
            Ok(media) => {
                draft.media = Some(media);
                draft.state = DraftState::Ready;
                draft.error = None;
            }
            Err(error) => {
                draft.media = None;
                draft.state = if error.kind == ErrorKind::Cancelled {
                    DraftState::Cancelled
                } else {
                    DraftState::Failed
                };
                draft.error = Some(format!(
                    "{}\n{error}",
                    safe_error(draft.state, true).unwrap_or_default()
                ));
            }
        }
        self.upsert(draft)
    }
    fn persist(&self, entries: &[Draft]) -> Result<(), String> {
        if self.blocked {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| "Draft writes blocked".into()));
        }
        let snapshot: Vec<_> = entries.iter().map(persisted_draft).collect();
        storage::save(
            self.path.as_ref().ok_or("Draft path unavailable")?,
            &snapshot,
        )
    }
    fn commit(&mut self, entries: Vec<Draft>) -> Result<(), String> {
        if let Err(error) = self.persist(&entries) {
            self.error = Some(error.clone());
            return Err(error);
        }
        self.entries = entries;
        self.error = None;
        Ok(())
    }
}
fn safe_error(state: DraftState, present: bool) -> Option<String> {
    if !present {
        return None;
    }
    Some(match state {
        DraftState::Failed => "Probe failed".into(),
        DraftState::Cancelled => "Probe cancelled".into(),
        DraftState::Interrupted => "Probe interrupted".into(),
        _ => return None,
    })
}
fn persisted_draft(draft: &Draft) -> Draft {
    let mut snapshot = draft.clone();
    // Tool messages may contain credentials; never write them to drafts.json.
    snapshot.error = safe_error(snapshot.state, snapshot.error.is_some());
    snapshot
}

/// Bound only the on-screen preview, never the diagnostic copied by the user.
/// Character iteration avoids slicing inside UTF-8 and normalizes terminal controls.
pub fn error_preview(error: &str) -> String {
    let mut chars = error.chars();
    let mut preview: String = chars
        .by_ref()
        .take(1200)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}
/// Access-affecting options invalidate extracted metadata; output selection,
/// naming, quality and format only change the subsequent download plan.
pub fn requires_reprobe(before: &DownloadProfile, after: &DownloadProfile) -> bool {
    before.cookies_browser != after.cookies_browser
        || before.browser_profile != after.browser_profile
        || before.proxy != after.proxy
}
