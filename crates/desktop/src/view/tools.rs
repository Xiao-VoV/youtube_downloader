//! Tool summaries are read-only; editable paths are a separate explicit mode.
use super::{
    controls::{self, Tone},
    design, muted,
};
use crate::{app::Downloader, i18n::tr, tool_settings::Operation};
use gpui_kit::{
    component::{tooltip::Tooltip, *},
    *,
};
impl Downloader {
    pub(super) fn tool_controls(&self, cx: &Context<Self>) -> Div {
        let mut list = design::group(cx).v_flex().overflow_hidden();
        for (index, (name, icon, input, effective)) in [
            (
                "yt-dlp",
                "download",
                &self.tools.yt_dlp,
                self.tools.effective.as_ref().map(|p| &p.yt_dlp),
            ),
            (
                "FFmpeg",
                "video",
                &self.tools.ffmpeg,
                self.tools
                    .effective
                    .as_ref()
                    .and_then(|p| p.ffmpeg.as_ref()),
            ),
            (
                "ffprobe",
                "info",
                &self.tools.ffprobe,
                self.tools
                    .effective
                    .as_ref()
                    .and_then(|p| p.ffprobe.as_ref()),
            ),
            (
                "Deno",
                "code",
                &self.tools.deno,
                self.tools.effective.as_ref().and_then(|p| p.deno.as_ref()),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let path = effective
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| tr("未配置"));
            let mut text = div().v_flex().gap_2().flex_1().min_w_0().child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_size(px(13.)).font_semibold().child(name))
                    .children(
                        (name == "yt-dlp").then(|| controls::badge(tr("下载引擎"), false, cx)),
                    ),
            );
            if self.tool_paths_open {
                text = text.child(controls::input(input, cx).disabled(self.tools.busy));
                let edited = input.read(cx).value().trim().to_owned();
                if edited != path && effective.is_some() {
                    text = text.child(muted(
                        cx,
                        format!("{}: {}", tr("当前生效路径"), compact_path(&path)),
                    ));
                }
            } else {
                text = text.child(
                    div()
                        .id(("tool-location", index))
                        .text_size(px(11.))
                        .text_color(design::muted(cx))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .tooltip({
                            let path = path.clone();
                            move |window, cx| Tooltip::new(path.clone()).build(window, cx)
                        })
                        .child(compact_path(&path)),
                );
            }
            list = list.child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .p_4()
                    .border_b_1()
                    .border_color(design::border(cx))
                    .child(
                        div()
                            .w(px(34.))
                            .h(px(34.))
                            .flex_shrink_0()
                            .rounded(px(10.))
                            .bg(design::canvas(cx))
                            .text_color(design::muted(cx))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(design::glyph(icon)),
                    )
                    .child(text)
                    .children((!self.tool_paths_open).then(|| {
                        controls::badge(
                            tr(if effective.is_some() {
                                "已定位"
                            } else {
                                "未配置"
                            }),
                            false,
                            cx,
                        )
                    })),
            );
        }
        let mut toolbar = div()
            .h_flex()
            .items_center()
            .gap_2()
            .child(design::heading(tr("本地工具")))
            .child(div().flex_1());
        toolbar = toolbar.child(
            controls::button(
                "edit-paths",
                tr(if self.tool_paths_open {
                    "收起编辑"
                } else {
                    "编辑路径"
                }),
                Tone::Quiet,
                cx,
            )
            .disabled(self.tools.busy)
            .on_click(cx.listener(|this, _, _, cx| {
                this.tool_paths_open = !this.tool_paths_open;
                cx.notify();
            })),
        );
        let actions = div()
            .h_flex()
            .items_center()
            .flex_wrap()
            .gap_2()
            .child(
                controls::button("tools-discover", tr("自动探测"), Tone::Secondary, cx)
                    .icon(IconName::Search)
                    .disabled(self.tools.busy)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.tool_paths_open = true;
                        this.rediscover_tools(window, cx);
                    })),
            )
            .child(
                controls::button("tools-validate", tr("检查输入路径"), Tone::Secondary, cx)
                    .icon(IconName::CircleCheck)
                    .disabled(self.tools.busy)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.manage_tools(Operation::Validate, cx)),
                    ),
            )
            .child(div().flex_1())
            .child(
                controls::button("tools-apply", tr("保存并应用"), Tone::Primary, cx)
                    .icon(IconName::Check)
                    .disabled(self.tools.busy)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.manage_tools(Operation::Apply, cx)),
                    ),
            );
        let mut managed = div().v_flex().gap_3().child(
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .child(design::heading(tr("应用托管的 yt-dlp")))
                .child(
                    controls::button(
                        "tools-check",
                        tr(if self.tools.checking {
                            "正在检查更新…"
                        } else {
                            "检查更新"
                        }),
                        Tone::Quiet,
                        cx,
                    )
                    .icon(IconName::RefreshCw)
                    .disabled(self.tools.busy || self.tools.checking)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.manage_tools(Operation::Check, cx)),
                    ),
                ),
        );
        if let Some(release) = &self.tools.release {
            managed = managed.child(
                design::group(cx)
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .v_flex()
                            .gap_2()
                            .flex_1()
                            .min_w_0()
                            .child(controls::badge(tr("官方版本"), true, cx))
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child(release.version.clone()),
                            ),
                    )
                    .child(
                        controls::button(
                            "tools-install",
                            tr("仅下载安装，不切换"),
                            Tone::Secondary,
                            cx,
                        )
                        .icon(IconName::ArrowDown)
                        .disabled(self.tools.busy)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.manage_tools(Operation::Install, cx)),
                        ),
                    ),
            );
        }
        managed = managed.child(self.managed_versions(cx));
        div()
            .v_flex()
            .gap_5()
            .w_full()
            .child(toolbar)
            .child(list)
            .children(self.tool_paths_open.then_some(actions))
            .child(managed)
            .children(
                (!self.tools.status.is_empty()
                    && self.tools.status != tr("自定义路径优先，其次托管版本，最后系统 PATH。"))
                .then(|| {
                    design::group(cx)
                        .px_4()
                        .py_3()
                        .child(muted(cx, self.tools.status.clone()))
                }),
            )
    }
    fn managed_versions(&self, cx: &Context<Self>) -> Div {
        let mut list = design::group(cx).v_flex().overflow_hidden();
        if self.tools.versions.is_empty() {
            return list.child(div().p_5().child(muted(cx, tr("暂无托管版本"))));
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
            let path = version.path.display().to_string();
            list = list.child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .p_4()
                    .border_b_1()
                    .border_color(design::border(cx))
                    .child(
                        div()
                            .w(px(34.))
                            .h(px(34.))
                            .flex_shrink_0()
                            .rounded(px(10.))
                            .bg(if selected {
                                design::selection(cx)
                            } else {
                                design::canvas(cx)
                            })
                            .text_color(if selected {
                                design::accent()
                            } else {
                                design::muted(cx)
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(design::glyph("square-terminal")),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_semibold()
                                            .child(version.version.clone()),
                                    )
                                    .children(
                                        selected.then(|| controls::badge(tr("使用中"), true, cx)),
                                    ),
                            )
                            .child(
                                div()
                                    .id(("managed-path", index))
                                    .text_size(px(11.))
                                    .text_color(design::muted(cx))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .tooltip({
                                        let path = path.clone();
                                        move |window, cx| {
                                            Tooltip::new(path.clone()).build(window, cx)
                                        }
                                    })
                                    .child(compact_path(&path)),
                            ),
                    )
                    .children((!selected).then(|| {
                        controls::button(
                            ("use-version", index),
                            tr("使用此版本"),
                            Tone::Secondary,
                            cx,
                        )
                        .disabled(self.tools.busy)
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.tools.yt_dlp.update(cx, |s, cx| {
                                    s.set_value(activate_path.display().to_string(), window, cx)
                                });
                                this.manage_tools(Operation::Activate(activate.clone()), cx);
                            },
                        ))
                    }))
                    .child(
                        controls::icon_button(
                            ("delete-version", index),
                            IconName::Delete,
                            tr("删除版本"),
                            cx,
                        )
                        .disabled(
                            self.tools.busy || self.busy || selected || version.active || protected,
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
fn compact_path(path: &str) -> String {
    std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .and_then(|home| {
            path.strip_prefix(&(home + "/"))
                .map(|tail| format!("~/{tail}"))
        })
        .unwrap_or_else(|| path.to_owned())
}
