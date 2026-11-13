//! Bulk actions use stable record IDs. File deletion is a separate confirmed choice.
use super::controls::{self, Tone};
use super::design;
use crate::{app::Downloader, history::HistoryStatus, history_columns::Field, i18n::tr};
use gpui_kit::{component::*, *};
impl Downloader {
    pub(super) fn history_actions(&self, cx: &Context<Self>) -> Div {
        let count = self.history_selected.len();
        let running = self.history.entries.iter().any(|e| {
            self.history_selected.contains(&e.id)
                && (e.status == HistoryStatus::Running || self.running.contains_key(&e.id))
        });
        let mut row = div()
            .h_flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .min_h(px(40.));
        if count > 0 {
            row = row
                .child(controls::badge(
                    format!("{} {count}", tr("已选择")),
                    true,
                    cx,
                ))
                .child(
                    controls::button("bulk-url", "", Tone::Quiet, cx)
                        .small()
                        .icon(IconName::ExternalLink)
                        .label(tr("打开原链接 ↗").replace(" ↗", ""))
                        .on_click(cx.listener(|this, _, _, cx| {
                            let urls = this
                                .history
                                .entries
                                .iter()
                                .filter(|e| this.history_selected.contains(&e.id))
                                .map(|e| {
                                    if e.metadata.original_url.is_empty() {
                                        e.url.clone()
                                    } else {
                                        e.metadata.original_url.clone()
                                    }
                                })
                                .collect::<std::collections::BTreeSet<_>>();
                            if urls.len() > 10 {
                                this.status = tr("一次最多打开 10 个链接");
                                cx.notify();
                                return;
                            }
                            for url in urls {
                                if youtube_engine::normalize_url(&url).is_ok() {
                                    cx.open_url(&url);
                                }
                            }
                        })),
                )
                .child(
                    controls::button("bulk-finder", "", Tone::Quiet, cx)
                        .small()
                        .icon(IconName::FolderOpen)
                        .label(tr("Finder 中显示"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            let found = crate::reveal::locations(
                                this.history
                                    .entries
                                    .iter()
                                    .filter(|e| this.history_selected.contains(&e.id)),
                            );
                            if found.paths.is_empty() {
                                this.status = tr("没有可用的文件或保存目录");
                                cx.notify();
                                return;
                            }
                            if found.paths.len() > 10 {
                                this.status = tr("一次最多显示 10 个文件");
                                cx.notify();
                                return;
                            }
                            let missing = found.missing;
                            let (tx, rx) = async_channel::bounded(1);
                            std::thread::spawn(move || {
                                let result = crate::reveal::reveal(&found.paths);
                                let text = match result {
                                    Ok(()) => format!(
                                        "{} · {} {missing}",
                                        tr("已请求 Finder 显示"),
                                        tr("不可用记录")
                                    ),
                                    Err(e) => e,
                                };
                                let _ = tx.send_blocking(crate::messages::Update::Notice(text));
                            });
                            Self::listen(rx, cx);
                        })),
                )
                .child(
                    controls::button("bulk-delete", "", Tone::Danger, cx)
                        .small()
                        .icon(IconName::Delete)
                        .label(tr("删除记录"))
                        .disabled(running)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.confirm_history_delete(window, cx)
                        })),
                )
                .child(
                    controls::button("bulk-clear", "", Tone::Quiet, cx)
                        .small()
                        .label(tr("取消选择"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.history_selected.clear();
                            this.history_delete = None;
                            cx.notify();
                        })),
                );
        }
        row.child(div().flex_1()).child(
            controls::button("columns-toggle", "", Tone::Secondary, cx)
                .small()
                .icon(IconName::Settings)
                .label(tr("显示列"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.history_columns_open = !this.history_columns_open;
                    cx.notify();
                })),
        )
    }
    pub(super) fn column_picker(&self, cx: &Context<Self>) -> Div {
        let mut groups = div().v_flex().gap_4();
        for (label, fields) in [
            (
                "基本信息",
                vec![
                    Field::Title,
                    Field::Status,
                    Field::Time,
                    Field::Site,
                    Field::Channel,
                    Field::Finished,
                ],
            ),
            (
                "媒体信息",
                vec![
                    Field::Resolution,
                    Field::Format,
                    Field::Size,
                    Field::Duration,
                    Field::Bitrate,
                    Field::VideoCodec,
                    Field::AudioCodec,
                ],
            ),
            ("文件与来源", vec![Field::Url, Field::Path]),
        ] {
            let mut choices = div().h_flex().items_center().gap_2().flex_wrap();
            for field in fields {
                let visible = self.history_columns.contains(&field);
                choices = choices.child(
                    controls::segment(
                        SharedString::from(format!("column-field-{field:?}")),
                        tr(field.label()),
                        visible,
                        cx,
                    )
                    .icon(if visible {
                        IconName::Check
                    } else {
                        IconName::Plus
                    })
                    .disabled(field.required())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let mut fields = this.history_columns.clone();
                        if fields.contains(&field) {
                            fields.retain(|f| *f != field);
                        } else {
                            fields.push(field);
                        }
                        fields = crate::history_columns::normalize(fields);
                        match crate::history_columns::save(&fields) {
                            Ok(()) => this.history_columns = fields,
                            Err(error) => this.status = error,
                        }
                        cx.notify();
                    })),
                );
            }
            groups = groups.child(
                div()
                    .v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(design::muted(cx))
                            .child(tr(label)),
                    )
                    .child(choices),
            );
        }
        design::group(cx)
            .v_flex()
            .gap_4()
            .p_4()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .child(design::heading(tr("显示列")))
                    .child(
                        controls::icon_button(
                            "close-column-picker",
                            IconName::Close,
                            tr("关闭"),
                            cx,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.history_columns_open = false;
                            cx.notify();
                        })),
                    ),
            )
            .child(groups)
    }
}
