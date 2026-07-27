mod agent_page;
mod diagnostics_page;
mod dialogs;
mod discovery_page;
mod document_page;
mod package_page;
mod pages;
mod plan_page;
mod render_page;
mod review_page;
mod task_panel;

use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use crate::{
    cli_bridge::{bundled_cli_path, default_cli_destination},
    components::{
        WorkflowTimelineAction, accessible_error, accessible_heading, accessible_live_region,
        cli_state_style, command_copy_row, diagnostic_row, execution_mode_label,
        keep_focused_visible, localized_receipt_summary, localized_workspace_alias_error,
        metric_card, page_accessible_label, paint_focus_ring, set_accesskit_role,
        source_kind_label, stage_label, validate_criteria_review, validate_evidence_review,
        validate_job_form, validate_plan_review, validate_profile_source_form,
        workflow_control_timeline, workflow_timeline,
    },
    i18n::{self, Language},
    registry::{WorkspaceRegistry, default_registry_path, validate_workspace_alias},
    state::{
        AgentDestinationIssue, AgentDestinationPreview, AgentIntegrationForm,
        CatalogInspectionForm, CatalogPanel, CriteriaMatchForm, DiscoveryImportForm,
        DiscoveryPanel, DiscoveryRefreshForm, DocumentWorkspaceForm, EvidenceReviewForm,
        FocusTarget, GuiPreferences, ImportForm, ImportKind, JobForm, JobPanel,
        PackageWorkspaceForm, Page, PendingConfirmation, PlanReviewForm, ProfileSourceForm,
        RenderWorkspaceForm, RestoreWorkspaceForm, ReviewWorkspaceForm, TaskPanelForm,
        WorkflowActionForm, WorkspaceForm, available_task_modes, available_task_operations,
        parse_workflow_artifact_id, validate_discovery_import_form,
        validate_discovery_refresh_form,
    },
    theme,
    worker::{WorkerEvent, WorkerRequest, execute},
};
use canisend_app::{
    AgentHost, AgentPackExportRequest, Application, ApplicationFailure,
    DiscoveryAdapterCatalogReadModel, DiscoveryLeadListReadModel, DiscoverySourceListReadModel,
    DiscoverySuggestionReadModel, DoctorSummary, JobDetailReadModel, PackageExportRequest,
    ProductSummary, ProfileSourceListReadModel, ProjectionCopyAsNewRequest,
    ProjectionReplaceRequest, RenderExportRequest, ResourceCatalogExportRequest, TaskExecutionMode,
    TaskInputExportRequest, TaskOperation, TaskPrepareRequest, UpdateCheckReadModel,
    WorkflowBeginRequest, WorkflowCompleteRequest, WorkflowControlReadModel, WorkflowRerunRequest,
    WorkspaceHealthReadModel, WorkspaceReadModel,
};
use canisend_app::{CliInstallState, CliInstallStatus, CliVersionRelation};
use canisend_contracts::{
    ApplicationDecision, ApplicationPlanCandidate, ApplicationPlanRecord, ArtifactKind,
    CriterionImportance, DiscoveryLeadRecord, DocumentKind, DocumentPlanCandidateRecord,
    DocumentRequirement, EntityId, ErrorCode, EvidenceKind, ExecutionMode, JobRecord,
    MatchStrength, NextAction, PlanBlockerSeverity, PrivacyClassification, ProfileSourceKind,
    TaskStateData, TaskStatus, WorkflowStage,
};
use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke};

const APP_ID: &str = "io.github.jxpeng98.canisend";
const GUI_PREFERENCES_KEY: &str = "canisend.gui-preferences/v1";

#[cfg(target_os = "macos")]
fn pick_directory(title: Option<&str>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(title) = title {
        dialog = dialog.set_title(title);
    }
    dialog.pick_folder()
}

#[cfg(not(target_os = "macos"))]
fn pick_directory(_title: Option<&str>) -> Option<PathBuf> {
    None
}

#[cfg(target_os = "macos")]
fn pick_job_source_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Job sources", &["md", "txt", "json", "pdf"])
        .pick_file()
}

#[cfg(target_os = "macos")]
fn pick_profile_source_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Profile sources", &["md", "txt", "json"])
        .pick_file()
}

#[cfg(target_os = "macos")]
fn pick_discovery_batch_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Discovery batches", &["csv", "json"])
        .pick_file()
}

#[cfg(target_os = "macos")]
fn pick_task_completion_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Task completion", &["json"])
        .pick_file()
}

#[cfg(not(target_os = "macos"))]
fn pick_profile_source_file() -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "macos"))]
fn pick_job_source_file() -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "macos"))]
fn pick_discovery_batch_file() -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "macos"))]
fn pick_task_completion_file() -> Option<PathBuf> {
    None
}

pub(crate) fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id(APP_ID)
            .with_title("CanISend")
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([800.0, 600.0])
            .with_drag_and_drop(true),
        renderer: eframe::Renderer::Glow,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        APP_ID,
        options,
        Box::new(|creation| Ok(Box::new(CanISendDesktop::new(creation)))),
    )
}

#[derive(Debug)]
struct Activity {
    label: String,
    started: std::time::Instant,
}

struct CanISendDesktop {
    registry_path: PathBuf,
    registry: WorkspaceRegistry,
    active_workspace: Option<PathBuf>,
    workspace: Option<WorkspaceReadModel>,
    health: Option<WorkspaceHealthReadModel>,
    jobs: Vec<JobRecord>,
    include_archived: bool,
    job_filter: String,
    discovery_adapters: Option<DiscoveryAdapterCatalogReadModel>,
    discovery_sources: Option<DiscoverySourceListReadModel>,
    discovery_leads: Option<DiscoveryLeadListReadModel>,
    selected_discovery_lead: Option<DiscoveryLeadRecord>,
    discovery_suggestions: Option<DiscoverySuggestionReadModel>,
    discovery_next_actions: Vec<NextAction>,
    discovery_panel: DiscoveryPanel,
    discovery_filter: String,
    discovery_include_history: bool,
    discovery_import_form: DiscoveryImportForm,
    discovery_refresh_form: DiscoveryRefreshForm,
    selected_job: Option<JobDetailReadModel>,
    selected_job_id: Option<String>,
    job_panel: JobPanel,
    document_form: DocumentWorkspaceForm,
    review_form: ReviewWorkspaceForm,
    package_form: PackageWorkspaceForm,
    render_form: RenderWorkspaceForm,
    profile_sources: Option<ProfileSourceListReadModel>,
    workflow_controls: Option<WorkflowControlReadModel>,
    workflow_action_form: Option<WorkflowActionForm>,
    task_form: TaskPanelForm,
    agent_form: AgentIntegrationForm,
    page: Page,
    job_form: JobForm,
    show_job_form: bool,
    import_form: ImportForm,
    show_import_form: bool,
    profile_source_form: ProfileSourceForm,
    show_profile_source_form: bool,
    evidence_review_form: EvidenceReviewForm,
    criteria_match_form: CriteriaMatchForm,
    plan_review_form: PlanReviewForm,
    workspace_form: WorkspaceForm,
    show_workspace_form: bool,
    restore_workspace_form: RestoreWorkspaceForm,
    show_restore_workspace_form: bool,
    product: ProductSummary,
    doctor: Option<DoctorSummary>,
    catalog_form: CatalogInspectionForm,
    cli_source: Option<PathBuf>,
    cli_destination: PathBuf,
    cli_status: Option<CliInstallStatus>,
    update_check: Option<UpdateCheckReadModel>,
    dark_mode: bool,
    compact: bool,
    reduce_motion: bool,
    language: Language,
    cjk_font_path: Option<&'static str>,
    activity: Option<Activity>,
    receiver: Option<Receiver<WorkerEvent>>,
    notice: Option<(bool, String)>,
    registry_error: Option<String>,
    pending_confirmation: Option<PendingConfirmation>,
    pending_focus: Option<FocusTarget>,
}

