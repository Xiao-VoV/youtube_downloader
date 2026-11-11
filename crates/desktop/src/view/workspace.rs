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
        let mut body = div().v_flex().gap_3();
        if self.running.is_empty() {
            return body.child(muted(cx, "暂无活动任务 · 已完成任务见下载历史"));
        }
        for (index, (id, task)) in self.running.iter().enumerate() {
            let id = id.clone();
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
                            .child(task.title.clone())
                            .child(muted(cx, task.status.clone())),
                    )
                    .child(Button::new(("cancel-task", index)).label("取消").on_click(
                        cx.listener(move |this, _, _, cx| {
                            if let Some(task) = this.running.get_mut(&id) {
                                task.cancel.cancel();
                                task.status = "正在停止…".into();
                            }
                            cx.notify();
                        }),
                    )),
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
    fn download_summary(&self, cx: &Context<Self>) -> Div {
        let profile = self.settings.snapshot(cx);
        let quality = if self.mode != youtube_engine::Mode::Video {
            "音频".into()
        } else if profile.best_resolution {
            "最佳可用分辨率".into()
        } else {
            format!("最高 {}p", profile.max_height)
        };
        let format = match self.mode {
            youtube_engine::Mode::Video => match profile.container {
                youtube_engine::VideoContainer::Mp4 => "MP4 · H.264/AAC",
                youtube_engine::VideoContainer::Mkv => "MKV",
            },
            youtube_engine::Mode::Audio => "原始音频",
            youtube_engine::Mode::MP3 => "MP3",
        };
        let cookies = profile
            .cookies_browser
            .map(|b| {
                format!(
                    "{b:?} · {}",
                    profile.browser_profile.as_deref().unwrap_or("默认 Profile")
                )
            })
            .unwrap_or("不使用".into());
        let mut card = div()
            .v_flex()
            .gap_3()
            .p_4()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_lg();
        for (label, value) in [
            (
                "视频",
                self.media
                    .as_ref()
                    .map(|m| m.title.clone())
                    .unwrap_or_default(),
            ),
            ("输出", format!("{format} / {quality}")),
            (
                "保存到",
                self.output
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
            (
                "代理",
                if profile.proxy.is_some() {
                    "已启用（地址隐藏）"
                } else {
                    "直连"
                }
                .into(),
            ),
            ("Cookie", cookies),
            ("高级参数", format!("{} 个参数项", profile.extra_args.len())),
        ] {
            card = card.child(
                div()
                    .h_flex()
                    .gap_4()
                    .child(muted(cx, label).w(px(80.)))
                    .child(div().text_sm().child(value)),
            );
        }
        card
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
            .map(|p| p.display_redacted_multiline());
        let valid = plan.is_some();
        let command = plan.unwrap_or_else(|| "配置无效，请检查高级参数或返回修改".into());
        let mut details = div().v_flex().gap_3().child(
            Button::new("command-toggle")
                .label(if self.settings.command_open {
                    "▾ 完整命令与高级参数"
                } else {
                    "▸ 完整命令与高级参数"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.settings.command_open = !this.settings.command_open;
                    cx.notify();
                })),
        );
        if self.settings.command_open {
            let mut lines = div()
                .v_flex()
                .gap_1()
                .p_4()
                .rounded_lg()
                .bg(cx.theme().muted)
                .text_xs();
            for line in command.lines() {
                lines = lines.child(div().child(line.to_owned()));
            }
            details = details
                .child(lines)
                .child(Textarea::new(&self.settings.extra).h(px(100.)))
                .child(muted(cx, "高级参数每行一个参数或值；认证信息已隐藏。"));
        }
        div()
            .v_flex()
            .gap_4()
            .child(self.download_summary(cx))
            .child(if valid {
                muted(cx, "参数校验通过")
            } else {
                muted(cx, "配置无效，请检查高级参数")
            })
            .child(details)
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
