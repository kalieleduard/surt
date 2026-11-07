#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod choice;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![choice::approve_choice])
        .run(tauri::generate_context!())
        .expect("the Surt desktop window could not start");
}
