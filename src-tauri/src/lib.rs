mod commands;
mod gate;
mod model;
mod monitor;
mod sensor;
mod state;

use commands::AppCtx;
use gate::Gate;
use state::Store;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Arc::new(Mutex::new(Store::new()));
    let gate = Arc::new(Gate::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppCtx {
            store: store.clone(),
            gate: gate.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_roster,
            commands::allow_payment,
            commands::deny_payment
        ])
        .setup(move |app| {
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
