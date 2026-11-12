#![cfg(unix)]
//! All process tests use local fixture executables, never real media tools/network.
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use youtube_engine::*;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    dir: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "youtube-engine-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self { dir }
    }
    fn engine(&self, body: &str) -> Engine {
        let executable = self.dir.join("fake-yt-dlp");
        fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let mut config = EngineConfig::default();
        config.terminate_grace = Duration::from_millis(50);
        config.probe_timeout = Duration::from_secs(3);
        config.download_timeout = Duration::from_secs(3);
        Engine::new(
            ToolPaths {
                yt_dlp: executable,
                ffmpeg: None,
                ffprobe: None,
                deno: None,
            },
            config,
        )
    }
    fn request(&self) -> DownloadRequest {
        DownloadRequest {
            url: "https://youtu.be/dQw4w9WgXcQ".into(),
            mode: Mode::Video,
            output_dir: self.dir.join("output"),
            format_id: None,
            task_id: "task".into(),
            attempt_id: "attempt".into(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
#[test]
fn normalize_single_video_and_reject_untrusted_inputs() {
    for input in [
        "https://youtu.be/dQw4w9WgXcQ?t=1",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL123",
        "http://m.youtube.com/shorts/dQw4w9WgXcQ",
        "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
    ] {
        assert_eq!(
            normalize_url(input).unwrap(),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }
    for input in [
        "https://youtube.com.evil.test/watch?v=dQw4w9WgXcQ",
        "https://evil.test/youtube.com",
        "https://user@youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtube.com/playlist?list=PL123",
        "file:///tmp/video",
        "https://youtu.be/../dQw4w9WgXcQ\n",
        "https://youtube.com/watch?v=dQw4w9WgXcQ&v=aaaaaaaaaaa",
        "https://youtube.com:444/watch?v=dQw4w9WgXcQ",
        "https://youtu.be/%64Qw4w9WgXcQ",
    ] {
        assert!(normalize_url(input).is_err(), "{input}");
    }
}
#[test]
fn progress_null_and_sentinel() {
    let p = parse_progress(
        r#"{"status":"downloading","downloaded_bytes":2,"total_bytes":"NA","eta":null}"#,
    )
    .unwrap();
    assert_eq!(p.downloaded_bytes, Some(2));
    assert_eq!(p.total_bytes, None);
    assert!(parse_progress("[]").is_err());
    assert!(parse_progress("not json").is_err());
}
#[test]
fn probe_uses_safe_arguments_and_serde() {
    let f = Fixture::new();
    let engine = f.engine(r#"
for required in --ignore-config --no-playlist --no-plugin-dirs --no-remote-components --skip-download --dump-single-json; do
  found=no
  for arg in "$@"; do if [ "$arg" = "$required" ]; then found=yes; fi; done
  if [ "$found" = no ]; then exit 9; fi
done
previous=''
for arg in "$@"; do
  if [ "$arg" = 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' ] && [ "$previous" != '--' ]; then exit 9; fi
  previous="$arg"
done
printf '%s\n' '{"id":"dQw4w9WgXcQ","title":"offline","formats":[{"format_id":"140","ext":"m4a"}]}'
"#);
    let info = engine.probe(&f.request().url, &CancelToken::new()).unwrap();
    assert_eq!(info.title, "offline");
    assert_eq!(info.formats[0].format_id, "140");
    assert!(serde_json::to_string(&info).unwrap().contains("offline"));
}
#[test]
fn download_validates_after_move_and_emits_scoped_events() {
    let f = Fixture::new();
    let engine = f.engine(
        r#"
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--paths' ]; then shift; case "$1" in home:*) destination="${1#home:}";; esac; fi
  shift
done
printf '%s\n' '__YT_ENGINE_PROGRESS__{"status":"downloading","downloaded_bytes":4}'
printf '%s\n' '__YT_ENGINE_POSTPROCESS__{"status":"started"}'
printf 'data' > "$destination/final.mp4"
printf '__YT_ENGINE_ARTIFACT__"%s/final.mp4"\n' "$destination"
"#,
    );
    let job = engine.spawn_download(f.request());
    let mut complete = false;
    for event in &job.events {
        assert_eq!(event.task_id, "task");
        assert_eq!(event.attempt_id, "attempt");
        if let EventKind::Completed(_) = event.kind {
            complete = true;
        }
    }
    let result = job.join().unwrap();
    assert!(complete);
    assert_eq!(result.artifacts[0].size, 4);
}
#[test]
fn success_without_artifact_and_nonzero_fail_closed() {
    let f = Fixture::new();
    let engine = f.engine("exit 0");
    assert_eq!(
        engine
            .download(&f.request(), &CancelToken::new(), None)
            .unwrap_err()
            .kind,
        ErrorKind::MissingArtifact
    );
    let engine = f.engine("exit 7");
    assert_eq!(
        engine
            .probe(&f.request().url, &CancelToken::new())
            .unwrap_err()
            .kind,
        ErrorKind::ProcessFailed
    );
}
#[test]
fn artifact_outside_attempt_is_rejected() {
    let f = Fixture::new();
    let engine = f.engine("printf '%s\\n' '__YT_ENGINE_ARTIFACT__\"/etc/hosts\"'");
    assert_eq!(
        engine
            .download(&f.request(), &CancelToken::new(), None)
            .unwrap_err()
            .kind,
        ErrorKind::UnsafeArtifact
    );
}
#[test]
fn bounded_stdout_and_stderr_backpressure() {
    let f = Fixture::new();
    let mut engine = f.engine(r#"
i=0
while [ "$i" -lt 10000 ]; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n'; printf 'stderr padding padding padding\n' >&2; i=$((i+1)); done
"#);
    engine.config.max_stdout_bytes = 1024;
    let at = Instant::now();
    assert_eq!(
        engine
            .probe(&f.request().url, &CancelToken::new())
            .unwrap_err()
            .kind,
        ErrorKind::OutputLimit
    );
    assert!(at.elapsed() < Duration::from_secs(3));
}
#[test]
fn long_progress_line_fails_bounded() {
    let f = Fixture::new();
    let mut engine = f.engine(
        "printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'",
    );
    engine.config.max_line_bytes = 32;
    assert_eq!(
        engine
            .download(&f.request(), &CancelToken::new(), None)
            .unwrap_err()
            .kind,
        ErrorKind::OutputLimit
    );
}
#[test]
fn cancellation_and_timeout_stop_term_ignoring_postprocessor() {
    let f = Fixture::new();
    // Both root and descendant ignore TERM. KILL must reach the whole group,
    // closing pipes and preventing the descendant's delayed side effect.
    let marker = f.dir.join("escaped-marker");
    let body = format!(
        "trap '' TERM\n(trap '' TERM; sleep 2; touch '{}') &\nwait",
        marker.display()
    );
    let mut engine = f.engine(&body);
    engine.config.download_timeout = Duration::from_millis(100);
    let at = Instant::now();
    assert_eq!(
        engine
            .download(&f.request(), &CancelToken::new(), None)
            .unwrap_err()
            .kind,
        ErrorKind::Timeout
    );
    assert!(at.elapsed() < Duration::from_secs(1));
    assert!(!marker.exists());
    let mut req = f.request();
    req.attempt_id = "cancel-attempt".into();
    engine.config.download_timeout = Duration::from_secs(10);
    let job = engine.spawn_download(req);
    std::thread::sleep(Duration::from_millis(100));
    job.cancel.cancel();
    for _ in &job.events {}
    assert_eq!(job.join().unwrap_err().kind, ErrorKind::Cancelled);
}
#[test]
fn playlist_metadata_is_rejected() {
    let f = Fixture::new();
    let engine = f.engine("printf '%s\\n' '{\"_type\":\"playlist\",\"entries\":[]}'");
    assert_eq!(
        engine
            .probe(&f.request().url, &CancelToken::new())
            .unwrap_err()
            .kind,
        ErrorKind::Protocol
    );
}

#[test]
fn symlink_artifact_escape_is_rejected() {
    let f = Fixture::new();
    let engine = f.engine(
        r#"
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--paths' ]; then shift; case "$1" in home:*) destination="${1#home:}";; esac; fi
  shift
done
ln -s /etc/hosts "$destination/final.mp4"
printf '__YT_ENGINE_ARTIFACT__"%s/final.mp4"\n' "$destination"
"#,
    );
    assert_eq!(
        engine
            .download(&f.request(), &CancelToken::new(), None)
            .unwrap_err()
            .kind,
        ErrorKind::UnsafeArtifact
    );
}

#[test]
fn tool_health_is_offline() {
    let f = Fixture::new();
    let engine = f.engine("printf 'fake-tool 1.0\\n'");
    let report = engine.health_check(&CancelToken::new());
    assert!(report.yt_dlp.healthy);
    assert_eq!(report.yt_dlp.version.as_deref(), Some("fake-tool 1.0"));
    assert!(report.yt_dlp.path.is_absolute());
}

#[test]
fn video_policy_is_h264_aac_mp4_and_1080p_without_transcode() {
    let f = Fixture::new();
    let engine = f.engine(r#"
selector=''
merge=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --format) shift; selector="$1" ;;
    --merge-output-format) shift; merge="$1" ;;
    --recode-video|--remux-video|--audio-format|--extract-audio) exit 19 ;;
  esac
  shift
done
[ "$merge" = mp4 ] || exit 18
[ "$selector" = 'bv[ext=mp4][height<=1080][vcodec^=avc1]+ba[ext=m4a][acodec^=mp4a]/b[ext=mp4][height<=1080][vcodec^=avc1][acodec^=mp4a]' ] || exit 17
"#);
    // Correct arguments cause successful fake exit; missing artifact remains
    // an intentional failure rather than a guessed successful download.
    assert_eq!(
        engine
            .download(&f.request(), &CancelToken::new(), None)
            .unwrap_err()
            .kind,
        ErrorKind::MissingArtifact
    );
    assert_eq!(DEFAULT_VIDEO_SELECTOR.split('/').count(), 2);
}

#[test]
fn successful_root_exit_kills_term_ignoring_descendant_promptly() {
    let f = Fixture::new();
    let engine = f.engine("(trap '' TERM; sleep 10) &\nprintf '%s\\n' '{\"id\":\"dQw4w9WgXcQ\",\"title\":\"offline\"}'\nexit 0");
    let at = Instant::now();
    assert!(engine.probe(&f.request().url, &CancelToken::new()).is_ok());
    assert!(at.elapsed() < Duration::from_secs(1));
}

#[test]
fn precancel_does_not_spawn() {
    let f = Fixture::new();
    let engine = f.engine("exit 99");
    let token = CancelToken::new();
    token.cancel();
    assert_eq!(
        engine.probe(&f.request().url, &token).unwrap_err().kind,
        ErrorKind::Cancelled
    );
}
