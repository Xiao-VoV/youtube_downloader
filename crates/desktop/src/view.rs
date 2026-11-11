//! Desktop shell: persistent navigation, task workspace, inspector and status bar.
use crate::app::Downloader;
use gpui_kit::component::*;
use gpui_kit::*;

mod inspector;
mod workspace;

pub(super) fn muted(cx: &Context<Downloader>, text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}
pub(super) fn eyebrow(cx: &Context<Downloader>, text: &'static str) -> Div {
    muted(cx, text).font_semibold()
}
impl Render for Downloader {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .v_flex()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar(cx))
                    .child(self.workspace(cx))
                    .child(self.inspector(cx)),
            )
            .child(self.status_bar(cx))
    }
}
impl Downloader {
    fn sidebar(&self, cx: &Context<Self>) -> Div {
        div()
            .w(px(176.))
            .flex_shrink_0()
            .h_full()
            .v_flex()
            .p_4()
            .gap_6()
            .bg(cx.theme().muted)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .items_center()
                    .py_3()
                    .child(
                        div()
                            .w(px(30.))
                            .h(px(30.))
                            .rounded_lg()
                            .bg(rgb(0xe84848))
                            .text_color(rgb(0xffffff))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("▶"),
                    )
                    .child(div().font_bold().text_base().child("拾光")),
            )
            .child(
                div().v_flex().gap_2().child(eyebrow(cx, "工作空间")).child(
                    div()
                        .h_flex()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(cx.theme().background)
                        .text_sm()
                        .font_semibold()
                        .child("↓  下载工作台")
                        .child("1"),
                ),
            )
            .child(div().flex_1())
            .child(
                div()
                    .v_flex()
                    .gap_2()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(div().text_xs().font_semibold().child("本地运行 · 隐私优先"))
                    .child(muted(cx, "不登录账户，不读取 Cookie"))
                    .child(muted(cx, "当前为单任务开发预览")),
            )
            .child(muted(cx, "SHIGUANG / 0.1"))
    }
    fn status_bar(&self, cx: &Context<Self>) -> Div {
        div()
            .h_flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted)
            .child(muted(cx, self.health.clone()))
            .child(muted(cx, "仅下载您拥有权利的内容"))
    }
}
