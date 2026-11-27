//! Keep the single GPUI window alive when AppKit's close button is pressed.
use crate::{app::Downloader, i18n};
use gpui_kit::*;
use objc2_app_kit::NSView;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::Cell, rc::Rc};

actions!(macos_window, [Quit]);

struct MainWindow(AnyWindowHandle);
impl Global for MainWindow {}

pub(crate) fn install_close_handler(window: &Window, cx: &App) {
    window.on_window_should_close(cx, |window, cx| {
        if let Ok(handle) = HasWindowHandle::window_handle(window)
            && let RawWindowHandle::AppKit(handle) = handle.as_raw()
        {
            // SAFETY: GPUI's AppKit handle refers to a live NSView while the
            // window is borrowed here. `window()` returns a retained NSWindow.
            let view = unsafe { &*handle.ns_view.cast::<NSView>().as_ptr() };
            if let Some(native_window) = view.window() {
                // AppKit can report focus/visibility changes synchronously. Do
                // not reenter the GPUI window update from the close callback.
                cx.spawn(async move |_| native_window.orderOut(None))
                    .detach();
            }
        }
        // Never remove the GPUI window (the root view owns the running jobs).
        false
    });
}

pub(crate) fn reopen(cx: &mut App) {
    if let Some(&MainWindow(handle)) = cx.try_global::<MainWindow>() {
        let _ = handle.update(cx, |_, window, _| window.activate_window());
    }
}

pub(crate) fn install_app_actions(
    handle: AnyWindowHandle,
    view: WeakEntity<Downloader>,
    cx: &mut App,
) {
    cx.set_global(MainWindow(handle));
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    let english = i18n::language() == i18n::Language::En;
    cx.set_menus([Menu::new("YTDer").items([MenuItem::action(
        if english {
            "Quit YTDer"
        } else {
            "退出 YTDer"
        },
        Quit,
    )])]);
    let prompt_open = Rc::new(Cell::new(false));
    cx.on_action(move |_: &Quit, cx| {
        // A keyboard action may originate inside a window update; defer until
        // the main window can be borrowed again (including when it is hidden).
        let view = view.clone();
        let prompt_open = prompt_open.clone();
        cx.defer(move |cx| {
            let active = view
                .update(cx, |app, _| {
                    app.busy
                        || app.probe_job.is_some()
                        || !app.running.is_empty()
                        || !app.queued.is_empty()
                })
                .unwrap_or(false);
            if !active {
                cx.quit();
            } else if !prompt_open.replace(true) {
                let english = i18n::language() == i18n::Language::En;
                let receiver = handle.update(cx, |_, window, cx| {
                    window.activate_window();
                    let (message, detail, button) = if english {
                        (
                            "Downloads or analysis are still in progress",
                            "Wait for the tasks to finish, or cancel them before quitting YTDer.",
                            "OK",
                        )
                    } else {
                        (
                            "仍有下载或解析任务正在进行",
                            "请等待任务完成，或先取消任务，再退出 YTDer。",
                            "好",
                        )
                    };
                    window.prompt(PromptLevel::Warning, message, Some(detail), &[button], cx)
                });
                cx.spawn(async move |_| {
                    if let Ok(receiver) = receiver {
                        let _ = receiver.await;
                    }
                    prompt_open.set(false);
                })
                .detach();
            }
        });
    });
}
