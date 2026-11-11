#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use youtube_engine::*;
#[test]
fn ffprobe_fields_are_optional_and_format_family_is_not_extension() {
    let root = std::env::temp_dir().join(format!("yt-inspection-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let tool = root.join("ffprobe");
    let media = root.join("actual.mp4");
    fs::write(&media, b"fake").unwrap();
    fs::write(&tool,"#!/bin/sh\nprintf '%s' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":1920,\"height\":1080},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}],\"format\":{\"duration\":\"12.5\",\"format_name\":\"mov,mp4,m4a\"}}'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let e = Engine::new(
        ToolPaths {
            yt_dlp: PathBuf::from("/bin/sh"),
            ffmpeg: None,
            ffprobe: Some(tool.clone()),
        },
        EngineConfig::default(),
    );
    let info = e.inspect_media(&media, &CancelToken::new()).unwrap();
    assert_eq!(info.width, Some(1920));
    assert_eq!(info.height, Some(1080));
    assert_eq!(info.vcodec.as_deref(), Some("h264"));
    assert_eq!(info.acodec.as_deref(), Some("aac"));
    assert_eq!(info.duration, Some(12.5));
    assert_eq!(info.ext.as_deref(), Some("mp4"));
    fs::write(&tool, "#!/bin/sh\nprintf '{}'\n").unwrap();
    let info = e.inspect_media(&media, &CancelToken::new()).unwrap();
    assert!(
        info.width.is_none()
            && info.height.is_none()
            && info.duration.is_none()
            && info.vcodec.is_none()
            && info.acodec.is_none()
    );
    fs::write(&tool, "#!/bin/sh\nprintf '[]'\n").unwrap();
    assert_eq!(
        e.inspect_media(&media, &CancelToken::new())
            .unwrap_err()
            .kind,
        ErrorKind::Protocol
    );
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("yt-inspection-"));
    fs::remove_dir_all(root).unwrap();
}
