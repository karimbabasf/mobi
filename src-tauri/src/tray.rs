use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconEvent, TrayIconBuilder},
    App, AppHandle, Manager, Runtime,
};
use tauri_plugin_positioner::{Position, WindowExt};

pub const TRAY_ID: &str = "mobi-tray";

/// Timestamp of the last blur-driven hide, used to keep a tray click that stole focus
/// from immediately reopening the window it just closed.
fn last_hide() -> &'static Mutex<Option<Instant>> {
    static L: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(None))
}

fn mark_hidden() {
    *last_hide().lock().unwrap() = Some(Instant::now());
}

fn hidden_within(ms: u64) -> bool {
    last_hide()
        .lock()
        .unwrap()
        .map(|t| t.elapsed() < Duration::from_millis(ms))
        .unwrap_or(false)
}

/// Builds the menu-bar tray icon and wires the click to toggle the panel under it.
pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let idle = Image::from_bytes(include_bytes!("../icons/tray-idle.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(idle)
        .icon_as_template(true)
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_panel(app);
            }
        })
        .build(app)?;
    Ok(())
}

fn toggle_panel<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let visible = win.is_visible().unwrap_or(false);
    if visible {
        let _ = win.hide();
        mark_hidden();
    } else if !hidden_within(250) {
        // Not a click that just blurred-and-hid the window: open it under the tray.
        let _ = win.move_window(Position::TrayCenter);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Hides the panel when it loses focus, so clicking away dismisses it like a real popover.
pub fn hide_on_blur<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        let w = win.clone();
        win.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = w.hide();
                mark_hidden();
            }
        });
    }
}

/// Swaps the tray icon between the quiet template mark and the amber-dot alert mark.
pub fn set_tray_alert<R: Runtime>(app: &AppHandle<R>, pending: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if pending {
        if let Ok(img) = Image::from_bytes(include_bytes!("../icons/tray-alert.png")) {
            let _ = tray.set_icon(Some(img));
            let _ = tray.set_icon_as_template(false);
        }
    } else if let Ok(img) = Image::from_bytes(include_bytes!("../icons/tray-idle.png")) {
        let _ = tray.set_icon(Some(img));
        let _ = tray.set_icon_as_template(true);
    }
}
