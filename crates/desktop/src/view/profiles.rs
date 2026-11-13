//! Preset editor: common controls first, advanced arguments explicitly separated.
use super::{
    controls::{self, Tone},
    muted,
};
use crate::i18n::tr;
use crate::{app::Downloader, settings::Page};
use gpui_kit::{
    component::{input::Textarea, *},
    *,
};
use youtube_engine::*;
impl Downloader {
    pub(super) fn preset_switcher(&self, cx: &Context<Self>) -> Div {
        let mut row = div().v_flex().gap_2().w(px(156.)).flex_shrink_0();
        for (i, preset) in self.settings.store.presets.iter().enumerate() {
            let preset = preset.clone();
            row = row.child(
                controls::button(("preset", i), preset.name.clone(), Tone::Quiet, cx)
                    .w_full()
                    .h(px(42.))
                    .icon(super::design::glyph(
                        if preset.profile.mode == Mode::Video {
                            "video"
                        } else {
                            "music"
                        },
                    ))
                    .bg(
                        if self.settings.store.active_preset_id.as_ref() == Some(&preset.id) {
                            super::design::selection(cx)
                        } else {
                            super::design::surface(cx)
                        },
                    )
                    .text_color(
                        if self.settings.store.active_preset_id.as_ref() == Some(&preset.id) {
                            super::design::accent()
                        } else {
                            super::design::muted(cx)
                        },
                    )
                    .disabled(self.busy)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.mode = preset.profile.mode;
                        this.settings.profile = preset.profile.clone();
                        this.output = preset.profile.output_dir.clone();
                        this.settings.store.active_preset_id = Some(preset.id.clone());
                        this.status = match crate::settings::save(&this.settings.store) {
                            Ok(()) => format!("{}: {}", tr("已切换"), preset.name),
                            Err(error) => {
                                format!("{}: {error}", tr("配置已用于本次会话，但无法保存选择"))
                            }
                        };
                        this.settings
                            .name
                            .update(cx, |s, cx| s.set_value(preset.name.clone(), window, cx));
                        this.settings.proxy.update(cx, |s, cx| {
                            s.set_value(
                                preset.profile.proxy.clone().unwrap_or_default(),
                                window,
                                cx,
                            )
                        });
                        this.settings.browser_profile.update(cx, |s, cx| {
                            s.set_value(
                                preset.profile.browser_profile.clone().unwrap_or_default(),
                                window,
                                cx,
                            )
                        });
                        this.settings.filename_template.update(cx, |s, cx| {
                            s.set_value(preset.profile.filename_template.clone(), window, cx)
                        });
                        this.settings.folder_template.update(cx, |s, cx| {
                            s.set_value(
                                preset.profile.folder_template.clone().unwrap_or_default(),
                                window,
                                cx,
                            )
                        });
                        this.settings.extra.update(cx, |s, cx| {
                            s.set_value(preset.profile.extra_args.join("\n"), window, cx)
                        });
                        this.media = None;
                        this.pending = None;
                        cx.notify();
                    })),
            );
        }
        row
    }
    pub(super) fn profile_editor(&self, cx: &Context<Self>) -> Div {
        let options = self.page == Page::TaskOptions;
        let content = div()
            .v_flex().gap_5().flex_1().min_w_0()
            .child(super::design::section(cx, tr("画质与格式"))
                .child(super::design::group(cx).v_flex()
                    .children((!options).then(|| super::fields::row(
                        div().child(tr("配置名称")), controls::input(&self.settings.name, cx), cx)))
                    .child(self.quality_controls(cx))))
            .child(super::design::section(cx, tr("保存位置"))
                .child(super::design::group(cx).v_flex()
                    .child(super::fields::row(div().child(tr("保存到")),
                        div().h_flex().items_center().gap_3()
                            .child(muted(cx, self.output.as_ref().map(|p| p.display().to_string()).unwrap_or(tr("未选择")))
                                .flex_1().min_w_0().overflow_hidden())
                            .child(controls::button("folder", tr("更改"), Tone::Secondary, cx).icon(IconName::FolderOpen)
                                .on_click(cx.listener(|this, _, _, cx| this.choose_directory(cx)))), cx))
                    .child(super::fields::row(
                        super::help::label("文件名模板", "支持 title/id/channel/uploader/extractor。文件名必须以 .%(ext)s 结尾；不允许绝对路径或越界。", "output-template"),
                        controls::input(&self.settings.filename_template, cx), cx))
                    .child(super::fields::row(
                        super::help::label("子目录模板", "留空直接保存；示例 %(extractor)s/%(channel)s。只允许安全相对路径。", "output-template"),
                        controls::input(&self.settings.folder_template, cx), cx))))
            .child(self.network_controls(cx))
            .child(self.advanced_controls(cx))
            .children((!options).then(|| div().h_flex().flex_wrap().items_center().gap_2()
                .pt_3()
                .child(controls::button("delete-preset", "", Tone::Danger, cx)
                    .label(if self.settings.delete_candidate == self.settings.store.active_preset_id
                        && self.settings.delete_candidate.is_some() { tr("确认删除配置") } else { tr("删除配置") })
                    .disabled(self.settings.store.active_preset_id.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.delete_preset(cx))))
                .child(div().flex_1())
                .child(controls::button("save-preset", tr("另存为配置"), Tone::Secondary, cx)
                    .on_click(cx.listener(|this, _, _, cx| this.save_preset(cx))))
                .child(controls::button("update-preset", tr("保存修改"), Tone::Primary, cx).disabled(self.settings.store.active_preset_id.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.update_preset(cx))))));
        div()
            .h_flex()
            .items_start()
            .gap_5()
            .w_full()
            .max_w(px(900.))
            .mx_auto()
            .children(
                (!options && !self.settings.store.presets.is_empty())
                    .then(|| self.preset_switcher(cx)),
            )
            .child(content)
    }
    fn network_controls(&self, cx: &Context<Self>) -> Div {
        super::design::section(cx, tr("网络与浏览器授权")).child(
            super::design::group(cx)
                .v_flex()
                .child(self.cookie_controls(cx))
                .child(super::fields::row(
                    super::help::label(
                        "代理 URL",
                        "留空直连；支持 http(s)、socks4/5。含凭据的代理仅本次会话使用。",
                        "network-options",
                    ),
                    controls::input(&self.settings.proxy, cx),
                    cx,
                )),
        )
    }
    fn advanced_controls(&self, cx: &Context<Self>) -> Div {
        let mut section = div().v_flex().gap_2().child(
            controls::disclosure(
                "advanced-toggle",
                tr("高级参数"),
                self.settings.advanced_open,
                cx,
            )
            .tooltip(tr("每行一个参数或值；支持字幕、限速与重试。"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.settings.advanced_open = !this.settings.advanced_open;
                cx.notify();
            })),
        );
        if self.settings.advanced_open {
            section = section.child(Textarea::new(&self.settings.extra).h(px(100.)));
        }
        section
    }
    fn quality_controls(&self, cx: &Context<Self>) -> Div {
        let mut row = div().v_flex();
        let mut heights = controls::segments(cx).flex_wrap().child(
            controls::segment(
                "best-resolution",
                tr("最佳可用"),
                self.settings.profile.best_resolution,
                cx,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.settings.profile.best_resolution = true;
                this.media = None;
                cx.notify();
            })),
        );
        for height in [480, 720, 1080, 1440, 2160] {
            heights = heights.child(
                controls::segment(
                    ("height", height as usize),
                    format!("{height}p"),
                    !self.settings.profile.best_resolution
                        && self.settings.profile.max_height == height,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings.profile.best_resolution = false;
                    this.settings.profile.max_height = height;
                    this.media = None;
                    cx.notify();
                })),
            );
        }
        if self.mode == Mode::Video {
            row = row.child(super::fields::row(
                super::help::label(
                    "分辨率上限",
                    "最佳可用不限制高度；MP4仍限定H.264/AAC，MKV允许其他源编码。",
                    "format-selection",
                ),
                heights,
                cx,
            ));
        }
        let mut formats = controls::segments(cx).flex_wrap();
        for (id, label, mode, container) in [
            ("mp4", "MP4", Mode::Video, VideoContainer::Mp4),
            ("mkv", "MKV", Mode::Video, VideoContainer::Mkv),
            ("audio", "原音频", Mode::Audio, VideoContainer::Mp4),
            ("mp3", "MP3", Mode::MP3, VideoContainer::Mp4),
        ] {
            formats = formats.child(
                controls::segment(
                    id,
                    tr(label),
                    self.mode == mode
                        && (mode != Mode::Video || self.settings.profile.container == container),
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.mode = mode;
                    this.settings.profile.container = container.clone();
                    cx.notify();
                })),
            );
        }
        row.child(super::fields::row(
            super::help::label(
                "输出格式",
                "MP4兼容编码；MKV保留源编码；原音频不转码；MP3显式转码。",
                "format-selection",
            ),
            formats,
            cx,
        ))
    }

    fn cookie_controls(&self, cx: &Context<Self>) -> Div {
        let mut row = controls::segments(cx).flex_wrap();
        for (id, label, browser) in [
            ("none", "不使用", None),
            ("chrome", "Chrome", Some(BrowserCookies::Chrome)),
            ("safari", "Safari", Some(BrowserCookies::Safari)),
            ("firefox", "Firefox", Some(BrowserCookies::Firefox)),
            ("edge", "Edge", Some(BrowserCookies::Edge)),
        ] {
            row = row.child(
                controls::segment(
                    id,
                    tr(label),
                    self.settings.profile.cookies_browser == browser,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings.profile.cookies_browser = browser.clone();
                    this.media = None;
                    cx.notify();
                })),
            );
        }
        let mut section = div().v_flex().child(super::fields::row(
            super::help::label(
                "浏览器 Cookie",
                "仅显式选择后读取浏览器Cookie；解析和下载共享选择，可能触发系统授权。",
                "authentication-options",
            ),
            row,
            cx,
        ));
        if self.settings.profile.cookies_browser.is_some() {
            section = section.child(super::fields::row(
                super::help::label(
                    "浏览器 Profile",
                    "留空默认；可填 Default、Profile 1 或目录路径。不接受复合cookie语法。",
                    "authentication-options",
                ),
                controls::input(&self.settings.browser_profile, cx),
                cx,
            ));
            if cfg!(target_os = "macos") {
                section = section.child(super::cookie_help::guidance(
                    "cookie-privacy-settings",
                    false,
                    cx,
                ));
            }
        }
        section
    }
}
