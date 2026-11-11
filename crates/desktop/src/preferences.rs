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
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library/Application Support/Shiguang/software.json"))
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
    let tmp = path.with_extension("json.new");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(&doc).map_err(|_| "设置编码失败")?,
    )
    .map_err(|_| "设置保存失败")?;
    fs::rename(tmp, path).map_err(|_| "设置提交失败".into())
}
pub(crate) fn apply(value: Appearance, window: &mut Window, cx: &mut App) {
    match value {
        Appearance::System => Theme::sync_system_appearance(Some(window), cx),
        Appearance::Light => Theme::change(ThemeMode::Light, Some(window), cx),
        Appearance::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
    }
}
