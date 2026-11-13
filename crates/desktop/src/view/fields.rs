//! Compact, aligned native settings groups.
use super::design;
use gpui_kit::{component::*, *};
pub(super) fn row(label: Div, control: impl IntoElement, cx: &App) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap_4()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(design::border(cx))
        .child(div().w(px(132.)).flex_shrink_0().child(label.text_xs()))
        .child(div().flex_1().min_w_0().child(control))
}
