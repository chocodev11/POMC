#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod downloader;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_all_accounts,
            commands::add_account,
            commands::remove_account,
            commands::ensure_assets,
            commands::get_versions,
            commands::refresh_account,
            commands::get_skin_url,
            commands::get_patch_notes,
            commands::get_patch_content,
            commands::launch_game,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run POMC launcher");
}