impl CanISendDesktop {
    fn new(creation: &eframe::CreationContext<'_>) -> Self {
        let registry_path = default_registry_path();
        let (registry, registry_error) = match WorkspaceRegistry::load(&registry_path) {
            Ok(registry) => (registry, None),
            Err(error) => (WorkspaceRegistry::default(), Some(error)),
        };
        let active_workspace = registry.default_path.clone();
        let stored_preferences = creation
            .storage
            .and_then(|storage| eframe::get_value::<GuiPreferences>(storage, GUI_PREFERENCES_KEY));
        let dark_mode = stored_preferences.map_or_else(
            || creation.egui_ctx.theme() == egui::Theme::Dark,
            |preferences| preferences.dark_mode,
        );
        let compact = stored_preferences.is_some_and(|preferences| preferences.compact);
        let reduce_motion = stored_preferences.is_some_and(|preferences| preferences.reduce_motion);
        let language =
            stored_preferences.map_or_else(Language::default, |preferences| preferences.language);
        let cjk_font_path = i18n::install_cjk_fallback(&creation.egui_ctx);
        theme::apply(&creation.egui_ctx, dark_mode, compact, reduce_motion);
        let mut application = Self {
            registry_path,
            registry,
            active_workspace,
            workspace: None,
            health: None,
            jobs: Vec::new(),
            include_archived: false,
            job_filter: String::new(),
            discovery_adapters: None,
            discovery_sources: None,
            discovery_leads: None,
            selected_discovery_lead: None,
            discovery_suggestions: None,
            discovery_next_actions: Vec::new(),
            discovery_panel: DiscoveryPanel::Leads,
            discovery_filter: String::new(),
            discovery_include_history: false,
            discovery_import_form: DiscoveryImportForm::default(),
            discovery_refresh_form: DiscoveryRefreshForm::default(),
            selected_job: None,
            selected_job_id: None,
            job_panel: JobPanel::Workflow,
            document_form: DocumentWorkspaceForm::default(),
            review_form: ReviewWorkspaceForm::default(),
            package_form: PackageWorkspaceForm::default(),
            render_form: RenderWorkspaceForm::default(),
            profile_sources: None,
            workflow_controls: None,
            workflow_action_form: None,
            task_form: TaskPanelForm::default(),
            agent_form: AgentIntegrationForm::default(),
            page: Page::Overview,
            job_form: JobForm::default(),
            show_job_form: false,
            import_form: ImportForm::default(),
            show_import_form: false,
            profile_source_form: ProfileSourceForm::default(),
            show_profile_source_form: false,
            evidence_review_form: EvidenceReviewForm::default(),
            criteria_match_form: CriteriaMatchForm::default(),
            plan_review_form: PlanReviewForm::default(),
            workspace_form: WorkspaceForm::default(),
            show_workspace_form: false,
            restore_workspace_form: RestoreWorkspaceForm::default(),
            show_restore_workspace_form: false,
            product: Application::product_summary(),
            doctor: None,
            catalog_form: CatalogInspectionForm::default(),
            cli_source: bundled_cli_path(),
            cli_destination: default_cli_destination(),
            cli_status: None,
            update_check: None,
            dark_mode,
            compact,
            reduce_motion,
            language,
            cjk_font_path,
            activity: None,
            receiver: None,
            notice: None,
            registry_error,
            pending_confirmation: None,
            pending_focus: None,
        };
        if let Some(path) = application.active_workspace.clone() {
            application.load_workspace(path, creation.egui_ctx.clone());
        }
        application
    }

    fn dispatch(&mut self, label: impl Into<String>, ctx: egui::Context, request: WorkerRequest) {
        if self.activity.is_some() {
            self.notice = Some((
                false,
                self.language
                    .text("Finish the current operation before starting another one.")
                    .to_owned(),
            ));
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.activity = Some(Activity {
            label: label.into(),
            started: std::time::Instant::now(),
        });
        thread::spawn(move || {
            let event = execute(request);
            let _ = sender.send(event);
            ctx.request_repaint();
        });
    }

    fn poll_worker(&mut self, ctx: &egui::Context) {
        let result = self.receiver.as_ref().map(Receiver::try_recv);
        match result {
            Some(Ok(event)) => {
                self.receiver = None;
                self.activity = None;
                self.apply_worker_event(event, ctx);
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.receiver = None;
                self.activity = None;
                self.fail(
                    self.language
                        .text(
                            "The background operation ended unexpectedly. No completion was recorded; review the current workspace state and try again.",
                        )
                        .to_owned(),
                );
            }
            Some(Err(TryRecvError::Empty)) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
            }
            None => {}
        }
    }

