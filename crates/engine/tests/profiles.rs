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
        output_dir: PathBuf::from("/tmp/video output"),
        format_id: None,
        task_id: "task".into(),
        attempt_id: "attempt".into(),
    }
}
#[test]
fn defaults_roundtrip_and_presets() {
    let p = DownloadProfile::default();
    assert!(p.cookies_browser.is_none());
    assert_eq!(p.max_height, 1080);
    let store = ProfileStore {
        active_preset_id: Some("one".into()),
        presets: vec![
            Preset {
                id: "one".into(),
                name: "Default".into(),
                profile: p.clone(),
            },
            Preset {
                id: "two".into(),
                name: "Other".into(),
                profile: p.clone(),
            },
        ],
    };
    let decoded: ProfileStore =
        serde_json::from_str(&serde_json::to_string(&store).unwrap()).unwrap();
    assert_eq!(decoded.presets.len(), 2);
    assert_eq!(decoded.presets[0].profile, p);
}
#[test]
fn preview_is_argv_and_redacts_credentials() {
    let p = DownloadProfile {
        proxy: Some("http://user:secret@localhost:8080".into()),
        cookies_browser: Some(BrowserCookies::Firefox),
        max_height: 720,
        ..Default::default()
    };
    let e = engine();
    let d = e.download_plan(&request(), &p).unwrap();
    let probe = e.probe_plan(&request().url, &p).unwrap();
    for option in ["--proxy", "--cookies-from-browser"] {
        let index = d.argv.iter().position(|v| v == option).unwrap();
        let pi = probe.argv.iter().position(|v| v == option).unwrap();
        assert_eq!(d.argv[index + 1], probe.argv[pi + 1]);
    }
    assert!(d.argv.iter().any(|v| v.contains("height<=720")));
    assert!(d
        .argv
        .iter()
        .any(|v| v == "/tmp/video output/youtube-attempt"));
    assert!(!d.display_redacted().contains("secret"));
    assert!(!format!("{d:?}").contains("secret"));
    assert!(d.display_redacted().contains("[REDACTED]"));
}
#[test]
fn advanced_flags_fail_closed() {
    for args in [
        vec!["--exec", "echo hacked"],
        vec!["--config-location", "x"],
        vec!["--paths", "/tmp"],
        vec!["--print", "x"],
        vec!["--plugin-dirs", "x"],
        vec!["--unknown"],
        vec!["--retries", "--exec"],
        vec!["--proxy=x"],
    ] {
        let p = DownloadProfile {
            extra_args: args.into_iter().map(str::to_owned).collect(),
            ..Default::default()
        };
        assert!(engine().download_plan(&request(), &p).is_err());
    }
    let p = DownloadProfile {
        extra_args: vec![
            "--write-subs",
            "--sub-langs",
            "en,zh-Hans",
            "--limit-rate",
            "2M",
            "--retries",
            "3",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        ..Default::default()
    };
    assert!(engine().download_plan(&request(), &p).is_ok());
}
#[test]
fn preview_does_not_create_directories() {
    let root = std::env::temp_dir().join(format!("yt-plan-only-{}", std::process::id()));
    assert!(!root.exists());
    let p = DownloadProfile {
        output_dir: Some(root.clone()),
        container: VideoContainer::Mkv,
        ..Default::default()
    };
    let plan = engine().download_plan(&request(), &p).unwrap();
    assert!(!root.exists());
    assert!(plan.argv.iter().any(|v| v == "mkv"));
}

#[cfg(unix)]
#[test]
fn executed_download_matches_preview_exactly() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let root = std::env::temp_dir().join(format!("yt-plan-execute-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let script = root.join("tool");
    let record = root.join("argv");
    fs::write(&script, format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nwhile [ \"$#\" -gt 0 ]; do\nif [ \"$1\" = --paths ]; then shift; destination=\"$1\"; fi\nshift\ndone\nprintf data > \"$destination/final.mp4\"\nprintf '__YT_ENGINE_ARTIFACT__\"%s/final.mp4\"\\n' \"$destination\"\n",record.display())).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let e = Engine::new(
        ToolPaths {
            yt_dlp: script,
            ffmpeg: None,
            ffprobe: None,
        },
        EngineConfig::default(),
    );
    let p = DownloadProfile {
        output_dir: Some(root.join("output")),
        proxy: Some("http://user:password@localhost:8080".into()),
        cookies_browser: Some(BrowserCookies::Firefox),
        ..Default::default()
    };
    let preview = e.download_plan(&request(), &p).unwrap();
    let result = e
        .download_with_profile(&request(), &p, &CancelToken::new(), None)
        .unwrap();
    assert_eq!(result.artifacts.len(), 1);
    let actual = fs::read_to_string(record).unwrap();
    assert_eq!(
        actual.lines().collect::<Vec<_>>(),
        preview.argv.iter().map(String::as_str).collect::<Vec<_>>()
    );
    // Fixture owns this exact unique directory, never a user-configured path.
    assert!(root.starts_with(std::env::temp_dir()));
    fs::remove_dir_all(root).unwrap();
}
