//! Focused task list, source workflow, and command review.
use super::muted;
use crate::i18n::tr;
use crate::{app::Downloader, settings::Page};
use gpui_kit::{
    base::{Scrollbar, ScrollbarMode},
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
            return body.child(
                div()
                    .v_flex()
                    .items_center()
                    .justify_center()
                    .py_20()
                    .gap_4()
                    .child(
                        div()
                            .p_4()
                            .rounded_full()
                            .bg(cx.theme().muted)
                            .child(super::design::glyph("download")),
                    )
                    .child(muted(cx, tr("暂无活动任务 · 已完成任务见下载历史")))
                    .child(
                        Button::new("queue-new")
                            .small()
                            .primary()
                            .icon(IconName::Plus)
                            .label(tr("新建任务"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::New;
                                cx.notify();
                            })),
                    ),
            );
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
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .mr_4()
                            .gap_2()
                            .child(
                                div()
                                    .relative()
                                    .w_full()
                                    .pb_4()
                                    .child(
                                        div()
                                            .id(("task-title-scroll", index))
                                            .w_full()
                                            .overflow_x_scroll()
                                            .restrict_scroll_to_axis()
                                            .track_scroll(&task.title_scroll)
                                            .tooltip({
                                                let title = task.title.clone();
                                                move |window, cx| {
                                                    gpui_kit::component::tooltip::Tooltip::new(
                                                        title.clone(),
                                                    )
                                                    .build(window, cx)
                                                }
                                            })
                                            .child(
                                                div().whitespace_nowrap().child(task.title.clone()),
                                            ),
                                    )
                                    .child(
                                        Scrollbar::horizontal(&task.title_scroll)
                                            .mode(ScrollbarMode::Always),
                                    ),
                            )
                            .child(muted(cx, task.status.clone())),
                    )
                    .child(
                        Button::new(("cancel-task", index))
                            .label(tr("取消"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(task) = this.running.get_mut(&id) {
                                    task.cancel.cancel();
                                    task.status = tr("正在停止…");
                                }
                                cx.notify();
                            })),
                    ),
            );
        }
        body
    }
    pub(super) fn new_download(&self, cx: &Context<Self>) -> Div {
        let mut body = div()
            .v_flex()
            .gap_6()
            .w_full()
            .max_w(px(900.))
            .mx_auto()
            .py_6()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w(px(44.))
                            .h(px(44.))
                            .rounded_xl()
                            .bg(super::design::accent())
                            .text_color(rgb(0xffffff))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(super::design::glyph("arrow-down-to-line")),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(div().text_xl().font_semibold().child(tr("新建任务")))
                            .child(muted(cx, tr("粘贴视频链接，选择你的下载方式。"))),
                    ),
            )
            .child(
                super::design::section(cx, tr("视频链接"))
                    .child(Input::new(&self.url).disabled(self.busy))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                Button::new("paste-url")
                                    .ghost()
                                    .small()
                                    .icon(super::design::glyph("clipboard"))
                                    .label(tr("从剪贴板粘贴"))
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(text) =
                                            cx.read_from_clipboard().and_then(|i| i.text())
                                        {
                                            this.url
                                                .update(cx, |s, cx| s.set_value(text, window, cx));
                                            this.media = None;
                                            cx.notify();
                                        }
                                    })),
                            )
                            .child(
                                Button::new("parse")
                                    .primary()
                                    .small()
                                    .icon(IconName::ArrowRight)
                                    .label(tr("解析链接"))
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| this.probe(cx))),
                            ),
                    ),
            )
            .child(
                super::design::section(cx, tr("使用下载配置"))
                    .child(self.preset_switcher(cx))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_3()
                            .pt_3()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .child(super::design::glyph("folder"))
                            .child(
                                muted(
                                    cx,
                                    self.output
                                        .as_ref()
                                        .map(|p| p.display().to_string())
                                        .unwrap_or(tr("尚未选择保存位置")),
                                )
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden(),
                            )
                            .child(
                                Button::new("edit-profile")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Settings)
                                    .label(tr("编辑下载配置"))
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.page = Page::Profiles;
                                        cx.notify();
                                    })),
                            ),
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
                        .label(tr("下一步：查看命令"))
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, _, cx| this.download(cx))),
                );
        }
        body
    }
    fn download_summary(&self, cx: &Context<Self>) -> Div {
        let profile = self.settings.snapshot(cx);
        let quality = if self.mode != youtube_engine::Mode::Video {
            tr("音频")
        } else if profile.best_resolution {
            tr("最佳可用分辨率")
        } else {
            format!("{} {}p", tr("最高"), profile.max_height)
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
                    profile
                        .browser_profile
                        .clone()
                        .unwrap_or_else(|| tr("默认 Profile"))
                )
            })
            .unwrap_or(tr("不使用"));
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
            ("输出", format!("{} / {quality}", tr(format))),
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
                    tr("已启用（地址隐藏）")
                } else {
                    tr("直连")
                }
                .into(),
            ),
            ("Cookie", cookies),
            (
                "高级参数",
                format!("{} {}", profile.extra_args.len(), tr("个参数项")),
            ),
        ] {
            card = card.child(
                div()
                    .h_flex()
                    .gap_4()
                    .child(muted(cx, tr(label)).w(px(80.)))
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
        let duplicate = self.pending.as_ref().is_some_and(|request| {
            self.history
                .entries
                .iter()
                .any(|r| crate::duplicates::same_video(&r.url, &request.url))
        });
        let plan_valid = plan.is_some();
        let valid = plan_valid && (!duplicate || self.duplicate_confirmed);
        let command = plan.unwrap_or_else(|| tr("配置无效，请检查高级参数或返回修改"));
        let mut details = div().v_flex().gap_3().child(
            Button::new("command-toggle")
                .label(if self.settings.command_open {
                    tr("▾ 完整命令与高级参数")
                } else {
                    tr("▸ 完整命令与高级参数")
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
                .child(muted(cx, tr("高级参数每行一个参数或值；认证信息已隐藏。")));
        }
        div()
            .v_flex()
            .gap_4()
            .child(self.download_summary(cx))
            .child(if duplicate {
                div()
                    .v_flex()
                    .gap_2()
                    .p_3()
                    .bg(cx.theme().muted)
                    .child(
                        div()
                            .text_sm()
                            .child(tr("下载历史中已存在此视频，是否重新下载？")),
                    )
                    .child(
                        Button::new("confirm-duplicate")
                            .label(if self.duplicate_confirmed {
                                tr("✓ 已确认重新下载")
                            } else {
                                tr("仍然重新下载")
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.duplicate_confirmed = true;
                                cx.notify();
                            })),
                    )
            } else {
                div()
            })
            .child(if valid {
                muted(cx, tr("参数校验通过"))
            } else if plan_valid && duplicate && !self.duplicate_confirmed {
                muted(cx, tr("等待确认重新下载"))
            } else {
                muted(cx, tr("配置无效，请检查高级参数"))
            })
            .child(details)
            .child(
                div()
                    .h_flex()
                    .gap_3()
                    .child(
                        Button::new("execute")
                            .primary()
                            .label(tr("确认下载"))
                            .disabled(self.busy || !valid)
                            .on_click(cx.listener(|this, _, _, cx| this.execute_confirmed(cx))),
                    )
                    .child(
                        Button::new("copy")
                            .label(tr("复制命令"))
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(command.clone()))
                            }),
                    )
                    .child(Button::new("back").label(tr("返回")).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.page = Page::New;
                            cx.notify();
                        },
                    ))),
            )
    }
}
