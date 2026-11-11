//! Shared visual primitives for the design branch. No domain state lives here.
use crate::app::Downloader;
use gpui_kit::{component::*, *};
pub(super) fn accent() -> Hsla {
    rgb(0x3975e8).into()
}
pub(super) fn glyph(name: &'static str) -> Icon {
    Icon::default().path(format!("icons/{name}.svg")).size_4()
}
pub(super) fn section(cx: &Context<Downloader>, title: impl Into<SharedString>) -> Div {
    div()
        .v_flex()
        .gap_4()
        .p_5()
        .rounded_lg()
        .bg(cx.theme().background)
        .border_1()
        .border_color(cx.theme().border)
        .child(div().text_sm().font_semibold().child(title.into()))
}
