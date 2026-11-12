//! Local discovery and managed installations have separate explicit actions.
use super::muted;
use crate::{app::Downloader, i18n::tr, tool_settings::Operation};
use gpui_kit::{
    component::{button::*, input::Input, tooltip::Tooltip, *},
    *,
};
impl Downloader {
    pub(super) fn tool_controls(&self, cx: &Context<Self>) -> Div {
        let mut local = card(cx)
            .child(section_title(IconName::FolderOpen, tr("本地工具配置")))
            .child(muted(
                cx,
                tr("自定义路径优先，其次托管版本，最后系统 PATH。"),
            ));
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
            (
                "Deno",
                &self.tools.deno,
                self.tools.effective.as_ref().and_then(|p| p.deno.as_ref()),
            ),
        ] {
            let effective = effective
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| tr("未配置"));
            local = local.child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(68.))
                                    .flex_shrink_0()
                                    .text_sm()
                                    .font_semibold()
                                    .child(label)
                                    .children((label=="Deno").then(||Button::new("deno-help").ghost().xsmall().label("?").tooltip(tr("可选 JavaScript 运行时；用于 YouTube 挑战。仅本机配置，不自动安装。")))),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(input).small().disabled(self.tools.busy)),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .gap_3()
                            .child(div().w(px(68.)).flex_shrink_0())
                            .child(
                                div()
                                    .id(SharedString::from(format!("effective-{label}")))
                                    .flex_1()
                                    .min_w_0()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .tooltip({
                                        let text = effective.clone();
                                        move |window, cx| {
                                            Tooltip::new(text.clone()).build(window, cx)
                                        }
                                    })
                                    .child(format!("{} · {effective}", tr("当前生效路径"))),
                            ),
                    ),
            );
        }
        local = local.child(
            div()
                .h_flex()
                .gap_2()
                .pt_2()
                .flex_wrap()
                .child(
                    Button::new("tools-discover")
                        .small()
                        .ghost()
                        .icon(IconName::Search)
                        .label(tr("自动探测"))
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, window, cx| this.rediscover_tools(window, cx)),
                        ),
                )
                .child(
                    Button::new("tools-validate")
                        .small()
                        .ghost()
                        .icon(IconName::CircleCheck)
                        .label(tr("检查输入路径"))
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| {
                                this.manage_tools(Operation::Validate, cx)
                            }),
                        ),
                )
                .child(div().flex_1())
                .child(
                    Button::new("tools-apply")
                        .small()
                        .primary()
                        .icon(IconName::Check)
                        .label(tr("保存并应用"))
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.manage_tools(Operation::Apply, cx)),
                        ),
                ),
        );
        let managed = card(cx)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .child(section_title(IconName::HardDrive, tr("应用托管的 yt-dlp")).flex_1())
                    .child(
                        Button::new("tools-check")
                            .small()
                            .ghost()
                            .icon(IconName::RefreshCw)
                            .label(tr(if self.tools.checking {
                                "正在检查更新…"
                            } else {
                                "检查 yt-dlp 更新"
                            }))
                            .disabled(self.tools.busy || self.tools.checking)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.manage_tools(Operation::Check, cx)
                            })),
                    ),
            )
            .children(self.tools.release.as_ref().map(|release| {
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(cx.theme().muted)
                    .child(div().flex_1().text_sm().child(format!(
                        "{} {} · SHA256",
                        tr("官方版本"),
                        release.version
                    )))
                    .child(
                        Button::new("tools-install")
                            .small()
                            .ghost()
                            .icon(IconName::ArrowDown)
                            .label(tr("仅下载安装，不切换"))
                            .disabled(self.tools.busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.manage_tools(Operation::Install, cx)
                            })),
                    )
            }))
            .child(self.managed_versions(cx));
        div()
            .v_flex()
            .gap_4()
            .max_w(px(900.))
            .w_full()
            .child(local)
            .child(managed)
            .children(
                (self.tools.status != tr("自定义路径优先，其次托管版本，最后系统 PATH。"))
                    .then(|| div().px_1().child(muted(cx, self.tools.status.clone()))),
            )
    }
    fn managed_versions(&self, cx: &Context<Self>) -> Div {
        let mut list = div().v_flex().gap_1();
        if self.tools.versions.is_empty() {
            return list.child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .py_5()
                    .text_color(cx.theme().muted_foreground)
                    .child(IconName::Inbox)
                    .child(muted(cx, tr("暂无托管版本"))),
            );
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
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .hover(|style| style.bg(cx.theme().muted))
                    .child(
                        div()
                            .text_color(if selected {
                                cx.theme().primary
                            } else {
                                cx.theme().muted_foreground
                            })
                            .child(if selected {
                                IconName::CircleCheck
                            } else {
                                IconName::SquareTerminal
                            }),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child(version.version.clone()),
                            )
                            .child(
                                muted(cx, version.path.display().to_string())
                                    .overflow_hidden()
                                    .whitespace_nowrap(),
                            ),
                    )
                    .child(
                        Button::new(("use-version", index))
                            .small()
                            .ghost()
                            .icon(IconName::Check)
                            .label(tr("使用此版本"))
                            .disabled(self.tools.busy || selected)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.tools.yt_dlp.update(cx, |s, cx| {
                                    s.set_value(activate_path.display().to_string(), window, cx)
                                });
                                this.manage_tools(Operation::Activate(activate.clone()), cx)
                            })),
                    )
                    .child(
                        Button::new(("delete-version", index))
                            .small()
                            .ghost()
                            .icon(IconName::Delete)
                            .tooltip(tr("删除版本"))
                            .disabled(
                                self.tools.busy
                                    || self.busy
                                    || selected
                                    || version.active
                                    || protected,
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.manage_tools(Operation::Remove(remove.clone()), cx)
                            })),
                    ),
            );
        }
        list
    }
}
fn card(cx: &Context<Downloader>) -> Div {
    div()
        .v_flex()
        .gap_3()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
}
fn section_title(icon: IconName, title: String) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap_2()
        .text_sm()
        .font_semibold()
        .child(icon)
        .child(title)
}
