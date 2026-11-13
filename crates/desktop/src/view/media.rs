//! Shared media presentation. Remote strings are content, never icon paths or markup.
use super::{controls, design};
use crate::i18n::tr;
use gpui_kit::{
    component::{tooltip::Tooltip, *},
    *,
};
use youtube_engine::{DownloadProfile, MediaInfo, Mode, VideoContainer};

pub(super) fn title(id: impl Into<ElementId>, text: String, cx: &App) -> impl IntoElement {
    div()
        .id(id)
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_size(px(13.))
        .font_semibold()
        .text_color(design::text(cx))
        .tooltip({
            let full = text.clone();
            move |window, cx| Tooltip::new(full.clone()).build(window, cx)
        })
        .child(text)
}
pub(super) fn metadata(media: Option<&MediaInfo>, profile: &DownloadProfile, cx: &App) -> Div {
    let mut row = div().h_flex().items_center().flex_wrap().gap_2();
    if let Some(media) = media {
        if let Some(channel) = media.channel.as_ref().or(media.uploader.as_ref()) {
            row = row.child(
                div()
                    .max_w(px(230.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(11.))
                    .text_color(design::muted(cx))
                    .child(channel.clone()),
            );
        }
        if let Some(duration) = duration(media.duration) {
            row = row.child(
                div()
                    .text_size(px(11.))
                    .text_color(design::muted(cx))
                    .child(duration),
            );
        }
    }
    row.child(controls::badge(tr(format_name(profile)), false, cx))
        .children((profile.mode == Mode::Video).then(|| {
            controls::badge(
                if profile.best_resolution {
                    tr("最佳可用")
                } else {
                    format!("≤ {}p", profile.max_height)
                },
                false,
                cx,
            )
        }))
}
pub(super) fn format_name(profile: &DownloadProfile) -> &'static str {
    match profile.mode {
        Mode::Audio => "原音频",
        Mode::MP3 => "MP3",
        Mode::Video => match profile.container {
            VideoContainer::Mp4 => "MP4",
            VideoContainer::Mkv => "MKV",
        },
    }
}
pub(super) fn duration(seconds: Option<f64>) -> Option<String> {
    let value = seconds.filter(|s| s.is_finite() && *s >= 0.0)?;
    let seconds = value as u64;
    Some(if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    })
}
#[cfg(test)]
mod tests {
    #[test]
    fn duration_handles_unknown_and_long_media() {
        assert_eq!(super::duration(None), None);
        assert_eq!(super::duration(Some(f64::NAN)), None);
        assert_eq!(super::duration(Some(-1.0)), None);
        assert_eq!(super::duration(Some(0.0)).as_deref(), Some("0:00"));
        assert_eq!(super::duration(Some(1122.9)).as_deref(), Some("18:42"));
        assert_eq!(super::duration(Some(3661.0)).as_deref(), Some("1:01:01"));
    }
}
