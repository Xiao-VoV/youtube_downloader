//! Bulk actions snapshot stable IDs. A delete never removes media files.
use super::muted;
use crate::{app::Downloader, history::HistoryStatus, history_columns::Field, i18n::tr};
use gpui_kit::{
    component::{button::*, *},
    *,
};
impl Downloader {
    pub(super) fn history_actions(&self, cx: &Context<Self>) -> Div {
        let count = self.history_selected.len();
        let running = self.history.entries.iter().any(|e| {
            self.history_selected.contains(&e.id)
                && (e.status == HistoryStatus::Running || self.running.contains_key(&e.id))
        });
        div()
            .h_flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(muted(cx, format!("{} {count}", tr("已选择"))))
            .child(
                Button::new("bulk-url")
                    .ghost()
                    .small()
                    .icon(IconName::ExternalLink)
                    .label(tr("打开原链接 ↗"))
                    .disabled(count == 0)
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
                Button::new("bulk-finder")
                    .ghost()
                    .small()
                    .icon(IconName::FolderOpen)
                    .label(tr("Finder 中显示"))
                    .disabled(count == 0)
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
                Button::new("bulk-delete")
                    .ghost()
                    .small()
                    .icon(IconName::Delete)
                    .label(tr("删除记录"))
                    .disabled(count == 0 || running)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.confirm_history_delete(window, cx)),
                    ),
            )
            .child(
                Button::new("bulk-clear")
                    .ghost()
                    .small()
                    .label(tr("取消选择"))
                    .disabled(count == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.history_selected.clear();
                        this.history_delete = None;
                        cx.notify();
                    })),
            )
            .child(div().flex_1())
            .child(
                Button::new("columns-toggle")
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
        let mut row = div()
            .h_flex()
            .flex_wrap()
            .gap_2()
            .p_3()
            .rounded_lg()
            .bg(cx.theme().muted);
        for (index, field) in Field::ALL.into_iter().enumerate() {
            let visible = self.history_columns.contains(&field);
            row = row.child(
                Button::new(("column-field", index))
                    .ghost()
                    .small()
                    .label(format!(
                        "{}{}",
                        if visible { "✓ " } else { "" },
                        tr(field.label())
                    ))
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
                            Err(e) => this.status = e,
                        };
                        cx.notify();
                    })),
            );
        }
        row
    }
}
