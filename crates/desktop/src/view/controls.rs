//! App-owned controls. Native buttons retain focus, tooltips and keyboard activation.
use super::design;
use gpui_kit::{
    component::{
        button::*,
        input::{Input, InputState},
        *,
    },
    *,
};

#[derive(Clone, Copy)]
pub(super) enum Tone {
    Primary,
    Secondary,
    Quiet,
    Danger,
}

/// Set the variant itself: styling only the outer Div leaves the kit's hover/active colors.
pub(super) fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    tone: Tone,
    cx: &App,
) -> Button {
    let (bg, fg, hover, active, line) = match tone {
        Tone::Primary => (
            design::accent(),
            rgb(0xffffff).into(),
            rgb(0x285bd7).into(),
            rgb(0x204fc4).into(),
            design::accent(),
        ),
        Tone::Secondary => (
            design::surface(cx),
            design::text(cx),
            design::canvas(cx),
            design::selection(cx),
            design::border(cx),
        ),
        Tone::Quiet => (
            transparent_black(),
            design::muted(cx),
            design::canvas(cx),
            design::selection(cx),
            transparent_black(),
        ),
        Tone::Danger => (
            transparent_black(),
            cx.theme().danger,
            design::canvas(cx),
            design::selection(cx),
            transparent_black(),
        ),
    };
    Button::new(id)
        .small()
        .label(label)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(bg)
                .foreground(fg)
                .hover(hover)
                .active(active)
                .shadow(false),
        )
        .bg(bg)
        .text_color(fg)
        .border_1()
        .border_color(line)
        .rounded(px(8.))
        .h(px(34.))
        .px(px(12.))
}
fn transparent_black() -> Hsla {
    rgba(0x00000000).into()
}

pub(super) fn icon_button(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
    cx: &App,
) -> Button {
    button(id, "", Tone::Quiet, cx)
        .icon(icon)
        .tooltip(label)
        .w(px(30.))
        .h(px(30.))
        .px_0()
}

/// A joined switch: one backing surface, an inset selected segment, no per-option border.
pub(super) fn segment(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    cx: &App,
) -> Button {
    let bg = if selected {
        design::surface(cx)
    } else {
        design::canvas(cx)
    };
    let fg = if selected {
        design::text(cx)
    } else {
        design::muted(cx)
    };
    Button::new(id)
        .small()
        .label(label)
        .toggled(selected)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(bg)
                .foreground(fg)
                .hover(design::surface(cx))
                .active(design::selection(cx))
                .shadow(false),
        )
        .bg(bg)
        .text_color(fg)
        .border_1()
        .border_color(if selected {
            design::border(cx)
        } else {
            design::canvas(cx)
        })
        .rounded(px(6.))
        .h(px(28.))
        .px(px(12.))
}
pub(super) fn segments(cx: &App) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap(px(2.))
        .p(px(3.))
        .rounded(px(9.))
        .bg(design::canvas(cx))
}

/// Keep the native editable control and its focus ring; replace its size and surface.
pub(super) fn input(state: &Entity<InputState>, cx: &App) -> Input {
    Styled::h(
        Input::new(state)
            .small()
            .bg(design::surface(cx))
            .rounded(px(7.))
            .border_color(design::border(cx))
            .text_color(design::text(cx))
            .px(px(10.)),
        px(34.),
    )
}
pub(super) fn badge(text: impl Into<SharedString>, accent: bool, cx: &App) -> Div {
    div()
        .h_flex()
        .items_center()
        .px(px(7.))
        .py(px(3.))
        .rounded(px(5.))
        .bg(if accent {
            design::selection(cx)
        } else {
            design::canvas(cx)
        })
        .text_color(if accent {
            design::accent()
        } else {
            design::muted(cx)
        })
        .text_size(px(10.))
        .child(text.into())
}
pub(super) fn inset_row(
    label: impl Into<SharedString>,
    control: impl IntoElement,
    cx: &App,
) -> Div {
    div()
        .h_flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_4()
        .border_b_1()
        .border_color(design::border(cx))
        .child(div().text_size(px(13.)).child(label.into()))
        .child(control)
}
pub(super) fn page_title(title: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_size(px(22.))
        .font_semibold()
        .text_color(design::text(cx))
        .child(title.into())
}
