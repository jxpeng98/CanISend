#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
mod agent;
#[cfg(target_os = "macos")]
mod commands;
#[cfg(target_os = "macos")]
mod delivery;
#[cfg(target_os = "macos")]
mod discovery;
#[cfg(target_os = "macos")]
mod profile;
#[cfg(target_os = "macos")]
mod system;
#[cfg(target_os = "macos")]
mod workflow;

#[cfg(target_os = "macos")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(discovery::DiscoveryPreviewStore::default())
        .manage(workflow::WorkflowPreviewStore::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            agent::agent_capabilities,
            agent::agent_context,
            agent::export_agent_pack,
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
            commands::workspace_status,
            delivery::build_render,
            delivery::check_package,
            delivery::confirm_review,
            delivery::copy_package_projection,
            delivery::current_package,
            delivery::current_package_export,
            delivery::current_render,
            delivery::document_workspace,
            delivery::export_package,
            delivery::export_render,
            delivery::reconcile_package,
            delivery::replace_package_projection,
            delivery::review_workspace,
            discovery::commit_discovery_preview,
            discovery::discard_discovery_preview,
            discovery::discovery_adapters,
            discovery::list_discovery_leads,
            discovery::list_discovery_sources,
            discovery::preview_discovery_file,
            discovery::preview_discovery_network,
            discovery::promote_discovery_lead,
            discovery::show_discovery_lead,
            discovery::suggest_discovery_duplicates,
            profile::confirm_criteria,
            profile::confirm_plan,
            profile::confirm_profile_evidence,
            profile::criteria_template,
            profile::current_matches,
            profile::current_plan,
            profile::import_profile_source,
            profile::list_profile_sources,
            profile::plan_template,
            profile::profile_evidence_template,
            system::check_for_updates,
            system::cli_install_status,
            system::desktop_cli_defaults,
            system::export_resource_catalog,
            system::inspection_catalog,
            system::install_cli,
            system::resource_detail,
            system::schema_detail,
            system::uninstall_cli,
            workflow::begin_workflow_stage,
            workflow::cancel_task,
            workflow::commit_task_completion_preview,
            workflow::commit_workflow_rerun,
            workflow::complete_workflow_stage,
            workflow::discard_workflow_preview,
            workflow::export_task_inputs,
            workflow::latest_task,
            workflow::prepare_task,
            workflow::prepare_task_again,
            workflow::preview_task_completion,
            workflow::preview_workflow_rerun,
            workflow::start_workflow,
            workflow::workflow_controls
        ])
        .run(tauri::generate_context!())
        .expect("failed to run the CanISend Tauri desktop application");
}
