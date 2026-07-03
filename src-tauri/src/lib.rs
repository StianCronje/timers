mod commands;
mod db;
mod models;
mod tray;

use commands::AppState;
use db::Db;
use tauri::{Manager, RunEvent, WindowEvent};

#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicI64, Ordering};

#[cfg(target_os = "macos")]
static LAST_CMD_Q_PRESS_MS: AtomicI64 = AtomicI64::new(0);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().expect("app data dir");
            std::fs::create_dir_all(&dir).ok();
            let db_path = dir.join("timers.sqlite");
            let db = Db::open(&db_path).expect("open database");
            app.manage(AppState { db });

            tray::setup(&app.handle())?;

            // Intercept the window close button: hide instead of quitting.
            if let Some(win) = app.get_webview_window("main") {
                let win_for_handler = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_for_handler.hide();
                    }
                });
            }

            #[cfg(target_os = "macos")]
            install_macos_app_menu(&app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_task,
            commands::update_task,
            commands::set_task_archived,
            commands::delete_task,
            commands::list_tasks,
            commands::get_task_detail,
            commands::start_timer,
            commands::stop_timer,
            commands::get_active_timer,
            commands::report,
            commands::show_main_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        // Safety net for any OS-driven exit signal we didn't intercept above.
        // Only allow exits initiated by `app.exit(code)` (which sets a code).
        RunEvent::ExitRequested { api, code, .. } if code.is_none() => {
            api.prevent_exit();
        }
        // macOS dock-icon click when no windows are visible: reopen the main window.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen {
            has_visible_windows,
            ..
        } if !has_visible_windows => {
            if let Some(win) = app_handle.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        _ => {
            let _ = app_handle;
        }
    });
}

#[cfg(target_os = "macos")]
fn install_macos_app_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{
        AboutMetadataBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder,
    };

    let about_metadata = AboutMetadataBuilder::new().build();

    // Cmd-Q on the App menu: hide the window instead of quitting.
    let cmd_q_hide = MenuItemBuilder::with_id("app_menu_hide", "Close Timers Window")
        .accelerator("Cmd+Q")
        .build(app)?;

    // Cmd-Shift-Q: actually quit.
    let cmd_shift_q_quit = MenuItemBuilder::with_id("app_menu_quit", "Quit Timers")
        .accelerator("Cmd+Shift+Q")
        .build(app)?;

    let app_submenu = SubmenuBuilder::new(app, "Timers")
        .about(Some(about_metadata))
        .separator()
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .separator()
        .item(&cmd_q_hide)
        .item(&cmd_shift_q_quit)
        .build()?;

    let edit_submenu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window_submenu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .item(&PredefinedMenuItem::close_window(app, None)?)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&app_submenu)
        .item(&edit_submenu)
        .item(&window_submenu)
        .build()?;

    app.set_menu(menu)?;

    app.on_menu_event(|app, event| match event.id().as_ref() {
        "app_menu_hide" => {
            // Cmd-Q hold-to-quit: macOS fires the menu shortcut on key-repeat
            // (~500ms initial delay). First press hides; any second press
            // within 2s — i.e., the user is holding — actually quits.
            let now_ms = chrono::Utc::now().timestamp_millis();
            let last = LAST_CMD_Q_PRESS_MS.load(Ordering::SeqCst);
            if last > 0 && now_ms - last < 2000 {
                LAST_CMD_Q_PRESS_MS.store(0, Ordering::SeqCst);
                app.exit(0);
                return;
            }
            LAST_CMD_Q_PRESS_MS.store(now_ms, Ordering::SeqCst);
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.hide();
            }
        }
        "app_menu_quit" => {
            app.exit(0);
        }
        _ => {}
    });

    Ok(())
}
