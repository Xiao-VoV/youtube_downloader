//! Preset mutations commit storage before replacing UI state.
use crate::app::Downloader;
use crate::i18n::tr;
use gpui_kit::*;
use youtube_engine::Preset;
impl Downloader {
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
