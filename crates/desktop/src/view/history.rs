//! Selectable history table; actions operate on stable record IDs, never row indexes.
use super::muted;
use crate::{
    app::Downloader,
    history::{HistoryEntry, HistorySort, PageOrder},
    history_columns::Field,
    history_table::{Column, size, status},
    i18n::tr,
};
use gpui_kit::{
    base::{Scrollbar, ScrollbarMode},
    component::{button::*, tooltip::Tooltip, *},
    *,
};
impl Downloader {
    pub(super) fn history_view(&self, cx: &Context<Self>) -> Div {
        let order = match self.history_sort.column {
            Column::Title => PageOrder::Title,
            Column::Status => PageOrder::Status,
            Column::Site => PageOrder::Site,
            Column::Time => PageOrder::Time,
            Column::Resolution => PageOrder::Resolution,
            Column::Format => PageOrder::Format,
            Column::Size => PageOrder::Size,
            Column::Channel => PageOrder::Channel,
            Column::Bitrate => PageOrder::Bitrate,
            Column::VideoCodec => PageOrder::VideoCodec,
            Column::AudioCodec => PageOrder::AudioCodec,
            Column::Duration => PageOrder::Duration,
            Column::Finished => PageOrder::Finished,
        };
        let mut body = div().v_flex().gap_3().child(self.history_actions(cx));
        if self.history_columns_open {
            body = body.child(self.column_picker(cx));
        }
        let page = match self.history.page(
            HistorySort {
                order,
                desc: self.history_sort.descending,
            },
            self.history_page,
            self.history_page_size,
        ) {
            Ok(p) => p,
            Err(e) => return body.child(muted(cx, e)),
        };
        body = body.child(self.history_pagination(page.total, cx));
        let width = 100.
            + self
                .history_columns
                .iter()
                .map(|f| f.width() + 8.)
                .sum::<f32>();
        let ids = page
            .entries
            .iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>();
        let all = !ids.is_empty() && ids.iter().all(|id| self.history_selected.contains(id));
        let mut header = div()
            .h_flex()
            .items_center()
            .gap_2()
            .px_2()
            .h(px(36.))
            .bg(cx.theme().muted)
            .child(
                Button::new("select-page")
                    .ghost()
                    .xsmall()
                    .w(px(36.))
                    .label(if all { "☑" } else { "☐" })
                    .tooltip(tr("选择当前页"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.history_delete = None;
                        for id in &ids {
                            if all {
                                this.history_selected.remove(id);
                            } else {
                                this.history_selected.insert(id.clone());
                            }
                        }
                        cx.notify();
                    })),
            )
            .child(muted(cx, tr("序号")).w(px(48.)).flex_shrink_0());
        for (n, field) in self.history_columns.iter().copied().enumerate() {
            let sort = field_sort(field);
            let arrow = if sort == Some(self.history_sort.column) {
                if self.history_sort.descending {
                    " ↓"
                } else {
                    " ↑"
                }
            } else {
                ""
            };
            header = header.child(
                div()
                    .id(("header", n))
                    .w(px(field.width()))
                    .flex_shrink_0()
                    .text_xs()
                    .font_semibold()
                    .cursor_pointer()
                    .child(format!("{}{arrow}", tr(field.label())))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(sort) = sort {
                            this.history_sort.toggle(sort);
                            this.history_page = 0;
                            cx.notify();
                        }
                    })),
            );
        }
        let mut table = div().v_flex().w(px(width)).child(header);
        for (index, e) in page.entries.iter().enumerate() {
            table = table.child(self.history_row(index, e, cx));
        }
        body.child(
            div()
                .relative()
                .w_full()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .overflow_hidden()
                .child(
                    div()
                        .id("history-scroll")
                        .overflow_x_scroll()
                        .restrict_scroll_to_axis()
                        .track_scroll(&self.history_scroll)
                        .pb(px(20.))
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

    fn history_row(&self, index: usize, entry: &HistoryEntry, cx: &Context<Self>) -> Div {
        let id = entry.id.clone();
        let selected = self.history_selected.contains(&id);
        let mut row = div()
            .h_flex()
            .items_center()
            .gap_2()
            .px_2()
            .h(px(36.))
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(if selected {
                cx.theme().muted
            } else {
                cx.theme().background
            })
            .hover(|s| s.bg(cx.theme().muted))
            .child(
                Button::new(("select-record", index))
                    .ghost()
                    .xsmall()
                    .w(px(36.))
                    .label(if selected { "☑" } else { "☐" })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.history_selected.remove(&id) {
                            this.history_selected.insert(id.clone());
                        }
                        this.history_delete = None;
                        cx.notify();
                    })),
            )
            .child(
                muted(
                    cx,
                    (self.history_page * self.history_page_size + index + 1).to_string(),
                )
                .w(px(48.))
                .flex_shrink_0(),
            );
        for (n, field) in self.history_columns.iter().copied().enumerate() {
            let text = field_text(field, entry);
            row = row.child(
                div()
                    .id(("history-cell", index * 32 + n))
                    .w(px(field.width()))
                    .flex_shrink_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_xs()
                    .tooltip({
                        let text = text.clone();
                        move |window, cx| Tooltip::new(text.clone()).build(window, cx)
                    })
                    .child(text),
            );
        }
        row
    }
}
fn field_sort(f: Field) -> Option<Column> {
    match f {
        Field::Title => Some(Column::Title),
        Field::Status => Some(Column::Status),
        Field::Site => Some(Column::Site),
        Field::Time => Some(Column::Time),
        Field::Resolution => Some(Column::Resolution),
        Field::Format => Some(Column::Format),
        Field::Size => Some(Column::Size),
        Field::Channel => Some(Column::Channel),
        Field::Bitrate => Some(Column::Bitrate),
        Field::VideoCodec => Some(Column::VideoCodec),
        Field::AudioCodec => Some(Column::AudioCodec),
        Field::Duration => Some(Column::Duration),
        Field::Finished => Some(Column::Finished),
        _ => None,
    }
}
fn field_text(f: Field, e: &HistoryEntry) -> String {
    let m = &e.metadata;
    match f {
        Field::Title => e.title.clone(),
        Field::Status => tr(status(e.status)),
        Field::Site => m.source_site.clone(),
        Field::Time => format_time(e.started_at),
        Field::Resolution => m.height.map(|h| format!("{h}p")).unwrap_or("—".into()),
        Field::Format => m.ext.clone().unwrap_or("—".into()),
        Field::Size => format!("{:.1} MB", size(e) as f64 / 1e6),
        Field::Channel => m.channel.clone().unwrap_or("—".into()),
        Field::Bitrate => m
            .bitrate
            .map(|b| format!("{:.0} kbps", b as f64 / 1000.))
            .unwrap_or("—".into()),
        Field::VideoCodec => m.vcodec.clone().unwrap_or("—".into()),
        Field::AudioCodec => m.acodec.clone().unwrap_or("—".into()),
        Field::Duration => m
            .duration
            .map(|d| format!("{d:.1} s"))
            .unwrap_or("—".into()),
        Field::Finished => e.finished_at.map(format_time).unwrap_or("—".into()),
        Field::Url => {
            if m.original_url.is_empty() {
                e.url.clone()
            } else {
                m.original_url.clone()
            }
        }
        Field::Path => e
            .artifacts
            .iter()
            .map(|a| a.path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
fn format_time(seconds: u64) -> String {
    chrono::DateTime::from_timestamp(i64::try_from(seconds).unwrap_or(i64::MAX), 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M %:z")
                .to_string()
        })
        .unwrap_or("—".into())
}
