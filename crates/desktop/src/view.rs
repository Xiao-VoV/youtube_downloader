//! Minimal desktop shell. Each workflow has a separate page, not a permanent inspector.
use crate::{app::Downloader, settings::Page};
use gpui_kit::{
    component::{button::*, *},
    *,
};
mod design;
mod help;
mod history;
mod profiles;
mod tools;
mod workspace;

pub(super) fn muted(cx: &Context<Downloader>, text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}
impl Render for Downloader {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.status_observed != self.status {
            self.status_observed = self.status.clone();
            self.status_since = std::time::Instant::now();
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(6))
                    .await;
                let _ = this.update(cx, |_, cx| cx.notify());
            })
            .detach();
        }
        div()
            .size_full()
            .h_flex()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.sidebar(cx))
            .child(
                div()
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
                            .child(match self.page {
                                Page::Tasks => self.tasks(cx),
                                Page::History => self.history_view(cx),
                                Page::New => self.new_download(cx),
                                Page::Profiles => self.profile_editor(cx),
                                Page::Confirm => self.confirmation(cx),
                                Page::Settings => self.software_settings(cx),
                            }),
                    )
                    .child(
                        div()
                            .h_flex()
                            .h(px(26.))
                            .flex_shrink_0()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().muted)
                            .child(muted(
                                cx,
                                if !self.status.is_empty()
                                    && self.status_since.elapsed()
                                        < std::time::Duration::from_secs(6)
                                {
                                    self.status.clone()
                                } else if self.busy {
                                    crate::i18n::tr("正在解析视频信息…")
                                } else {
                                    format!("{} · {}", crate::i18n::tr("就绪"), self.running.len())
                                },
                            ))
                            .child(muted(
                                cx,
                                if self.tools.busy || self.tools.checking {
                                    crate::i18n::tr("正在处理工具请求…")
                                } else {
                                    String::new()
                                },
                            )),
                    ),
            )
    }
}
impl Downloader {
    fn sidebar(&self, cx: &Context<Self>) -> Div {
        let mut nav = div()
            .w(px(208.))
            .h_full()
            .flex_shrink_0()
            .v_flex()
            .p_3()
            .gap_1()
            .bg(cx.theme().muted)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_5()
                    .child(
                        div()
                            .w(px(30.))
                            .h(px(30.))
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
                            .font_semibold()
                            .text_sm()
                            .child(crate::i18n::tr("视频下载器")),
                    ),
            );
        for (id, label, icon, page) in [
            ("new", "新建任务", "plus", Page::New),
            ("tasks", "下载队列", "list", Page::Tasks),
            ("history", "下载历史", "clock", Page::History),
            ("profiles", "下载配置", "sliders-horizontal", Page::Profiles),
        ] {
            let selected = self.page == page || (page == Page::New && self.page == Page::Confirm);
            nav = nav.child(
                div()
                    .id(id)
                    .h(px(36.))
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .bg(if selected {
                        cx.theme().background
                    } else {
                        cx.theme().muted
                    })
                    .text_color(if selected {
                        design::accent()
                    } else {
                        cx.theme().muted_foreground
                    })
                    .hover(|s| s.bg(cx.theme().background))
                    .child(design::glyph(icon))
                    .child(crate::i18n::tr(label))
                    .child(div().flex_1())
                    .children(
                        (page == Page::Tasks && !self.running.is_empty())
                            .then(|| div().text_xs().child(self.running.len().to_string())),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.busy || matches!(page, Page::Tasks | Page::History) {
                            this.page = page;
                            cx.notify();
                        }
                    })),
            );
        }
        nav.child(div().flex_1())
            .child(div().h(px(1.)).my_2().bg(cx.theme().border))
            .child(
                Button::new("settings-nav")
                    .ghost()
                    .small()
                    .icon(IconName::Settings)
                    .label(crate::i18n::tr("软件设置"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.page = Page::Settings;
                        cx.notify();
                    })),
            )
    }
    fn software_settings(&self, cx: &Context<Self>) -> Div {
        use crate::preferences::Appearance;
        let mut choices = div()
            .h_flex()
            .gap_2()
            .child(div().text_sm().child(crate::i18n::tr("外观")));
        for (id, label, value) in [
            ("system", "跟随系统", Appearance::System),
            ("light", "浅色", Appearance::Light),
            ("dark", "深色", Appearance::Dark),
        ] {
            choices = choices.child(
                Button::new(id)
                    .small()
                    .label(format!(
                        "{}{}",
                        if self.appearance == value { "✓ " } else { "" },
                        crate::i18n::tr(label)
                    ))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        match crate::preferences::save(value) {
                            Ok(()) => {
                                this.appearance = value;
                                crate::preferences::apply(value, window, cx);
                            }
                            Err(e) => this.status = e,
                        };
                        cx.notify();
                    })),
            );
        }
        let mut panel = div().v_flex().gap_4().max_w(px(680.)).child(choices);
        let mut languages = div()
            .h_flex()
            .gap_2()
            .child(div().text_sm().child(crate::i18n::tr("语言")));
        for (id, label, value) in [
            ("zh", "简体中文", crate::i18n::Language::Zh),
            ("en", "English", crate::i18n::Language::En),
        ] {
            languages = languages.child(
                Button::new(id)
                    .small()
                    .label(format!(
                        "{}{}",
                        if crate::i18n::language() == value {
                            "✓ "
                        } else {
                            ""
                        },
                        label
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        match crate::preferences::save_language(value) {
                            Ok(()) => {
                                crate::i18n::set_language(value);
                                cx.refresh_windows();
                            }
                            Err(e) => this.status = e,
                        };
                        cx.notify();
                    })),
            );
        }
        panel = panel.child(languages);
        panel.child(self.tool_controls(cx))
    }
    fn toolbar(&self, cx: &Context<Self>) -> Div {
        let title = match self.page {
            Page::Tasks => "下载队列",
            Page::History => "下载历史",
            Page::New => "新建下载",
            Page::Profiles => "下载配置",
            Page::Confirm => "确认下载",
            Page::Settings => "软件设置",
        };
        div()
            .h_flex()
            .h(px(52.))
            .items_center()
            .justify_between()
            .px_5()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .child(muted(cx, crate::i18n::tr("视频下载器")))
                    .child(muted(cx, "/"))
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .child(crate::i18n::tr(title)),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .size(px(6.))
                            .rounded_full()
                            .bg(if self.running.is_empty() {
                                cx.theme().muted_foreground
                            } else {
                                design::accent()
                            }),
                    )
                    .child(
                        Button::new("add")
                            .primary()
                            .small()
                            .icon(IconName::Plus)
                            .label(crate::i18n::tr("新建任务"))
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::New;
                                cx.notify();
                            })),
                    ),
            )
    }
}
