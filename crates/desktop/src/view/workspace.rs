//! Focused task list, source workflow, and command review.
use super::controls::{self, Tone};
use super::muted;
use crate::i18n::tr;
use crate::{app::Downloader, settings::Page};
use gpui_kit::{
    component::{input::Textarea, *},
    *,
};
impl Downloader {
    pub(super) fn tasks(&self, cx: &Context<Self>) -> Div {
        use super::{design, media};
        let recovered = self
            .drafts
            .entries
            .iter()
            .filter(|d| {
                d.state == crate::drafts::DraftState::Queued
                    && !self.running.contains_key(&d.id)
                    && !self.queued.iter().any(|(r, _)| r.task_id == d.id)
            })
            .count();
        let mut body = div()
            .v_flex()
            .gap_4()
            .w_full()
            .max_w(px(900.))
            .mx_auto()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .child(controls::badge(
                        format!("{} {}", self.running.len(), tr("进行中")),
                        true,
                        cx,
                    ))
                    .child(controls::badge(
                        format!("{} {}", self.queued.len() + recovered, tr("等待下载")),
                        false,
                        cx,
                    )),
            );
        let mut list = design::group(cx).v_flex().overflow_hidden();
        for (id, task) in &self.running {
            let cancel_id = id.clone();
            let audio = task.record.metadata.requested_format.starts_with("Audio")
                || task.record.metadata.requested_format.starts_with("MP3");
            let mut meta = div().h_flex().items_center().flex_wrap().gap_2();
            if let Some(channel) = &task.record.metadata.channel {
                meta = meta.child(
                    muted(cx, channel.clone())
                        .max_w(px(220.))
                        .overflow_hidden()
                        .whitespace_nowrap(),
                );
            }
            let format = if audio {
                if task.record.metadata.requested_format.starts_with("MP3") {
                    tr("MP3")
                } else {
                    tr("原音频")
                }
            } else if task.record.metadata.requested_format.contains("Mkv") {
                "MKV".into()
            } else {
                "MP4".into()
            };
            meta = meta.child(controls::badge(format, false, cx));
            if !audio {
                let quality = task
                    .record
                    .metadata
                    .requested_height
                    .map(|h| format!("≤ {h}p"))
                    .unwrap_or_else(|| tr("最佳可用"));
                meta = meta.child(controls::badge(quality, false, cx));
            }
            let mut info = div()
                .v_flex()
                .flex_1()
                .min_w_0()
                .gap_2()
                .child(media::title(
                    SharedString::from(format!("active-title-{id}")),
                    task.title.clone(),
                    cx,
                ))
                .child(meta)
                .child(
                    div()
                        .text_xs()
                        .text_color(design::accent())
                        .child(task.status.clone()),
                );
            if let Some(value) = task.progress_fraction {
                info = info.child(
                    div()
                        .h(px(4.))
                        .w_full()
                        .rounded_full()
                        .bg(design::canvas(cx))
                        .child(
                            div()
                                .h(px(4.))
                                .w(relative(value))
                                .rounded_full()
                                .bg(design::accent()),
                        ),
                );
            }
            list = list.child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_4()
                    .p_4()
                    .border_b_1()
                    .border_color(design::border(cx))
                    .child(design::media_tile(cx, audio))
                    .child(info)
                    .child(
                        controls::icon_button(
                            SharedString::from(format!("cancel-task-{id}")),
                            IconName::Close,
                            tr("取消任务"),
                            cx,
                        )
                        .disabled(task.cancel.is_cancelled())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(task) = this.running.get_mut(&cancel_id) {
                                task.cancel.cancel();
                                task.progress_fraction = None;
                                task.status = tr("正在停止…");
                            }
                            cx.notify();
                        })),
                    ),
            );
        }
        for (request, profile) in &self.queued {
            let draft = self.drafts.entries.iter().find(|d| d.id == request.task_id);
            let source = draft.and_then(|d| d.media.as_ref());
            let title = source
                .map(|m| m.title.clone())
                .unwrap_or(request.url.clone());
            let task_id = request.task_id.clone();
            list = list.child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_4()
                    .p_4()
                    .border_b_1()
                    .border_color(design::border(cx))
                    .child(design::media_tile(
                        cx,
                        profile.mode != youtube_engine::Mode::Video,
                    ))
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(media::title(
                                SharedString::from(format!("queued-title-{}", request.task_id)),
                                title,
                                cx,
                            ))
                            .child(media::metadata(source, profile, cx))
                            .child(muted(cx, tr("等待下载"))),
                    )
                    .child(
                        controls::icon_button(
                            SharedString::from(format!("queue-remove-{}", request.task_id)),
                            IconName::Close,
                            tr("取消任务"),
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(position) =
                                this.queued.iter().position(|(r, _)| r.task_id == task_id)
                            {
                                match this.drafts.remove(&task_id) {
                                    Ok(()) => {
                                        this.queued.remove(position);
                                    }
                                    Err(error) => {
                                        this.status = error;
                                    }
                                }
                            }
                            cx.notify();
                        })),
                    ),
            );
        }
        for draft in self.drafts.entries.iter().filter(|d| {
            d.state == crate::drafts::DraftState::Queued
                && !self.running.contains_key(&d.id)
                && !self.queued.iter().any(|(r, _)| r.task_id == d.id)
        }) {
            let id = draft.id.clone();
            let cancel_id = id.clone();
            let title = draft
                .media
                .as_ref()
                .map(|m| m.title.clone())
                .unwrap_or(draft.url.clone());
            list = list.child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_4()
                    .p_4()
                    .border_b_1()
                    .border_color(design::border(cx))
                    .child(design::media_tile(
                        cx,
                        draft.profile.mode != youtube_engine::Mode::Video,
                    ))
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(media::title(
                                SharedString::from(format!("restored-title-{}", draft.id)),
                                title,
                                cx,
                            ))
                            .child(media::metadata(draft.media.as_ref(), &draft.profile, cx))
                            .child(muted(cx, tr("已中断"))),
                    )
                    .child(
                        controls::button(
                            SharedString::from(format!("queue-resume-{}", draft.id)),
                            tr("恢复队列"),
                            Tone::Secondary,
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| this.resume_queued(&id, cx))),
                    )
                    .child(
                        controls::icon_button(
                            SharedString::from(format!("cancel-restored-{}", draft.id)),
                            IconName::Close,
                            tr("取消任务"),
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Err(error) = this.drafts.remove(&cancel_id) {
                                this.status = error;
                            }
                            cx.notify();
                        })),
                    ),
            );
        }
        if self.running.is_empty() && self.queued.is_empty() && recovered == 0 {
            return div().w_full().max_w(px(900.)).mx_auto().child(
                design::group(cx)
                    .v_flex()
                    .items_center()
                    .justify_center()
                    .py_20()
                    .gap_4()
                    .child(design::media_tile(cx, false))
                    .child(design::heading(tr("暂无活动任务")))
                    .child(
                        controls::button("queue-new", tr("新建下载"), Tone::Primary, cx)
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.refresh_preset_select(window, cx);
                                this.page = Page::New;
                                cx.notify();
                            })),
                    ),
            );
        }
        body = body.child(list);
        body
    }
    pub(super) fn download_summary(&self, cx: &Context<Self>) -> Div {
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
            .as_ref()
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
        let mut card = super::design::group(cx).v_flex().overflow_hidden();
        for (label, value) in [
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
            if (label == "代理" && profile.proxy.is_none())
                || (label == "Cookie" && profile.cookies_browser.is_none())
                || (label == "高级参数" && profile.extra_args.is_empty())
            {
                continue;
            }
            card = card.child(
                div()
                    .h_flex()
                    .items_start()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(super::design::border(cx))
                    .gap_4()
                    .child(muted(cx, tr(label)).w(px(80.)).flex_shrink_0())
                    .child(div().flex_1().min_w_0().text_sm().child(value)),
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
            controls::button("command-toggle", "", Tone::Quiet, cx)
                .small()
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
                .bg(super::design::canvas(cx))
                .text_xs();
            for line in command.lines() {
                lines = lines.child(div().child(line.to_owned()));
            }
            let copy_command = command.clone();
            details = details.child(
                super::design::group(cx)
                    .v_flex()
                    .gap_3()
                    .p_4()
                    .child(lines)
                    .child(Textarea::new(&self.settings.extra).h(px(100.)))
                    .child(
                        div().h_flex().justify_end().child(
                            controls::button("copy", tr("复制命令"), Tone::Quiet, cx)
                                .icon(IconName::Copy)
                                .disabled(!plan_valid)
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        copy_command.clone(),
                                    ))
                                }),
                        ),
                    ),
            );
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
                    .bg(super::design::canvas(cx))
                    .child(
                        div()
                            .text_sm()
                            .child(tr("下载历史中已存在此视频，是否重新下载？")),
                    )
                    .child(
                        controls::button("confirm-duplicate", "", Tone::Secondary, cx)
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
            .children((!plan_valid).then(|| muted(cx, tr("配置无效，请检查高级参数"))))
            .child(details)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .flex_wrap()
                    .gap_3()
                    .pt_4()
                    .border_t_1()
                    .border_color(super::design::border(cx))
                    .child(
                        controls::button("preview-edit", "", Tone::Secondary, cx)
                            .small()
                            .icon(IconName::Settings)
                            .label(tr("修改选项"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::TaskOptions;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        controls::button("execute", "", Tone::Primary, cx)
                            .small()
                            .label(tr("加入下载队列"))
                            .disabled(self.busy || !valid || self.media.is_none())
                            .on_click(cx.listener(|this, _, _, cx| this.execute_confirmed(cx))),
                    ),
            )
    }
}
