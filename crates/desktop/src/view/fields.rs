//! Aligned settings rows keep labels, help affordances and inputs on one grid.
use gpui_kit::{component::*, *};
pub(super) fn row(label: Div, control: impl IntoElement, cx: &App) -> Div {
    div()
        .h_flex()
        .items_start()
        .gap_5()
        .py_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(div().w(px(150.)).flex_shrink_0().pt_1().child(label))
        .child(div().flex_1().min_w_0().child(control))
}
pub(super) fn panel(title: impl Into<SharedString>, icon: IconName, cx: &App) -> Div {
    div()
        .v_flex()
        .p_5()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_2()
                .pb_3()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(icon).size_4())
                .child(div().text_sm().font_semibold().child(title.into())),
        )
}
