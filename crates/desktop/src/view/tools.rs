//! Explicit actions: no network activity occurs merely by rendering settings.
use super::muted;
use crate::{app::Downloader, tool_settings::Operation};
use gpui_kit::{
    component::{button::*, input::Input, *},
    *,
};
impl Downloader {
    pub(super) fn tool_controls(&self, cx: &Context<Self>) -> Div {
        let mut panel = div()
            .v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(div().font_semibold().child("下载工具"));
        for (label, input) in [
            ("yt-dlp", &self.tools.yt_dlp),
            ("FFmpeg", &self.tools.ffmpeg),
            ("ffprobe", &self.tools.ffprobe),
        ] {
            panel = panel
                .child(
                    div()
                        .text_sm()
                        .child(format!("{label} 自定义路径（留空自动发现）")),
                )
                .child(Input::new(input).disabled(self.tools.busy));
        }
        panel = panel.child(
            div()
                .h_flex()
                .gap_2()
                .child(
                    Button::new("tools-apply")
                        .label("保存并检查工具")
                        .disabled(self.tools.busy)
                        .on_click(cx.listener(|this, _, _, cx| {
                            match this.persist_tool_paths(cx) {
                                Ok(()) => this.manage_tools(Operation::Apply, cx),
                                Err(e) => this.tools.status = e,
                            };
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("tools-check")
                        .label("检查 yt-dlp 更新")
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.manage_tools(Operation::Check, cx)),
                        ),
                )
                .child(
                    Button::new("tools-revert")
                        .label("回退托管版本")
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.manage_tools(Operation::Revert, cx)),
                        ),
                ),
        );
        if let Some(release) = &self.tools.release {
            panel = panel
                .child(muted(
                    cx,
                    format!(
                        "官方版本 {} · SHA256完整性校验（非签名验证）",
                        release.version
                    ),
                ))
                .child(
                    Button::new("tools-install")
                        .label("确认下载并安装 yt-dlp")
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.manage_tools(Operation::Install, cx)),
                        ),
                );
        }
        panel
            .child(muted(cx, self.tools.status.clone()))
            .child(muted(
                cx,
                "FFmpeg / ffprobe 不自动下载；可选择已安装工具。现有下载不受工具切换影响。",
            ))
    }
}
