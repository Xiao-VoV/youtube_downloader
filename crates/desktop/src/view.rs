//! Minimal desktop shell. Each workflow has a separate page, not a permanent inspector.
use crate::{app::Downloader, settings::Page};
use gpui_kit::{
    component::{button::*, *},
    *,
};
mod history;
mod profiles;
mod workspace;

pub(super) fn muted(cx: &Context<Downloader>, text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}
impl Render for Downloader {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .p_6()
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
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .px_6()
                            .py_2()
                            .child(muted(cx, self.status.clone())),
                    ),
            )
    }
}
impl Downloader {
    fn sidebar(&self, cx: &Context<Self>) -> Div {
        let mut nav = div()
            .w(px(168.))
            .h_full()
            .flex_shrink_0()
            .v_flex()
            .gap_3()
            .p_4()
            .bg(cx.theme().muted)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(div().py_4().font_bold().text_lg().child("▶  拾光"));
        for (id, label, page) in [
            ("new", "新建任务", Page::New),
            ("tasks", "下载队列", Page::Tasks),
            ("history", "下载历史", Page::History),
            ("profiles", "下载配置", Page::Profiles),
            ("settings", "软件设置", Page::Settings),
        ] {
            nav = nav.child(
                Button::new(id)
                    .label(format!(
                        "{}{}",
                        if self.page == page || (page == Page::New && self.page == Page::Confirm) {
                            "▸ "
                        } else {
                            ""
                        },
                        label
                    ))
                    .bg(if self.page == page {
                        cx.theme().muted
                    } else {
                        cx.theme().background
                    })
                    .border_color(if self.page == page {
                        cx.theme().foreground
                    } else {
                        cx.theme().border
                    })
                    .disabled(self.busy && matches!(page, Page::New | Page::Profiles))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.page = page;
                        cx.notify();
                    })),
            );
        }
        nav.child(div().flex_1()).child(muted(cx, "开发预览 0.1"))
    }
    fn software_settings(&self, cx: &Context<Self>) -> Div {
        let mut panel = div().v_flex().gap_4().max_w(px(680.));
        for (label, value) in [
            ("外观", "跟随系统 · 主题切换待实现"),
            ("语言", "简体中文 · 多语言待实现"),
            ("yt-dlp 路径", "自动发现 · 手动配置待实现"),
            ("FFmpeg / ffprobe", "自动发现 · 手动配置待实现"),
        ] {
            panel = panel.child(
                div()
                    .h_flex()
                    .justify_between()
                    .p_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().text_sm().child(label))
                    .child(muted(cx, value)),
            );
        }
        panel.child(muted(cx, self.health.clone()))
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
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().font_semibold().child(title))
            .child(
                Button::new("add")
                    .primary()
                    .label("＋ 新建下载")
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.page = Page::New;
                        cx.notify();
                    })),
            )
    }
}
