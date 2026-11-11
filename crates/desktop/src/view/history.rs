//! Compact sortable history table. Actions never delete downloaded media.
use super::muted;
use crate::i18n::tr;
use crate::{
    app::Downloader,
    history::{HistoryEntry, HistoryStatus},
    history_table::{Column, size, status},
};
use gpui_kit::{
    base::{Scrollbar, ScrollbarMode},
    component::{button::*, tooltip::Tooltip, *},
    *,
};
impl Downloader {
    pub(super) fn history_view(&self, cx: &Context<Self>) -> Div {
        let mut body = div().v_flex().gap_2();
        if let Some(error) = &self.history.error {
            body = body.child(muted(cx, format!("{}: {error}", tr("历史存储"))));
        }
        if self.history.entries.is_empty() {
            return body.child(muted(cx, tr("暂无下载历史")));
        }
        use crate::history::{HistorySort, PageOrder};
        let order = match self.history_sort.column {
            Column::Title => PageOrder::Title,
            Column::Status => PageOrder::Status,
            Column::Site => PageOrder::Site,
            Column::Time => PageOrder::Time,
            Column::Resolution => PageOrder::Resolution,
            Column::Format => PageOrder::Format,
            Column::Size => PageOrder::Size,
        };
        let page = match self.history.page(
            HistorySort {
                order,
                desc: self.history_sort.descending,
            },
            self.history_page,
            self.history_page_size,
        ) {
            Ok(page) => page,
            Err(error) => return body.child(muted(cx, error)),
        };
        body = body.child(self.history_pagination(page.total, cx));
        let mut table = div().v_flex().w(px(1120.)).child(self.history_header(cx));
        for (index, entry) in page.entries.iter().enumerate() {
            table = table.child(self.history_row(index, entry, cx));
        }
        body.child(
            div()
                .relative()
                .w_full()
                .border_1()
                .border_color(cx.theme().border)
                .rounded_lg()
                .overflow_hidden()
                .bg(cx.theme().background)
                .pb_2()
                .child(
                    div()
                        .id("history-table-horizontal")
                        .w_full()
                        .overflow_x_scroll()
                        .restrict_scroll_to_axis()
                        .track_scroll(&self.history_scroll)
                        .child(table),
                )
                .child(Scrollbar::horizontal(&self.history_scroll).mode(ScrollbarMode::Always)),
        )
    }
    fn history_pagination(&self, total: usize, cx: &Context<Self>) -> Div {
        let pages = total.div_ceil(self.history_page_size).max(1);
        let mut row = div()
            .h_flex()
            .items_center()
            .gap_1()
            .py_2()
            .child(muted(
                cx,
                format!(
                    "{} {total} {} · {} {} / {pages}",
                    tr("共"),
                    tr("条"),
                    tr("第"),
                    self.history_page + 1
                ),
            ))
            .child(
                Button::new("history-prev")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronLeft)
                    .tooltip(tr("上一页"))
                    .disabled(self.history_page == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.history_page = this.history_page.saturating_sub(1);
                        cx.notify();
                    })),
            )
            .child(
                Button::new("history-next")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronRight)
                    .tooltip(tr("下一页"))
                    .disabled(self.history_page + 1 >= pages)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.history_page += 1;
                        cx.notify();
                    })),
            );
        for size in [25, 50, 100] {
            row = row.child(
                Button::new(("history-size", size))
                    .ghost()
                    .xsmall()
                    .label(format!(
                        "{}{size} / {}",
                        if self.history_page_size == size {
                            "✓ "
                        } else {
                            ""
                        },
                        tr("页")
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.history_page_size = size;
                        this.history_page = 0;
                        cx.notify();
                    })),
            );
        }
        row
    }
    fn history_header(&self, cx: &Context<Self>) -> Div {
        let mut row = div()
            .h_flex()
            .items_center()
            .gap_2()
            .px_2()
            .h(px(34.))
            .border_b_1()
            .border_color(cx.theme().border)
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
            let arrow = if self.history_sort.column == column {
                if self.history_sort.descending {
                    " ↓"
                } else {
                    " ↑"
                }
            } else {
                ""
            };
            row = row.child(
                div()
                    .id(("history-sort", index))
                    .w(px(width))
                    .h(px(32.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .px_2()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.theme().border))
                    .child(format!("{}{arrow}", tr(label)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.history_sort.toggle(column);
                        this.history_page = 0;
                        cx.notify();
                    })),
            );
        }
        row.child(header_label(cx, "原始链接", 48.))
            .child(header_label(cx, "保存位置", 48.))
            .child(header_label(cx, "操作", 48.))
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
            .px_2()
            .h(px(36.))
            .hover(|style| style.bg(cx.theme().muted))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(scroll_cell(
                ("history-title", index),
                entry.title.clone(),
                220.,
            ))
            .child(status_badge(cx, entry.status))
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
                div().w(px(48.)).flex_shrink_0().child(
                    Button::new(("history-url", index))
                        .ghost()
                        .xsmall()
                        .icon(IconName::ExternalLink)
                        .tooltip(tr("打开原链接 ↗"))
                        .disabled(!safe)
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                ),
            );
        row = row.child(
            div().w(px(48.)).flex_shrink_0().child(
                Button::new(("history-path", index))
                    .ghost()
                    .xsmall()
                    .icon(IconName::FolderOpen)
                    .tooltip(if path_exists {
                        tr("Finder 中显示")
                    } else {
                        tr("路径不可用")
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
            div().w(px(48.)).flex_shrink_0().child(
                Button::new(("history-delete", index))
                    .ghost()
                    .xsmall()
                    .icon(if deleting {
                        IconName::Check
                    } else {
                        IconName::Delete
                    })
                    .text_color(if deleting {
                        cx.theme().danger
                    } else {
                        cx.theme().muted_foreground
                    })
                    .tooltip(if deleting {
                        tr("确认删除")
                    } else {
                        tr("删除")
                    })
                    .disabled(
                        entry.status == HistoryStatus::Running
                            || self.running.contains_key(&entry.id),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.history_delete.as_ref() != Some(&id) {
                            this.history_delete = Some(id.clone());
                        } else {
                            match this.history.delete(&id) {
                                Ok(()) => this.status = tr("历史记录已删除，媒体文件保留。"),
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
fn status_badge(cx: &Context<Downloader>, value: HistoryStatus) -> Div {
    let color = match value {
        HistoryStatus::Succeeded => cx.theme().success,
        HistoryStatus::Failed => cx.theme().danger,
        HistoryStatus::Running => cx.theme().primary,
        HistoryStatus::Interrupted => cx.theme().warning,
        HistoryStatus::Cancelled => cx.theme().muted_foreground,
    };
    div().w(px(80.)).flex_shrink_0().child(
        div()
            .h_flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .text_xs()
            .text_color(color)
            .bg(cx.theme().muted)
            .child(div().size(px(5.)).rounded_full().bg(color))
            .child(tr(status(value))),
    )
}
fn header_label(cx: &Context<Downloader>, label: &'static str, width: f32) -> Div {
    muted(cx, tr(label))
        .w(px(width))
        .h(px(32.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .px_2()
        .whitespace_nowrap()
        .overflow_hidden()
        .font_semibold()
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
        .restrict_scroll_to_axis()
        .tooltip({
            let text = text.clone();
            move |window, cx| Tooltip::new(text.clone()).build(window, cx)
        })
        .child(div().whitespace_nowrap().text_xs().child(text))
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
