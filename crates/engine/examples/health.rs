//! Offline smoke check for the installed toolchain; never probes a URL.
//! Run: cargo run -p youtube-engine --example health
use youtube_engine::{CancelToken, Engine, EngineConfig, ToolPaths};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new(ToolPaths::discover()?, EngineConfig::default());
    let report = engine.health_check(&CancelToken::new());
    println!("{report:#?}");
    if !report.yt_dlp.healthy
        || !report.ffmpeg.as_ref().is_some_and(|tool| tool.healthy)
        || !report.ffprobe.as_ref().is_some_and(|tool| tool.healthy)
    {
        return Err("One or more local tools failed the offline health check".into());
    }
    Ok(())
}
