//! History is metadata only: removing records never deletes media files.
use super::muted;
use crate::{app::Downloader, history::HistoryStatus};
use gpui_kit::{
    component::{button::*, *},
    *,
};
impl Downloader {
    pub(super) fn history_view(&self, cx: &Context<Self>) -> Div {
        let mut body = div().v_flex().gap_3();
        if let Some(error) = &self.history.error {
            body = body.child(muted(cx, format!("历史存储：{error}")));
        }
        if self.history.entries.is_empty() {
            return body.child(muted(cx, "暂无下载历史"));
        }
        for (index, entry) in self.history.entries.iter().rev().enumerate() {
            let status = match entry.status {
                HistoryStatus::Running => "进行中",
                HistoryStatus::Succeeded => "已完成",
                HistoryStatus::Failed => "失败",
                HistoryStatus::Cancelled => "已取消",
                HistoryStatus::Interrupted => "已中断",
            };
            let mut card = div()
                .v_flex()
                .gap_2()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .h_flex()
                        .justify_between()
                        .child(entry.title.clone())
                        .child(muted(cx, status)),
                )
                .child(muted(cx, entry.url.clone()))
                .child(muted(cx, format!("开始时间：Unix {}", entry.started_at)));
            for (n, artifact) in entry.artifacts.iter().enumerate() {
                let path = artifact.path.clone();
                let exists = path.is_file();
                card = card.child(
                    Button::new(("history-file", index * 32 + n))
                        .label(if exists {
                            format!(
                                "Finder · {}",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            )
                        } else {
                            "文件已移动或删除".into()
                        })
                        .disabled(!exists)
                        .on_click(move |_, _, cx| cx.reveal_path(&path)),
                );
            }
            body = body.child(card);
        }
        body
    }
}
