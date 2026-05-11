use tauri::Manager;

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
            commands::openTodayWindow
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let conn = persistence::open_app_database(&app_handle)?;
            let pending_recovery = commands::pending_recovery_from_startup(&conn)
                .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(pending_recovery);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Thread");
}
pub mod commands;
pub mod persistence;
