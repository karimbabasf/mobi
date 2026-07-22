mod commands;
mod gate;
mod model;
mod monitor;
mod sensor;
mod state;
mod tray;

use commands::AppCtx;
use gate::Gate;
use state::Store;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Arc::new(Mutex::new(Store::new()));
    let gate = Arc::new(Gate::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .manage(AppCtx {
            store: store.clone(),
            gate: gate.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_roster,
            commands::allow_payment,
            commands::deny_payment,
            commands::sync_tray
        ])
        .setup(move |app| {
            // Menu-bar app: no dock icon, panel dismisses on click-away.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::setup_tray(app)?;
            tray::hide_on_blur(&app.handle().clone());

            // Frosted panel. Non-fatal: a solid dark fallback background is in the CSS.
            #[cfg(target_os = "macos")]
            if let Some(win) = app.get_webview_window("main") {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
                let _ = apply_vibrancy(
                    &win,
                    NSVisualEffectMaterial::HudWindow,
                    Some(NSVisualEffectState::Active),
                    Some(12.0),
                );
            }

            // Agent monitor: rescan running processes every few seconds.
            let monitor_handle = app.handle().clone();
            let monitor_store = store.clone();
            tauri::async_runtime::spawn(async move {
                let mut sys = sysinfo::System::new();
                loop {
                    monitor::refresh_agents(&monitor_store, &mut sys);
                    let _ = monitor_handle.emit("roster-updated", ());
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            });

            // Mock payment sensor: synthetic payments through the real gate path.
            let sensor_handle = app.handle().clone();
            let sensor_store = store.clone();
            let sensor_gate = gate.clone();
            tauri::async_runtime::spawn(async move {
                sensor::run_mock_sensor(sensor_store, sensor_gate, sensor_handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
