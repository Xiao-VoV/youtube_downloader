//! Local discovery and managed installations have separate explicit actions.
use super::muted;
use crate::{app::Downloader, i18n::tr, tool_settings::Operation};
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
            .child(div().font_semibold().child(tr("本地工具配置")));
        for (label, input, effective) in [
            (
                "yt-dlp",
                &self.tools.yt_dlp,
                self.tools.effective.as_ref().map(|p| &p.yt_dlp),
            ),
            (
                "FFmpeg",
                &self.tools.ffmpeg,
                self.tools
                    .effective
                    .as_ref()
                    .and_then(|p| p.ffmpeg.as_ref()),
            ),
            (
                "ffprobe",
                &self.tools.ffprobe,
                self.tools
                    .effective
                    .as_ref()
                    .and_then(|p| p.ffprobe.as_ref()),
            ),
        ] {
            panel = panel
                .child(div().text_sm().child(label))
                .child(Input::new(input).disabled(self.tools.busy))
                .child(muted(
                    cx,
                    format!(
                        "{}: {}",
                        tr("当前生效路径"),
                        effective
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| tr("未配置"))
                    ),
                ));
        }
        panel = panel.child(
            div()
                .h_flex()
                .gap_2()
                .flex_wrap()
                .child(
                    Button::new("tools-discover")
                        .label(tr(if self.tools.rediscover_confirm {
                            "确认重新发现"
                        } else {
                            "重新发现"
                        }))
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, window, cx| this.rediscover_tools(window, cx)),
                        ),
                )
                .child(
                    Button::new("tools-validate")
                        .label(tr("检查输入路径"))
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| {
                                this.manage_tools(Operation::Validate, cx)
                            }),
                        ),
                )
                .child(
                    Button::new("tools-apply")
                        .primary()
                        .label(tr("保存并应用"))
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.manage_tools(Operation::Apply, cx)),
                        ),
                ),
        );
        panel = panel.child(
            div()
                .v_flex()
                .gap_3()
                .mt_4()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .child(div().font_semibold().child(tr("应用托管的 yt-dlp")))
                .child(
                    Button::new("tools-check")
                        .label(tr("检查 yt-dlp 更新"))
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.manage_tools(Operation::Check, cx)),
                        ),
                )
                .children(self.tools.release.as_ref().map(|release| {
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(muted(
                            cx,
                            format!("{} {} · SHA256", tr("官方版本"), release.version),
                        ))
                        .child(
                            Button::new("tools-install")
                                .label(tr("仅下载安装，不切换"))
                                .disabled(self.tools.busy)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.manage_tools(Operation::Install, cx)
                                })),
                        )
                }))
                .child(self.managed_versions(cx)),
        );
        panel
            .child(muted(cx, self.tools.status.clone()))
            .child(muted(
                cx,
                tr("FFmpeg / ffprobe 不自动下载；可选择已安装工具。现有下载不受工具切换影响。"),
            ))
    }
    fn managed_versions(&self, cx: &Context<Self>) -> Div {
        let mut list = div().v_flex().gap_2();
        if self.tools.versions.is_empty() {
            return list.child(muted(cx, tr("暂无托管版本")));
        }
        for (index, version) in self.tools.versions.iter().enumerate() {
            let selected = self
                .tools
                .effective
                .as_ref()
                .is_some_and(|p| p.yt_dlp == version.path);
            let protected = self.running.values().any(|t| t.tool_path == version.path);
            let activate_path = version.path.clone();
            let activate = version.version.clone();
            let remove = activate.clone();
            list = list.child(
                div()
                    .v_flex()
                    .gap_1()
                    .p_3()
                    .bg(cx.theme().muted)
                    .child(
                        div()
                            .h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().flex_1().child(format!(
                                "{}{}",
                                if selected { "✓ " } else { "" },
                                version.version
                            )))
                            .child(
                                Button::new(("use-version", index))
                                    .label(tr("使用此版本"))
                                    .disabled(self.tools.busy || selected)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.tools.yt_dlp.update(cx, |s, cx| {
                                            s.set_value(
                                                activate_path.display().to_string(),
                                                window,
                                                cx,
                                            )
                                        });
                                        this.manage_tools(Operation::Activate(activate.clone()), cx)
                                    })),
                            )
                            .child(
                                Button::new(("delete-version", index))
                                    .label(tr("删除版本"))
                                    .disabled(
                                        self.tools.busy || self.busy || selected || version.active || protected,
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.manage_tools(Operation::Remove(remove.clone()), cx)
                                    })),
                            ),
                    )
                    .child(muted(cx, version.path.display().to_string())),
            );
        }
        list
    }
}
