//! Task canvas: source entry, preview/empty state, and lifecycle feedback.
use super::{eyebrow, muted};
use crate::app::Downloader;
use gpui_kit::component::{button::*, input::Input, *};
use gpui_kit::*;

impl Downloader {
    pub(super) fn workspace(&self, cx: &Context<Self>) -> Div {
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .v_flex()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .px_6()
                    .py_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(div().text_lg().font_semibold().child("下载工作台"))
                            .child(muted(cx, "从一个链接开始，留住值得收藏的内容。")),
                    )
                    .child(muted(
                        cx,
                        if self.busy {
                            "●  任务进行中"
                        } else {
                            "○  就绪"
                        },
                    )),
            )
            .child(
                div()
                    .id("workspace-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_6()
                    .child(
                        div()
                            .v_flex()
                            .gap_6()
                            .child(self.source_entry(cx))
                            .child(self.preview(cx))
                            .child(self.task_feedback(cx)),
                    ),
            )
    }
    fn source_entry(&self, cx: &Context<Self>) -> Div {
        div()
            .v_flex()
            .gap_3()
            .child(eyebrow(cx, "01 / 添加视频"))
            .child(Input::new(&self.url).disabled(self.busy))
            .child(
                div()
                    .h_flex()
                    .justify_between()
                    .items_center()
                    .child(muted(cx, "支持 YouTube 单视频、Shorts 链接"))
                    .child(
                        Button::new("parse")
                            .primary()
                            .label("解析链接 →")
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| this.probe(cx))),
                    ),
            )
    }
    fn preview(&self, cx: &Context<Self>) -> Div {
        let frame = div()
            .v_flex()
            .gap_4()
            .p_5()
            .rounded_xl()
            .border_1()
            .border_color(cx.theme().border);
        if let Some(info) = &self.media {
            frame
                .child(eyebrow(cx, "02 / 视频预览"))
                .child(
                    div()
                        .h_flex()
                        .gap_4()
                        .items_center()
                        .child(
                            div()
                                .w(px(70.))
                                .h(px(70.))
                                .rounded_lg()
                                .bg(cx.theme().muted)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_2xl()
                                .child("▶"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .v_flex()
                                .gap_2()
                                .child(div().font_semibold().text_base().child(info.title.clone()))
                                .child(muted(
                                    cx,
                                    format!(
                                        "{} · {} 个格式",
                                        info.channel
                                            .as_deref()
                                            .or(info.uploader.as_deref())
                                            .unwrap_or("未知频道"),
                                        info.formats.len()
                                    ),
                                )),
                        ),
                )
                .child(muted(cx, "链接已解析 · 在右侧确认保存选项后开始下载"))
        } else {
            frame.child(
                div()
                    .v_flex()
                    .items_center()
                    .justify_center()
                    .py_8()
                    .gap_3()
                    .child(
                        div()
                            .w(px(56.))
                            .h(px(56.))
                            .rounded_xl()
                            .bg(cx.theme().muted)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_2xl()
                            .text_color(cx.theme().muted_foreground)
                            .child("↓"),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("您的下一个收藏，在这里开始"),
                    )
                    .child(muted(cx, "粘贴链接并解析，视频信息将在此显示。"))
                    .child(muted(cx, "无需账户 · 不读取 Cookie")),
            )
        }
    }
    fn task_feedback(&self, cx: &Context<Self>) -> Div {
        let mut panel = div().v_flex().gap_3().child(eyebrow(cx, "任务动态")).child(
            div()
                .p_4()
                .rounded_lg()
                .bg(cx.theme().muted)
                .v_flex()
                .gap_2()
                .child(div().text_sm().font_medium().child(self.status.clone()))
                .child(muted(cx, "完成状态以进程退出和最终文件验证为准。")),
        );
        for (index, artifact) in self.artifacts.iter().enumerate() {
            let path = artifact.path.clone();
            panel = panel.child(
                Button::new(("artifact", index))
                    .label(format!(
                        "在 Finder 中显示 · {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ))
                    .on_click(move |_, _, cx| cx.reveal_path(&path)),
            );
        }
        panel
    }
}
