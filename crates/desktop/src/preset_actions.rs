//! Preset mutations and shared preset activation for both selectors.
use crate::{app::Downloader, i18n::tr, preset_option, settings};
use gpui_kit::{component::select::SearchableVec, *};
use youtube_engine::Preset;
impl Downloader {
    pub(crate) fn refresh_preset_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let presets = preset_option::items(&self.settings.store);
        let active = self.settings.store.active_preset_id.as_ref();
        self.preset_select.update(cx, |select, cx| {
            select.set_items(SearchableVec::new(presets), window, cx);
            if let Some(id) = active {
                select.set_selected_value(id, window, cx);
            } else {
                select.set_selected_index(None, window, cx);
            }
        });
    }

    pub(crate) fn activate_preset(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((preset, saved)) =
            settings::select_preset(&mut self.settings.store, id, settings::save)
        else {
            return;
        };
        self.status = match saved {
            Ok(()) => format!("{}: {}", tr("已切换"), preset.name),
            Err(error) => format!("{}: {error}", tr("配置已用于本次会话，但无法保存选择")),
        };
        let profile = preset.profile;
        self.mode = profile.mode;
        self.output = profile.output_dir.clone();
        self.settings
            .name
            .update(cx, |s, cx| s.set_value(preset.name, window, cx));
        self.settings.proxy.update(cx, |s, cx| {
            s.set_value(profile.proxy.as_deref().unwrap_or_default(), window, cx)
        });
        self.settings.browser_profile.update(cx, |s, cx| {
            s.set_value(
                profile.browser_profile.as_deref().unwrap_or_default(),
                window,
                cx,
            )
        });
        self.settings.filename_template.update(cx, |s, cx| {
            s.set_value(profile.filename_template.as_str(), window, cx)
        });
        self.settings.folder_template.update(cx, |s, cx| {
            s.set_value(
                profile.folder_template.as_deref().unwrap_or_default(),
                window,
                cx,
            )
        });
        self.settings.extra.update(cx, |s, cx| {
            s.set_value(profile.extra_args.join("\n"), window, cx)
        });
        self.settings.profile = profile;
        self.settings.delete_candidate = None;
        self.media = None;
        self.pending = None;
        cx.notify();
    }

    pub(crate) fn update_preset(&mut self, cx: &mut Context<Self>) {
        let mut store = self.settings.store.clone();
        let Some(entry) = store
            .presets
            .iter_mut()
            .find(|p| Some(&p.id) == store.active_preset_id.as_ref())
        else {
            return;
        };
        let name = self.settings.name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            self.status = tr("请输入配置名称");
            cx.notify();
            return;
        }
        entry.name = name;
        entry.profile = self.settings.snapshot(cx);
        entry.profile.mode = self.mode;
        match crate::settings::save(&store) {
            Ok(()) => {
                self.settings.store = store;
                self.status = tr("配置已更新");
            }
            Err(e) => self.status = e,
        }
        cx.notify();
    }
    pub(crate) fn delete_preset(&mut self, cx: &mut Context<Self>) {
        let mut store = self.settings.store.clone();
        let Some(id) = store.active_preset_id.take() else {
            return;
        };
        if self.settings.delete_candidate.as_ref() != Some(&id) {
            self.settings.delete_candidate = Some(id);
            self.status = tr("再次点击确认删除配置；不会影响运行任务或已下载文件。");
            cx.notify();
            return;
        }
        self.settings.delete_candidate = None;
        store.presets.retain(|p| p.id != id);
        match crate::settings::save(&store) {
            Ok(()) => {
                self.settings.store = store;
                self.status = tr("配置已删除，当前参数保留为未保存草稿");
            }
            Err(e) => self.status = e,
        }
        cx.notify();
    }
    pub(crate) fn save_preset(&mut self, cx: &mut Context<Self>) {
        let name = self.settings.name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            self.status = tr("请输入配置名称");
            cx.notify();
            return;
        }
        let mut profile = self.settings.snapshot(cx);
        profile.mode = self.mode;
        let id = format!(
            "preset-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let mut store = self.settings.store.clone();
        store.presets.push(Preset {
            id: id.clone(),
            name,
            profile,
        });
        store.active_preset_id = Some(id);
        match crate::settings::save(&store) {
            Ok(()) => {
                self.settings.store = store;
                self.status = tr("配置已保存");
            }
            Err(e) => self.status = e,
        }
        cx.notify();
    }
}
