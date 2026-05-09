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
            commands::completeSession,
            commands::stopSession,
            commands::switchTask,
            commands::resolveSessionRecovery,
            commands::getSettings,
            commands::updateSettings,
            commands::saveFloatingWindowPosition,
            commands::exportDatabase,
            commands::openDataFolder
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            persistence::open_app_database(&app_handle)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Thread");
}
pub mod commands;
pub mod persistence;
