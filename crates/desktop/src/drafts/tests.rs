use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static ID: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "shiguang-drafts-test-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn path(&self) -> PathBuf {
        self.0.join("drafts.json")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(
            self.0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("shiguang-drafts-test-")
        );
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn task_profile_edits_never_mutate_other_task_or_source_preset() {
    let f = Fixture::new();
    let mut store = DraftStore::load_at(f.path());
    let source = DownloadProfile::default();
    let mut first = draft("first");
    first.profile = source.clone();
    let mut second = draft("second");
    second.profile = source.clone();
    store.upsert(first).unwrap();
    store.upsert(second).unwrap();
    let mut edited = store.entries[0].clone();
    edited.profile.max_height = 2160;
    edited.profile.extra_args = vec!["--limit-rate".into(), "2M".into()];
    edited.state = DraftState::Queued;
    store.upsert(edited).unwrap();
    let reloaded = DraftStore::load_at(f.path());
    assert_eq!(source.max_height, 1080);
    assert_eq!(
        reloaded
            .entries
            .iter()
            .find(|d| d.id == "second")
            .unwrap()
            .profile,
        source
    );
    let queued = reloaded.entries.iter().find(|d| d.id == "first").unwrap();
    assert_eq!(queued.profile.max_height, 2160);
    assert_eq!(queued.profile.extra_args[1], "2M");
    assert_eq!(queued.state, DraftState::Queued);
}
fn draft(id: &str) -> Draft {
    Draft {
        id: id.into(),
        url: "https://youtu.be/dQw4w9WgXcQ".into(),
        profile: DownloadProfile::default(),
        media: None,
        state: DraftState::Waiting,
        error: None,
    }
}
#[test]
fn multiple_drafts_apply_probe_by_id_only() {
    let f = Fixture::new();
    let mut s = DraftStore::load_at(f.path());
    for id in ["a", "b"] {
        let mut d = draft(id);
        d.state = DraftState::Probing;
        s.upsert(d).unwrap();
    }
    s.apply_probe(
        "a",
        Ok(MediaInfo {
            id: "dQw4w9WgXcQ".into(),
            title: "Media title".into(),
            ..Default::default()
        }),
    )
    .unwrap();
    assert_eq!(s.entries[0].state, DraftState::Ready);
    assert_eq!(s.entries[1].state, DraftState::Probing);
    assert!(s.entries[1].media.is_none());
    assert!(
        s.apply_probe(
            "missing",
            Err(youtube_engine::EngineError::new(
                ErrorKind::Internal,
                "private"
            ))
        )
        .is_err()
    );
    assert!(
        s.apply_probe(
            "a",
            Err(youtube_engine::EngineError::new(
                ErrorKind::Internal,
                "private"
            ))
        )
        .is_err()
    );
    s.apply_probe(
        "b",
        Err(youtube_engine::EngineError::new(
            ErrorKind::Cancelled,
            "raw stderr password",
        )),
    )
    .unwrap();
    assert_eq!(s.entries[1].state, DraftState::Cancelled);
    assert!(
        !String::from_utf8(fs::read(f.path()).unwrap())
            .unwrap()
            .contains("password")
    );
}
#[test]
fn recovery_interrupts_probing_but_keeps_queued_without_running() {
    let f = Fixture::new();
    let mut s = DraftStore::load_at(f.path());
    let mut a = draft("a");
    a.state = DraftState::Probing;
    let mut b = draft("b");
    b.state = DraftState::Queued;
    s.upsert(a).unwrap();
    s.upsert(b).unwrap();
    let mut recovered = DraftStore::load_at(f.path());
    assert!(recovered.error.is_none());
    assert_eq!(recovered.entries[0].state, DraftState::Interrupted);
    assert_eq!(recovered.entries[1].state, DraftState::Queued);
    recovered.remove("a").unwrap();
    assert_eq!(DraftStore::load_at(f.path()).entries.len(), 1);
}
#[test]
fn only_access_configuration_requires_new_probe() {
    let before = DownloadProfile::default();
    let mut after = before.clone();
    after.max_height = 720;
    after.output_dir = Some(PathBuf::from("/tmp/another"));
    after.filename_template = "%(id)s.%(ext)s".into();
    after.mode = youtube_engine::Mode::MP3;
    assert!(!requires_reprobe(&before, &after));
    after.proxy = Some("http://localhost:8080".into());
    assert!(requires_reprobe(&before, &after));
    after = before.clone();
    after.cookies_browser = Some(youtube_engine::BrowserCookies::Firefox);
    assert!(requires_reprobe(&before, &after));
    after = before.clone();
    after.browser_profile = Some("Profile 1".into());
    assert!(requires_reprobe(&before, &after));
}
#[test]
fn corrupt_file_and_session_credentials_never_overwrite() {
    let f = Fixture::new();
    fs::write(f.path(), b"broken").unwrap();
    let mut blocked = DraftStore::load_at(f.path());
    assert!(blocked.error.is_some());
    assert!(blocked.upsert(draft("a")).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"broken");
    let f = Fixture::new();
    let mut s = DraftStore::load_at(f.path());
    s.upsert(draft("safe")).unwrap();
    let original = fs::read(f.path()).unwrap();
    let mut d = draft("secret");
    d.profile.proxy = Some("http://user:secret@localhost:8080".into());
    assert!(s.upsert(d).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), original);
    fs::write(f.path(), b"later corruption").unwrap();
    assert!(s.remove("safe").is_err());
    assert_eq!(s.entries.len(), 1);
}
#[cfg(unix)]
#[test]
fn private_permissions_symlinks_and_limits() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = Fixture::new();
    let mut s = DraftStore::load_at(f.path());
    s.upsert(draft("a")).unwrap();
    assert_eq!(
        fs::metadata(f.path()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let link = f.0.join("link.json");
    symlink(f.path(), &link).unwrap();
    assert!(DraftStore::load_at(link).error.is_some());
    let mut huge = draft("b");
    huge.id = "x".repeat(257);
    assert!(s.upsert(huge).is_err());
}
