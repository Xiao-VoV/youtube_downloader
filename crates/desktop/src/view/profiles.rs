//! Preset editor: common controls first, advanced arguments explicitly separated.
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
use youtube_engine::*;
impl Downloader {
    pub(super) fn preset_switcher(&self, cx: &Context<Self>) -> Div {
        let mut row = div().h_flex().gap_2().flex_wrap();
        for (i, preset) in self.settings.store.presets.iter().enumerate() {
            let preset = preset.clone();
            row = row.child(
                Button::new(("preset", i))
                    .label(format!(
                        "{}{}",
                        if self.settings.store.active_preset_id.as_ref() == Some(&preset.id) {
                            "✓ "
                        } else {
                            ""
                        },
                        preset.name
                    ))
                    .bg(
                        if self.settings.store.active_preset_id.as_ref() == Some(&preset.id) {
                            cx.theme().muted
                        } else {
                            cx.theme().background
                        },
                    )
                    .border_color(
                        if self.settings.store.active_preset_id.as_ref() == Some(&preset.id) {
                            cx.theme().foreground
                        } else {
                            cx.theme().border
                        },
                    )
                    .disabled(self.busy)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.mode = preset.profile.mode;
                        this.settings.profile = preset.profile.clone();
                        this.output = preset.profile.output_dir.clone();
                        this.settings.store.active_preset_id = Some(preset.id.clone());
                        this.status = match crate::settings::save(&this.settings.store) {
                            Ok(()) => format!("已切换：{}", preset.name),
                            Err(error) => format!("配置已用于本次会话，但无法保存选择：{error}"),
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
        div()
            .v_flex()
            .gap_3()
            .max_w(px(700.))
            .child(self.preset_switcher(cx))
            .child(div().text_sm().child("配置名称"))
            .child(Input::new(&self.settings.name))
            .child(self.quality_controls(cx))
            .child(self.cookie_controls(cx))
            .child(div().text_sm().child("代理 URL（留空直连）"))
            .child(Input::new(&self.settings.proxy))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Button::new("folder")
                            .label("保存位置…")
                            .on_click(cx.listener(|this, _, _, cx| this.choose_directory(cx))),
                    )
                    .child(muted(
                        cx,
                        self.output
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or("未选择".into()),
                    )),
            )
            .child(self.advanced_controls(cx))
            .child(
                div()
                    .h_flex()
                    .gap_3()
                    .child(
                        Button::new("save-preset")
                            .primary()
                            .label("另存为配置")
                            .on_click(cx.listener(|this, _, _, cx| this.save_preset(cx))),
                    )
                    .child(
                        Button::new("update-preset")
                            .label("保存修改")
                            .disabled(self.settings.store.active_preset_id.is_none())
                            .on_click(cx.listener(|this, _, _, cx| this.update_preset(cx))),
                    )
                    .child(
                        Button::new("delete-preset")
                            .label(
                                if self.settings.delete_candidate
                                    == self.settings.store.active_preset_id
                                    && self.settings.delete_candidate.is_some()
                                {
                                    "确认删除配置"
                                } else {
                                    "删除配置"
                                },
                            )
                            .disabled(self.settings.store.active_preset_id.is_none())
                            .on_click(cx.listener(|this, _, _, cx| this.delete_preset(cx))),
                    )
                    .child(Button::new("use").label("应用并返回").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.media = None;
                            this.page = Page::New;
                            cx.notify();
                        },
                    ))),
            )
    }
    fn advanced_controls(&self, cx: &Context<Self>) -> Div {
        let mut section = div().v_flex().gap_2().child(
            Button::new("advanced-toggle")
                .label(if self.settings.advanced_open {
                    "▾ 高级参数"
                } else {
                    "▸ 高级参数"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.settings.advanced_open = !this.settings.advanced_open;
                    cx.notify();
                })),
        );
        if self.settings.advanced_open {
            section = section
                .child(Textarea::new(&self.settings.extra).h(px(100.)))
                .child(muted(cx, "每行一个参数或值；支持字幕、限速与重试。"));
        }
        section
    }
    fn quality_controls(&self, cx: &Context<Self>) -> Div {
        let mut row = div()
            .v_flex()
            .gap_2()
            .child(div().text_sm().child("分辨率上限"));
        let mut heights = div().h_flex().gap_2().flex_wrap().child(
            Button::new("best-resolution")
                .label(if self.settings.profile.best_resolution {
                    "✓ 最佳可用"
                } else {
                    "最佳可用"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.settings.profile.best_resolution = true;
                    this.media = None;
                    cx.notify();
                })),
        );

        for height in [480, 720, 1080, 1440, 2160] {
            heights = heights.child(
                Button::new(("height", height as usize))
                    .label(format!(
                        "{}{}p",
                        if !self.settings.profile.best_resolution
                            && self.settings.profile.max_height == height
                        {
                            "✓ "
                        } else {
                            ""
                        },
                        height
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.profile.best_resolution = false;
                        this.settings.profile.max_height = height;
                        this.media = None;
                        cx.notify();
                    })),
            );
        }
        row = row.child(heights);
        let mut formats = div().h_flex().gap_2();
        for (id, label, mode, container) in [
            ("mp4", "MP4", Mode::Video, VideoContainer::Mp4),
            ("mkv", "MKV", Mode::Video, VideoContainer::Mkv),
            ("audio", "原音频", Mode::Audio, VideoContainer::Mp4),
            ("mp3", "MP3", Mode::MP3, VideoContainer::Mp4),
        ] {
            formats = formats.child(
                Button::new(id)
                    .label(format!(
                        "{}{}",
                        if self.mode == mode
                            && (mode != Mode::Video || self.settings.profile.container == container)
                        {
                            "✓ "
                        } else {
                            ""
                        },
                        label
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.mode = mode;
                        this.settings.profile.container = container.clone();
                        cx.notify();
                    })),
            );
        }
        row.child(div().text_sm().child("输出格式")).child(formats)
    }
    fn cookie_controls(&self, cx: &Context<Self>) -> Div {
        let mut row = div().h_flex().gap_2().flex_wrap();
        for (id, label, browser) in [
            ("none", "不使用", None),
            ("chrome", "Chrome", Some(BrowserCookies::Chrome)),
            ("safari", "Safari", Some(BrowserCookies::Safari)),
            ("firefox", "Firefox", Some(BrowserCookies::Firefox)),
            ("edge", "Edge", Some(BrowserCookies::Edge)),
        ] {
            row = row.child(
                Button::new(id)
                    .label(format!(
                        "{}{}",
                        if self.settings.profile.cookies_browser == browser {
                            "✓ "
                        } else {
                            ""
                        },
                        label
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.profile.cookies_browser = browser.clone();
                        this.media = None;
                        cx.notify();
                    })),
            );
        }
        let mut section = div()
            .v_flex()
            .gap_2()
            .child(div().text_sm().child("浏览器 Cookie"))
            .child(row);
        if self.settings.profile.cookies_browser.is_some() {
            section = section
                .child(div().text_xs().child("浏览器 Profile（留空使用默认选择）"))
                .child(Input::new(&self.settings.browser_profile))
                .child(muted(
                    cx,
                    "例如 Default、Profile 1 或配置目录路径；可能触发系统授权。",
                ));
        }
        section
    }
}
