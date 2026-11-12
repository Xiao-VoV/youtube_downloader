//! Confirmation snapshots record IDs. Optional media deletion uses the macOS Trash.
use crate::{
    app::Downloader,
    history::{HistoryEntry, HistoryStatus},
    i18n::tr,
    messages::Update,
};
use gpui_kit::{
    component::{WindowExt, button::*, checkbox::Checkbox, *},
    *,
};
use std::{cell::Cell, rc::Rc};
impl Downloader {
    pub(crate) fn confirm_history_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let records = self
            .history
            .entries
            .iter()
            .filter(|e| self.history_selected.contains(&e.id))
            .cloned()
            .collect::<Vec<_>>();
        if records.is_empty()
            || records
                .iter()
                .any(|e| e.status == HistoryStatus::Running || self.running.contains_key(&e.id))
        {
            return;
        }
        let trash = Rc::new(Cell::new(false));
        let weak = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let toggle = trash.clone();
            let checked = trash.get();
            let weak = weak.clone();
            let records = records.clone();
            let choice = trash.clone();
            dialog
                .title(tr("删除历史记录"))
                .child(
                    div()
                        .v_flex()
                        .gap_4()
                        .child(format!("{} {}", tr("选中记录数量："), records.len()))
                        .child(
                            Checkbox::new("trash-media")
                                .label(tr("同时将媒体文件移到废纸篓"))
                                .checked(checked)
                                .on_click(move |value, _, cx| {
                                    toggle.set(*value);
                                    cx.refresh_windows();
                                }),
                        ),
                )
                .footer(
                    div()
                        .h_flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            Button::new("cancel-history-delete")
                                .label(tr("取消"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("confirm-history-delete")
                                .danger()
                                .label(tr("确认删除记录"))
                                .on_click(move |_, window, cx| {
                                    let records = records.clone();
                                    let delete_files = choice.get();
                                    let _ = weak.update(cx, |this, cx| {
                                        this.begin_history_delete(records, delete_files, cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }
    fn begin_history_delete(
        &mut self,
        records: Vec<HistoryEntry>,
        delete_files: bool,
        cx: &mut Context<Self>,
    ) {
        if records.iter().any(|r| {
            self.running.contains_key(&r.id)
                || self
                    .history
                    .entries
                    .iter()
                    .any(|e| e.id == r.id && e.status == HistoryStatus::Running)
        }) {
            self.status = tr("运行中的任务不能删除");
            cx.notify();
            return;
        }
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let mut results = Vec::new();
            for record in records {
                let result = if delete_files {
                    trash_artifacts(&record)
                } else {
                    Ok(())
                };
                results.push((record.id, result));
            }
            let _ = tx.send_blocking(Update::HistoryDelete(results));
        });
        Self::listen(rx, cx);
    }
}
fn trash_artifacts(record: &HistoryEntry) -> Result<(), String> {
    for artifact in &record.artifacts {
        let path = &artifact.path;
        if !path.is_absolute() {
            return Err("Relative media path rejected".into());
        }
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("Cannot inspect media file".into()),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("Only regular media files may be trashed".into());
        }
        #[cfg(target_os = "macos")]
        {
            // Script is fixed; untrusted paths are argv data, never interpolated code.
            let script = "on run argv\n tell application \"Finder\"\n delete (POSIX file (item 1 of argv))\n end tell\nend run";
            let mut child = std::process::Command::new("/usr/bin/osascript")
                .args(["-e", script, "--"])
                .arg(path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|_| "Cannot request Trash")?;
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if !status.success() {
                            return Err("Trash request failed; history retained".into());
                        }
                        break;
                    }
                    Ok(None) => {}
                    Err(_) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("Cannot observe Trash request".into());
                    }
                }
                if start.elapsed() > std::time::Duration::from_secs(20) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Trash request timed out; verify file state".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        #[cfg(not(target_os = "macos"))]
        return Err("Trash is currently supported on macOS only".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::trash_artifacts;
    use crate::history::{HistoryEntry, HistoryStatus};
    #[test]
    fn missing_files_are_not_errors_and_directories_are_rejected() {
        let mut r = HistoryEntry {
            id: "test".into(),
            title: "x".into(),
            url: "https://youtu.be/dQw4w9WgXcQ".into(),
            started_at: 0,
            finished_at: None,
            status: HistoryStatus::Succeeded,
            artifacts: vec![],
            metadata: Default::default(),
        };
        assert!(trash_artifacts(&r).is_ok());
        r.artifacts.push(youtube_engine::Artifact {
            path: std::env::temp_dir(),
            size: 0,
        });
        assert!(trash_artifacts(&r).is_err());
    }
}