    fn apply_worker_event(&mut self, event: WorkerEvent, ctx: &egui::Context) {
        match event {
            WorkerEvent::WorkspaceLoaded(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.workspace = Some(receipt.data);
                    self.notice = Some((true, summary));
                    self.refresh_jobs(ctx.clone());
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::WorkspaceCreated { alias, result } => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let path = receipt.data.path.clone();
                    match self.registry.register(&alias, &path) {
                        Ok(canonical) => {
                            self.active_workspace = Some(canonical);
                            self.workspace = Some(receipt.data);
                            self.show_workspace_form = false;
                            self.workspace_form = WorkspaceForm::default();
                            self.save_registry();
                            self.notice = Some((true, summary));
                            self.refresh_jobs(ctx.clone());
                        }
                        Err(error) => self.fail(error),
                    }
                }
                Err(error) => self.workspace_form.error = Some(error),
            },
            WorkerEvent::WorkspaceChecked(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.health = Some(receipt.data);
                    self.notice = Some((true, summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::BackupCreated(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.notice = Some((true, summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::WorkspaceRestored { alias, result } => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let path = receipt.data.destination.clone();
                    match self.registry.register(&alias, &path) {
                        Ok(canonical) => {
                            self.active_workspace = Some(canonical.clone());
                            self.workspace = Some(WorkspaceReadModel {
                                path: canonical,
                                status: receipt.data.workspace,
                            });
                            self.health = None;
                            self.show_restore_workspace_form = false;
                            self.restore_workspace_form = RestoreWorkspaceForm::default();
                            self.save_registry();
                            self.notice = Some((true, summary));
                            self.refresh_jobs(ctx.clone());
                        }
                        Err(error) => self.restore_workspace_form.error = Some(error),
                    }
                }
                Err(error) => self.restore_workspace_form.error = Some(error),
            },
            WorkerEvent::WorkspaceRepaired(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.health = Some(WorkspaceHealthReadModel {
                        path: receipt.data.workspace,
                        check: receipt.data.check,
                    });
                    self.notice = Some((true, summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::JobsLoaded(result) => match result {
                Ok(receipt) => {
                    self.jobs = receipt.data.jobs;
                    if self.page == Page::Profile && self.profile_sources.is_none() {
                        self.refresh_profile_sources(ctx.clone());
                    } else if self.page == Page::Discovery && self.discovery_sources.is_none() {
                        if self.discovery_adapters.is_none() {
                            self.load_discovery_catalog(ctx.clone());
                        } else {
                            self.refresh_discovery_workspace(ctx.clone());
                        }
                    } else if self.page == Page::AgentIntegration
                        && self.agent_form.context.is_none()
                    {
                        self.load_agent_integration(ctx.clone());
                    }
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::DiscoveryCatalogLoaded(result) => match result {
                Ok(receipt) => {
                    self.discovery_adapters = Some(receipt.data);
                    if let Some(path) = self.active_workspace.clone() {
                        self.dispatch(
                            self.language.text("Loading discovery workspace"),
                            ctx.clone(),
                            WorkerRequest::LoadDiscoveryWorkspace {
                                path,
                                include_history: self.discovery_include_history,
                            },
                        );
                    }
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::DiscoveryWorkspaceLoaded(result) => match result {
                Ok(discovery) => self.apply_discovery_workspace(discovery),
                Err(error) => self.fail(error),
            },
            WorkerEvent::DiscoveryLeadLoaded(result) => match result {
                Ok(receipt) => {
                    self.selected_discovery_lead = Some(receipt.data);
                    self.discovery_suggestions = None;
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::DiscoveryImportPreviewed(result) => match result {
                Ok(receipt) => {
                    self.discovery_import_form.preview = Some(receipt.data);
                    self.discovery_import_form.error = None;
                }
                Err(error) => self.discovery_import_form.error = Some(error),
            },
            WorkerEvent::DiscoveryImportCommitted(result) => match result {
                Ok(committed) => {
                    let summary = localized_receipt_summary(&committed.receipt, self.language);
                    let refresh_error = match committed.discovery {
                        Ok(discovery) => {
                            self.apply_discovery_workspace(discovery);
                            None
                        }
                        Err(error) => {
                            self.discovery_sources = None;
                            self.discovery_leads = None;
                            Some(error)
                        }
                    };
                    self.discovery_import_form = DiscoveryImportForm::default();
                    self.discovery_panel = DiscoveryPanel::Leads;
                    self.notice = Some(match refresh_error {
                        Some(error) => (
                            false,
                            self.language.select(
                                &format!(
                                    "{summary}; the workspace lists could not be refreshed: {error}"
                                ),
                                &format!(
                                    "{summary}；但无法刷新工作区列表：{error}"
                                ),
                            ).to_owned(),
                        ),
                        None => (true, summary),
                    });
                }
                Err(error) => self.discovery_import_form.error = Some(error),
            },
            WorkerEvent::DiscoveryRefreshPreviewed(result) => match result {
                Ok(receipt) => {
                    self.discovery_refresh_form.preview = Some(receipt.data);
                    self.discovery_refresh_form.error = None;
                }
                Err(error) => self.discovery_refresh_form.error = Some(error),
            },
            WorkerEvent::DiscoveryRefreshCommitted(result) => match result {
                Ok(committed) => {
                    let summary = localized_receipt_summary(&committed.receipt, self.language);
                    let refresh_error = match committed.discovery {
                        Ok(discovery) => {
                            self.apply_discovery_workspace(discovery);
                            None
                        }
                        Err(error) => {
                            self.discovery_sources = None;
                            self.discovery_leads = None;
                            Some(error)
                        }
                    };
                    self.discovery_refresh_form = DiscoveryRefreshForm::default();
                    self.discovery_panel = DiscoveryPanel::Leads;
                    self.notice = Some(match refresh_error {
                        Some(error) => (
                            false,
                            self.language.select(
                                &format!(
                                    "{summary}; the workspace lists could not be refreshed: {error}"
                                ),
                                &format!(
                                    "{summary}；但无法刷新工作区列表：{error}"
                                ),
                            ).to_owned(),
                        ),
                        None => (true, summary),
                    });
                }
                Err(error) => self.discovery_refresh_form.error = Some(error),
            },
            WorkerEvent::DiscoverySuggestionsLoaded(result) => match result {
                Ok(receipt) => self.discovery_suggestions = Some(receipt.data),
                Err(error) => self.fail(error),
            },
            WorkerEvent::DiscoveryLeadPromoted(result) => match result {
                Ok(promoted) => {
                    let summary = localized_receipt_summary(&promoted.receipt, self.language);
                    self.discovery_next_actions = promoted.receipt.next_actions;
                    self.selected_discovery_lead = None;
                    self.discovery_suggestions = None;
                    let mut refresh_errors = Vec::new();
                    match promoted.jobs {
                        Ok(jobs) => self.jobs = jobs.jobs,
                        Err(error) => refresh_errors.push(error),
                    }
                    match promoted.discovery {
                        Ok(discovery) => self.apply_discovery_workspace(discovery),
                        Err(error) => {
                            self.discovery_sources = None;
                            self.discovery_leads = None;
                            refresh_errors.push(error);
                        }
                    }
                    self.notice = Some(if refresh_errors.is_empty() {
                        (true, summary)
                    } else {
                        (
                            false,
                            match self.language {
                                Language::English => format!(
                                    "{summary}; current lists could not be refreshed: {}",
                                    refresh_errors.join("; ")
                                ),
                                Language::SimplifiedChinese => format!(
                                    "{summary}；但无法刷新当前列表：{}",
                                    refresh_errors.join("；")
                                ),
                            },
                        )
                    });
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::JobCreated(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let id = receipt.data.id.to_string();
                    self.show_job_form = false;
                    self.job_form = JobForm::default();
                    self.notice = Some((true, summary));
                    self.selected_job_id = Some(id.clone());
                    self.page = Page::Jobs;
                    self.job_panel = JobPanel::Workflow;
                    self.document_form = DocumentWorkspaceForm::default();
                    self.review_form = ReviewWorkspaceForm::default();
                    self.package_form = PackageWorkspaceForm::default();
                    self.render_form = RenderWorkspaceForm::default();
                    self.load_job(id, ctx.clone());
                }
                Err(error) => self.job_form.error = Some(error),
            },
            WorkerEvent::JobLoaded(result) => match result {
                Ok(receipt) => {
                    let job_id = receipt.data.job.id.to_string();
                    let has_workflow = receipt.data.workflow.is_some();
                    if self.criteria_match_form.job_id.as_deref() != Some(job_id.as_str()) {
                        self.criteria_match_form = CriteriaMatchForm {
                            job_id: Some(job_id.clone()),
                            ..CriteriaMatchForm::default()
                        };
                    }
                    if self.plan_review_form.job_id.as_deref() != Some(job_id.as_str()) {
                        self.plan_review_form = PlanReviewForm {
                            job_id: Some(job_id.clone()),
                            ..PlanReviewForm::default()
                        };
                    }
                    self.selected_job_id = Some(job_id.clone());
                    self.document_form.select_job(&job_id);
                    self.review_form.select_job(&job_id);
                    self.package_form.select_job(&job_id);
                    self.render_form.select_job(&job_id);
                    self.task_form.select_job(&job_id);
                    self.selected_job = Some(receipt.data);
                    if has_workflow {
                        self.load_workflow_controls(job_id, ctx.clone());
                    } else {
                        self.workflow_controls = None;
                        self.load_latest_task(job_id, ctx.clone());
                    }
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::JobArchived(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.selected_job = None;
                    self.selected_job_id = None;
                    self.workflow_controls = None;
                    self.workflow_action_form = None;
                    self.task_form = TaskPanelForm::default();
                    self.job_panel = JobPanel::Workflow;
                    self.document_form = DocumentWorkspaceForm::default();
                    self.review_form = ReviewWorkspaceForm::default();
                    self.package_form = PackageWorkspaceForm::default();
                    self.render_form = RenderWorkspaceForm::default();
                    self.criteria_match_form = CriteriaMatchForm::default();
                    self.plan_review_form = PlanReviewForm::default();
                    self.notice = Some((true, summary));
                    self.refresh_jobs(ctx.clone());
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::SourceImported(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let id = receipt.data.job.id.to_string();
                    self.show_import_form = false;
                    self.import_form = ImportForm::default();
                    self.criteria_match_form = CriteriaMatchForm {
                        job_id: Some(id.clone()),
                        ..CriteriaMatchForm::default()
                    };
                    self.plan_review_form = PlanReviewForm {
                        job_id: Some(id.clone()),
                        ..PlanReviewForm::default()
                    };
                    self.document_form.clear_loaded_private_data();
                    self.review_form.clear_loaded_private_data();
                    self.package_form.clear_loaded_data();
                    self.render_form.clear_loaded_data();
                    self.notice = Some((true, summary));
                    self.load_job(id, ctx.clone());
                }
                Err(error) => self.import_form.error = Some(error),
            },
            WorkerEvent::ProfileSourcesLoaded(result) => match result {
                Ok(receipt) => self.profile_sources = Some(receipt.data),
                Err(error) => self.fail(error),
            },
            WorkerEvent::ProfileSourceImported(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.show_profile_source_form = false;
                    self.profile_source_form = ProfileSourceForm::default();
                    self.document_form.clear_loaded_private_data();
                    self.review_form.clear_loaded_private_data();
                    self.package_form.clear_loaded_data();
                    self.render_form.clear_loaded_data();
                    self.notice = Some((true, summary));
                    self.refresh_profile_sources(ctx.clone());
                }
                Err(error) => self.profile_source_form.error = Some(error),
            },
            WorkerEvent::ProfileEvidenceLoaded { job_id, result } => match result {
                Ok(receipt) => {
                    let mut candidate = receipt.data;
                    for item in &mut candidate.items {
                        item.confirmed = false;
                    }
                    self.evidence_review_form.job_id = Some(job_id);
                    self.evidence_review_form.candidate = Some(candidate);
                    self.evidence_review_form.downstream_effects_confirmed = false;
                    self.evidence_review_form.error = None;
                }
                Err(error) => self.evidence_review_form.error = Some(error),
            },
            WorkerEvent::ProfileEvidenceConfirmed { job_id, result } => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.evidence_review_form.job_id = Some(job_id.clone());
                    self.evidence_review_form.candidate = Some(receipt.data);
                    self.evidence_review_form.downstream_effects_confirmed = false;
                    self.evidence_review_form.error = None;
                    self.notice = Some((true, summary));
                    if self.plan_review_form.job_id.as_deref() == Some(job_id.as_str()) {
                        self.plan_review_form = PlanReviewForm {
                            job_id: Some(job_id.clone()),
                            ..PlanReviewForm::default()
                        };
                    }
                    self.document_form.clear_loaded_private_data();
                    self.review_form.clear_loaded_private_data();
                    self.package_form.clear_loaded_data();
                    self.render_form.clear_loaded_data();
                    if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                        self.load_job(job_id, ctx.clone());
                    }
                }
                Err(error) => self.evidence_review_form.error = Some(error),
            },
            WorkerEvent::CriteriaCandidateLoaded { job_id, result } => match result {
                Ok(receipt) => {
                    let mut candidate = receipt.data;
                    for criterion in &mut candidate.criteria {
                        criterion.confirmed = false;
                    }
                    self.criteria_match_form.job_id = Some(job_id);
                    self.criteria_match_form.candidate = Some(candidate);
                    self.criteria_match_form.downstream_effects_confirmed = false;
                    self.criteria_match_form.criteria_error = None;
                    self.criteria_match_form.matches = None;
                    self.criteria_match_form.match_error = None;
                }
                Err(error) => self.criteria_match_form.criteria_error = Some(error),
            },
            WorkerEvent::CriteriaConfirmed { job_id, result } => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.criteria_match_form.job_id = Some(job_id.clone());
                    self.criteria_match_form.candidate = Some(receipt.data);
                    self.criteria_match_form.downstream_effects_confirmed = false;
                    self.criteria_match_form.criteria_error = None;
                    self.criteria_match_form.matches = None;
                    self.criteria_match_form.match_error = None;
                    self.plan_review_form = PlanReviewForm::default();
                    self.document_form.clear_loaded_private_data();
                    self.review_form.clear_loaded_private_data();
                    self.package_form.clear_loaded_data();
                    self.render_form.clear_loaded_data();
                    self.notice = Some((true, summary));
                    if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                        self.load_job(job_id, ctx.clone());
                    }
                }
                Err(error) => self.criteria_match_form.criteria_error = Some(error),
            },
            WorkerEvent::CurrentMatchesLoaded { job_id, result } => match result {
                Ok(receipt) => {
                    self.criteria_match_form.job_id = Some(job_id);
                    self.criteria_match_form.matches = Some(receipt.data);
                    self.criteria_match_form.match_error = None;
                }
                Err(error) => self.criteria_match_form.match_error = Some(error),
            },
            WorkerEvent::PlanCandidateLoaded { job_id, result } => match result {
                Ok(receipt) => {
                    self.plan_review_form.job_id = Some(job_id);
                    self.plan_review_form.candidate = Some(receipt.data);
                    self.plan_review_form.current = None;
                    self.plan_review_form.decision_confirmed = false;
                    self.plan_review_form.error = None;
                }
                Err(error) => self.plan_review_form.error = Some(error),
            },
            WorkerEvent::CurrentPlanLoaded { job_id, result } => match result {
                Ok(receipt) => {
                    let current = receipt.data;
                    self.plan_review_form.job_id = Some(job_id);
                    self.plan_review_form.candidate = Some(editable_plan(&current));
                    self.plan_review_form.current = Some(current);
                    self.plan_review_form.decision_confirmed = false;
                    self.plan_review_form.error = None;
                }
                Err(error) => self.plan_review_form.error = Some(error),
            },
            WorkerEvent::PlanConfirmed { job_id, result } => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let current = receipt.data;
                    self.plan_review_form.job_id = Some(job_id.clone());
                    self.plan_review_form.candidate = Some(editable_plan(&current));
                    self.plan_review_form.current = Some(current);
                    self.plan_review_form.decision_confirmed = false;
                    self.plan_review_form.error = None;
                    self.document_form.clear_loaded_private_data();
                    self.review_form.clear_loaded_private_data();
                    self.package_form.clear_loaded_data();
                    self.render_form.clear_loaded_data();
                    self.notice = Some((true, summary));
                    if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                        self.load_job(job_id, ctx.clone());
                    }
                }
                Err(error) => self.plan_review_form.error = Some(error),
            },
            WorkerEvent::DocumentsLoaded { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(workspace) => {
                            self.document_form.job_id = Some(job_id);
                            self.document_form.documents = Some(workspace.documents);
                            self.document_form.accepted_set = workspace.accepted_set;
                            self.document_form.acceptance_blocker = workspace.acceptance_blocker;
                            self.document_form.error = None;
                            self.notice = Some((
                                true,
                                self.language
                                    .select(
                                        "Current structured documents loaded",
                                        "已加载当前结构化申请文档",
                                    )
                                    .to_owned(),
                            ));
                            self.pending_focus = Some(FocusTarget::DocumentLoad);
                        }
                        Err(error) => {
                            self.document_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::DocumentLoad);
                        }
                    }
                }
            }
            WorkerEvent::ReviewLoaded { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(workspace) => {
                            self.review_form.job_id = Some(job_id);
                            self.review_form.current = Some(workspace.current);
                            self.review_form.candidate = Some(workspace.disposition_candidate);
                            self.review_form.downstream_effects_confirmed = false;
                            self.review_form.error = None;
                            self.notice = Some((
                                true,
                                self.language
                                    .select("Current review findings loaded", "已加载当前审阅发现")
                                    .to_owned(),
                            ));
                            self.pending_focus = Some(FocusTarget::ReviewLoad);
                        }
                        Err(error) => {
                            self.review_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::ReviewLoad);
                        }
                    }
                }
            }
            WorkerEvent::ReviewConfirmed { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(committed) => {
                            let summary =
                                localized_receipt_summary(&committed.receipt, self.language);
                            self.package_form.clear_loaded_data();
                            self.render_form.clear_loaded_data();
                            match committed.workspace {
                                Ok(workspace) => {
                                    self.review_form.job_id = Some(job_id);
                                    self.review_form.current = Some(workspace.current);
                                    self.review_form.candidate =
                                        Some(workspace.disposition_candidate);
                                    self.review_form.downstream_effects_confirmed = false;
                                    self.review_form.error = None;
                                    self.notice = Some((true, summary));
                                }
                                Err(error) => {
                                    self.review_form.clear_loaded_private_data();
                                    self.review_form.error = Some(error.clone());
                                    self.notice = Some((
                                        false,
                                        self.language
                                            .select(
                                                &format!(
                                                    "{summary}; confirmation succeeded but the current review could not be refreshed: {error}"
                                                ),
                                                &format!(
                                                    "{summary}；处置已确认，但无法刷新当前审阅：{error}"
                                                ),
                                            )
                                            .to_owned(),
                                    ));
                                }
                            }
                            self.pending_focus = Some(FocusTarget::ReviewConfirm);
                        }
                        Err(error) => {
                            self.review_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::ReviewConfirm);
                        }
                    }
                }
            }
            WorkerEvent::PackageChecked { job_id, result }
            | WorkerEvent::PackageLoaded { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(receipt) => {
                            let summary = localized_receipt_summary(&receipt, self.language);
                            self.render_form.clear_loaded_data();
                            self.package_form.job_id = Some(job_id);
                            self.package_form.manifest = Some(receipt.data);
                            self.package_form.error = None;
                            self.notice = Some((true, summary));
                            self.pending_focus = Some(FocusTarget::PackageCheck);
                        }
                        Err(error) => {
                            self.package_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::PackageCheck);
                        }
                    }
                }
            }
            WorkerEvent::PackageExported { job_id, result }
            | WorkerEvent::PackageExportLoaded { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(receipt) => {
                            let summary = localized_receipt_summary(&receipt, self.language);
                            self.package_form.job_id = Some(job_id);
                            self.package_form.export_receipt = Some(receipt.data);
                            self.package_form.reconciliation = None;
                            self.package_form.private_export_consent = false;
                            self.package_form.error = None;
                            self.notice = Some((true, summary));
                            self.pending_focus = Some(FocusTarget::PackageExport);
                        }
                        Err(error) => {
                            self.package_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::PackageExport);
                        }
                    }
                }
            }
            WorkerEvent::PackageReconciled { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(receipt) => {
                            let summary = localized_receipt_summary(&receipt, self.language);
                            self.package_form.job_id = Some(job_id);
                            self.package_form.reconciliation = Some(receipt.data);
                            self.package_form.selected_projection = None;
                            self.package_form.copy_destination.clear();
                            self.package_form.error = None;
                            self.notice = Some((true, summary));
                            self.pending_focus = Some(FocusTarget::PackageReconcile);
                        }
                        Err(error) => {
                            self.package_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::PackageReconcile);
                        }
                    }
                }
            }
            WorkerEvent::ProjectionReplaced { job_id, result }
            | WorkerEvent::ProjectionCopiedAsNew { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(receipt) => {
                            let summary = localized_receipt_summary(&receipt, self.language);
                            if let Some(records) = &mut self.package_form.reconciliation
                                && let Some(existing) = records.iter_mut().find(|record| {
                                    record.projection.relative_path
                                        == receipt.data.projection.relative_path
                                })
                            {
                                *existing = receipt.data;
                            }
                            self.package_form.selected_projection = None;
                            self.package_form.copy_destination.clear();
                            self.package_form.error = None;
                            self.notice = Some((true, summary));
                            self.pending_focus = Some(FocusTarget::PackageReconcile);
                        }
                        Err(error) => {
                            self.package_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::PackageReconcile);
                        }
                    }
                }
            }
            WorkerEvent::RenderBuilt { job_id, result }
            | WorkerEvent::RenderLoaded { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(receipt) => {
                            let summary = localized_receipt_summary(&receipt, self.language);
                            self.render_form.job_id = Some(job_id);
                            self.render_form.manifest = Some(receipt.data);
                            self.render_form.export = None;
                            self.render_form.error = None;
                            self.notice = Some((true, summary));
                            self.pending_focus = Some(FocusTarget::RenderBuild);
                        }
                        Err(error) => {
                            self.render_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::RenderBuild);
                        }
                    }
                }
            }
            WorkerEvent::RenderExported { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(receipt) => {
                            let summary = localized_receipt_summary(&receipt, self.language);
                            self.render_form.job_id = Some(job_id);
                            self.render_form.manifest = Some(receipt.data.render_manifest.clone());
                            self.render_form.export = Some(receipt.data);
                            self.render_form.private_export_consent = false;
                            self.render_form.error = None;
                            self.notice = Some((true, summary));
                            self.pending_focus = Some(FocusTarget::RenderExport);
                        }
                        Err(error) => {
                            self.render_form.error = Some(error);
                            self.pending_focus = Some(FocusTarget::RenderExport);
                        }
                    }
                }
            }
            WorkerEvent::WorkflowLoaded(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let job_id = receipt.data.job_id.to_string();
                    if let Some(job) = self.selected_job.as_mut() {
                        job.workflow = Some(receipt.data);
                    }
                    self.notice = Some((true, summary));
                    self.load_workflow_controls(job_id, ctx.clone());
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::WorkflowControlsLoaded(result) => match result {
                Ok(receipt) => {
                    self.workflow_controls = Some(receipt.data);
                    if let Some(job_id) = self.selected_job_id.clone() {
                        self.load_latest_task(job_id, ctx.clone());
                    }
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::WorkflowMutated(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    if let Some(job) = self.selected_job.as_mut() {
                        job.workflow = Some(receipt.data.status.clone());
                    }
                    self.workflow_controls = Some(receipt.data);
                    self.workflow_action_form = None;
                    self.plan_review_form = PlanReviewForm {
                        job_id: self.selected_job_id.clone(),
                        ..PlanReviewForm::default()
                    };
                    self.document_form.clear_loaded_private_data();
                    self.review_form.clear_loaded_private_data();
                    self.package_form.clear_loaded_data();
                    self.render_form.clear_loaded_data();
                    self.notice = Some((true, summary));
                    if let Some(job_id) = self.selected_job_id.clone() {
                        self.load_latest_task(job_id, ctx.clone());
                    }
                }
                Err(error) => {
                    if let Some(form) = self.workflow_action_form.as_mut() {
                        form.set_error(error);
                    } else {
                        self.fail(error);
                    }
                }
            },
            WorkerEvent::WorkflowRerunPreviewed(result) => match result {
                Ok(receipt) => {
                    self.pending_confirmation = Some(PendingConfirmation::RerunWorkflow {
                        preview: receipt.data,
                    });
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::LatestTaskLoaded { job_id, result } => {
                if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                    match result {
                        Ok(receipt) => self.task_form.apply_state(receipt.data),
                        Err(failure) => self.apply_task_failure(failure, FocusTarget::TaskPrepare),
                    }
                }
            }
            WorkerEvent::TaskPrepared(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.task_form.apply_state(Some(TaskStateData {
                        descriptor: receipt.data,
                        status: TaskStatus::Prepared,
                        result: None,
                    }));
                    self.task_form.failure = None;
                    self.notice = Some((true, summary));
                    self.pending_focus = Some(FocusTarget::TaskExport);
                    self.reload_selected_job_after_task(ctx.clone());
                }
                Err(failure) => self.apply_task_failure(failure, FocusTarget::TaskPrepare),
            },
            WorkerEvent::TaskInputsExported(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.task_form.exported = Some(receipt.data);
                    self.task_form.failure = None;
                    self.notice = Some((true, summary));
                    self.pending_focus = Some(FocusTarget::TaskCompletionFile);
                }
                Err(failure) => self.apply_task_failure(failure, FocusTarget::TaskExport),
            },
            WorkerEvent::TaskCompletionPreviewed(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let preview = receipt.data;
                    self.task_form.apply_state(Some(preview.state.clone()));
                    self.task_form.completion_preview = Some(preview);
                    self.task_form.failure = None;
                    self.task_form.stale_detected = false;
                    self.notice = Some((true, summary));
                    self.pending_focus = Some(FocusTarget::TaskCommit);
                }
                Err(failure) => {
                    let focus = if failure.code == ErrorCode::TaskStale {
                        FocusTarget::TaskCancel
                    } else {
                        FocusTarget::TaskCompletionFile
                    };
                    self.apply_task_failure(failure, focus);
                }
            },
            WorkerEvent::TaskCompleted(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    let committed = receipt.data;
                    if let Some(current) = self.task_form.state.as_ref()
                        && current.descriptor.id == committed.task_id
                    {
                        self.task_form.apply_state(Some(TaskStateData {
                            descriptor: current.descriptor.clone(),
                            status: TaskStatus::Committed,
                            result: Some(committed.artifact),
                        }));
                    }
                    self.task_form.completion_preview = None;
                    self.task_form.failure = None;
                    self.document_form.clear_loaded_private_data();
                    self.review_form.clear_loaded_private_data();
                    self.package_form.clear_loaded_data();
                    self.render_form.clear_loaded_data();
                    self.notice = Some((true, summary));
                    self.reload_selected_job_after_task(ctx.clone());
                }
                Err(failure) => {
                    let stale = failure.code == ErrorCode::TaskStale;
                    self.apply_task_failure(
                        failure,
                        if stale {
                            FocusTarget::TaskPrepareAgain
                        } else {
                            FocusTarget::TaskCommit
                        },
                    );
                    if stale && let Some(job_id) = self.selected_job_id.clone() {
                        self.load_latest_task(job_id, ctx.clone());
                    }
                }
            },
            WorkerEvent::TaskCancelled(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.task_form.apply_state(Some(receipt.data));
                    self.task_form.failure = None;
                    self.notice = Some((true, summary));
                    self.pending_focus = Some(FocusTarget::TaskPrepareAgain);
                    self.reload_selected_job_after_task(ctx.clone());
                }
                Err(failure) => self.apply_task_failure(failure, FocusTarget::TaskCancel),
            },
            WorkerEvent::TaskPreparedAgain(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.task_form.apply_state(Some(TaskStateData {
                        descriptor: receipt.data.descriptor,
                        status: TaskStatus::Prepared,
                        result: None,
                    }));
                    self.task_form.failure = None;
                    self.notice = Some((true, summary));
                    self.pending_focus = Some(FocusTarget::TaskExport);
                    self.reload_selected_job_after_task(ctx.clone());
                }
                Err(failure) => self.apply_task_failure(failure, FocusTarget::TaskPrepareAgain),
            },
            WorkerEvent::AgentCapabilitiesLoaded(result) => match result {
                Ok(receipt) => {
                    self.agent_form.capabilities = Some(receipt.data);
                    self.agent_form.failure = None;
                    if self.page == Page::AgentIntegration {
                        self.load_agent_context(ctx.clone());
                    }
                }
                Err(failure) => {
                    self.apply_agent_failure(failure, FocusTarget::AgentContextRefresh);
                }
            },
            WorkerEvent::AgentContextLoaded {
                selected_job_id,
                result,
            } => {
                if self.agent_form.selected_job_id != selected_job_id {
                    self.load_agent_context(ctx.clone());
                } else {
                    match result {
                        Ok(receipt) => {
                            self.agent_form.context = Some(receipt.data);
                            self.agent_form.failure = None;
                        }
                        Err(failure) => {
                            self.apply_agent_failure(failure, FocusTarget::AgentContextRefresh);
                        }
                    }
                }
            }
            WorkerEvent::AgentPackExported { request, result } => {
                let selection_matches = self.agent_form.host == request.host
                    && self.agent_form.destination.as_ref() == Some(&request.destination);
                match result {
                    Ok(receipt) => {
                        let summary = localized_receipt_summary(&receipt, self.language);
                        if selection_matches {
                            self.agent_form.exported = Some(receipt.data);
                            self.agent_form.failure = None;
                        }
                        self.notice = Some((true, summary));
                    }
                    Err(failure) if selection_matches => {
                        self.apply_agent_failure(failure, FocusTarget::AgentExport);
                    }
                    Err(failure) => self.notice = Some((false, failure.message)),
                }
            }
            WorkerEvent::InspectionCatalogLoaded(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.catalog_form.catalog = Some(receipt.data);
                    self.catalog_form.failure = None;
                    self.notice = Some((true, summary));
                    self.pending_focus = Some(FocusTarget::CatalogExport);
                }
                Err(failure) => {
                    self.notice = Some((false, failure.message.clone()));
                    self.catalog_form.failure = Some(failure);
                    self.pending_focus = Some(FocusTarget::CatalogLoad);
                }
            },
            WorkerEvent::ResourceCatalogExported { request, result } => {
                let selection_matches =
                    self.catalog_form.destination.as_ref() == Some(&request.destination);
                match result {
                    Ok(receipt) => {
                        let summary = localized_receipt_summary(&receipt, self.language);
                        if selection_matches {
                            self.catalog_form.exported = Some(receipt.data);
                            self.catalog_form.failure = None;
                        }
                        self.notice = Some((true, summary));
                    }
                    Err(failure) if selection_matches => {
                        self.notice = Some((false, failure.message.clone()));
                        self.catalog_form.failure = Some(failure);
                        self.pending_focus = Some(FocusTarget::CatalogExport);
                    }
                    Err(failure) => self.notice = Some((false, failure.message)),
                }
            }
            WorkerEvent::CliStatusLoaded(result) => match result {
                Ok(receipt) => self.cli_status = Some(receipt.data),
                Err(error) => self.fail(error),
            },
            WorkerEvent::CliInstalled(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.cli_status = Some(receipt.data);
                    self.notice = Some((true, summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::CliUninstalled(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.cli_status = Some(receipt.data);
                    self.notice = Some((true, summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::UpdateCheckFinished(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.update_check = Some(receipt.data);
                    self.notice = Some((true, summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::DoctorFinished(result) => match result {
                Ok(receipt) => {
                    let summary = localized_receipt_summary(&receipt, self.language);
                    self.doctor = Some(receipt.data);
                    self.notice = Some((true, summary));
                }
                Err(error) => self.fail(error),
            },
        }
    }

    fn fail(&mut self, error: String) {
        self.notice = Some((false, error));
    }

    fn apply_task_failure(&mut self, failure: ApplicationFailure, focus: FocusTarget) {
        self.task_form.stale_detected = failure.code == ErrorCode::TaskStale;
        self.notice = Some((false, failure.message.clone()));
        self.task_form.failure = Some(failure);
        self.pending_focus = Some(focus);
    }

    fn apply_agent_failure(&mut self, failure: ApplicationFailure, focus: FocusTarget) {
        self.notice = Some((false, failure.message.clone()));
        self.agent_form.failure = Some(failure);
        self.pending_focus = Some(focus);
    }

    fn reload_selected_job_after_task(&mut self, ctx: egui::Context) {
        if let Some(job_id) = self.selected_job_id.clone() {
            self.load_job(job_id, ctx);
        }
    }

    fn save_registry(&mut self) {
        match self.registry.save(&self.registry_path) {
            Ok(()) => self.registry_error = None,
            Err(error) => self.registry_error = Some(error),
        }
    }

    fn load_workspace(&mut self, path: PathBuf, ctx: egui::Context) {
        self.active_workspace = Some(path.clone());
        self.workspace = None;
        self.health = None;
        self.jobs.clear();
        self.discovery_sources = None;
        self.discovery_leads = None;
        self.selected_discovery_lead = None;
        self.discovery_suggestions = None;
        self.discovery_next_actions.clear();
        self.discovery_import_form = DiscoveryImportForm::default();
        self.discovery_refresh_form = DiscoveryRefreshForm::default();
        self.profile_sources = None;
        self.evidence_review_form = EvidenceReviewForm::default();
        self.criteria_match_form = CriteriaMatchForm::default();
        self.plan_review_form = PlanReviewForm::default();
        self.selected_job = None;
        self.selected_job_id = None;
        self.job_panel = JobPanel::Workflow;
        self.document_form = DocumentWorkspaceForm::default();
        self.review_form = ReviewWorkspaceForm::default();
        self.package_form = PackageWorkspaceForm::default();
        self.render_form = RenderWorkspaceForm::default();
        self.workflow_controls = None;
        self.workflow_action_form = None;
        self.task_form = TaskPanelForm::default();
        self.agent_form = AgentIntegrationForm::default();
        let _ = self.registry.touch(&path);
        self.save_registry();
        self.dispatch(
            self.language.text("Opening workspace"),
            ctx,
            WorkerRequest::LoadWorkspace { path },
        );
    }

    fn refresh_jobs(&mut self, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        let include_archived = self.include_archived;
        self.dispatch(
            self.language.text("Loading jobs"),
            ctx,
            WorkerRequest::LoadJobs {
                path,
                include_archived,
            },
        );
    }

    fn load_discovery_catalog(&mut self, ctx: egui::Context) {
        self.dispatch(
            self.language.text("Loading discovery catalog"),
            ctx,
            WorkerRequest::LoadDiscoveryCatalog,
        );
    }

    fn refresh_discovery_workspace(&mut self, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language.text("Loading discovery workspace"),
            ctx,
            WorkerRequest::LoadDiscoveryWorkspace {
                path,
                include_history: self.discovery_include_history,
            },
        );
    }

    fn apply_discovery_workspace(&mut self, discovery: crate::worker::DiscoveryWorkspaceReadModel) {
        self.selected_discovery_lead = self.selected_discovery_lead.as_ref().and_then(|selected| {
            discovery
                .leads
                .leads
                .iter()
                .find(|lead| lead.id == selected.id)
                .cloned()
        });
        self.discovery_suggestions = None;
        self.discovery_sources = Some(discovery.sources);
        self.discovery_leads = Some(discovery.leads);
    }

    fn refresh_profile_sources(&mut self, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language.text("Loading profile sources"),
            ctx,
            WorkerRequest::LoadProfileSources { path },
        );
    }

    fn load_job(&mut self, id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language.text("Loading job"),
            ctx,
            WorkerRequest::LoadJob { path, id },
        );
    }

    fn load_documents(&mut self, job_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            self.document_form.error = Some(
                self.language
                    .select(
                        "Choose a workspace before loading documents",
                        "加载申请文档前请先选择工作区",
                    )
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language
                .select("Loading current documents", "正在加载当前申请文档"),
            ctx,
            WorkerRequest::LoadDocuments { path, job_id },
        );
    }

    fn load_review(&mut self, job_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            self.review_form.error = Some(
                self.language
                    .select(
                        "Choose a workspace before loading review findings",
                        "加载审阅发现前请先选择工作区",
                    )
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language
                .select("Loading current review", "正在加载当前审阅"),
            ctx,
            WorkerRequest::LoadReview { path, job_id },
        );
    }

    fn confirm_review(
        &mut self,
        job_id: String,
        candidate: canisend_contracts::ReviewDispositionCandidate,
        ctx: egui::Context,
    ) {
        let Some(path) = self.active_workspace.clone() else {
            self.review_form.error = Some(
                self.language
                    .select(
                        "Choose a workspace before confirming review dispositions",
                        "确认审阅处置前请先选择工作区",
                    )
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language
                .select("Confirming review dispositions", "正在确认审阅处置"),
            ctx,
            WorkerRequest::ConfirmReview {
                path,
                job_id,
                candidate,
            },
        );
    }

    fn check_package(&mut self, job_id: String, load_only: bool, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            self.package_form.error = Some(
                self.language
                    .select(
                        "Choose a workspace before checking package readiness",
                        "检查申请包就绪状态前请先选择工作区",
                    )
                    .to_owned(),
            );
            return;
        };
        let request = if load_only {
            WorkerRequest::LoadPackage { path, job_id }
        } else {
            WorkerRequest::CheckPackage { path, job_id }
        };
        self.dispatch(
            if load_only {
                self.language
                    .select("Loading current package", "正在加载当前申请包")
            } else {
                self.language
                    .select("Checking package readiness", "正在检查申请包就绪状态")
            },
            ctx,
            request,
        );
    }

    fn export_package(
        &mut self,
        request: PackageExportRequest,
        private_export_consent: bool,
        ctx: egui::Context,
    ) {
        let Some(path) = self.active_workspace.clone() else {
            self.package_form.error = Some(
                self.language
                    .select(
                        "Choose a workspace before exporting private projections",
                        "导出私密投影前请先选择工作区",
                    )
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language
                .select("Exporting private projections", "正在导出私密投影"),
            ctx,
            WorkerRequest::ExportPackage {
                path,
                request,
                private_export_consent,
            },
        );
    }

    fn load_package_export(&mut self, job_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language
                .select("Loading current export receipt", "正在加载当前导出收据"),
            ctx,
            WorkerRequest::LoadPackageExport { path, job_id },
        );
    }

    fn reconcile_package(&mut self, job_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language
                .select("Reconciling managed projections", "正在对账受管理投影"),
            ctx,
            WorkerRequest::ReconcilePackage { path, job_id },
        );
    }

    fn replace_projection(&mut self, request: ProjectionReplaceRequest, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language
                .select("Restoring managed projection", "正在恢复受管理投影"),
            ctx,
            WorkerRequest::ReplaceProjection { path, request },
        );
    }

    fn copy_projection_as_new(&mut self, request: ProjectionCopyAsNewRequest, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language.select(
                "Preserving edit and restoring projection",
                "正在保留编辑稿并恢复投影",
            ),
            ctx,
            WorkerRequest::CopyProjectionAsNew { path, request },
        );
    }

    fn render_manifest(&mut self, job_id: String, build: bool, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            self.render_form.error = Some(
                self.language
                    .select(
                        "Choose a workspace before building PDFs",
                        "构建 PDF 前请先选择工作区",
                    )
                    .to_owned(),
            );
            return;
        };
        let request = if build {
            WorkerRequest::BuildRender { path, job_id }
        } else {
            WorkerRequest::LoadRender { path, job_id }
        };
        self.dispatch(
            if build {
                self.language
                    .select("Building validated PDFs", "正在构建并验证 PDF")
            } else {
                self.language
                    .select("Loading current render", "正在加载当前渲染")
            },
            ctx,
            request,
        );
    }

    fn export_render(
        &mut self,
        request: RenderExportRequest,
        private_export_consent: bool,
        ctx: egui::Context,
    ) {
        let Some(path) = self.active_workspace.clone() else {
            self.render_form.error = Some(
                self.language
                    .select(
                        "Choose a workspace before exporting PDFs",
                        "导出 PDF 前请先选择工作区",
                    )
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language
                .select("Exporting validated PDFs", "正在导出已验证 PDF"),
            ctx,
            WorkerRequest::ExportRender {
                path,
                request,
                private_export_consent,
            },
        );
    }

    fn load_workflow_controls(&mut self, id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language.text("Loading workflow controls"),
            ctx,
            WorkerRequest::LoadWorkflowControls { path, id },
        );
    }

    fn load_latest_task(&mut self, job_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language
                .select("Loading latest task", "正在加载最新任务"),
            ctx,
            WorkerRequest::LoadLatestTask { path, job_id },
        );
    }

    fn load_agent_integration(&mut self, ctx: egui::Context) {
        self.agent_form.failure = None;
        if self.agent_form.capabilities.is_none() {
            self.dispatch(
                self.language
                    .select("Loading Agent capabilities", "正在加载 Agent 能力"),
                ctx,
                WorkerRequest::LoadAgentCapabilities,
            );
        } else {
            self.load_agent_context(ctx);
        }
    }

    fn load_agent_context(&mut self, ctx: egui::Context) {
        self.agent_form.context = None;
        self.agent_form.failure = None;
        self.dispatch(
            self.language.select(
                "Loading body-free Agent context",
                "正在加载不含正文的 Agent 上下文",
            ),
            ctx,
            WorkerRequest::LoadAgentContext {
                root: self.active_workspace.clone(),
                selected_job_id: self.agent_form.selected_job_id.clone(),
            },
        );
    }
}

