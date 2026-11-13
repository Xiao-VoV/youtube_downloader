//! Native shell matching the approved quiet Mac prototype.
use crate::{app::Downloader, settings::Page};
use gpui_kit::{component::*, *};
mod controls;
mod design;
mod drafts;
mod fields;
mod help;
mod history;
mod history_actions;
mod media;
mod preferences;
mod profiles;
mod tools;
mod workspace;

pub(super) fn muted(cx: &Context<Downloader>, text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .text_color(design::muted(cx))
        .child(text.into())
}
impl Render for Downloader {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(message) = (!self.status.is_empty()).then(|| self.status.clone()) {
            window.push_notification(
                gpui_kit::component::notification::Notification::info(message).autohide(true),
                cx,
            );
            self.status.clear();
        }
        div()
            .size_full()
            .h_flex()
            .bg(design::surface(cx))
            .text_color(design::text(cx))
            .child(self.sidebar(cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .v_flex()
                    .h_full()
                    .child(self.toolbar(cx))
                    .child(
                        div()
                            .id("page")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p_5()
                            .bg(design::canvas(cx))
                            .child(match self.page {
                                Page::Tasks => self.tasks(cx),
                                Page::History => self.history_view(cx),
                                Page::New => self.draft_home(cx),
                                Page::Detail => self.draft_detail(cx),
                                Page::TaskOptions => self.draft_options(cx),
                                Page::Profiles => self.profile_editor(cx),
                                Page::Settings => self.software_settings(cx),
                            }),
                    ),
            )
    }
}
impl Downloader {
    fn sidebar(&self, cx: &Context<Self>) -> Div {
        let mut nav = div()
            .w(px(202.))
            .h_full()
            .flex_shrink_0()
            .v_flex()
            .p_3()
            .gap_1()
            .bg(design::sidebar(cx))
            .border_r_1()
            .border_color(design::border(cx))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .px_2()
                    .py_5()
                    .child(
                        div()
                            .w(px(36.))
                            .h(px(36.))
                            .rounded_lg()
                            .bg(design::accent())
                            .text_color(rgb(0xffffff))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(design::glyph("arrow-down-to-line")),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .child(crate::i18n::tr("拾光")),
                    ),
            );
        for (id, label, icon, page) in [
            ("new", "新建下载", "plus", Page::New),
            ("tasks", "下载队列", "list", Page::Tasks),
            ("history", "下载历史", "clock", Page::History),
            ("profiles", "下载配置", "sliders-horizontal", Page::Profiles),
        ] {
            let selected = self.page == page
                || (page == Page::New && matches!(self.page, Page::Detail | Page::TaskOptions));
            let count = match page {
                Page::Tasks => {
                    self.running.len()
                        + self
                            .drafts
                            .entries
                            .iter()
                            .filter(|d| {
                                d.state == crate::drafts::DraftState::Queued
                                    && !self.running.contains_key(&d.id)
                            })
                            .count()
                }
                Page::New => self
                    .drafts
                    .entries
                    .iter()
                    .filter(|d| d.state == crate::drafts::DraftState::Ready)
                    .count(),
                _ => 0,
            };
            nav = nav.child(
                div()
                    .id(id)
                    .h(px(38.))
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .bg(if selected {
                        design::selection(cx)
                    } else {
                        design::sidebar(cx)
                    })
                    .text_color(if selected {
                        design::accent()
                    } else {
                        design::muted(cx)
                    })
                    .hover(|s| s.bg(design::selection(cx)))
                    .child(design::glyph(icon))
                    .child(crate::i18n::tr(label))
                    .child(div().flex_1())
                    .children((count > 0).then(|| div().text_xs().child(count.to_string())))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if page == Page::New {
                            this.refresh_preset_select(window, cx);
                        }
                        if !this.busy || matches!(page, Page::Tasks | Page::History) {
                            if matches!(this.page, Page::TaskOptions) {
                                this.save_draft_editor(cx);
                            }
                            if page == Page::Profiles {
                                this.selected_draft = None;
                                this.pending = None;
                            }
                            this.page = page;
                            cx.notify();
                        }
                    })),
            );
        }
        nav.child(div().flex_1()).child(
            div()
                .id("settings-nav")
                .h(px(38.))
                .h_flex()
                .items_center()
                .gap_3()
                .px_3()
                .rounded_md()
                .text_sm()
                .cursor_pointer()
                .bg(if self.page == Page::Settings {
                    design::selection(cx)
                } else {
                    design::sidebar(cx)
                })
                .text_color(if self.page == Page::Settings {
                    design::accent()
                } else {
                    design::muted(cx)
                })
                .hover(|s| s.bg(design::selection(cx)))
                .child(design::glyph("settings"))
                .child(crate::i18n::tr("软件设置"))
                .on_click(cx.listener(|this, _, _, cx| {
                    if this.page == Page::TaskOptions {
                        this.save_draft_editor(cx);
                    }
                    this.page = Page::Settings;
                    cx.notify();
                })),
        )
    }
    fn breadcrumbs(&self, title: &'static str, cx: &Context<Self>) -> Div {
        let nested = matches!(self.page, Page::Detail | Page::TaskOptions);
        let mut row = div().h_flex().items_center().gap_2();
        if nested {
            row = row
                .child(
                    controls::button("crumb-new", "", controls::Tone::Quiet, cx)
                        .label(crate::i18n::tr("新建下载"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.page == Page::TaskOptions {
                                this.save_draft_editor(cx);
                            }
                            this.page = Page::New;
                            cx.notify();
                        })),
                )
                .child(muted(cx, "/"));
            if self.page == Page::TaskOptions {
                row = row
                    .child(
                        controls::button("crumb-preview", "", controls::Tone::Quiet, cx)
                            .label(crate::i18n::tr("下载预览"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.save_draft_editor(cx) {
                                    this.download(cx);
                                } else {
                                    this.pending = None;
                                }
                                this.page = Page::Detail;
                                cx.notify();
                            })),
                    )
                    .child(muted(cx, "/"));
            }
        }
        row.child(design::heading(crate::i18n::tr(title)))
    }
    fn toolbar(&self, cx: &Context<Self>) -> Div {
        let title = match self.page {
            Page::Tasks => "下载队列",
            Page::History => "下载历史",
            Page::New => "新建下载",
            Page::Profiles => "下载配置",
            Page::Detail => "下载预览",
            Page::TaskOptions => "本次下载选项",
            Page::Settings => "软件设置",
        };
        div()
            .h_flex()
            .h(px(64.))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_5()
            .bg(design::surface(cx))
            .border_b_1()
            .border_color(design::border(cx))
            .child(self.breadcrumbs(title, cx))
            .child(
                controls::button(
                    "add",
                    crate::i18n::tr("新建下载"),
                    controls::Tone::Primary,
                    cx,
                )
                .icon(IconName::Plus)
                .label(crate::i18n::tr("新建下载"))
                .disabled(self.busy)
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.page == Page::TaskOptions {
                        this.save_draft_editor(cx);
                    }
                    this.refresh_preset_select(window, cx);
                    this.page = Page::New;
                    cx.notify();
                })),
            )
    }
}
