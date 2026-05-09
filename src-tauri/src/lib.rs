pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            persistence::open_app_database(&app_handle)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Thread");
}
pub mod persistence;
