//! Minimal desktop shell. Each workflow has a separate page, not a permanent inspector.
use crate::{app::Downloader, settings::Page};
use gpui_kit::{
    component::{button::*, *},
    *,
};
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
                                Page::New => self.new_download(cx),
                                Page::Profiles => self.profile_editor(cx),
                                Page::Confirm => self.confirmation(cx),
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
            ("tasks", "下载任务", Page::Tasks),
            ("new", "新建下载", Page::New),
            ("profiles", "下载配置", Page::Profiles),
        ] {
            nav = nav.child(
                Button::new(id)
                    .label(label)
                    .disabled(self.busy && page != Page::Tasks)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.page = page;
                        cx.notify();
                    })),
            );
        }
        nav.child(div().flex_1()).child(muted(cx, "开发预览 0.1"))
    }
    fn toolbar(&self, cx: &Context<Self>) -> Div {
        let title = match self.page {
            Page::Tasks => "下载任务",
            Page::New => "新建下载",
            Page::Profiles => "下载配置",
            Page::Confirm => "确认下载",
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
