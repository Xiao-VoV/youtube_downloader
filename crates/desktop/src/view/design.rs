//! Native surface tokens shared by every page; dark mode follows the existing theme.
use crate::app::Downloader;
use gpui_kit::{component::*, *};
fn color(cx: &App, light: u32, dark: u32) -> Hsla {
    rgb(if cx.theme().background.l < 0.5 {
        dark
    } else {
        light
    })
    .into()
}
pub(super) fn accent() -> Hsla {
    rgb(0x3466e8).into()
}
pub(super) fn canvas(cx: &App) -> Hsla {
    color(cx, 0xf5f6f8, 0x20242c)
}
pub(super) fn surface(cx: &App) -> Hsla {
    color(cx, 0xffffff, 0x252a33)
}
pub(super) fn sidebar(cx: &App) -> Hsla {
    color(cx, 0xeceef2, 0x1d222b)
}
pub(super) fn selection(cx: &App) -> Hsla {
    color(cx, 0xdce5fa, 0x293d63)
}
pub(super) fn border(cx: &App) -> Hsla {
    color(cx, 0xe7e9ee, 0x363e4a)
}
pub(super) fn text(cx: &App) -> Hsla {
    color(cx, 0x222631, 0xe5eaf2)
}
pub(super) fn muted(cx: &App) -> Hsla {
    color(cx, 0x737986, 0xa5afbf)
}
pub(super) fn glyph(name: &'static str) -> Icon {
    Icon::default().path(format!("icons/{name}.svg")).size_4()
}
pub(super) fn heading(title: impl Into<SharedString>) -> Div {
    div().text_sm().font_semibold().child(title.into())
}
pub(super) fn group(cx: &App) -> Div {
    div()
        .rounded_lg()
        .bg(surface(cx))
        .border_1()
        .border_color(border(cx))
}
pub(super) fn section(_cx: &Context<Downloader>, title: impl Into<SharedString>) -> Div {
    div().v_flex().gap_3().child(heading(title))
}
/// Neutral media fallback; never invent artwork or fetch arbitrary metadata URLs.
pub(super) fn media_tile(cx: &App, audio: bool) -> Div {
    div()
        .w(px(112.))
        .h(px(70.))
        .flex_shrink_0()
        .rounded_lg()
        .bg(canvas(cx))
        .text_color(muted(cx))
        .flex()
        .items_center()
        .justify_center()
        .child(glyph(if audio { "music" } else { "video" }))
}
