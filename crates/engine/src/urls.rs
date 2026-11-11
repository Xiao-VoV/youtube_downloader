use crate::{EngineError, ErrorKind, Result};
use url::Url;

/// Accept one YouTube video, drop tracking/list parameters, and canonicalize its
/// identity. A watch URL containing a list is still a single-video request.
pub fn normalize_url(input: &str) -> Result<String> {
    let bad = || {
        EngineError::new(
            ErrorKind::InvalidUrl,
            "expected a single HTTP(S) YouTube video URL",
        )
    };
    if input.chars().any(|c| c.is_control()) {
        return Err(bad());
    }
    let u = Url::parse(input.trim()).map_err(|_| bad())?;
    if !matches!(u.scheme(), "http" | "https")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
    {
        return Err(bad());
    }
    let host = u.host_str().ok_or_else(bad)?;
    let segments: Vec<_> = u.path().split('/').filter(|s| !s.is_empty()).collect();
    let id = match host {
        "youtu.be" | "www.youtu.be" if segments.len() == 1 => Some(segments[0].to_owned()),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            if u.path() == "/watch" {
                let ids: Vec<_> = u
                    .query_pairs()
                    .filter(|(k, _)| k == "v")
                    .map(|(_, v)| v.into_owned())
                    .collect();
                if ids.len() == 1 {
                    Some(ids[0].clone())
                } else {
                    None
                }
            } else if segments.len() == 2 && matches!(segments[0], "shorts" | "embed" | "live") {
                Some(segments[1].to_owned())
            } else {
                None
            }
        }
        _ => None,
    }
    .ok_or_else(bad)?;
    if id.len() != 11
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(bad());
    }
    Ok(format!("https://www.youtube.com/watch?v={id}"))
}
