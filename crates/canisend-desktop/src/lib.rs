#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
mod agent;
#[cfg(target_os = "macos")]
mod agent_runtime;
#[cfg(target_os = "macos")]
mod commands;
#[cfg(target_os = "macos")]
mod delivery;
#[cfg(target_os = "macos")]
mod discovery;
#[cfg(target_os = "macos")]
mod job_intake;
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
        .manage(agent_runtime::AgentRuntimeState::default())
        .manage(discovery::DiscoveryPreviewStore::default())
        .manage(job_intake::JobIntakePreviewStore::default())
        .manage(workflow::WorkflowPreviewStore::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            agent::agent_capabilities,
            agent::agent_assistance,
            agent::agent_context,
            agent::copy_agent_mcp_configuration,
            agent::copy_agent_handoff,
            agent::export_agent_pack,
            agent::install_agent_skills,
            agent::prepare_agent_handoff,
            agent::prepare_agent_mcp_configuration,
            agent_runtime::agent_runtime_catalog,
            agent_runtime::cancel_agent_turn,
            agent_runtime::run_agent_turn,
            commands::archive_job,
            commands::application_dossier,
            commands::backup_workspace,
            commands::check_workspace,
            commands::connect_workspace,
            commands::content_catalog,
            commands::create_job,
            commands::create_workspace,
            commands::import_local_job_source,
            commands::import_url_job_source,
            commands::list_jobs,
            commands::list_application_dossiers,
            commands::list_workspaces,
            commands::product_summary,
            commands::remove_workspace,
            commands::repair_workspace,
            commands::restore_workspace,
            commands::run_doctor,
            commands::select_workspace,
            commands::search_content,
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
            job_intake::commit_job_source_preview,
            job_intake::discard_job_source_preview,
            job_intake::preview_local_job_source,
            job_intake::preview_url_job_source,
            profile::confirm_criteria,
            profile::confirm_plan,
            profile::confirm_profile_evidence,
            profile::criteria_template,
            profile::current_matches,
            profile::current_plan,
            profile::import_profile_source,
            profile::initialize_profile,
            profile::list_profile_sources,
            profile::plan_template,
            profile::profile_evidence_template,
            system::check_for_updates,
            system::cli_install_status,
            system::configure_cli_path,
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
