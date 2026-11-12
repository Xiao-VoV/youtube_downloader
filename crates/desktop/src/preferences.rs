//! Software preferences are separate from yt-dlp download presets.
use gpui_kit::{
    component::{Theme, ThemeMode},
    *,
};
use std::{fs, path::PathBuf};
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}
pub(crate) fn path() -> Option<PathBuf> {
    youtube_engine::app_data_dir()
        .ok()
        .map(|root| root.join("software.json"))
}
pub(crate) fn load() -> Appearance {
    path()
        .and_then(|p| fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .map(|v| match v["appearance"].as_str() {
            Some("light") => Appearance::Light,
            Some("dark") => Appearance::Dark,
            _ => Appearance::System,
        })
        .unwrap_or_default()
}
pub(crate) fn save(value: Appearance) -> std::result::Result<(), String> {
    let path = path().ok_or("无法定位设置目录")?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|_| "无法创建设置目录")?;
    let mut doc = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    doc["appearance"] = serde_json::json!(match value {
        Appearance::System => "system",
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    });
    write_document(&path, &doc)
}
pub(crate) fn write_document(
    path: &std::path::Path,
    doc: &serde_json::Value,
) -> std::result::Result<(), String> {
    use std::io::Write;
    if !doc.is_object() {
        return Err("Invalid software settings".into());
    }
    fs::create_dir_all(path.parent().ok_or("Invalid settings path")?)
        .map_err(|_| "Cannot create settings directory")?;
    let tmp = path.with_extension(format!("{}.new", std::process::id()));
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&tmp)
        .map_err(|_| "Cannot create settings temporary file")?;
    file.write_all(&serde_json::to_vec_pretty(doc).map_err(|_| "Settings encoding failed")?)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Settings write failed")?;
    fs::rename(&tmp, path).map_err(|_| "Settings commit failed".into())
}
pub(crate) fn load_language() -> crate::i18n::Language {
    let en = path()
        .and_then(|p| fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|v| v["language"] == "en");
    if en {
        crate::i18n::Language::En
    } else {
        crate::i18n::Language::Zh
    }
}
pub(crate) fn save_language(value: crate::i18n::Language) -> std::result::Result<(), String> {
    let path = path().ok_or("Cannot locate software settings")?;
    let mut doc = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    doc["language"] = serde_json::json!(if value == crate::i18n::Language::En {
        "en"
    } else {
        "zh"
    });
    write_document(&path, &doc)
}
pub(crate) fn apply(value: Appearance, window: &mut Window, cx: &mut App) {
    match value {
        Appearance::System => Theme::sync_system_appearance(Some(window), cx),
        Appearance::Light => Theme::change(ThemeMode::Light, Some(window), cx),
        Appearance::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
    }
}
