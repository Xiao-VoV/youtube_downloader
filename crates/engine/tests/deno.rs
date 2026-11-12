#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use youtube_engine::*;
fn engine(deno: Option<PathBuf>) -> Engine {
    Engine::new(
        ToolPaths {
            yt_dlp: PathBuf::from("/bin/sh"),
            ffmpeg: None,
            ffprobe: None,
            deno,
        },
        EngineConfig::default(),
    )
}
#[test]
fn none_disables_runtime_without_implicit_discovery() {
    let e = engine(None);
    let plan = e
        .probe_plan("https://youtu.be/dQw4w9WgXcQ", &DownloadProfile::default())
        .unwrap();
    assert!(plan.argv.iter().any(|v| v == "--no-js-runtimes"));
    assert!(!plan.argv.iter().any(|v| v == "--js-runtimes"));
    assert!(e.health_check(&CancelToken::new()).deno.is_none());
}
#[test]
fn explicit_runtime_plan_and_health_use_selected_executable() {
    let root = std::env::temp_dir().join(format!("shiguang-deno-test-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let deno = root.join("custom deno");
    fs::write(&deno,"#!/bin/sh\n[ \"$#\" = 1 ] && [ \"$1\" = --version ] || exit 4\nprintf 'deno fake 1.0\\n'\n").unwrap();
    fs::set_permissions(&deno, fs::Permissions::from_mode(0o700)).unwrap();
    let e = engine(Some(deno.clone()));
    let profile = DownloadProfile::default();
    let probe = e
        .probe_plan("https://youtu.be/dQw4w9WgXcQ", &profile)
        .unwrap();
    let r = DownloadRequest {
        url: "https://youtu.be/dQw4w9WgXcQ".into(),
        mode: Mode::Video,
        output_dir: root.clone(),
        format_id: None,
        task_id: "t".into(),
        attempt_id: "a".into(),
    };
    let download = e.download_plan(&r, &profile).unwrap();
    for plan in [probe, download] {
        let index = plan.argv.iter().position(|v| v == "--js-runtimes").unwrap();
        assert_eq!(
            plan.argv[index + 1],
            format!("deno:{}", fs::canonicalize(&deno).unwrap().display())
        );
    }
    let health = e.health_check(&CancelToken::new()).deno.unwrap();
    assert!(health.healthy);
    assert_eq!(health.version.as_deref(), Some("deno fake 1.0"));
    let resolved = ToolManager::new(root.join("managed"))
        .resolve(&CustomToolPaths {
            yt_dlp: Some(PathBuf::from("/bin/sh")),
            deno: Some(deno),
            ..Default::default()
        })
        .unwrap();
    assert!(resolved.paths.deno.is_some());
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("shiguang-deno-test-"));
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn invalid_selected_runtime_fails_and_old_health_json_remains_compatible() {
    let e = engine(Some(PathBuf::from("/missing/shiguang-deno")));
    assert_eq!(
        e.probe_plan("https://youtu.be/dQw4w9WgXcQ", &DownloadProfile::default())
            .unwrap_err()
            .kind,
        ErrorKind::ToolNotFound
    );
    let report = e.health_check(&CancelToken::new());
    assert!(!report.deno.as_ref().unwrap().healthy);
    assert!(ToolManager::new(PathBuf::from("/tmp/unused-deno-manager"))
        .resolve(&CustomToolPaths {
            yt_dlp: Some(PathBuf::from("/bin/sh")),
            deno: Some(PathBuf::from("/missing/shiguang-deno")),
            ..Default::default()
        })
        .is_err());
    let mut value = serde_json::to_value(report).unwrap();
    value.as_object_mut().unwrap().remove("deno");
    assert!(serde_json::from_value::<HealthReport>(value)
        .unwrap()
        .deno
        .is_none());
}
