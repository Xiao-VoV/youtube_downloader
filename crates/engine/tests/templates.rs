use std::path::PathBuf;
use youtube_engine::*;
fn engine() -> Engine {
    Engine::new(
        ToolPaths {
            yt_dlp: PathBuf::from("/bin/sh"),
            ffmpeg: None,
            ffprobe: None,
            deno: None,
        },
        EngineConfig::default(),
    )
}
fn request() -> DownloadRequest {
    DownloadRequest {
        url: "https://youtu.be/dQw4w9WgXcQ".into(),
        mode: Mode::Video,
        output_dir: PathBuf::from("/tmp/template-output"),
        format_id: None,
        task_id: "t".into(),
        attempt_id: "a".into(),
    }
}
#[test]
fn safe_templates_and_private_temp_are_planned() {
    let p = DownloadProfile {
        folder_template: Some("%(extractor)s/%(channel)s".into()),
        ..Default::default()
    };
    let plan = engine().download_plan(&request(), &p).unwrap();
    assert!(plan.argv.iter().any(|v| v == "home:/tmp/template-output"));
    assert!(plan
        .argv
        .iter()
        .any(|v| v == "temp:/tmp/template-output/.shiguang-tmp/a"));
    assert!(plan
        .argv
        .iter()
        .any(|v| v == "%(extractor)s/%(channel)s/%(title).180B [%(id)s].%(ext)s"));
    let old: DownloadProfile = serde_json::from_str("{}").unwrap();
    assert_eq!(old.filename_template, "%(title).180B [%(id)s].%(ext)s");
    assert!(old.folder_template.is_none());
    assert!(plan.argv.iter().any(|a| a == "--no-restrict-filenames"));
    assert!(plan.argv.iter().any(|a| a == "--windows-filenames"));
}
#[test]
fn malicious_templates_fail_closed() {
    for name in [
        "/absolute.%(ext)s",
        "../escape.%(ext)s",
        "%(title)s/escape.%(ext)s",
        "%(title)s\\escape.%(ext)s",
        "%(unknown)s.%(ext)s",
        "%(title|fallback)s.%(ext)s",
        "%(title).9999B.%(ext)s",
        "%(id)s.mp4",
        "x\n.%(ext)s",
    ] {
        let p = DownloadProfile {
            filename_template: name.into(),
            ..Default::default()
        };
        assert!(engine().download_plan(&request(), &p).is_err(), "{name}");
    }
    for folder in [
        "/absolute",
        "../escape",
        "a/../b",
        "a//b",
        "a\\b",
        "%(title|x)s",
        ".shiguang-tmp",
        "%(ext)s",
    ] {
        let p = DownloadProfile {
            folder_template: Some(folder.into()),
            ..Default::default()
        };
        assert!(engine().download_plan(&request(), &p).is_err(), "{folder}");
    }
}
#[cfg(unix)]
#[test]
fn existing_skipped_file_is_not_a_success() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let root = std::env::temp_dir().join(format!("yt-template-conflict-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let output = root.join("output");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("existing.mp4"), b"original").unwrap();
    let tool = root.join("tool");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nprintf '__YT_ENGINE_ARTIFACT__\"{}/existing.mp4\"\\n'\n",
            output.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let e = Engine::new(
        ToolPaths {
            yt_dlp: tool,
            ffmpeg: None,
            ffprobe: None,
            deno: None,
        },
        EngineConfig::default(),
    );
    let p = DownloadProfile {
        output_dir: Some(output.clone()),
        ..Default::default()
    };
    assert_eq!(
        e.download_with_profile(&request(), &p, &CancelToken::new(), None)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidRequest
    );
    assert_eq!(fs::read(output.join("existing.mp4")).unwrap(), b"original");
    assert!(output.join(".shiguang-tmp/a").is_dir());
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("yt-template-conflict-"));
    fs::remove_dir_all(root).unwrap();
}
