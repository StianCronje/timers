use crate::db::{Db, DbResult};
use crate::models::{ReportEntry, Session, Task, TaskDetail, TaskWithStats};
use crate::tray;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct AppState {
    pub db: Db,
}

#[derive(Serialize, Clone)]
pub struct ActiveTimerInfo {
    pub task: Task,
    pub session: Session,
}

fn emit_active_changed(app: &AppHandle) {
    let state: State<'_, AppState> = app.state();
    let active = state.db.get_active_session().ok().flatten().map(|(t, s)| ActiveTimerInfo { task: t, session: s });
    let _ = app.emit("timer:active-changed", active.clone());
    tray::refresh_tray(app, active.as_ref());
}

#[tauri::command]
pub fn create_task(
    state: State<'_, AppState>,
    app: AppHandle,
    name: String,
    description: Option<String>,
) -> DbResult<Task> {
    let task = state
        .db
        .create_task(name.trim(), description.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()))?;
    let _ = app.emit("tasks:changed", ());
    Ok(task)
}

#[tauri::command]
pub fn update_task(
    state: State<'_, AppState>,
    app: AppHandle,
    id: i64,
    name: String,
    description: Option<String>,
) -> DbResult<Task> {
    let task = state.db.update_task(
        id,
        name.trim(),
        description.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()),
    )?;
    let _ = app.emit("tasks:changed", ());
    emit_active_changed(&app);
    Ok(task)
}

#[tauri::command]
pub fn set_task_archived(
    state: State<'_, AppState>,
    app: AppHandle,
    id: i64,
    archived: bool,
) -> DbResult<()> {
    state.db.set_archived(id, archived)?;
    let _ = app.emit("tasks:changed", ());
    emit_active_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_task(
    state: State<'_, AppState>,
    app: AppHandle,
    id: i64,
) -> DbResult<()> {
    state.db.delete_task(id)?;
    let _ = app.emit("tasks:changed", ());
    emit_active_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_tasks(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> DbResult<Vec<TaskWithStats>> {
    state.db.list_tasks(include_archived.unwrap_or(false))
}

#[tauri::command]
pub fn get_task_detail(state: State<'_, AppState>, id: i64) -> DbResult<TaskDetail> {
    state.db.get_task_detail(id)
}

#[tauri::command]
pub fn start_timer(
    state: State<'_, AppState>,
    app: AppHandle,
    task_id: i64,
) -> DbResult<Session> {
    let s = state.db.start_timer(task_id)?;
    emit_active_changed(&app);
    let _ = app.emit("tasks:changed", ());
    Ok(s)
}

#[tauri::command]
pub fn stop_timer(
    state: State<'_, AppState>,
    app: AppHandle,
) -> DbResult<Option<Session>> {
    let s = state.db.stop_active_timer()?;
    emit_active_changed(&app);
    let _ = app.emit("tasks:changed", ());
    Ok(s)
}

#[tauri::command]
pub fn get_active_timer(state: State<'_, AppState>) -> DbResult<Option<ActiveTimerInfo>> {
    Ok(state
        .db
        .get_active_session()?
        .map(|(task, session)| ActiveTimerInfo { task, session }))
}

#[tauri::command]
pub fn report(
    state: State<'_, AppState>,
    range_start: i64,
    range_end: i64,
) -> DbResult<Vec<ReportEntry>> {
    state.db.report(range_start, range_end)
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
    Ok(())
}
