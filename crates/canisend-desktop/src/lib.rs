#![forbid(unsafe_code)]

mod commands;

#[cfg(target_os = "macos")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::product_summary,
            commands::run_doctor
        ])
        .run(tauri::generate_context!())
        .expect("failed to run the CanISend Tauri desktop application");
}
