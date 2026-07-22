use crate::gate::{Decision, Gate};
use crate::model::Roster;
use crate::state::Store;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

/// Shared handles the frontend commands reach through Tauri managed state.
pub struct AppCtx {
    pub store: Arc<Mutex<Store>>,
    pub gate: Arc<Gate>,
}

#[tauri::command]
pub fn get_roster(ctx: State<AppCtx>) -> Roster {
    ctx.store.lock().unwrap().roster()
}

#[tauri::command]
pub fn allow_payment(id: String, ctx: State<AppCtx>, app: AppHandle) -> bool {
    let ok = ctx.gate.resolve(&id, Decision::Allow);
    let _ = app.emit("roster-updated", ());
    ok
}

#[tauri::command]
pub fn deny_payment(id: String, ctx: State<AppCtx>, app: AppHandle) -> bool {
    let ok = ctx.gate.resolve(&id, Decision::Deny);
    let _ = app.emit("roster-updated", ());
    ok
}

/// Frontend calls this after each render so the menu-bar icon reflects pending approvals.
#[tauri::command]
pub fn sync_tray(app: AppHandle, pending: bool) {
    crate::tray::set_tray_alert(&app, pending);
}
