use std::path::PathBuf;
use youtube_engine::*;
fn engine() -> Engine {
    Engine::new(
        ToolPaths {
            yt_dlp: PathBuf::from("/bin/sh"),
            ffmpeg: None,
            ffprobe: None,
        },
        EngineConfig::default(),
    )
}
fn request() -> DownloadRequest {
    DownloadRequest {
        url: "https://youtu.be/dQw4w9WgXcQ".into(),
        mode: Mode::Video,
        output_dir: PathBuf::from("/tmp/output"),
        format_id: None,
        task_id: "t".into(),
        attempt_id: "a".into(),
    }
}
fn value<'a>(plan: &'a CommandPlan, flag: &str) -> &'a str {
    &plan.argv[plan.argv.iter().position(|s| s == flag).unwrap() + 1]
}
#[test]
fn old_config_defaults_and_new_fields_roundtrip() {
    let old: DownloadProfile = serde_json::from_str(r#"{"max_height":720}"#).unwrap();
    assert!(!old.best_resolution);
    assert!(old.browser_profile.is_none());
    let new = DownloadProfile {
        best_resolution: true,
        browser_profile: Some("Profile 2".into()),
        cookies_browser: Some(BrowserCookies::Chrome),
        ..old
    };
    let decoded: DownloadProfile =
        serde_json::from_str(&serde_json::to_string(&new).unwrap()).unwrap();
    assert_eq!(decoded, new);
}
#[test]
fn best_removes_height_only_and_preserves_container_policy() {
    for container in [VideoContainer::Mp4, VideoContainer::Mkv] {
        let profile = DownloadProfile {
            best_resolution: true,
            container: container.clone(),
            ..Default::default()
        };
        let plan = engine().download_plan(&request(), &profile).unwrap();
        let selector = value(&plan, "--format");
        assert!(!selector.contains("height"));
        if container == VideoContainer::Mp4 {
            assert!(selector.contains("avc1"));
            assert!(selector.contains("mp4a"));
        }
    }
}
#[test]
fn browser_path_is_one_shared_argument_and_display_is_redacted() {
    let profile = DownloadProfile {
        cookies_browser: Some(BrowserCookies::Firefox),
        browser_profile: Some("/Users/example/Browser Profiles/main".into()),
        proxy: Some("http://name:secret@localhost:8080".into()),
        ..Default::default()
    };
    let e = engine();
    let download = e.download_plan(&request(), &profile).unwrap();
    let probe = e.probe_plan(&request().url, &profile).unwrap();
    assert_eq!(
        value(&probe, "--cookies-from-browser"),
        "firefox:/Users/example/Browser Profiles/main"
    );
    assert_eq!(
        value(&probe, "--cookies-from-browser"),
        value(&download, "--cookies-from-browser")
    );
    let display = download.display_redacted_multiline();
    assert_eq!(display.lines().count(), download.argv.len() + 1);
    assert!(!display.contains("Browser Profiles"));
    assert!(!display.contains("secret"));
    assert!(display.contains("[REDACTED]"));
}
#[test]
fn cookie_grammar_and_missing_browser_fail_closed() {
    for name in [
        "",
        " ",
        " Profile 2",
        "Profile 2 ",
        "p::container",
        "p:keyring",
        "p+keyring",
        "p#keyring",
        "p\n--exec",
        "p\0x",
        "p\tx",
    ] {
        let p = DownloadProfile {
            cookies_browser: Some(BrowserCookies::Chrome),
            browser_profile: Some(name.into()),
            ..Default::default()
        };
        assert!(engine().probe_plan(&request().url, &p).is_err(), "{name:?}");
        assert!(engine().download_plan(&request(), &p).is_err(), "{name:?}");
    }
    let p = DownloadProfile {
        browser_profile: Some("Profile 2".into()),
        ..Default::default()
    };
    assert!(engine().probe_plan(&request().url, &p).is_err());
}
