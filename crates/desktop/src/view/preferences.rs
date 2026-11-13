//! Focused settings pages: appearance first, path editing only on demand.
use super::{controls, design, muted};
use crate::{app::Downloader, i18n::tr, preferences::Appearance};
use gpui_kit::{component::*, *};

impl Downloader {
    pub(super) fn software_settings(&self, cx: &Context<Self>) -> Div {
        let mut tabs = controls::segments(cx);
        for (id, label, tools) in [
            ("settings-general", "通用", false),
            ("settings-tools", "下载工具", true),
        ] {
            tabs = tabs.child(
                controls::segment(id, tr(label), self.settings_tools_tab == tools, cx).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.settings_tools_tab = tools;
                        cx.notify();
                    }),
                ),
            );
        }
        let page = div()
            .v_flex()
            .gap_5()
            .w_full()
            .max_w(px(900.))
            .mx_auto()
            .child(div().h_flex().justify_start().child(tabs));
        if self.settings_tools_tab {
            return page.child(
                design::group(cx)
                    .v_flex()
                    .p_5()
                    .gap_5()
                    .child(self.tool_controls(cx)),
            );
        }
        let mut looks = div().h_flex().gap_3().w_full();
        for (id, label, value) in [
            ("appearance-system", "跟随系统", Appearance::System),
            ("appearance-light", "浅色", Appearance::Light),
            ("appearance-dark", "深色", Appearance::Dark),
        ] {
            let selected = self.appearance == value;
            looks = looks.child(
                div()
                    .v_flex()
                    .gap_2()
                    .flex_1()
                    .min_w_0()
                    .child(appearance_preview(value, selected, cx))
                    .child(
                        controls::segment(id, tr(label), selected, cx)
                            .w_full()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                match crate::preferences::save(value) {
                                    Ok(()) => {
                                        this.appearance = value;
                                        crate::preferences::apply(value, window, cx);
                                    }
                                    Err(error) => this.status = error,
                                }
                                cx.notify();
                            })),
                    ),
            );
        }
        let mut languages = controls::segments(cx);
        for (id, label, value) in [
            ("language-zh", "简体中文", crate::i18n::Language::Zh),
            ("language-en", "English", crate::i18n::Language::En),
        ] {
            languages = languages.child(
                controls::segment(id, label, crate::i18n::language() == value, cx).on_click(
                    cx.listener(move |this, _, _, cx| {
                        match crate::preferences::save_language(value) {
                            Ok(()) => {
                                crate::i18n::set_language(value);
                                cx.refresh_windows();
                            }
                            Err(error) => this.status = error,
                        }
                        cx.notify();
                    }),
                ),
            );
        }
        page.child(
            design::group(cx)
                .v_flex()
                .child(
                    div()
                        .v_flex()
                        .gap_4()
                        .p_5()
                        .child(design::heading(tr("外观")))
                        .child(looks),
                )
                .child(controls::inset_row(tr("语言"), languages, cx)),
        )
        .child(
            design::group(cx)
                .h_flex()
                .items_center()
                .gap_3()
                .p_4()
                .child(
                    div()
                        .w(px(36.))
                        .h(px(36.))
                        .rounded_lg()
                        .bg(design::selection(cx))
                        .text_color(design::accent())
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(design::glyph("arrow-down-to-line")),
                )
                .child(
                    div()
                        .v_flex()
                        .gap_1()
                        .flex_1()
                        .child(div().text_sm().font_semibold().child(tr("拾光")))
                        .child(muted(
                            cx,
                            format!("{} {}", tr("版本"), env!("CARGO_PKG_VERSION")),
                        )),
                ),
        )
    }
}

/// Drawn with GPUI primitives: previews communicate the setting without extra prose.
fn appearance_preview(value: Appearance, selected: bool, cx: &App) -> Div {
    let dark = value == Appearance::Dark;
    let bg: Hsla = rgb(if dark { 0x252c38 } else { 0xffffff }).into();
    let side: Hsla = rgb(if dark { 0x1b2230 } else { 0xe9edf5 }).into();
    let line: Hsla = rgb(if dark { 0x414e63 } else { 0xe3e8f1 }).into();
    let preview = div()
        .h(px(106.))
        .w_full()
        .overflow_hidden()
        .rounded(px(9.))
        .border_1()
        .border_color(if selected {
            design::accent()
        } else {
            design::border(cx)
        })
        .bg(bg)
        .h_flex()
        .child(
            div()
                .w(px(36.))
                .h_full()
                .bg(side)
                .v_flex()
                .gap_2()
                .px_2()
                .py_3()
                .child(
                    div()
                        .w(px(16.))
                        .h(px(4.))
                        .rounded_full()
                        .bg(design::accent()),
                )
                .child(div().w(px(14.)).h(px(3.)).rounded_full().bg(line))
                .child(div().w(px(14.)).h(px(3.)).rounded_full().bg(line)),
        );
    let mut content = div().flex_1().v_flex().gap_2().p_3();
    if value == Appearance::System {
        content = content.child(
            div()
                .h_flex()
                .gap_1()
                .child(div().flex_1().h(px(27.)).rounded(px(4.)).bg(rgb(0xeef2f8)))
                .child(div().flex_1().h(px(27.)).rounded(px(4.)).bg(rgb(0x303d53))),
        );
    } else {
        content = content.child(div().w_full().h(px(27.)).rounded(px(4.)).bg(side));
    }
    content = content
        .child(div().w_full().h(px(4.)).rounded_full().bg(line))
        .child(div().w(px(46.)).h(px(4.)).rounded_full().bg(line));
    preview.child(content)
}
