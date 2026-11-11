//! Stable typed ordering independent of GPUI and localized display strings.
use crate::history::{HistoryEntry, HistoryStatus};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Column {
    Title,
    Status,
    Site,
    Time,
    Resolution,
    Format,
    Size,
}
#[derive(Clone, Copy)]
pub(crate) struct Sort {
    pub column: Column,
    pub descending: bool,
}
impl Default for Sort {
    fn default() -> Self {
        Self {
            column: Column::Time,
            descending: true,
        }
    }
}
impl Sort {
    pub fn toggle(&mut self, column: Column) {
        if self.column == column {
            self.descending = !self.descending;
        } else {
            self.column = column;
            self.descending = false;
        }
    }
    pub fn rows<'a>(&self, entries: &'a [HistoryEntry]) -> Vec<&'a HistoryEntry> {
        let mut rows: Vec<_> = entries.iter().collect();
        rows.sort_by(|a, b| {
            let order = match self.column {
                Column::Title => a.title.cmp(&b.title),
                Column::Status => status(a.status).cmp(status(b.status)),
                Column::Site => a.metadata.source_site.cmp(&b.metadata.source_site),
                Column::Time => a.started_at.cmp(&b.started_at),
                Column::Resolution => a.metadata.height.cmp(&b.metadata.height),
                Column::Format => a.metadata.ext.cmp(&b.metadata.ext),
                Column::Size => size(a).cmp(&size(b)),
            };
            let order = if self.descending {
                order.reverse()
            } else {
                order
            };
            order.then_with(|| a.id.cmp(&b.id))
        });
        rows
    }
}
pub(crate) fn size(entry: &HistoryEntry) -> u64 {
    entry
        .artifacts
        .iter()
        .fold(0u64, |sum, a| sum.saturating_add(a.size))
}
pub(crate) fn status(value: HistoryStatus) -> &'static str {
    match value {
        HistoryStatus::Running => "进行中",
        HistoryStatus::Succeeded => "已完成",
        HistoryStatus::Failed => "失败",
        HistoryStatus::Cancelled => "已取消",
        HistoryStatus::Interrupted => "已中断",
    }
}
#[cfg(test)]
mod tests {
    use super::{Column, Sort};
    use crate::history::{HistoryEntry, HistoryStatus};
    #[test]
    fn headers_toggle_and_numeric_order_is_not_lexical() {
        let mut rows = Vec::new();
        for (id, time, height) in [("b", 9, 720), ("a", 100, 1080)] {
            rows.push(HistoryEntry {
                id: id.into(),
                title: id.into(),
                url: "https://youtu.be/abcdefghijk".into(),
                started_at: time,
                finished_at: None,
                status: HistoryStatus::Succeeded,
                artifacts: vec![],
                metadata: crate::history::HistoryMetadata {
                    height: Some(height),
                    ..Default::default()
                },
            });
        }
        let mut sort = Sort::default();
        assert_eq!(sort.rows(&rows)[0].id, "a");
        sort.toggle(Column::Time);
        assert_eq!(sort.rows(&rows)[0].id, "b");
        sort.toggle(Column::Resolution);
        assert_eq!(sort.rows(&rows)[0].id, "b");
        sort.toggle(Column::Resolution);
        assert_eq!(sort.rows(&rows)[0].id, "a");
    }
}
