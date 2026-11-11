//! Focused task list, source workflow, and command review.
use super::muted;
use crate::{app::Downloader, settings::Page};
use gpui_kit::{
    component::{
        button::*,
        input::{Input, Textarea},
        *,
    },
    *,
};
impl Downloader {
    pub(super) fn tasks(&self, cx: &Context<Self>) -> Div {
        let mut body = div().v_flex().gap_4();
        if self.media.is_none() && !self.busy && self.artifacts.is_empty() {
            return body.child(
                div()
                    .v_flex()
                    .items_center()
                    .py_20()
                    .gap_4()
                    .child(
                        div()
                            .text_2xl()
                            .text_color(cx.theme().muted_foreground)
                            .child("↓"),
                    )
                    .child(muted(cx, "暂无下载任务")),
            );
        }
        body = body.child(
            div()
                .h_flex()
                .justify_between()
                .p_4()
                .border_1()
                .border_color(cx.theme().border)
                .rounded_lg()
                .child(
                    div()
                        .v_flex()
                        .gap_2()
                        .child(
                            self.media
                                .as_ref()
                                .map(|m| m.title.clone())
                                .unwrap_or("正在解析…".into()),
                        )
                        .child(muted(cx, self.status.clone())),
                )
                .child(
                    Button::new("stop")
                        .label("取消")
                        .disabled(!self.busy)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(cancel) = &this.cancel {
                                cancel.cancel();
                                this.status = "正在停止…".into();
                                cx.notify();
                            }
                        })),
                ),
        );
        for (i, a) in self.artifacts.iter().enumerate() {
            let path = a.path.clone();
            body = body.child(
                Button::new(("reveal", i))
                    .label("在 Finder 中显示")
                    .on_click(move |_, _, cx| cx.reveal_path(&path)),
            );
        }
        body
    }
    pub(super) fn new_download(&self, cx: &Context<Self>) -> Div {
        let mut body = div()
            .v_flex()
            .gap_5()
            .max_w(px(680.))
            .child(div().text_sm().child("视频链接"))
            .child(Input::new(&self.url).disabled(self.busy))
            .child(self.preset_switcher(cx))
            .child(
                div()
                    .h_flex()
                    .gap_3()
                    .child(
                        Button::new("parse")
                            .label("解析链接")
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| this.probe(cx))),
                    )
                    .child(
                        Button::new("edit-profile")
                            .label("编辑下载配置")
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Profiles;
                                cx.notify();
                            })),
                    ),
            );
        if let Some(info) = &self.media {
            body = body
                .child(
                    div()
                        .p_4()
                        .rounded_lg()
                        .bg(cx.theme().muted)
                        .v_flex()
                        .gap_2()
                        .child(div().font_semibold().child(info.title.clone()))
                        .child(muted(cx, info.channel.clone().unwrap_or_default())),
                )
                .child(
                    Button::new("review")
                        .primary()
                        .label("下一步：查看命令")
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, _, cx| this.download(cx))),
                );
        }
        body
    }
    pub(super) fn confirmation(&self, cx: &Context<Self>) -> Div {
        let plan = self
            .pending
            .as_ref()
            .and_then(|r| {
                self.engine
                    .download_plan(r, &self.settings.snapshot(cx))
                    .ok()
            })
            .map(|p| p.display_redacted());
        let valid = plan.is_some();
        let command = plan.unwrap_or_else(|| "配置无效，请检查高级参数或返回修改".into());
        div()
            .v_flex()
            .gap_4()
            .child(div().text_sm().child("下载命令"))
            .child(
                div()
                    .p_4()
                    .rounded_lg()
                    .bg(cx.theme().muted)
                    .text_xs()
                    .child(command.clone()),
            )
            .child(muted(cx, "认证信息已隐藏 · 参数按 argv 执行，不经过 shell"))
            .child(Textarea::new(&self.settings.extra).h(px(120.)))
            .child(muted(cx, "高级参数：每行一个参数或值；可在确认前调整。"))
            .child(
                div()
                    .h_flex()
                    .gap_3()
                    .child(
                        Button::new("execute")
                            .primary()
                            .label("确认下载")
                            .disabled(self.busy || !valid)
                            .on_click(cx.listener(|this, _, _, cx| this.execute_confirmed(cx))),
                    )
                    .child(
                        Button::new("copy")
                            .label("复制命令")
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(command.clone()))
                            }),
                    )
                    .child(Button::new("back").label("返回").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.page = Page::New;
                            cx.notify();
                        },
                    ))),
            )
    }
}
