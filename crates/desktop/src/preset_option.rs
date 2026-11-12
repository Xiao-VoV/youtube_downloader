//! Keep stable preset identity out of the visible dropdown label.
use gpui_kit::{SharedString, component::select::SelectItem};
#[derive(Clone)]
pub(crate) struct PresetOption {
    pub id: String,
    pub name: String,
}
#[cfg(test)]
mod tests {
    use super::*;
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
