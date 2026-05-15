use tauri::{menu::MenuBuilder, tray::TrayIconBuilder, Emitter, Manager};

const TRAY_OPEN_TODAY: &str = "open_today";
const TRAY_SHOW_FLOATING_TASK: &str = "show_floating_task";
const TRAY_STOP_CURRENT_TASK: &str = "stop_current_task";
const TRAY_SETTINGS: &str = "settings";
const TRAY_QUIT: &str = "quit";

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::createTask,
            commands::updateTask,
            commands::archiveTask,
            commands::listToday,
            commands::listBacklog,
            commands::listRecentThreads,
            commands::startSession,
            commands::getActiveSession,
            commands::getPendingSessionRecovery,
            commands::completeSession,
            commands::stopSession,
            commands::switchTask,
            commands::resolveSessionRecovery,
            commands::getSettings,
            commands::updateSettings,
            commands::saveFloatingWindowPosition,
            commands::exportDatabase,
            commands::openDataFolder,
            commands::openTodayWindow,
            commands::openSettingsWindow,
            commands::showFloatingTask,
            commands::stopCurrentTask,
            commands::resetFloatingWindowPosition,
            commands::quitApp
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let conn = persistence::open_app_database(&app_handle)?;
            let pending_recovery = commands::pending_recovery_from_startup(&conn)
                .map_err(|error| std::io::Error::other(error.message))?;
            let show_today = commands::settings_show_today_on_startup(&conn)
                .map_err(|error| std::io::Error::other(error.message))?;
            let has_recovery = commands::pending_recovery_session_id(&pending_recovery)
                .map_err(|error| std::io::Error::other(error.message))?
                .is_some();
            app.manage(pending_recovery);
            let tray_installed = install_tray(app).is_ok();
            if !show_today && !has_recovery && tray_installed {
                if let Some(today) = app.get_webview_window("today") {
                    today.hide()?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Thread");
}
pub mod commands;
pub mod persistence;

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text(TRAY_OPEN_TODAY, "Open Today")
        .text(TRAY_SHOW_FLOATING_TASK, "Show Floating Task")
        .text(TRAY_STOP_CURRENT_TASK, "Stop Current Task")
        .separator()
        .text(TRAY_SETTINGS, "Settings")
        .text(TRAY_QUIT, "Quit")
        .build()?;

    let icon = app.default_window_icon().cloned();
    let mut tray = TrayIconBuilder::with_id("thread")
        .menu(&menu)
        .tooltip("Thread")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let result = match event.id().as_ref() {
                TRAY_OPEN_TODAY => commands::open_today_window(app),
                TRAY_SHOW_FLOATING_TASK => commands::show_floating_task(app),
                TRAY_STOP_CURRENT_TASK => commands::stop_current_task(app).map(|_| ()),
                TRAY_SETTINGS => commands::open_settings_window(app),
                TRAY_QUIT => commands::quit_app(app),
                _ => Ok(()),
            };

            if let Err(error) = result {
                let _ = commands::open_today_window(app);
                let _ = app.emit("command-error", error);
            }
        });

    if let Some(icon) = icon {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}
