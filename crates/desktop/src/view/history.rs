//! Selectable history table; actions operate on stable record IDs, never row indexes.
use super::controls::{self, Tone};
use super::{design, muted};
use crate::{
    app::Downloader,
    history::{HistoryEntry, HistorySort, PageOrder},
    history_columns::Field,
    history_table::{Column, size, status},
    i18n::tr,
};
use gpui_kit::{
    base::{Scrollbar, ScrollbarMode},
    component::{checkbox::Checkbox, tooltip::Tooltip, *},
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
        let mut body = div()
            .v_flex()
            .gap_3()
            .w_full()
            .child(self.history_actions(cx));
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
        let width = 116.
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
            .px_3()
            .h(px(38.))
            .border_b_1()
            .border_color(design::border(cx))
            .bg(design::canvas(cx))
            .text_color(design::muted(cx))
            .child(
                div()
                    .w(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Checkbox::new("select-page")
                            .checked(all)
                            .label("")
                            .on_click(cx.listener(move |this, checked, _, cx| {
                                this.history_delete = None;
                                for id in &ids {
                                    if *checked {
                                        this.history_selected.insert(id.clone());
                                    } else {
                                        this.history_selected.remove(id);
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(muted(cx, tr("序号")).w(px(48.)).flex_shrink_0());
        for field in self.history_columns.iter().copied() {
            let sort = field_sort(field);
            let active = sort == Some(self.history_sort.column);
            let mut cell = controls::button(
                SharedString::from(format!("history-header-{field:?}")),
                tr(field.label()),
                Tone::Quiet,
                cx,
            )
            .w(px(field.width()))
            .h(px(30.))
            .px_0()
            .text_color(if active {
                design::accent()
            } else {
                design::muted(cx)
            })
            .disabled(sort.is_none());
            if active {
                cell = cell.icon(design::glyph(if self.history_sort.descending {
                    "chevron-down"
                } else {
                    "chevron-up"
                }));
            }
            header = header.child(cell.on_click(cx.listener(move |this, _, _, cx| {
                if let Some(sort) = sort {
                    this.history_sort.toggle(sort);
                    this.history_page = 0;
                    cx.notify();
                }
            })));
        }
        let mut table = div().v_flex().w(px(width)).child(header);
        for (index, e) in page.entries.iter().enumerate() {
            table = table.child(self.history_row(index, e, cx));
        }
        body.child(
            design::group(cx)
                .v_flex()
                .relative()
                .w_full()
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
                .child(Scrollbar::horizontal(&self.history_scroll).mode(ScrollbarMode::Always))
                .children(page.entries.is_empty().then(|| {
                    div()
                        .h_flex()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .py_5()
                        .text_color(design::muted(cx))
                        .child(IconName::Inbox)
                        .child(muted(cx, tr("暂无下载历史")))
                })),
        )
        .child(self.history_pagination(page.total, cx))
    }
    fn history_pagination(&self, total: usize, cx: &Context<Self>) -> Div {
        let pages = total.div_ceil(self.history_page_size).max(1);
        let mut row = div()
            .h_flex()
            .items_center()
            .flex_wrap()
            .gap_1()
            .py_1()
            .child(muted(cx, format!("{} {total} {}", tr("共"), tr("条"))));
        let mut sizes = controls::segments(cx);
        for size in [25, 50, 100] {
            sizes = sizes.child(
                controls::segment(
                    ("history-size", size),
                    format!("{size} / {}", tr("页")),
                    self.history_page_size == size,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.history_page_size = size;
                    this.history_page = 0;
                    cx.notify();
                })),
            );
        }
        row = row.child(sizes);
        row.child(div().flex_1())
            .child(
                controls::icon_button("history-prev", IconName::ChevronLeft, tr("上一页"), cx)
                    .disabled(self.history_page == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.history_page = this.history_page.saturating_sub(1);
                        cx.notify();
                    })),
            )
            .child(muted(cx, format!("{} / {pages}", self.history_page + 1)))
            .child(
                controls::icon_button("history-next", IconName::ChevronRight, tr("下一页"), cx)
                    .disabled(self.history_page + 1 >= pages)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.history_page += 1;
                        cx.notify();
                    })),
            )
    }

    fn history_row(&self, index: usize, entry: &HistoryEntry, cx: &Context<Self>) -> Div {
        let id = entry.id.clone();
        let selected = self.history_selected.contains(&id);
        let mut row = div()
            .h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .h(px(52.))
            .border_b_1()
            .border_color(design::border(cx))
            .bg(if selected {
                design::selection(cx)
            } else {
                design::surface(cx)
            })
            .hover(|s| {
                s.bg(if selected {
                    design::selection(cx)
                } else {
                    design::canvas(cx)
                })
            })
            .child(
                div()
                    .w(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Checkbox::new(SharedString::from(format!("select-record-{}", entry.id)))
                            .label("")
                            .checked(selected)
                            .on_click(cx.listener(move |this, checked, _, cx| {
                                if *checked {
                                    this.history_selected.insert(id.clone());
                                } else {
                                    this.history_selected.remove(&id);
                                }
                                this.history_delete = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                muted(
                    cx,
                    (self.history_page * self.history_page_size + index + 1).to_string(),
                )
                .w(px(48.))
                .flex_shrink_0(),
            );
        for field in self.history_columns.iter().copied() {
            let text = field_text(field, entry);
            let mut cell = div()
                .id(SharedString::from(format!(
                    "history-cell-{}-{field:?}",
                    entry.id
                )))
                .w(px(field.width()))
                .flex_shrink_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_xs()
                .text_color(if field == Field::Title {
                    design::text(cx)
                } else {
                    design::muted(cx)
                })
                .tooltip({
                    let text = text.clone();
                    move |window, cx| Tooltip::new(text.clone()).build(window, cx)
                });
            if field == Field::Title {
                cell = cell.child(div().font_semibold().child(text));
            } else if field == Field::Status {
                let (icon, color) = match entry.status {
                    crate::history::HistoryStatus::Succeeded => {
                        (IconName::CircleCheck, status_color(cx, 0x287b58, 0x79cfa6))
                    }
                    crate::history::HistoryStatus::Failed => (IconName::Info, cx.theme().danger),
                    crate::history::HistoryStatus::Running => {
                        (IconName::ArrowDown, design::accent())
                    }
                    crate::history::HistoryStatus::Cancelled => {
                        (IconName::Close, design::muted(cx))
                    }
                    crate::history::HistoryStatus::Interrupted => {
                        (IconName::RefreshCw, status_color(cx, 0x947036, 0xd8b57a))
                    }
                };
                cell = cell.child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_1()
                        .text_color(color)
                        .child(Icon::new(icon).size_3())
                        .child(text),
                );
            } else {
                cell = cell.child(text);
            }
            row = row.child(cell);
        }
        row
    }
}
fn status_color(cx: &App, light: u32, dark: u32) -> Hsla {
    rgb(if cx.theme().background.l < 0.5 {
        dark
    } else {
        light
    })
    .into()
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
        Field::Size => {
            if e.artifacts.is_empty() {
                "—".into()
            } else {
                format!("{:.1} MB", size(e) as f64 / 1e6)
            }
        }
        Field::Channel => m.channel.clone().unwrap_or("—".into()),
        Field::Bitrate => m
            .bitrate
            .map(|b| format!("{:.0} kbps", b as f64 / 1000.))
            .unwrap_or("—".into()),
        Field::VideoCodec => m.vcodec.clone().unwrap_or("—".into()),
        Field::AudioCodec => m.acodec.clone().unwrap_or("—".into()),
        Field::Duration => m.duration.map(format_duration).unwrap_or("—".into()),
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
fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "—".into();
    }
    let seconds = seconds.floor() as u64;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
#[cfg(test)]
mod duration_tests {
    #[test]
    fn missing_artifacts_do_not_show_zero_megabytes() {
        let entry = crate::history::HistoryEntry {
            id: "unknown-size".into(),
            title: "Example".into(),
            url: "https://youtu.be/abcdefghijk".into(),
            started_at: 0,
            finished_at: None,
            status: crate::history::HistoryStatus::Failed,
            artifacts: vec![],
            metadata: Default::default(),
        };
        assert_eq!(
            super::field_text(crate::history_columns::Field::Size, &entry),
            "—"
        );
    }
    #[test]
    fn duration_uses_hours_minutes_seconds() {
        for (seconds, expected) in [
            (0.0, "00:00:00"),
            (65.9, "00:01:05"),
            (3661.0, "01:01:01"),
            (360000.0, "100:00:00"),
        ] {
            assert_eq!(super::format_duration(seconds), expected);
        }
        assert_eq!(super::format_duration(f64::NAN), "—");
        assert_eq!(super::format_duration(-1.0), "—");
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