fn editable_plan(plan: &ApplicationPlanRecord) -> ApplicationPlanCandidate {
    ApplicationPlanCandidate {
        job_id: plan.job_id.clone(),
        matches_artifact: plan.matches_artifact.clone(),
        decision: plan.decision,
        strategy: plan.strategy.clone(),
        documents: plan
            .documents
            .iter()
            .map(|document| DocumentPlanCandidateRecord {
                kind: document.kind,
                requirement: document.requirement,
                rationale: document.rationale.clone(),
                constraints: document.constraints.clone(),
                executor: document.executor,
            })
            .collect(),
        blockers: plan.blockers.clone(),
    }
}

impl eframe::App for CanISendDesktop {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker(ctx);
        if self.compact {
            let current = ctx.global_style().spacing.item_spacing;
            if current != egui::vec2(6.0, 6.0) {
                theme::apply(ctx, self.dark_mode, true, self.reduce_motion);
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            GUI_PREFERENCES_KEY,
            &GuiPreferences {
                dark_mode: self.dark_mode,
                compact: self.compact,
                reduce_motion: self.reduce_motion,
                language: self.language,
            },
        );
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show_top_bar(ui);
        self.show_status_bar(ui);
        self.show_navigation(ui);
        let panel_fill = theme::panel_background(self.dark_mode);
        let panel = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(panel_fill)
                    .inner_margin(egui::Margin::same(if self.compact { 16 } else { 24 })),
            )
            .show(ui, |ui| {
                self.show_notice(ui);
                egui::ScrollArea::vertical()
                    .id_salt(("main_page", self.page))
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        Page::Overview => self.show_overview(ui),
                        Page::Jobs => self.show_jobs(ui),
                        Page::Discovery => self.show_discovery(ui),
                        Page::Profile => self.show_profile(ui),
                        Page::AgentIntegration => self.show_agent_integration(ui),
                        Page::Workspaces => self.show_workspaces(ui),
                        Page::CommandLine => self.show_command_line(ui),
                        Page::Diagnostics => self.show_diagnostics(ui),
                    });
            });
        set_accesskit_role(
            ui.ctx(),
            panel.response.id,
            egui::accesskit::Role::Main,
            Some(page_accessible_label(self.page, self.language)),
        );
        self.show_job_dialog(ui);
        self.show_import_dialog(ui);
        self.show_profile_source_dialog(ui);
        self.show_workspace_dialog(ui);
        self.show_restore_workspace_dialog(ui);
        self.show_workflow_action_dialog(ui);
        self.show_pending_confirmation(ui);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_app::{Application, PrivateReadConsent};

    use super::{
        GuiPreferences, Language, Page, WorkspaceRegistry, accessible_error, accessible_heading,
        accessible_live_region, localized_receipt_summary, localized_workspace_alias_error,
        page_accessible_label, validate_job_form, validate_profile_source_form,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-gui-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn job_form_requires_both_bounded_labels() {
        assert!(validate_job_form("", "University X", Language::English).is_err());
        assert!(validate_job_form("Lecturer", " ", Language::English).is_err());
        assert!(validate_job_form("Lecturer", "University X", Language::English).is_ok());
        assert!(validate_job_form(&"x".repeat(513), "University X", Language::English).is_err());
        assert_eq!(
            validate_job_form("", "University X", Language::SimplifiedChinese)
                .expect_err("empty Chinese title"),
            "必须填写职位名称"
        );
        assert_eq!(
            localized_workspace_alias_error(
                "Workspace name is required".to_owned(),
                Language::SimplifiedChinese,
            ),
            "必须填写工作区名称"
        );
    }

    #[test]
    fn successful_application_receipts_use_the_active_gui_language() {
        let receipt = Application::doctor().expect("native doctor receipt");
        assert_eq!(
            localized_receipt_summary(&receipt, Language::English),
            receipt.summary
        );
        assert_eq!(
            localized_receipt_summary(&receipt, Language::SimplifiedChinese),
            "原生自检已完成"
        );
        let agent = Application::agent_capabilities().expect("Agent capabilities receipt");
        assert_eq!(
            localized_receipt_summary(&agent, Language::SimplifiedChinese),
            "Agent v2 能力已加载"
        );
    }

    #[test]
    fn accessibility_preferences_are_strict_and_round_trip() {
        let preferences = GuiPreferences {
            dark_mode: true,
            compact: false,
            reduce_motion: true,
            language: Language::SimplifiedChinese,
        };
        let encoded = serde_json::to_string(&preferences).expect("serialize GUI preferences");
        assert_eq!(
            serde_json::from_str::<GuiPreferences>(&encoded).expect("parse GUI preferences"),
            preferences
        );
        assert!(
            serde_json::from_str::<GuiPreferences>(
                r#"{"dark_mode":true,"compact":false,"reduce_motion":true,"unknown":1}"#
            )
            .is_err()
        );
        let migrated: GuiPreferences =
            serde_json::from_str(r#"{"dark_mode":true,"compact":false,"reduce_motion":true}"#)
                .expect("migrate preferences written before localization");
        assert_eq!(migrated.language, Language::English);
        assert_eq!(
            page_accessible_label(Page::Jobs, Language::SimplifiedChinese),
            "职位内容"
        );
        assert_eq!(
            page_accessible_label(Page::Profile, Language::SimplifiedChinese),
            "个人资料内容"
        );
        assert_eq!(
            page_accessible_label(Page::Discovery, Language::SimplifiedChinese),
            "职位发现内容"
        );
        assert_eq!(
            page_accessible_label(Page::AgentIntegration, Language::SimplifiedChinese),
            "Agent 集成内容"
        );
    }

    #[test]
    fn profile_source_form_requires_a_file_and_explicit_read_consent() {
        let source = std::path::Path::new("/tmp/profile.md");
        assert!(validate_profile_source_form(None, true, Language::English).is_err());
        assert!(validate_profile_source_form(Some(source), false, Language::English).is_err());
        assert!(validate_profile_source_form(Some(source), true, Language::English).is_ok());
        assert_eq!(
            validate_profile_source_form(None, true, Language::SimplifiedChinese)
                .expect_err("missing Chinese profile source"),
            "请选择个人资料来源文件"
        );
    }

    #[test]
    fn accesskit_exposes_heading_and_live_region_semantics() {
        let context = eframe::egui::Context::default();
        context.enable_accesskit();
        let mut heading_id = None;
        let mut status_id = None;
        let mut alert_id = None;
        let output = context.run_ui(Default::default(), |ui| {
            heading_id = Some(accessible_heading(ui, "Jobs", 1).id);
            status_id = Some(accessible_live_region(ui, "Completed: saved".to_owned(), true).id);
            alert_id =
                Some(accessible_error(ui, eframe::egui::Color32::RED, "Job title is required").id);
        });
        let update = output
            .platform_output
            .accesskit_update
            .expect("AccessKit update");
        let node = |id: eframe::egui::Id| {
            update
                .nodes
                .iter()
                .find(|(node_id, _)| *node_id == id.accesskit_id())
                .map(|(_, node)| node)
                .expect("semantic node")
        };
        assert_eq!(
            node(heading_id.expect("heading id")).role(),
            eframe::egui::accesskit::Role::Heading
        );
        assert_eq!(node(heading_id.expect("heading id")).level(), Some(1));
        assert_eq!(
            node(status_id.expect("status id")).role(),
            eframe::egui::accesskit::Role::Status
        );
        assert_eq!(
            node(status_id.expect("status id")).live(),
            Some(eframe::egui::accesskit::Live::Polite)
        );
        assert_eq!(
            node(alert_id.expect("alert id")).role(),
            eframe::egui::accesskit::Role::Alert
        );
        assert_eq!(
            node(alert_id.expect("alert id")).live(),
            Some(eframe::egui::accesskit::Live::Assertive)
        );
    }

    #[test]
    fn registry_reopens_the_complete_synthetic_gui_slice() {
        let root = temporary_root("reopen");
        let workspace = root.join("workspace");
        let registry_path = root.join("config/workspaces.json");
        let source = root.join("lecturer.md");
        let sentinel = "SYNTHETIC-PRIVATE-ADVERT-BODY";
        fs::create_dir_all(&root).expect("create GUI fixture root");
        fs::write(
            &source,
            format!(
                "# Lecturer in Economics\n\n{sentinel}\nTeach and publish synthetic research.\n"
            ),
        )
        .expect("write GUI source fixture");

        Application::initialize_workspace(&workspace).expect("initialize GUI workspace");
        let job =
            Application::create_job(&workspace, "Lecturer in Economics", "Synthetic University")
                .expect("create GUI job")
                .data;
        Application::import_local_job_source(
            &workspace,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import GUI source");
        Application::start_workflow(&workspace, job.id.as_str()).expect("start GUI workflow");

        let mut registry = WorkspaceRegistry::default();
        let canonical = registry
            .register("Synthetic applications", &workspace)
            .expect("register GUI workspace");
        registry.save(&registry_path).expect("persist GUI registry");
        drop(registry);

        let reopened = WorkspaceRegistry::load(&registry_path).expect("reopen GUI registry");
        assert_eq!(reopened.default_path.as_ref(), Some(&canonical));
        assert_eq!(reopened.entries.len(), 1);
        let status = Application::workspace_status(&canonical).expect("reopen workspace");
        assert_eq!(status.data.status.job_count, 1);
        let jobs = Application::list_jobs(&canonical, false).expect("reload GUI jobs");
        assert_eq!(jobs.data.jobs.len(), 1);
        let detail =
            Application::job_detail(&canonical, job.id.as_str()).expect("reload GUI job detail");
        assert_eq!(detail.data.sources.len(), 1);
        assert_eq!(
            detail
                .data
                .workflow
                .as_ref()
                .expect("reopened workflow")
                .stages
                .len(),
            10
        );
        assert!(
            !serde_json::to_string(&reopened)
                .expect("serialize GUI registry")
                .contains(sentinel)
        );

        fs::remove_dir_all(root).expect("remove GUI reopen fixture");
    }
}
