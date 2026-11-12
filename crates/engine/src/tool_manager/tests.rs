use super::*;
#[test]
fn install_only_activate_list_and_safe_remove() {
    let f = Fixture::new();
    let manager = ToolManager::new(f.0.clone());
    let one = release_info("2026.01.01").unwrap();
    let two = release_info("2026.02.02").unwrap();
    let first = manager.install_only_with(&one, &fake()).unwrap();
    assert!(manager.marker().unwrap().is_none());
    assert!(!manager.installed_versions().unwrap()[0].active);
    manager.activate(&one.version).unwrap();
    let second = manager.install_only_with(&two, &fake()).unwrap();
    assert_eq!(manager.marker().unwrap().unwrap().active, one.version);
    assert!(manager.remove_version(&one.version, &[]).is_err());
    assert!(
        manager
            .remove_version(&two.version, &[second.clone()])
            .is_err()
    );
    manager.activate(&two.version).unwrap();
    assert_eq!(
        manager.marker().unwrap().unwrap().previous,
        Some(one.version.clone())
    );
    assert_eq!(
        manager
            .installed_versions()
            .unwrap()
            .iter()
            .filter(|v| v.active)
            .count(),
        1
    );
    manager.remove_version(&one.version, &[]).unwrap();
    assert!(!first.exists());
    assert!(second.exists());
    assert!(manager.marker().unwrap().unwrap().previous.is_none());
}
#[test]
fn removal_rejects_unknown_files_and_nonexecutables() {
    let f = Fixture::new();
    let manager = ToolManager::new(f.0.clone());
    let release = release_info("2026.01.01").unwrap();
    let path = manager.install_only_with(&release, &fake()).unwrap();
    let unknown = path.parent().unwrap().join("do-not-delete");
    fs::write(&unknown, b"user data").unwrap();
    assert!(manager.remove_version(&release.version, &[]).is_err());
    assert!(path.exists() && unknown.exists());
    assert!(manager.activate(&release.version).is_err());
    fs::remove_file(unknown).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(manager.activate(&release.version).is_err());
        assert!(manager.remove_version(&release.version, &[]).is_err());
    }
}

struct Fake {
    payload: Vec<u8>,
    bad_checksum: bool,
    fail_asset: bool,
}
impl Transport for Fake {
    fn get(&self, url: &str, limit: usize) -> ManagerResult<Vec<u8>> {
        let bytes = if url == RELEASE_API {
            br#"{"tag_name":"2026.01.01"}"#.to_vec()
        } else if url.ends_with("/SHA2-256SUMS") {
            format!(
                "{:x}  yt-dlp_macos\n",
                Sha256::digest(if self.bad_checksum {
                    b"wrong"
                } else {
                    self.payload.as_slice()
                })
            )
            .into_bytes()
        } else if self.fail_asset {
            return Err(failure("fake unavailable"));
        } else {
            self.payload.clone()
        };
        assert!(bytes.len() <= limit);
        Ok(bytes)
    }
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "shiguang-tools-test-{}-{}",
            std::process::id(),
            TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(
            self.0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("shiguang-tools-test-")
        );
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[cfg(unix)]
#[test]
fn managed_root_version_binary_and_dangling_marker_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let target = f.0.join("target");
    fs::create_dir(&target).unwrap();
    let alias = f.0.join("alias");
    symlink(&target, &alias).unwrap();
    assert!(
        ToolManager::new(alias)
            .install_with(&release_info("2026.01.01").unwrap(), &fake())
            .is_err()
    );
    let root = f.0.join("managed");
    fs::create_dir(&root).unwrap();
    let manager = ToolManager::new(root.clone());
    symlink(target.join("missing"), root.join("active.json")).unwrap();
    assert!(manager.marker().is_err());
    fs::remove_file(root.join("active.json")).unwrap();
    symlink(&target, root.join("versions")).unwrap();
    assert!(
        manager
            .install_with(&release_info("2026.01.01").unwrap(), &fake())
            .is_err()
    );
    fs::remove_file(root.join("versions")).unwrap();
    manager
        .install_with(&release_info("2026.01.01").unwrap(), &fake())
        .unwrap();
    let binary = manager.binary("2026.01.01");
    fs::remove_file(&binary).unwrap();
    symlink("/bin/sh", &binary).unwrap();
    assert!(manager.resolve(&CustomToolPaths::default()).is_err());
    assert!(ToolManager::new(PathBuf::from("/")).marker().is_err());
    assert!(
        ToolManager::new(PathBuf::from("relative"))
            .marker()
            .is_err()
    );
}

