//! Local preset storage. Never persist browser cookie contents or proxy credentials.
use gpui_kit::{
    component::input::{InputState, TextareaState},
    *,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use youtube_engine::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Tasks,
    New,
    Profiles,
    Confirm,
}

pub(crate) struct Settings {
    pub store: ProfileStore,
    pub profile: DownloadProfile,
    pub advanced_open: bool,
    pub name: Entity<InputState>,
    pub proxy: Entity<InputState>,
    pub extra: Entity<TextareaState>,
}
impl Settings {
    pub fn new(window: &mut Window, cx: &mut Context<crate::app::Downloader>) -> Self {
        let store = load().unwrap_or_default();
        let active = store
            .presets
            .iter()
            .find(|p| Some(&p.id) == store.active_preset_id.as_ref());
        let profile = active.map(|p| p.profile.clone()).unwrap_or_default();
        let name = active.map(|p| p.name.as_str()).unwrap_or("默认配置");
        let mut input = |text: &str, cx: &mut Context<crate::app::Downloader>| {
            cx.new(|cx| InputState::new(window, cx).default_value(text))
        };
        Self {
            name: input(name, cx),
            proxy: input(profile.proxy.as_deref().unwrap_or(""), cx),
            extra: cx.new(|cx| {
                TextareaState::new(window, cx).default_value(profile.extra_args.join("\n"))
            }),
            store,
            advanced_open: false,
            profile,
        }
    }
    pub fn snapshot(&self, cx: &App) -> DownloadProfile {
        let mut profile = self.profile.clone();
        let proxy = self.proxy.read(cx).value().trim().to_owned();
        profile.proxy = (!proxy.is_empty()).then_some(proxy);
        // One argv token per line: no shell parsing, substitutions or expansions.
        profile.extra_args = self
            .extra
            .read(cx)
            .value()
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        profile
    }
}
fn path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support/Shiguang/profiles.json"))
}
fn load() -> Option<ProfileStore> {
    load_at(&path()?)
}
fn load_at(path: &Path) -> Option<ProfileStore> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() > 1024 * 1024 {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
pub(crate) fn save(store: &ProfileStore) -> std::result::Result<(), String> {
    save_at(&path().ok_or("无法定位用户配置目录")?, store)
}
fn save_at(path: &Path, store: &ProfileStore) -> std::result::Result<(), String> {
    let bytes = encode_store(store)?;
    let parent = path.parent().ok_or("配置路径无效")?;
    fs::create_dir_all(parent).map_err(|_| "无法创建配置目录")?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    // Exclusive creation refuses preexisting files/symlinks; restrictive permissions
    // protect local paths and browsing preferences even though credentials are denied.
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|_| "无法创建配置临时文件")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "无法写入配置")?;
    // Both paths share the validated application-data parent; rename is atomic.
    fs::rename(&temporary, path).map_err(|_| "无法提交配置")?;
    Ok(())
}
fn encode_store(store: &ProfileStore) -> std::result::Result<Vec<u8>, String> {
    for preset in &store.presets {
        // Credentials may be used for this session, but never serialized to disk.
        if preset
            .profile
            .proxy
            .as_ref()
            .is_some_and(|p| p.contains('@'))
        {
            return Err("含认证信息的代理仅用于本次会话，请移除凭据后保存配置".into());
        }
    }
    serde_json::to_vec_pretty(store).map_err(|_| "无法编码配置".into())
}

#[cfg(test)]
mod tests {
    use super::{encode_store, load_at, save_at};
    use youtube_engine::{DownloadProfile, Preset, ProfileStore};
    fn store(proxy: &str) -> ProfileStore {
        ProfileStore {
            presets: vec![Preset {
                id: "local".into(),
                name: "常用".into(),
                profile: DownloadProfile {
                    proxy: Some(proxy.into()),
                    ..Default::default()
                },
            }],
            active_preset_id: Some("local".into()),
        }
    }
    #[test]
    fn credentials_never_reach_serialized_presets() {
        assert!(encode_store(&store("http://user:secret@localhost:8080")).is_err());
        assert!(encode_store(&store("socks5://user@localhost:1080")).is_err());
    }
    #[test]
    fn persisted_presets_reload_and_rejected_save_preserves_existing() {
        let root = std::env::temp_dir().join(format!(
            "shiguang-presets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("profiles.json");
        let mut original = store("http://localhost:8080");
        let mut second = original.presets[0].clone();
        second.id = "second".into();
        second.name = "音频".into();
        second.profile.mode = youtube_engine::Mode::MP3;
        original.presets.push(second);
        original.active_preset_id = Some("second".into());
        save_at(&path, &original).unwrap();
        let loaded = load_at(&path).unwrap();
        assert_eq!(loaded.presets.len(), 2);
        assert_eq!(loaded.active_preset_id.as_deref(), Some("second"));
        assert_eq!(loaded.presets[1].profile.mode, youtube_engine::Mode::MP3);
        let before = std::fs::read(&path).unwrap();
        assert!(save_at(&path, &store("http://user:secret@localhost:8080")).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        // Only remove this test's exclusively named file and now-empty directory.
        assert_eq!(path.parent(), Some(root.as_path()));
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&root).unwrap();
    }
    #[test]
    fn local_presets_roundtrip_without_filesystem_effects() {
        let bytes = encode_store(&store("http://localhost:8080")).unwrap();
        let decoded: ProfileStore = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.presets[0].name, "常用");
        assert_eq!(decoded.active_preset_id.as_deref(), Some("local"));
        assert!(decoded.presets[0].profile.cookies_browser.is_none());
    }
}
