//! Persistent inspector for the current task; no placeholder settings or fake queues.
use super::{eyebrow, muted};
use crate::app::Downloader;
use crate::i18n::tr;
use gpui_kit::component::{button::*, *};
use gpui_kit::*;
use youtube_engine::Mode;

impl Downloader {
    pub(super) fn inspector(&self, cx: &Context<Self>) -> Div {
        div()
            .w(px(272.))
            .flex_shrink_0()
            .h_full()
            .v_flex()
            .border_l_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .px_5()
                    .py_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().text_sm().font_semibold().child(tr("保存选项")))
                    .child(muted(cx, tr("应用于当前视频"))),
            )
            .child(
                div()
                    .id("inspector-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_5()
                    .child(
                        div()
                            .v_flex()
                            .gap_6()
                            .child(self.presets(cx))
                            .child(self.destination(cx))
                            .child(
                                div()
                                    .v_flex()
                                    .gap_2()
                                    .child(eyebrow(cx, tr("文件保护")))
                                    .child(muted(cx, tr("每次下载创建独立子目录")))
                                    .child(muted(cx, tr("不覆盖已有文件")))
                                    .child(muted(cx, tr("仅使用已安装的本地工具"))),
                            ),
                    ),
            )
            .child(self.download_action(cx))
    }
    fn presets(&self, cx: &Context<Self>) -> Div {
        let mut choices = div().v_flex().gap_3().child(eyebrow(cx, tr("格式与质量")));
        for (id, mode, title, detail) in [
            ("video", Mode::Video, "兼容 MP4", "最高 1080p · H.264 / AAC"),
            ("audio", Mode::Audio, "原始音频", "保留源格式，不进行转码"),
            ("mp3", Mode::MP3, "MP3 音频", "转换为常用音频格式"),
        ] {
            let selected = self.mode == mode;
            choices = choices.child(
                div()
                    .v_flex()
                    .gap_2()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(if selected {
                        cx.theme().foreground
                    } else {
                        cx.theme().border
                    })
                    .bg(if selected {
                        cx.theme().muted
                    } else {
                        cx.theme().background
                    })
                    .child(
                        Button::new(id)
                            .label(if selected {
                                format!("●  {}", tr(title))
                            } else {
                                format!("○  {}", tr(title))
                            })
                            .disabled(self.busy)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.mode = mode;
                                cx.notify();
                            })),
                    )
                    .child(muted(cx, tr(detail))),
            );
        }
        choices
    }
    fn destination(&self, cx: &Context<Self>) -> Div {
        div()
            .v_flex()
            .gap_3()
            .child(eyebrow(cx, tr("保存位置")))
            .child(
                div()
                    .p_3()
                    .rounded_lg()
                    .bg(cx.theme().muted)
                    .text_xs()
                    .child(
                        self.output
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| tr("尚未选择文件夹")),
                    ),
            )
            .child(
                Button::new("folder")
                    .label(tr("选择文件夹…"))
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, _, cx| this.choose_directory(cx))),
            )
    }
    fn download_action(&self, cx: &Context<Self>) -> Div {
        div()
            .v_flex()
            .gap_3()
            .p_5()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("download")
                    .primary()
                    .label(tr("↓  开始下载"))
                    .disabled(self.busy || self.media.is_none() || self.output.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.download(cx))),
            )
            .child(
                Button::new("cancel")
                    .label(tr("取消当前任务"))
                    .disabled(!self.busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(cancel) = &this.cancel {
                            cancel.cancel();
                            this.status = tr("正在取消并回收进程，请稍候…");
                            cx.notify();
                        }
                    })),
            )
            .child(muted(cx, tr("无队列 · 无自动重试")))
    }
}
