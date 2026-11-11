//! Bounded engine-to-UI bridge. Terminal results are applied only after process cleanup.
use crate::{
    app::Downloader,
    messages::{Update, error_text},
};
use gpui_kit::*;
use youtube_engine::*;
impl Downloader {
    pub(crate) fn listen(rx: async_channel::Receiver<Update>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            while let Ok(update) = rx.recv().await {
                if this
                    .update(cx, |this, cx| {
                        match update {
                            Update::Health(text) => this.health = text,
                            Update::Preview(result) => {
                                this.busy = false;
                                this.cancel = None;
                                match result {
                                    Ok(info) => {
                                        this.status = "解析完成，请确认内容和保存方式。".into();
                                        this.media = Some(info);
                                    }
                                    Err(error) => this.status = error_text(&error),
                                }
                            }
                            Update::Event(event) => match event.kind {
                                EventKind::Starting => this.status = "正在启动下载…".into(),
                                EventKind::Progress(p) => {
                                    let total =
                                        p.total_bytes.or(p.total_bytes_estimate).filter(|n| *n > 0);
                                    this.status = match (p.downloaded_bytes, total) {
                                        (Some(done), Some(total)) => format!(
                                            "正在下载 · {:.1}% · {:.1} MB",
                                            (done as f64 / total as f64 * 100.).min(100.),
                                            done as f64 / 1e6
                                        ),
                                        (Some(done), _) => format!(
                                            "正在下载 · {:.1} MB · 总大小未知",
                                            done as f64 / 1e6
                                        ),
                                        _ => "正在下载 · 正在获取进度…".into(),
                                    };
                                }
                                EventKind::PostProcessing => {
                                    this.status = "正在合并或转换 · 请稍候…".into()
                                }
                                _ => {}
                            },
                            Update::Finished(result) => {
                                this.busy = false;
                                this.cancel = None;
                                match result {
                                    Ok(result) => {
                                        this.artifacts = result.artifacts;
                                        this.status = "下载完成，文件已验证。".into();
                                    }
                                    Err(error) => this.status = error_text(&error),
                                }
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
}
