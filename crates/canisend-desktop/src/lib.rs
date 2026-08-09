#![forbid(unsafe_code)]

mod agent;
mod agent_runtime;
mod application_intake;
mod approval;
mod association_v4;
mod commands;
mod delivery;
mod discovery;
mod generic_application;
mod job_intake;
mod profile;
mod system;
mod workflow;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .manage(agent_runtime::AgentRuntimeState::default())
        .manage(approval::DesktopApprovalStore::default());
    #[cfg(feature = "preview-qualification")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            agent::agent_assistance,
            agent::agent_skills_status,
            agent::copy_agent_mcp_configuration,
            agent::copy_agent_handoff,
            agent::export_agent_pack,
            agent::install_agent_skills,
            agent::prepare_agent_handoff,
            agent::prepare_agent_mcp_configuration,
            agent::uninstall_agent_skills,
            agent_runtime::agent_runtime_catalog,
            agent_runtime::cancel_agent_turn,
            agent_runtime::run_agent_turn,
            application_intake::commit_application_intake_preview,
            application_intake::discard_application_intake_preview,
            application_intake::preview_local_application_intake,
            application_intake::preview_pasted_application_intake,
            application_intake::preview_url_application_intake,
            association_v4::evidence_association_commit,
            association_v4::evidence_association_list,
            association_v4::evidence_association_preview,
            association_v4::profile_association_commit,
            association_v4::profile_association_list,
            association_v4::profile_association_preview,
            commands::application_dossier,
            commands::backup_workspace,
            commands::check_workspace,
            commands::connect_workspace,
            commands::content_catalog,
            commands::create_workspace,
            commands::import_local_job_source,
            commands::import_url_job_source,
            commands::list_application_dossiers,
            commands::list_workspaces,
            commands::product_summary,
            commands::remove_workspace,
            commands::repair_workspace,
            commands::restore_workspace,
            commands::run_doctor,
            commands::select_workspace,
            commands::search_content,
            commands::workspace_status,
            commands::workflow_pack_presentation,
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
            delivery::export_render_and_open,
            delivery::preview_render,
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
            generic_application::approve_generic_application,
            generic_application::compose_generic_application,
            generic_application::create_generic_application,
            generic_application::export_generic_application,
            generic_application::list_generic_applications,
            generic_application::plan_generic_application,
            generic_application::review_generic_application,
            generic_application::show_generic_application,
            job_intake::discard_job_source_preview,
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
            workflow::commit_workflow_rerun,
            workflow::complete_workflow_stage,
            workflow::discard_workflow_preview,
            workflow::preview_workflow_rerun,
            workflow::start_workflow
        ])
        .run(tauri::generate_context!())
        .expect("failed to run the CanISend Tauri desktop application");
}
