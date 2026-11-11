//! Compact sortable history table. Actions never delete downloaded media.
use super::muted;
use crate::{
    app::Downloader,
    history::{HistoryEntry, HistoryStatus},
    history_table::{Column, size, status},
};
use gpui_kit::{
    component::{button::*, *},
    *,
};
impl Downloader {
    pub(super) fn history_view(&self, cx: &Context<Self>) -> Div {
        let mut body = div().v_flex().gap_2();
        if let Some(error) = &self.history.error {
            body = body.child(muted(cx, format!("历史存储：{error}")));
        }
        if self.history.entries.is_empty() {
            return body.child(muted(cx, "暂无下载历史"));
        }
        let mut table = div().v_flex().w(px(1290.)).child(self.history_header(cx));
        for (index, entry) in self
            .history_sort
            .rows(&self.history.entries)
            .iter()
            .enumerate()
        {
            table = table.child(self.history_row(index, entry, cx));
        }
        body.child(
            div()
                .id("history-table-horizontal")
                .w_full()
                .overflow_x_scroll()
                .child(table),
        )
        .child(muted(
            cx,
            "点击表头排序 · 横向滚动查看更多列 · 删除记录不删除媒体文件",
        ))
    }
    fn history_header(&self, cx: &Context<Self>) -> Div {
        let mut row = div()
            .h_flex()
            .items_center()
            .gap_2()
            .p_2()
            .bg(cx.theme().muted);
        for (index, (label, column, width)) in [
            ("名称", Column::Title, 220.),
            ("状态", Column::Status, 80.),
            ("来源", Column::Site, 90.),
            ("下载时间", Column::Time, 145.),
            ("分辨率", Column::Resolution, 90.),
            ("格式", Column::Format, 80.),
            ("大小", Column::Size, 90.),
        ]
        .into_iter()
        .enumerate()
        {
            row = row.child(
                div().w(px(width)).flex_shrink_0().child(
                    Button::new(("history-sort", index))
                        .label(format!(
                            "{}{}",
                            label,
                            if self.history_sort.column == column {
                                if self.history_sort.descending {
                                    " ↓"
                                } else {
                                    " ↑"
                                }
                            } else {
                                ""
                            }
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.history_sort.toggle(column);
                            cx.notify();
                        })),
                ),
            );
        }
        row.child(cell(cx, "原始链接".into(), 120.))
            .child(cell(cx, "保存位置".into(), 145.))
            .child(cell(cx, "操作".into(), 90.))
    }
    fn history_row(&self, index: usize, entry: &HistoryEntry, cx: &Context<Self>) -> Div {
        let url = if entry.metadata.original_url.is_empty() {
            entry.url.clone()
        } else {
            entry.metadata.original_url.clone()
        };
        let safe = youtube_engine::normalize_url(&url).is_ok();
        let path = entry
            .artifacts
            .first()
            .map(|a| a.path.clone())
            .or(entry.metadata.output_dir.clone());
        let path_exists = path.as_ref().is_some_and(|p| p.exists());
        let id = entry.id.clone();
        let deleting = self.history_delete.as_ref() == Some(&entry.id);
        let mut row = div()
            .h_flex()
            .items_center()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(scroll_cell(
                ("history-title", index),
                entry.title.clone(),
                220.,
            ))
            .child(cell(cx, status(entry.status).into(), 80.))
            .child(cell(cx, entry.metadata.source_site.clone(), 90.))
            .child(cell(cx, format_time(entry.started_at), 145.))
            .child(cell(
                cx,
                entry
                    .metadata
                    .height
                    .map(|h| format!("{h}p"))
                    .unwrap_or("—".into()),
                90.,
            ))
            .child(cell(
                cx,
                entry.metadata.ext.clone().unwrap_or("—".into()),
                80.,
            ))
            .child(cell(cx, format!("{:.1} MB", size(entry) as f64 / 1e6), 90.))
            .child(
                div().w(px(120.)).flex_shrink_0().child(
                    Button::new(("history-url", index))
                        .label("打开原链接 ↗")
                        .disabled(!safe)
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                ),
            );
        row = row.child(
            div().w(px(145.)).flex_shrink_0().child(
                Button::new(("history-path", index))
                    .label(if path_exists {
                        "Finder 中显示"
                    } else {
                        "路径不可用"
                    })
                    .disabled(!path_exists)
                    .on_click(move |_, _, cx| {
                        if let Some(path) = &path {
                            cx.reveal_path(path);
                        }
                    }),
            ),
        );
        row.child(
            div().w(px(90.)).flex_shrink_0().child(
                Button::new(("history-delete", index))
                    .label(if deleting { "确认删除" } else { "删除" })
                    .disabled(
                        entry.status == HistoryStatus::Running
                            || self.running.contains_key(&entry.id),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.history_delete.as_ref() != Some(&id) {
                            this.history_delete = Some(id.clone());
                        } else {
                            match this.history.delete(&id) {
                                Ok(()) => this.status = "历史记录已删除，媒体文件保留。".into(),
                                Err(e) => this.status = e,
                            };
                            this.history_delete = None;
                        }
                        cx.notify();
                    })),
            ),
        )
    }
}
fn cell(cx: &Context<Downloader>, text: String, width: f32) -> Div {
    muted(cx, text)
        .w(px(width))
        .flex_shrink_0()
        .overflow_hidden()
}
pub(super) fn scroll_cell(id: (&'static str, usize), text: String, width: f32) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(width))
        .flex_shrink_0()
        .overflow_x_scroll()
        .child(div().whitespace_nowrap().text_sm().child(text))
}
#[cfg(test)]
mod tests {
    #[test]
    fn utc_dates_handle_epoch_and_leap_day() {
        assert_eq!(super::format_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(super::format_time(1709164800), "2024-02-29 00:00 UTC");
    }
}
fn format_time(seconds: u64) -> String {
    // UTC avoids an additional locale/timezone dependency; persist Unix seconds.
    let days = (seconds / 86400) as i64;
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += if m <= 2 { 1 } else { 0 };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        seconds % 86400 / 3600,
        seconds % 3600 / 60
    )
}
