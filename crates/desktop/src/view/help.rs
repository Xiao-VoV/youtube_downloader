//! Trusted documentation links only; remote metadata never supplies these URLs.
use crate::i18n::tr;
use gpui_kit::{
    component::{button::*, *},
    *,
};
pub(super) fn label(text: &'static str, tip: &'static str, section: &'static str) -> Div {
    let url = format!("https://github.com/yt-dlp/yt-dlp#{}", section);
    div()
        .h_flex()
        .items_center()
        .gap_2()
        .child(div().text_xs().child(tr(text)))
        .child(
            Button::new(SharedString::from(format!("help-{text}")))
                .small()
                .ghost()
                .xsmall()
                .icon(IconName::Info)
                .tooltip(tr(tip))
                .on_click(move |_, _, cx| cx.open_url(&url)),
        )
}
