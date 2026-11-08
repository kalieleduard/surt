#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod checkouts;
mod commands;
mod control;
mod definition;
mod drivers;
mod editing;
mod gates;
mod new_run;
mod runtime;
mod scripts;
mod status;
mod workspace;

use tauri::{Emitter, Manager, Window, WindowEvent};

use drivers::Drivers;

const CLOSING_EVENT: &str = "surt://closing";

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(workspace::Workspace::from_env())
        .manage(Drivers::default())
        .manage(scripts::Scripts::default())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && stops_drivers_first(window)
            {
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::current_project,
            commands::open_project,
            commands::runs,
            commands::run_detail,
            gates::approve,
            gates::reject,
            gates::waiting_input,
            gates::answer,
            control::continue_run,
            control::stop,
            control::cancel_impact,
            control::cancel,
            new_run::workflow_definitions,
            new_run::new_run,
            editing::definitions,
            editing::read_definition,
            editing::save_definition,
            editing::delete_definition,
            checkouts::checkouts,
            checkouts::switch_branch,
            checkouts::create_worktree,
            scripts::project_scripts,
            scripts::start_script,
            scripts::stop_script,
        ])
        .run(tauri::generate_context!())
        .expect("the Surt desktop window could not start");
}

fn stops_drivers_first(window: &Window) -> bool {
    let drivers = window.state::<Drivers>().inner().clone();
    if !drivers.is_driving() {
        return false;
    }
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = window.emit(CLOSING_EVENT, ()) {
            eprintln!("cannot announce the closing: {error}");
        }
        drivers.stop_all().await;
        if let Err(error) = window.destroy() {
            eprintln!("cannot close the window: {error}");
        }
    });
    true
}
