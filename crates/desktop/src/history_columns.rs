//! Visible column preferences. Structural columns cannot be hidden.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(crate) enum Field {
    Title,
    Status,
    Site,
    Time,
    Resolution,
    Format,
    Size,
    Channel,
    Bitrate,
    VideoCodec,
    AudioCodec,
    Duration,
    Finished,
    Url,
    Path,
}
impl Field {
    pub const ALL: [Self; 15] = [
        Self::Title,
        Self::Status,
        Self::Site,
        Self::Time,
        Self::Resolution,
        Self::Format,
        Self::Size,
        Self::Channel,
        Self::Bitrate,
        Self::VideoCodec,
        Self::AudioCodec,
        Self::Duration,
        Self::Finished,
        Self::Url,
        Self::Path,
    ];
    pub fn required(self) -> bool {
        matches!(self, Self::Title | Self::Status | Self::Time)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "名称",
            Self::Status => "状态",
            Self::Site => "来源",
            Self::Time => "下载时间",
            Self::Resolution => "分辨率",
            Self::Format => "格式",
            Self::Size => "视频大小",
            Self::Channel => "UP主 / 频道",
            Self::Bitrate => "码率",
            Self::VideoCodec => "视频编码",
            Self::AudioCodec => "音频编码",
            Self::Duration => "时长",
            Self::Finished => "完成时间",
            Self::Url => "原始链接",
            Self::Path => "保存位置",
        }
    }
    pub fn width(self) -> f32 {
        match self {
            Self::Title => 240.,
            Self::Time | Self::Finished => 190.,
            Self::Channel => 160.,
            Self::Url | Self::Path => 260.,
            _ => 110.,
        }
    }
}
pub(crate) fn normalize(mut fields: Vec<Field>) -> Vec<Field> {
    fields.dedup();
    for f in Field::ALL {
        if f.required() && !fields.contains(&f) {
            fields.push(f);
        }
    }
    Field::ALL
        .into_iter()
        .filter(|f| fields.contains(f))
        .collect()
}
pub(crate) fn load() -> Vec<Field> {
    crate::preferences::path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v["history_columns"].clone()).ok())
        .map(normalize)
        .unwrap_or_else(|| {
            vec![
                Field::Title,
                Field::Status,
                Field::Site,
                Field::Time,
                Field::Resolution,
                Field::Format,
                Field::Size,
                Field::Channel,
            ]
        })
}
pub(crate) fn save(fields: &[Field]) -> Result<(), String> {
    let path = crate::preferences::path().ok_or("Settings unavailable")?;
    let mut doc = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    doc["history_columns"] =
        serde_json::to_value(normalize(fields.to_vec())).map_err(|_| "Invalid columns")?;
    crate::preferences::write_document(&path, &doc)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cannot_hide_required_fields() {
        let fields = normalize(vec![]);
        assert!(fields.contains(&Field::Title));
        assert!(fields.contains(&Field::Status));
        assert!(fields.contains(&Field::Time));
        assert_eq!(
            normalize(vec![Field::Bitrate, Field::Bitrate])
                .iter()
                .filter(|f| **f == Field::Bitrate)
                .count(),
            1
        );
    }
}
