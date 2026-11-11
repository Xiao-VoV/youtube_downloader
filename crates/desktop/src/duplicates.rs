//! Duplicate identity is the normalized video URL, independent of output options.
pub(crate) fn same_video(left: &str, right: &str) -> bool {
    match (
        youtube_engine::normalize_url(left),
        youtube_engine::normalize_url(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
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
