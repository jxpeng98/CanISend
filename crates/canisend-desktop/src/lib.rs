#![forbid(unsafe_code)]

mod commands;

#[cfg(target_os = "macos")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::archive_job,
            commands::backup_workspace,
            commands::check_workspace,
            commands::connect_workspace,
            commands::create_job,
            commands::create_workspace,
            commands::import_local_job_source,
            commands::import_url_job_source,
            commands::list_jobs,
            commands::list_workspaces,
            commands::product_summary,
            commands::remove_workspace,
            commands::repair_workspace,
            commands::restore_workspace,
            commands::run_doctor,
            commands::select_workspace,
            commands::show_job,
            commands::workspace_status
        ])
        .run(tauri::generate_context!())
        .expect("failed to run the CanISend Tauri desktop application");
}
