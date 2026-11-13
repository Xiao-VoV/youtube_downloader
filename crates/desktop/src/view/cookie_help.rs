//! Contextual Cookie help; never infer TCC grant state or read browser data.
use super::{
    controls::{self, Tone},
    muted,
};
use crate::{app::Downloader, i18n::tr};
use gpui_kit::{component::*, *};

pub(super) fn database_access_failure(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    // Match tool diagnostics, not arbitrary cookie-related network/auth failures.
    error.lines().any(|line| {
        (line.contains("could not find") && line.contains("cookies database"))
            || (line.contains("cookie")
                && (line.contains("permission denied") || line.contains("operation not permitted")))
    })
}

pub(super) fn guidance(id: impl Into<ElementId>, failed: bool, cx: &Context<Downloader>) -> Div {
    let mut panel = div().v_flex().gap_2();
    if failed {
        panel = panel
            .child(muted(cx, tr("无法找到或访问浏览器 Cookie 数据库")))
            .child(muted(cx, tr("请先核对浏览器 Profile 路径；Chrome 可在 chrome://version 查看 Profile Path。此错误也可能由 macOS 隐私权限限制引起，并不一定是权限不足。")));
    }
    panel
        .child(muted(cx, tr("macOS 可能限制读取浏览器 Cookie。若路径正确但仍无法访问，请在系统设置 → 隐私与安全性 → 完全磁盘访问权限中添加已安装的 YTDer，授权后完全退出并重新启动。自动化 → Chrome 不是此权限。仅向信任的应用授予此权限。")))
        .child(
            controls::button(id, tr("打开隐私设置"), Tone::Quiet, cx)
                .on_click(|_, _, cx| {
                    cx.stop_propagation();
                    cx.open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles");
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::database_access_failure;
    #[test]
    fn recognizes_database_discovery_or_access_errors_only() {
        for message in [
            "ERROR: could not find chrome cookies database in /profile",
            "Could not find firefox cookies database",
            "Cookies: Permission denied (os error 13)",
            "cookie database: Operation not permitted",
        ] {
            assert!(database_access_failure(message));
        }
        for message in [
            "HTTP 403: sign in with cookies",
            "Unable to decrypt cookies",
            "Permission denied: output.mp4",
            "Probe failed",
            "Cookies loaded successfully",
            "Cookies loaded successfully\nPermission denied: output.mp4",
        ] {
            assert!(!database_access_failure(message));
        }
    }
}
