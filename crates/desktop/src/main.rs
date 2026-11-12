//! Native application bootstrap.
mod app;
mod bridge;
mod delete_history;
mod draft_actions;
mod drafts;
mod duplicates;
mod history;
mod history_columns;
mod history_table;
mod i18n;
mod jobs;
mod messages;
mod preferences;
mod preset_actions;
mod preset_option;
mod reveal;
mod settings;
mod tool_settings;
mod view;
use app::Downloader;
use gpui_kit::*;
fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            let bounds = Bounds::centered(None, size(px(1040.), px(780.)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    let view = cx.new(|cx| {
                        let mut app = Downloader::new(window, cx);
                        // A local deep entry point also permits offline layout smoke tests.
                        if std::env::args().any(|arg| arg == "--preferences") {
                            app.page = settings::Page::Profiles;
                        } else if std::env::args().any(|arg| arg == "--history") {
                            app.page = settings::Page::History;
                        } else if std::env::args().any(|arg| arg == "--settings") {
                            app.page = settings::Page::Settings;
                        }
                        app
                    });
                    let weak = view.downgrade();
                    window.on_window_should_close(cx, move |_, cx| {
                        weak.update(cx, |this, cx| {
                            if this.busy || this.probe_job.is_some() || !this.running.is_empty() {
                                this.shutting_down = true;
                                if let Some((_, cancel)) = &this.probe_job {
                                    cancel.cancel();
                                }
                                for task in this.running.values() {
                                    task.cancel.cancel();
                                }
                                if let Some(cancel) = &this.cancel {
                                    cancel.cancel();
                                }
                                this.status = "正在停止任务，请等待结束后再次关闭窗口。".into();
                                cx.notify();
                                false
                            } else {
                                true
                            }
                        })
                        .unwrap_or(true)
                    });
                    view
                },
            )
            .expect("Unable to open downloader window");
        });
}
