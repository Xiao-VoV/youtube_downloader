//! Keep stable preset identity out of the visible dropdown label.
use gpui_kit::{
    SharedString,
    component::{IndexPath, select::SelectItem},
};
use youtube_engine::ProfileStore;

pub(crate) fn items(store: &ProfileStore) -> Vec<PresetOption> {
    store
        .presets
        .iter()
        .map(|preset| PresetOption {
            id: preset.id.clone(),
            name: preset.name.clone(),
        })
        .collect()
}

pub(crate) fn selected_index(store: &ProfileStore) -> Option<IndexPath> {
    crate::settings::active_preset_index(store).map(|index| IndexPath::default().row(index))
}
#[derive(Clone)]
pub(crate) struct PresetOption {
    pub id: String,
    pub name: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    use youtube_engine::Preset;

    #[test]
    fn dropdown_uses_saved_id_after_presets_are_reordered() {
        let preset = |id: &str| Preset {
            id: id.into(),
            name: "同名配置".into(),
            profile: Default::default(),
        };
        let mut store = ProfileStore {
            presets: vec![preset("a"), preset("b")],
            active_preset_id: Some("b".into()),
        };
        assert_eq!(selected_index(&store), Some(IndexPath::default().row(1)));
        store.presets.swap(0, 1);
        assert_eq!(selected_index(&store), Some(IndexPath::default().row(0)));
        assert_eq!(items(&store)[0].value(), "b");
        store.presets.remove(0);
        assert_eq!(selected_index(&store), None);
    }

    #[test]
    fn duplicate_names_keep_distinct_ids_without_leaking_ids_into_labels() {
        let a = PresetOption {
            id: "internal-a".into(),
            name: "默认配置".into(),
        };
        let b = PresetOption {
            id: "internal-b".into(),
            name: "默认配置".into(),
        };
        assert_eq!(a.title(), b.title());
        assert_ne!(a.value(), b.value());
        assert!(!a.title().contains("internal"));
    }
}
impl SelectItem for PresetOption {
    type Value = String;
    fn title(&self) -> SharedString {
        self.name.clone().into()
    }
    fn value(&self) -> &String {
        &self.id
    }
}
