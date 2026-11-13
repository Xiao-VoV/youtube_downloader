//! Duplicate identity is the normalized video URL, independent of output options.
pub(crate) fn same_video(left: &str, right: &str) -> bool {
    if let (Some(left), Some(right)) = (youtube_id(left), youtube_id(right)) {
        return left == right;
    }
    match (
        youtube_engine::normalize_url(left),
        youtube_engine::normalize_url(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}
fn youtube_id(input: &str) -> Option<String> {
    let url = url::Url::parse(input.trim()).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let segments = url
        .path()
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let id = match host.as_str() {
        "youtu.be" | "www.youtu.be" if segments.len() == 1 => Some(segments[0].to_owned()),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com"
            if url.path() == "/watch" =>
        {
            url.query_pairs()
                .find(|(key, _)| key == "v")
                .map(|(_, value)| value.into_owned())
        }
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com"
            if segments.len() == 2 && matches!(segments[0], "shorts" | "embed" | "live") =>
        {
            Some(segments[1].to_owned())
        }
        _ => None,
    }?;
    (id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
    .then_some(id)
}
#[cfg(test)]
mod tests {
    #[test]
    fn alternate_links_match_but_invalid_or_other_video_do_not() {
        use super::same_video;
        assert!(same_video(
            "https://youtu.be/dQw4w9WgXcQ",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLtest"
        ));
        assert!(!same_video(
            "https://youtu.be/dQw4w9WgXcQ",
            "https://youtu.be/abcdefghijk"
        ));
        assert!(!same_video("not a url", "not a url"));
    }
}