fn fake() -> Fake {
    Fake {
        payload: b"fake nonexecuted binary".to_vec(),
        bad_checksum: false,
        fail_asset: false,
    }
}
#[test]
fn explicit_install_update_revert_and_priority() {
    let f = Fixture::new();
    let manager = ToolManager::new(f.0.clone());
    let transport = fake();
    let release = manager.check_with(&transport).unwrap();
    manager.install_with(&release, &transport).unwrap();
    let second = release_info("2026.02.02").unwrap();
    manager.install_with(&second, &transport).unwrap();
    assert_eq!(manager.marker().unwrap().unwrap().active, second.version);
    manager.revert().unwrap();
    assert_eq!(manager.marker().unwrap().unwrap().active, release.version);
    let resolved = manager.resolve(&CustomToolPaths::default()).unwrap();
    assert_eq!(resolved.yt_dlp_source, ToolSource::Managed);
    let custom = CustomToolPaths {
        yt_dlp: Some(PathBuf::from("/bin/sh")),
        ..Default::default()
    };
    assert_eq!(
        manager.resolve(&custom).unwrap().yt_dlp_source,
        ToolSource::Custom
    );
}
#[test]
fn repeated_active_or_retained_version_never_overwrites_binary() {
    let f = Fixture::new();
    let manager = ToolManager::new(f.0.clone());
    let first = release_info("2026.01.01").unwrap();
    manager.install_with(&first, &fake()).unwrap();
    let binary = manager.binary(&first.version);
    let bytes = fs::read(&binary).unwrap();
    let marker = fs::read(f.0.join("active.json")).unwrap();
    let error = manager.install_with(&first, &fake()).unwrap_err();
    assert!(error.contains("already active"));
    assert_eq!(fs::read(&binary).unwrap(), bytes);
    assert_eq!(fs::read(f.0.join("active.json")).unwrap(), marker);
    let second = release_info("2026.02.02").unwrap();
    manager.install_with(&second, &fake()).unwrap();
    let marker = fs::read(f.0.join("active.json")).unwrap();
    // Retained versions are immutable. Reuse is intentionally via revert,
    // never install-overwrite, even if a new remote payload has another hash.
    assert!(
        manager
            .install_with(&first, &fake())
            .unwrap_err()
            .contains("version directory exists")
    );
    assert_eq!(fs::read(&binary).unwrap(), bytes);
    assert_eq!(fs::read(f.0.join("active.json")).unwrap(), marker);
}

#[test]
fn failures_never_change_active_marker() {
    let f = Fixture::new();
    let manager = ToolManager::new(f.0.clone());
    manager
        .install_with(&release_info("2026.01.01").unwrap(), &fake())
        .unwrap();
    let marker = fs::read(f.0.join("active.json")).unwrap();
    for transport in [
        Fake {
            bad_checksum: true,
            ..fake()
        },
        Fake {
            fail_asset: true,
            ..fake()
        },
    ] {
        assert!(
            manager
                .install_with(&release_info("2026.02.01").unwrap(), &transport)
                .is_err()
        );
        assert_eq!(fs::read(f.0.join("active.json")).unwrap(), marker);
    }
    let mut malicious = release_info("2026.02.01").unwrap();
    malicious.asset_url = "https://evil.test/file".into();
    assert!(manager.install_with(&malicious, &fake()).is_err());
    assert_eq!(fs::read(f.0.join("active.json")).unwrap(), marker);
}
#[test]
fn versions_checksums_and_transport_hosts_fail_closed() {
    for version in ["../escape", "2026/01", "2026..01", "v2026", "2026?x"] {
        assert!(release_info(version).is_err());
    }
    assert!(checksum(b"none").is_err());
    assert!(
        checksum(
            format!(
                "{} yt-dlp_macos\n{} yt-dlp_macos\n",
                "a".repeat(64),
                "b".repeat(64)
            )
            .as_bytes()
        )
        .is_err()
    );
    for url in [
        "http://github.com/a",
        "https://evil.test/a",
        "https://github.com.evil.test/a",
        "https://user:pw@github.com/a",
        "https://github.com:444/a",
    ] {
        assert!(!transport::allowed(&url::Url::parse(url).unwrap()));
    }
    assert!(transport::allowed(
        &url::Url::parse("https://release-assets.githubusercontent.com/a").unwrap()
    ));
}
