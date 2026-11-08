#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod definition;
mod status;
mod workspace;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(workspace::Workspace::from_env())
        .invoke_handler(tauri::generate_handler![
            commands::current_project,
            commands::open_project,
            commands::runs,
            commands::run_detail,
        ])
        .run(tauri::generate_context!())
        .expect("the Surt desktop window could not start");
}
