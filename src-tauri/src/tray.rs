use crate::commands::{ActiveTimerInfo, AppState};
use chrono::Utc;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

const STOP_ID: &str = "stop_timer";
const SHOW_ID: &str = "show_window";
const QUIT_ID: &str = "quit_app";
const STATUS_ID: &str = "status";

pub struct TrayHandle {
    #[allow(dead_code)]
    pub running: Arc<AtomicBool>,
}

/// Holds clones of menu-item handles so we can update labels in place
/// without rebuilding the menu (which would close it on each tick).
pub struct TrayItems {
    pub status: Mutex<MenuItem<Wry>>,
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let (menu, status_item) = build_menu(app, None)?;
    app.manage(TrayItems {
        status: Mutex::new(status_item),
    });

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("No active timer")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            STOP_ID => {
                let state: tauri::State<'_, AppState> = app.state();
                let _ = state.db.stop_active_timer();
                refresh_after_change(app);
            }
            SHOW_ID => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            }
            QUIT_ID => {
                app.exit(0);
            }
            other if other.starts_with("start_") => {
                if let Some(id_str) = other.strip_prefix("start_") {
                    if let Ok(task_id) = id_str.parse::<i64>() {
                        let state: tauri::State<'_, AppState> = app.state();
                        let _ = state.db.start_timer(task_id);
                        refresh_after_change(app);
                    }
                }
            }
            _ => {}
        })
        .build(app)?;

    let running = Arc::new(AtomicBool::new(true));
    app.manage(TrayHandle {
        running: running.clone(),
    });

    // Initial refresh from DB.
    let app_clone = app.clone();
    let _ = std::thread::Builder::new()
        .name("tray-init".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            refresh_after_change(&app_clone);
        });

    // Tick every second to update the menu-bar title, tooltip, and status menu item.
    let app_clone = app.clone();
    let running_clone = running.clone();
    let _ = std::thread::Builder::new()
        .name("tray-tick".into())
        .spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(1));
                tick(&app_clone);
            }
        });

    Ok(())
}

/// Build the menu and return it together with the status MenuItem handle so
/// callers can update its text in place later.
fn build_menu(
    app: &AppHandle,
    active: Option<&ActiveTimerInfo>,
) -> tauri::Result<(Menu<Wry>, MenuItem<Wry>)> {
    let mut builder = MenuBuilder::new(app);

    let show = MenuItemBuilder::with_id(SHOW_ID, "Show window").build(app)?;
    builder = builder.item(&show);
    builder = builder.item(&PredefinedMenuItem::separator(app)?);

    let status_item: MenuItem<Wry>;
    if let Some(info) = active {
        let label = status_label(info);
        status_item = MenuItemBuilder::with_id(STATUS_ID, label).enabled(false).build(app)?;
        builder = builder.item(&status_item);
        let stop = MenuItemBuilder::with_id(STOP_ID, "Pause timer").build(app)?;
        builder = builder.item(&stop);
    } else {
        status_item = MenuItemBuilder::with_id(STATUS_ID, "No active timer").enabled(false).build(app)?;
        builder = builder.item(&status_item);
    }

    builder = builder.item(&PredefinedMenuItem::separator(app)?);

    // Quick-start items for the most-recent tasks (up to 5, non-archived).
    let state = app.state::<AppState>();
    if let Ok(tasks) = state.db.list_tasks(false) {
        let mut any = false;
        for t in tasks.iter().take(5) {
            if active.map(|a| a.task.id) == Some(t.task.id) {
                continue;
            }
            let id = format!("start_{}", t.task.id);
            let label = format!("Start: {}", truncate(&t.task.name, 40));
            let item: MenuItem<Wry> = MenuItemBuilder::with_id(id, label).build(app)?;
            builder = builder.item(&item);
            any = true;
        }
        if any {
            builder = builder.item(&PredefinedMenuItem::separator(app)?);
        }
    }

    let quit = MenuItemBuilder::with_id(QUIT_ID, "Quit").build(app)?;
    builder = builder.item(&quit);

    Ok((builder.build()?, status_item))
}

fn refresh_after_change(app: &AppHandle) {
    let active = current_active(app);
    refresh_tray(app, active.as_ref());
    use tauri::Emitter;
    let _ = app.emit("timer:active-changed", active);
    let _ = app.emit("tasks:changed", ());
}

fn current_active(app: &AppHandle) -> Option<ActiveTimerInfo> {
    let state: tauri::State<'_, AppState> = app.state();
    state
        .db
        .get_active_session()
        .ok()
        .flatten()
        .map(|(task, session)| ActiveTimerInfo { task, session })
}

/// Called when state changes (start/stop/archive/etc). Rebuilds the menu —
/// will close the menu if it happens to be open, but that only happens on
/// user-driven actions, not on every tick.
pub fn refresh_tray(app: &AppHandle, active: Option<&ActiveTimerInfo>) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Ok((menu, status_item)) = build_menu(app, active) {
            let _ = tray.set_menu(Some(menu));
            if let Some(items) = app.try_state::<TrayItems>() {
                *items.status.lock() = status_item;
            }
        }
        apply_dynamic(&tray, app, active);
    }
}

/// Called every second. Updates only the title, tooltip, and the status
/// menu item's text — no menu rebuild, so an open menu stays open.
fn tick(app: &AppHandle) {
    let active = current_active(app);
    if let Some(tray) = app.tray_by_id("main-tray") {
        apply_dynamic(&tray, app, active.as_ref());
    }
}

fn apply_dynamic(
    tray: &tauri::tray::TrayIcon,
    app: &AppHandle,
    active: Option<&ActiveTimerInfo>,
) {
    let now = Utc::now().timestamp();
    match active {
        Some(info) => {
            let elapsed = format_elapsed(now - info.session.start_time);
            let _ = tray.set_title(Some(&format!(" {}", elapsed)));
            let _ = tray.set_tooltip(Some(&format!(
                "{} — {}",
                truncate(&info.task.name, 40),
                elapsed
            )));
            if let Some(items) = app.try_state::<TrayItems>() {
                let _ = items.status.lock().set_text(status_label(info));
            }
        }
        None => {
            let _ = tray.set_title(None::<&str>);
            let _ = tray.set_tooltip(Some("No active timer"));
            if let Some(items) = app.try_state::<TrayItems>() {
                let _ = items.status.lock().set_text("No active timer");
            }
        }
    }
}

fn status_label(info: &ActiveTimerInfo) -> String {
    format!(
        "Running: {} — {}",
        truncate(&info.task.name, 40),
        format_elapsed(Utc::now().timestamp() - info.session.start_time)
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn format_elapsed(secs: i64) -> String {
    let secs = secs.max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}
