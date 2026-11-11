//! UI-only catalog: never translate remote titles, paths, command text or diagnostics.
use std::sync::atomic::{AtomicU8, Ordering};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Zh,
    En,
}
static LANGUAGE: AtomicU8 = AtomicU8::new(0);
pub fn set_language(value: Language) {
    LANGUAGE.store(if value == Language::En { 1 } else { 0 }, Ordering::Relaxed);
}
pub fn language() -> Language {
    if LANGUAGE.load(Ordering::Relaxed) == 1 {
        Language::En
    } else {
        Language::Zh
    }
}
pub fn tr(source: &str) -> String {
    translate(source, language()).to_owned()
}
fn translate(source: &str, language: Language) -> &str {
    if language == Language::Zh {
        return source;
    }
    CATALOG
        .iter()
        .find(|(zh, _)| *zh == source)
        .map(|(_, en)| *en)
        .unwrap_or(source)
}
const CATALOG: &[(&str, &str)] = &[
    (
        "粘贴 YouTube 视频链接，例如 https://youtu.be/…",
        "Paste a YouTube video link, e.g. https://youtu.be/…",
    ),
    (
        "选择保存目录，然后粘贴视频链接。",
        "Choose a save location, then paste a video link.",
    ),
    ("正在检查本地工具…", "Checking local tools…"),
    ("正在解析视频信息…", "Analyzing video…"),
    ("请先选择保存目录。", "Choose a save location first."),
    (
        "历史已有此视频，请先确认重新下载。",
        "This video is in history. Confirm downloading again.",
    ),
    (
        "已有 3 个活动下载，请等待任务结束后确认。",
        "Three downloads are active. Wait for a task to finish.",
    ),
    (
        "任务已启动，可继续新建下载或编辑配置。",
        "Download started. You can create another task or edit presets.",
    ),
    ("选择下载保存目录", "Choose download folder"),
    (
        "无法打开目录选择器，请重试。",
        "Cannot open folder picker. Try again.",
    ),
    ("外观", "Appearance"),
    ("跟随系统", "System"),
    ("浅色", "Light"),
    ("深色", "Dark"),
    ("语言", "Language"),
    ("新建任务", "New task"),
    ("新建下载", "New download"),
    ("＋ 新建下载", "＋ New download"),
    ("下载队列", "Downloads"),
    ("下载历史", "History"),
    ("下载配置", "Presets"),
    ("软件设置", "Settings"),
    ("开发预览 0.1", "Development preview 0.1"),
    ("简体中文", "Simplified Chinese"),
    ("下载工具", "Download tools"),
    (
        "自定义路径（留空自动发现）",
        "Custom path (leave blank to discover)",
    ),
    ("保存并检查工具", "Save and check tools"),
    ("检查 yt-dlp 更新", "Check yt-dlp updates"),
    ("回退托管版本", "Revert managed version"),
    (
        "SHA256完整性校验（非签名验证）",
        "SHA256 integrity check (not signature verification)",
    ),
    (
        "确认下载并安装 yt-dlp",
        "Confirm download and install yt-dlp",
    ),
    (
        "FFmpeg / ffprobe 不自动下载；可选择已安装工具。现有下载不受工具切换影响。",
        "FFmpeg / ffprobe are never downloaded automatically; select installed tools. Existing downloads are unaffected by tool changes.",
    ),
    ("保存选项", "Save options"),
    ("应用于当前视频", "For the current video"),
    ("文件保护", "File protection"),
    (
        "每次下载创建独立子目录",
        "Separate temporary directory per download",
    ),
    ("不覆盖已有文件", "Never overwrite existing files"),
    ("仅使用已安装的本地工具", "Use installed local tools only"),
    ("格式与质量", "Format and quality"),
    ("兼容 MP4", "Compatible MP4"),
    ("最高 1080p · H.264 / AAC", "Up to 1080p · H.264 / AAC"),
    ("原始音频", "Original audio"),
    (
        "保留源格式，不进行转码",
        "Keep source format without transcoding",
    ),
    ("MP3 音频", "MP3 audio"),
    (
        "转换为常用音频格式",
        "Convert to a widely supported audio format",
    ),
    ("保存位置", "Save location"),
    ("尚未选择文件夹", "No folder selected"),
    ("选择文件夹…", "Choose folder…"),
    ("↓  开始下载", "↓  Start download"),
    ("取消当前任务", "Cancel current task"),
    (
        "正在取消并回收进程，请稍候…",
        "Cancelling and stopping processes…",
    ),
    ("无队列 · 无自动重试", "No queue · No automatic retries"),
    ("已切换", "Switched to"),
    (
        "配置已用于本次会话，但无法保存选择",
        "Preset applied for this session, but selection could not be saved",
    ),
    ("配置名称", "Preset name"),
    ("代理 URL", "Proxy URL"),
    (
        "留空直连；支持 http(s)、socks4/5。含凭据的代理仅本次会话使用。",
        "Leave blank for a direct connection; supports HTTP(S) and SOCKS4/5. Authenticated proxies are session-only.",
    ),
    ("保存位置…", "Save location…"),
    ("未选择", "Not selected"),
    ("文件名模板", "Filename template"),
    (
        "支持 title/id/channel/uploader/extractor。文件名必须以 .%(ext)s 结尾；不允许绝对路径或越界。",
        "Supports title/id/channel/uploader/extractor. Must end in .%(ext)s; absolute paths and traversal are forbidden.",
    ),
    ("子目录模板", "Subfolder template"),
    (
        "留空直接保存；示例 %(extractor)s/%(channel)s。只允许安全相对路径。",
        "Leave blank to save directly; example: %(extractor)s/%(channel)s. Safe relative paths only.",
    ),
    ("另存为配置", "Save as preset"),
    ("保存修改", "Save changes"),
    ("确认删除配置", "Confirm preset deletion"),
    ("删除配置", "Delete preset"),
    ("应用并返回", "Apply and return"),
    ("▾ 高级参数", "▾ Advanced options"),
    ("▸ 高级参数", "▸ Advanced options"),
    (
        "每行一个参数或值；支持字幕、限速与重试。",
        "One argument or value per line; subtitles, rate limits and retries are supported.",
    ),
    ("分辨率上限", "Resolution limit"),
    (
        "最佳可用不限制高度；MP4仍限定H.264/AAC，MKV允许其他源编码。",
        "Best available has no height limit; MP4 requires H.264/AAC, while MKV allows other source codecs.",
    ),
    ("✓ 最佳可用", "✓ Best available"),
    ("最佳可用", "Best available"),
    ("原音频", "Original audio"),
    ("输出格式", "Output format"),
    (
        "MP4兼容编码；MKV保留源编码；原音频不转码；MP3显式转码。",
        "MP4 uses compatible codecs; MKV keeps source codecs; original audio is not transcoded; MP3 is explicitly converted.",
    ),
    ("不使用", "None"),
    ("浏览器 Cookie", "Browser cookies"),
    (
        "仅显式选择后读取浏览器Cookie；解析和下载共享选择，可能触发系统授权。",
        "Browser cookies are read only when selected; analysis and download share this setting and may request system permission.",
    ),
    ("浏览器 Profile", "Browser profile"),
    (
        "留空默认；可填 Default、Profile 1 或目录路径。不接受复合cookie语法。",
        "Leave blank for the default; enter Default, Profile 1 or a directory path. Compound cookie syntax is not accepted.",
    ),
    ("历史存储", "History storage"),
    ("暂无下载历史", "No download history yet"),
    ("共", "Total"),
    ("条", "entries"),
    ("第", "Page"),
    ("页", "page"),
    ("上一页", "Previous"),
    ("下一页", "Next"),
    ("名称", "Name"),
    ("状态", "Status"),
    ("来源", "Source"),
    ("下载时间", "Downloaded at"),
    ("分辨率", "Resolution"),
    ("格式", "Format"),
    ("大小", "Size"),
    ("原始链接", "Original link"),
    ("操作", "Actions"),
    ("打开原链接 ↗", "Open original link ↗"),
    ("Finder 中显示", "Show in Finder"),
    ("路径不可用", "Path unavailable"),
    ("确认删除", "Confirm deletion"),
    ("删除", "Delete"),
    (
        "历史记录已删除，媒体文件保留。",
        "History entry deleted; media files were kept.",
    ),
    (
        "暂无活动任务 · 已完成任务见下载历史",
        "No active downloads · Completed tasks appear in history",
    ),
    ("取消", "Cancel"),
    ("正在停止…", "Stopping…"),
    ("SHIGUANG / 新建任务", "SHIGUANG / NEW DOWNLOAD"),
    ("值得收藏，一键留下。", "Worth keeping. One click away."),
    (
        "粘贴视频链接，选择你的下载方式。",
        "Paste a video link and choose how to download.",
    ),
    ("视频链接", "Video link"),
    ("从剪贴板粘贴", "Paste from clipboard"),
    ("使用下载配置", "Download preset"),
    ("最高", "Up to"),
    ("尚未选择保存位置", "No save location selected"),
    ("解析链接", "Analyze link"),
    ("编辑下载配置", "Edit download preset"),
    ("下一步：查看命令", "Next: review command"),
    ("音频", "Audio"),
    ("最佳可用分辨率", "Best available resolution"),
    ("默认 Profile", "Default profile"),
    ("视频", "Video"),
    ("输出", "Output"),
    ("保存到", "Save to"),
    ("代理", "Proxy"),
    ("已启用（地址隐藏）", "Enabled (address hidden)"),
    ("直连", "Direct connection"),
    ("高级参数", "Advanced options"),
    ("个参数项", "arguments"),
    (
        "配置无效，请检查高级参数或返回修改",
        "Invalid configuration; check advanced options or go back",
    ),
    (
        "▾ 完整命令与高级参数",
        "▾ Full command and advanced options",
    ),
    (
        "▸ 完整命令与高级参数",
        "▸ Full command and advanced options",
    ),
    (
        "高级参数每行一个参数或值；认证信息已隐藏。",
        "One argument or value per line; credentials are hidden.",
    ),
    (
        "下载历史中已存在此视频，是否重新下载？",
        "This video is already in history. Download it again?",
    ),
    ("✓ 已确认重新下载", "✓ Download again confirmed"),
    ("仍然重新下载", "Download again anyway"),
    ("参数校验通过", "Configuration validated"),
    (
        "等待确认重新下载",
        "Waiting for download-again confirmation",
    ),
    (
        "配置无效，请检查高级参数",
        "Invalid configuration; check advanced options",
    ),
    ("确认下载", "Confirm download"),
    ("复制命令", "Copy command"),
    ("返回", "Back"),
    ("已取消，进程已回收", "Cancelled; processes stopped"),
    (
        "链接无效：仅支持 YouTube 单视频链接",
        "Invalid link: only individual YouTube video links are supported",
    ),
    (
        "未找到本地下载工具，请配置 yt-dlp / FFmpeg",
        "Local download tools not found; configure yt-dlp / FFmpeg",
    ),
    ("任务超时，进程已停止", "Task timed out; processes stopped"),
    (
        "引擎执行失败：请检查网络、工具版本及站点挑战依赖",
        "Engine failed: check the network, tool version and site challenge dependencies",
    ),
    ("任务失败", "Task failed"),
    ("解析完成", "Analysis complete"),
    ("历史保存失败", "Could not save history"),
    ("正在启动…", "Starting…"),
    ("正在合并或转换…", "Merging or converting…"),
    ("下载中", "Downloading"),
    ("正在下载…", "Downloading…"),
    ("准备下载…", "Preparing download…"),
    ("请输入配置名称", "Enter a preset name"),
    ("配置已更新", "Preset updated"),
    (
        "再次点击确认删除配置；不会影响运行任务或已下载文件。",
        "Click again to delete the preset; running tasks and downloaded files are unaffected.",
    ),
    (
        "配置已删除，当前参数保留为未保存草稿",
        "Preset deleted; current options remain as an unsaved draft",
    ),
    ("配置已保存", "Preset saved"),
    (
        "自定义路径优先，其次托管版本，最后系统 PATH。",
        "Priority: custom path, managed version, then system PATH.",
    ),
    ("正在处理工具请求…", "Processing tool request…"),
    ("官方版本", "Official version"),
    (
        "可用；点击确认安装才会下载可执行文件。",
        "available; the executable is downloaded only after confirming installation.",
    ),
    (
        "yt-dlp健康检查失败；当前运行任务不受影响。",
        "yt-dlp health check failed; running tasks are unaffected.",
    ),
    ("就绪", "Ready"),
    ("未配置", "Not configured"),
    ("无法定位设置目录", "Cannot locate settings directory"),
    ("无法创建设置目录", "Cannot create settings directory"),
    ("工具设置编码失败", "Could not encode tool settings"),
    ("工具设置保存失败", "Could not save tool settings"),
    ("工具设置提交失败", "Could not commit tool settings"),
    ("进行中", "Running"),
    ("已完成", "Completed"),
    ("失败", "Failed"),
    ("已取消", "Cancelled"),
    ("已中断", "Interrupted"),
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_unique_nonempty_translations() {
        let mut keys = std::collections::HashSet::new();
        for (zh, en) in CATALOG {
            assert!(keys.insert(zh), "duplicate {zh}");
            assert!(!en.is_empty());
            assert!(
                !en.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                "untranslated {zh}"
            );
            assert_eq!(translate(zh, Language::En), *en);
            assert_eq!(translate(zh, Language::Zh), *zh);
        }
    }
    #[test]
    fn main_flow_source_catalog_is_complete() {
        let sources = [
            include_str!("view/workspace.rs"),
            include_str!("view/profiles.rs"),
            include_str!("view/history.rs"),
            include_str!("view/inspector.rs"),
            include_str!("messages.rs"),
            include_str!("preset_actions.rs"),
            include_str!("tool_settings.rs"),
        ];
        for source in sources {
            // Static UI literals and tips must be catalogued. Dynamic format
            // strings translate fixed fragments separately, not remote values.
            for (index, literal) in source.split('"').enumerate() {
                if index % 2 == 1
                    && literal
                        .chars()
                        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
                    && !literal.contains('{')
                {
                    assert!(
                        CATALOG.iter().any(|(zh, _)| *zh == literal),
                        "missing UI catalog entry: {literal}"
                    );
                }
            }
        }
    }
    #[test]
    fn unknown_remote_content_is_not_translated() {
        assert_eq!(
            translate("A media title /tmp/path", Language::En),
            "A media title /tmp/path"
        );
    }
}
