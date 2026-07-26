mod dialogs;
mod pages;
mod plan_page;

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
        CriteriaMatchForm, EvidenceReviewForm, FocusTarget, GuiPreferences, ImportForm, ImportKind,
        JobForm, Page, PendingConfirmation, PlanReviewForm, ProfileSourceForm,
        RestoreWorkspaceForm, WorkflowActionForm, WorkspaceForm, parse_workflow_artifact_id,
    },
    theme,
    worker::{WorkerEvent, WorkerRequest, execute},
};
use canisend_app::{
    Application, DoctorSummary, JobDetailReadModel, ProductSummary, ProfileSourceListReadModel,
    UpdateCheckReadModel, WorkflowBeginRequest, WorkflowCompleteRequest, WorkflowControlReadModel,
    WorkflowRerunRequest, WorkspaceHealthReadModel, WorkspaceReadModel,
};
use canisend_app::{CliInstallState, CliInstallStatus, CliVersionRelation};
use canisend_contracts::{
    ApplicationDecision, ApplicationPlanCandidate, ApplicationPlanRecord, ArtifactKind,
    CriterionImportance, DocumentKind, DocumentPlanCandidateRecord, DocumentRequirement, EntityId,
    EvidenceKind, ExecutionMode, JobRecord, MatchStrength, PlanBlockerSeverity,
    PrivacyClassification, ProfileSourceKind, WorkflowStage,
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

#[cfg(not(target_os = "macos"))]
fn pick_profile_source_file() -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "macos"))]
fn pick_job_source_file() -> Option<PathBuf> {
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
    selected_job: Option<JobDetailReadModel>,
    selected_job_id: Option<String>,
    profile_sources: Option<ProfileSourceListReadModel>,
    workflow_controls: Option<WorkflowControlReadModel>,
    workflow_action_form: Option<WorkflowActionForm>,
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
            selected_job: None,
            selected_job_id: None,
            profile_sources: None,
            workflow_controls: None,
            workflow_action_form: None,
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
                    }
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
                    self.selected_job = Some(receipt.data);
                    if has_workflow {
                        self.load_workflow_controls(job_id, ctx.clone());
                    } else {
                        self.workflow_controls = None;
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
                    self.notice = Some((true, summary));
                    if self.selected_job_id.as_deref() == Some(job_id.as_str()) {
                        self.load_job(job_id, ctx.clone());
                    }
                }
                Err(error) => self.plan_review_form.error = Some(error),
            },
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
                Ok(receipt) => self.workflow_controls = Some(receipt.data),
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
                    self.notice = Some((true, summary));
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
        self.profile_sources = None;
        self.evidence_review_form = EvidenceReviewForm::default();
        self.criteria_match_form = CriteriaMatchForm::default();
        self.plan_review_form = PlanReviewForm::default();
        self.selected_job = None;
        self.selected_job_id = None;
        self.workflow_controls = None;
        self.workflow_action_form = None;
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
                        Page::Profile => self.show_profile(ui),
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
