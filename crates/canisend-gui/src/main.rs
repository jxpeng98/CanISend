#![forbid(unsafe_code)]

mod cli_bridge;
mod registry;
mod theme;

use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use canisend_app::{
    ActionReceipt, Application, BackupReadModel, DoctorSummary, JobDetailReadModel,
    JobListReadModel, NetworkFetchConsent, PrivateReadConsent, ProductSummary,
    SourceImportReadModel, TerminalInstallConsent, UpdateCheckReadModel, WorkspaceHealthReadModel,
    WorkspaceReadModel,
};
use canisend_app::{CliInstallState, CliInstallStatus, CliVersionRelation};
use canisend_contracts::{JobRecord, StageExecutionStatus, WorkflowStage, WorkflowStatusData};
use cli_bridge::{bundled_cli_path, default_cli_destination};
use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke};
use registry::{WorkspaceRegistry, default_registry_path, validate_workspace_alias};
use serde::{Deserialize, Serialize};

const APP_ID: &str = "io.github.jxpeng98.canisend";
const GUI_PREFERENCES_KEY: &str = "canisend.gui-preferences/v1";

fn main() -> eframe::Result {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Page {
    Overview,
    Jobs,
    Workspaces,
    CommandLine,
    Diagnostics,
}

#[derive(Debug, Clone)]
enum PendingConfirmation {
    ArchiveJob { title: String },
    UninstallCli { restores_previous: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusTarget {
    JobTitle,
    ImportKind,
    WorkspaceAlias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuiPreferences {
    dark_mode: bool,
    compact: bool,
    reduce_motion: bool,
}

impl Page {
    const ALL: [(Self, &'static str); 5] = [
        (Self::Overview, "Overview"),
        (Self::Jobs, "Jobs"),
        (Self::Workspaces, "Workspaces"),
        (Self::CommandLine, "Command line"),
        (Self::Diagnostics, "Diagnostics"),
    ];
}

#[derive(Debug, Default)]
struct JobForm {
    title: String,
    institution: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKind {
    File,
    Url,
}

#[derive(Debug)]
struct ImportForm {
    kind: ImportKind,
    file: Option<PathBuf>,
    url: String,
    network_consent: bool,
    private_read_consent: bool,
    error: Option<String>,
}

impl Default for ImportForm {
    fn default() -> Self {
        Self {
            kind: ImportKind::File,
            file: None,
            url: String::new(),
            network_consent: false,
            private_read_consent: false,
            error: None,
        }
    }
}

#[derive(Debug, Default)]
struct WorkspaceForm {
    alias: String,
    path: Option<PathBuf>,
    create_new: bool,
    error: Option<String>,
}

#[derive(Debug)]
enum WorkerEvent {
    WorkspaceLoaded(Result<ActionReceipt<WorkspaceReadModel>, String>),
    WorkspaceCreated {
        alias: String,
        result: Result<ActionReceipt<WorkspaceReadModel>, String>,
    },
    WorkspaceChecked(Result<ActionReceipt<WorkspaceHealthReadModel>, String>),
    BackupCreated(Result<ActionReceipt<BackupReadModel>, String>),
    JobsLoaded(Result<ActionReceipt<JobListReadModel>, String>),
    JobCreated(Result<ActionReceipt<JobRecord>, String>),
    JobLoaded(Result<ActionReceipt<JobDetailReadModel>, String>),
    JobArchived(Result<ActionReceipt<JobRecord>, String>),
    SourceImported(Result<ActionReceipt<SourceImportReadModel>, String>),
    WorkflowLoaded(Result<ActionReceipt<WorkflowStatusData>, String>),
    CliStatusLoaded(Result<ActionReceipt<CliInstallStatus>, String>),
    CliInstalled(Result<ActionReceipt<CliInstallStatus>, String>),
    CliUninstalled(Result<ActionReceipt<CliInstallStatus>, String>),
    UpdateCheckFinished(Result<ActionReceipt<UpdateCheckReadModel>, String>),
    DoctorFinished(Result<ActionReceipt<DoctorSummary>, String>),
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
    page: Page,
    job_form: JobForm,
    show_job_form: bool,
    import_form: ImportForm,
    show_import_form: bool,
    workspace_form: WorkspaceForm,
    show_workspace_form: bool,
    product: ProductSummary,
    doctor: Option<DoctorSummary>,
    cli_source: Option<PathBuf>,
    cli_destination: PathBuf,
    cli_status: Option<CliInstallStatus>,
    update_check: Option<UpdateCheckReadModel>,
    dark_mode: bool,
    compact: bool,
    reduce_motion: bool,
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
            page: Page::Overview,
            job_form: JobForm::default(),
            show_job_form: false,
            import_form: ImportForm::default(),
            show_import_form: false,
            workspace_form: WorkspaceForm::default(),
            show_workspace_form: false,
            product: Application::product_summary(),
            doctor: None,
            cli_source: bundled_cli_path(),
            cli_destination: default_cli_destination(),
            cli_status: None,
            update_check: None,
            dark_mode,
            compact,
            reduce_motion,
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

    fn dispatch(
        &mut self,
        label: impl Into<String>,
        ctx: egui::Context,
        work: impl FnOnce() -> WorkerEvent + Send + 'static,
    ) {
        if self.activity.is_some() {
            self.notice = Some((
                false,
                "Finish the current operation before starting another one.".to_owned(),
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
            let event = work();
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
                    "The background operation ended unexpectedly. No completion was recorded; \
                     review the current workspace state and try again."
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
                    self.workspace = Some(receipt.data);
                    self.notice = Some((true, receipt.summary));
                    self.refresh_jobs(ctx.clone());
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::WorkspaceCreated { alias, result } => match result {
                Ok(receipt) => {
                    let path = receipt.data.path.clone();
                    match self.registry.register(&alias, &path) {
                        Ok(canonical) => {
                            self.active_workspace = Some(canonical);
                            self.workspace = Some(receipt.data);
                            self.show_workspace_form = false;
                            self.workspace_form = WorkspaceForm::default();
                            self.save_registry();
                            self.notice = Some((true, receipt.summary));
                            self.refresh_jobs(ctx.clone());
                        }
                        Err(error) => self.fail(error),
                    }
                }
                Err(error) => self.workspace_form.error = Some(error),
            },
            WorkerEvent::WorkspaceChecked(result) => match result {
                Ok(receipt) => {
                    self.health = Some(receipt.data);
                    self.notice = Some((true, receipt.summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::BackupCreated(result) => match result {
                Ok(receipt) => self.notice = Some((true, receipt.summary)),
                Err(error) => self.fail(error),
            },
            WorkerEvent::JobsLoaded(result) => match result {
                Ok(receipt) => {
                    self.jobs = receipt.data.jobs;
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::JobCreated(result) => match result {
                Ok(receipt) => {
                    let id = receipt.data.id.to_string();
                    self.show_job_form = false;
                    self.job_form = JobForm::default();
                    self.notice = Some((true, receipt.summary));
                    self.selected_job_id = Some(id.clone());
                    self.page = Page::Jobs;
                    self.load_job(id, ctx.clone());
                }
                Err(error) => self.job_form.error = Some(error),
            },
            WorkerEvent::JobLoaded(result) => match result {
                Ok(receipt) => {
                    self.selected_job_id = Some(receipt.data.job.id.to_string());
                    self.selected_job = Some(receipt.data);
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::JobArchived(result) => match result {
                Ok(receipt) => {
                    self.selected_job = None;
                    self.selected_job_id = None;
                    self.notice = Some((true, receipt.summary));
                    self.refresh_jobs(ctx.clone());
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::SourceImported(result) => match result {
                Ok(receipt) => {
                    let id = receipt.data.job.id.to_string();
                    self.show_import_form = false;
                    self.import_form = ImportForm::default();
                    self.notice = Some((true, receipt.summary));
                    self.load_job(id, ctx.clone());
                }
                Err(error) => self.import_form.error = Some(error),
            },
            WorkerEvent::WorkflowLoaded(result) => match result {
                Ok(receipt) => {
                    if let Some(job) = self.selected_job.as_mut() {
                        job.workflow = Some(receipt.data);
                    }
                    self.notice = Some((true, receipt.summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::CliStatusLoaded(result) => match result {
                Ok(receipt) => self.cli_status = Some(receipt.data),
                Err(error) => self.fail(error),
            },
            WorkerEvent::CliInstalled(result) => match result {
                Ok(receipt) => {
                    self.cli_status = Some(receipt.data);
                    self.notice = Some((true, receipt.summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::CliUninstalled(result) => match result {
                Ok(receipt) => {
                    self.cli_status = Some(receipt.data);
                    self.notice = Some((true, receipt.summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::UpdateCheckFinished(result) => match result {
                Ok(receipt) => {
                    self.update_check = Some(receipt.data);
                    self.notice = Some((true, receipt.summary));
                }
                Err(error) => self.fail(error),
            },
            WorkerEvent::DoctorFinished(result) => match result {
                Ok(receipt) => {
                    self.doctor = Some(receipt.data);
                    self.notice = Some((true, receipt.summary));
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
        self.selected_job = None;
        self.selected_job_id = None;
        let _ = self.registry.touch(&path);
        self.save_registry();
        self.dispatch("Opening workspace", ctx, move || {
            WorkerEvent::WorkspaceLoaded(
                Application::workspace_status(&path).map_err(|error| error.to_string()),
            )
        });
    }

    fn refresh_jobs(&mut self, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        let include_archived = self.include_archived;
        self.dispatch("Loading jobs", ctx, move || {
            WorkerEvent::JobsLoaded(
                Application::list_jobs(&path, include_archived).map_err(|error| error.to_string()),
            )
        });
    }

    fn load_job(&mut self, id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch("Loading job", ctx, move || {
            WorkerEvent::JobLoaded(
                Application::job_detail(&path, &id).map_err(|error| error.to_string()),
            )
        });
    }

    fn show_top_bar(&mut self, ui: &mut egui::Ui) {
        let panel = egui::Panel::top("top_bar")
            .exact_size(58.0)
            .frame(
                egui::Frame::new()
                    .fill(if self.dark_mode {
                        Color32::from_rgb(29, 43, 47)
                    } else {
                        Color32::WHITE
                    })
                    .inner_margin(egui::Margin::symmetric(18, 10))
                    .stroke(Stroke::new(1.0, theme::SLATE_300)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let narrow_header = ui.available_width() < 650.0;
                    ui.label(RichText::new("CanISend").size(22.0).strong().color(
                        if self.dark_mode {
                            theme::TEAL_100
                        } else {
                            theme::TEAL_700
                        },
                    ));
                    ui.add_space(if narrow_header { 10.0 } else { 24.0 });
                    let workspace_label = ui.label("Workspace");
                    let selected = self
                        .active_workspace
                        .as_ref()
                        .and_then(|path| {
                            self.registry
                                .entries
                                .iter()
                                .find(|entry| &entry.path == path)
                        })
                        .map_or("Choose a workspace", |entry| entry.alias.as_str());
                    let mut chosen = None;
                    ui.add_enabled_ui(self.activity.is_none(), |ui| {
                        let combo = egui::ComboBox::from_id_salt("workspace_switcher")
                            .selected_text(selected)
                            .width(if narrow_header { 140.0 } else { 260.0 })
                            .show_ui(ui, |ui| {
                                for entry in &self.registry.entries {
                                    if ui
                                        .selectable_label(
                                            self.active_workspace.as_ref() == Some(&entry.path),
                                            &entry.alias,
                                        )
                                        .on_hover_text(entry.path.display().to_string())
                                        .clicked()
                                    {
                                        chosen = Some(entry.path.clone());
                                    }
                                }
                            });
                        combo.response.labelled_by(workspace_label.id);
                    });
                    if let Some(path) = chosen {
                        self.load_workspace(path, ui.ctx().clone());
                    }
                    if !narrow_header {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let health = if self.active_workspace.is_none() {
                                "No workspace"
                            } else {
                                self.health.as_ref().map_or("Not checked", |health| {
                                    if health.check.ok {
                                        "Healthy"
                                    } else {
                                        "Needs attention"
                                    }
                                })
                            };
                            ui.label(RichText::new(health).color(if self.dark_mode {
                                theme::TEAL_100
                            } else {
                                theme::TEAL_700
                            }));
                            ui.label(RichText::new("Health").weak());
                        });
                    }
                });
            });
        set_accesskit_role(
            ui.ctx(),
            panel.response.id,
            egui::accesskit::Role::Banner,
            Some("CanISend workspace header"),
        );
    }

    fn show_navigation(&mut self, ui: &mut egui::Ui) {
        let panel = egui::Panel::left("navigation")
            .resizable(false)
            .exact_size(180.0)
            .frame(
                egui::Frame::new()
                    .fill(if self.dark_mode {
                        Color32::from_rgb(24, 52, 54)
                    } else {
                        theme::SLATE_100
                    })
                    .inner_margin(egui::Margin::same(12))
                    .stroke(Stroke::new(1.0, theme::SLATE_300)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        let mut refresh_cli = false;
                        for (page, label) in Page::ALL {
                            let selected = self.page == page;
                            let text = RichText::new(label).size(15.0).color(if selected {
                                Color32::WHITE
                            } else {
                                theme::neutral(self.dark_mode)
                            });
                            let button = egui::Button::new(text)
                                .fill(if selected {
                                    theme::TEAL_700
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(if selected {
                                    Stroke::NONE
                                } else {
                                    Stroke::new(1.0, theme::SLATE_300)
                                })
                                .min_size(egui::vec2(154.0, 38.0));
                            let response = ui.add(button);
                            paint_focus_ring(ui, &response);
                            keep_focused_visible(&response);
                            if response.clicked() {
                                self.page = page;
                                refresh_cli = page == Page::CommandLine;
                            }
                        }
                        if refresh_cli {
                            self.refresh_cli_status(ui.ctx().clone());
                        }
                        ui.separator();
                        accessible_heading(ui, "Accessibility & appearance", 2);
                        let dark_response = ui.checkbox(&mut self.dark_mode, "Dark appearance");
                        keep_focused_visible(&dark_response);
                        if dark_response.changed() {
                            theme::apply(
                                ui.ctx(),
                                self.dark_mode,
                                self.compact,
                                self.reduce_motion,
                            );
                        }
                        let compact_response = ui.checkbox(&mut self.compact, "Compact density");
                        keep_focused_visible(&compact_response);
                        if compact_response.changed() {
                            theme::apply(
                                ui.ctx(),
                                self.dark_mode,
                                self.compact,
                                self.reduce_motion,
                            );
                        }
                        let motion_response = ui.checkbox(&mut self.reduce_motion, "Reduce motion");
                        keep_focused_visible(&motion_response);
                        if motion_response.changed() {
                            theme::apply(
                                ui.ctx(),
                                self.dark_mode,
                                self.compact,
                                self.reduce_motion,
                            );
                        }
                        let mut zoom = ui.ctx().zoom_factor();
                        let previous_zoom = zoom;
                        let zoom_combo = egui::ComboBox::from_label("Text size")
                            .selected_text(format!("{:.0}%", zoom * 100.0))
                            .show_ui(ui, |ui| {
                                for (factor, label) in
                                    [(1.0, "100%"), (1.25, "125%"), (1.5, "150%"), (2.0, "200%")]
                                {
                                    if ui.selectable_value(&mut zoom, factor, label).clicked() {
                                        ui.close();
                                    }
                                }
                            });
                        keep_focused_visible(&zoom_combo.response);
                        if zoom != previous_zoom {
                            ui.ctx().set_zoom_factor(zoom);
                        }
                        ui.add_space(8.0);
                    });
            });
        set_accesskit_role(
            ui.ctx(),
            panel.response.id,
            egui::accesskit::Role::Navigation,
            Some("Primary navigation"),
        );
    }

    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        let panel = egui::Panel::bottom("status_bar")
            .exact_size(34.0)
            .frame(
                egui::Frame::new()
                    .fill(if self.dark_mode {
                        Color32::from_rgb(24, 31, 34)
                    } else {
                        Color32::from_rgb(248, 250, 250)
                    })
                    .inner_margin(egui::Margin::symmetric(16, 7))
                    .stroke(Stroke::new(1.0, theme::SLATE_300)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(activity) = &self.activity {
                        ui.spinner();
                        ui.label(&activity.label);
                        ui.label(
                            RichText::new(format!(
                                "{:.1}s",
                                activity.started.elapsed().as_secs_f32()
                            ))
                            .weak(),
                        );
                    } else {
                        ui.label(RichText::new("Local workspace state").weak());
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(format!("v{}", self.product.version));
                    });
                });
            });
        set_accesskit_role(
            ui.ctx(),
            panel.response.id,
            egui::accesskit::Role::Status,
            Some("Application status"),
        );
    }

    fn show_notice(&mut self, ui: &mut egui::Ui) {
        if let Some((success, message)) = self.notice.clone() {
            let fill = if success {
                if self.dark_mode {
                    Color32::from_rgb(26, 72, 65)
                } else {
                    theme::TEAL_100
                }
            } else if self.dark_mode {
                Color32::from_rgb(91, 39, 39)
            } else {
                Color32::from_rgb(254, 226, 226)
            };
            egui::Frame::new()
                .fill(fill)
                .corner_radius(6)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        accessible_live_region(
                            ui,
                            if success {
                                format!("Completed: {message}")
                            } else {
                                format!("Needs attention: {message}")
                            },
                            success,
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Dismiss").clicked() {
                                self.notice = None;
                            }
                        });
                    });
                });
            ui.add_space(10.0);
        }
        if let Some(error) = &self.registry_error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }
    }

    fn show_overview(&mut self, ui: &mut egui::Ui) {
        self.page_header(ui, "Overview", "Current local workspace and next actions");
        let Some(workspace) = &self.workspace else {
            self.empty_workspace(ui);
            return;
        };
        let active_jobs = self
            .jobs
            .iter()
            .filter(|job| !job.archived)
            .count()
            .to_string();
        let artifacts = workspace.status.artifact_count.to_string();
        let health = self.health.as_ref().map_or("Not checked", |health| {
            if health.check.ok { "Healthy" } else { "Issues" }
        });
        if ui.available_width() >= 640.0 {
            ui.columns(3, |columns| {
                metric_card(
                    &mut columns[0],
                    "Active jobs",
                    &active_jobs,
                    "Stored in this workspace",
                );
                metric_card(
                    &mut columns[1],
                    "Artifacts",
                    &artifacts,
                    "Revisioned local records",
                );
                metric_card(
                    &mut columns[2],
                    "Workspace health",
                    health,
                    "Run an integrity check regularly",
                );
            });
        } else {
            metric_card(ui, "Active jobs", &active_jobs, "Stored in this workspace");
            ui.add_space(8.0);
            metric_card(ui, "Artifacts", &artifacts, "Revisioned local records");
            ui.add_space(8.0);
            metric_card(
                ui,
                "Workspace health",
                health,
                "Run an integrity check regularly",
            );
        }
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::next_button("Add job").min_size(egui::vec2(120.0, 36.0)),
                )
                .clicked()
            {
                self.open_job_form();
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new("Check workspace"),
                )
                .clicked()
            {
                self.check_active_workspace(ui.ctx().clone());
            }
            if ui.button("View all jobs").clicked() {
                self.page = Page::Jobs;
            }
        });
        ui.add_space(22.0);
        accessible_heading(ui, "Recently updated jobs", 2);
        ui.separator();
        if self.jobs.is_empty() {
            ui.label("No jobs yet. Add a job from a URL, PDF, Markdown, text, or JSON file.");
        } else {
            let recent = self.jobs.iter().rev().take(5).cloned().collect::<Vec<_>>();
            for job in recent {
                self.job_row(ui, &job);
            }
        }
    }

    fn show_jobs(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            "Jobs",
            "Application records, supplied sources, and workflow state",
        );
        if self.workspace.is_none() {
            self.empty_workspace(ui);
            return;
        }
        if self.selected_job.is_some() {
            self.show_job_detail(ui);
            return;
        }
        ui.horizontal(|ui| {
            let search_label = ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.job_filter)
                    .hint_text("Title or institution")
                    .desired_width(280.0),
            )
            .labelled_by(search_label.id);
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Checkbox::new(&mut self.include_archived, "Include archived"),
                )
                .changed()
            {
                self.refresh_jobs(ui.ctx().clone());
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(self.activity.is_none(), theme::primary_button("Add job"))
                    .clicked()
                {
                    self.open_job_form();
                }
            });
        });
        ui.add_space(12.0);
        ui.separator();
        let filter = self.job_filter.to_lowercase();
        let visible = self
            .jobs
            .iter()
            .filter(|job| {
                filter.is_empty()
                    || job.title.to_lowercase().contains(&filter)
                    || job.institution.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect::<Vec<_>>();
        if visible.is_empty() {
            ui.add_space(28.0);
            ui.label("No jobs match the current filter.");
        }
        for job in visible {
            self.job_row(ui, &job);
        }
    }

    fn job_row(&mut self, ui: &mut egui::Ui, job: &JobRecord) {
        let response = egui::Frame::new()
            .fill(if self.dark_mode {
                Color32::from_rgb(38, 48, 52)
            } else {
                Color32::WHITE
            })
            .stroke(Stroke::new(1.0, theme::SLATE_300))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&job.title).strong().size(16.0));
                        ui.label(&job.institution);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(format!("Revision {}", job.revision.get()));
                        if job.archived {
                            ui.colored_label(theme::neutral(self.dark_mode), "Archived");
                        }
                    });
                });
            })
            .response
            .interact(if self.activity.is_none() {
                Sense::click()
            } else {
                Sense::hover()
            })
            .on_hover_text("Open job");
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                self.activity.is_none(),
                format!("Open {} at {}", job.title, job.institution),
            )
        });
        paint_focus_ring(ui, &response);
        if response.clicked() {
            self.selected_job_id = Some(job.id.to_string());
            self.load_job(job.id.to_string(), ui.ctx().clone());
        }
        ui.add_space(8.0);
    }

    fn show_job_detail(&mut self, ui: &mut egui::Ui) {
        let Some(detail) = self.selected_job.clone() else {
            return;
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.activity.is_none(), egui::Button::new("Back to jobs"))
                .clicked()
            {
                self.selected_job = None;
                self.selected_job_id = None;
                return;
            }
            ui.separator();
            let title = ui.label(RichText::new(&detail.job.title).size(20.0).strong());
            set_accesskit_role(ui.ctx(), title.id, egui::accesskit::Role::Heading, None);
            ui.ctx()
                .accesskit_node_builder(title.id, |node| node.set_level(1));
            ui.label(&detail.job.institution);
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::primary_button("Import source"),
                )
                .clicked()
            {
                self.open_import_form();
            }
            if detail.workflow.is_none()
                && ui
                    .add_enabled(
                        self.activity.is_none() && !detail.sources.is_empty(),
                        theme::next_button("Start workflow"),
                    )
                    .on_disabled_hover_text("Import at least one source first")
                    .clicked()
            {
                self.start_selected_workflow(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.activity.is_none() && !detail.job.archived,
                    theme::destructive_button("Archive"),
                )
                .clicked()
            {
                self.pending_confirmation = Some(PendingConfirmation::ArchiveJob {
                    title: detail.job.title.clone(),
                });
            }
        });
        ui.add_space(18.0);
        ui.columns(2, |columns| {
            accessible_heading(&mut columns[0], "Sources", 2);
            columns[0].separator();
            if detail.sources.is_empty() {
                columns[0].label("No source has been imported.");
            }
            for source in &detail.sources {
                columns[0].label(
                    RichText::new(format!("{:?}", source.kind))
                        .strong()
                        .color(theme::positive(self.dark_mode)),
                );
                columns[0].label(&source.content_type);
                if let Some(url) = &source.final_url {
                    columns[0].label(url);
                }
                columns[0].label(RichText::new(source.retrieved_at.as_str()).weak());
                columns[0].add_space(8.0);
            }
            accessible_heading(&mut columns[1], "Workflow", 2);
            columns[1].separator();
            if let Some(workflow) = &detail.workflow {
                workflow_timeline(&mut columns[1], workflow);
            } else {
                columns[1].label("Workflow has not started.");
                columns[1].label("Import a source, then start the durable stage graph.");
            }
        });
    }

    fn show_workspaces(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            "Workspaces",
            "Local workspace registry, integrity, and backups",
        );
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::primary_button("Create workspace"),
                )
                .clicked()
            {
                self.workspace_form = WorkspaceForm {
                    create_new: true,
                    ..Default::default()
                };
                self.open_workspace_form();
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new("Register existing"),
                )
                .clicked()
            {
                self.workspace_form = WorkspaceForm::default();
                self.open_workspace_form();
            }
            if ui
                .add_enabled(
                    self.active_workspace.is_some() && self.activity.is_none(),
                    egui::Button::new("Check active"),
                )
                .clicked()
            {
                self.check_active_workspace(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.active_workspace.is_some() && self.activity.is_none(),
                    egui::Button::new("Back up active"),
                )
                .clicked()
            {
                self.backup_active_workspace(ui.ctx().clone());
            }
        });
        ui.add_space(18.0);
        if self.registry.entries.is_empty() {
            self.empty_workspace(ui);
            return;
        }
        let entries = self.registry.entries.clone();
        for entry in entries {
            egui::Frame::new()
                .fill(if self.dark_mode {
                    Color32::from_rgb(38, 48, 52)
                } else {
                    Color32::WHITE
                })
                .stroke(Stroke::new(1.0, theme::SLATE_300))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&entry.alias).strong().size(16.0));
                            ui.label(RichText::new(entry.path.display().to_string()).weak());
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    self.activity.is_none(),
                                    theme::destructive_button("Remove from list"),
                                )
                                .on_hover_text("This does not delete workspace data")
                                .clicked()
                            {
                                let removed_active =
                                    self.active_workspace.as_ref() == Some(&entry.path);
                                self.registry.remove(&entry.path);
                                if removed_active {
                                    self.active_workspace = None;
                                    self.workspace = None;
                                    self.jobs.clear();
                                    self.selected_job = None;
                                    self.selected_job_id = None;
                                }
                                self.save_registry();
                                if removed_active
                                    && let Some(next) = self.registry.default_path.clone()
                                {
                                    self.load_workspace(next, ui.ctx().clone());
                                }
                            }
                            if ui
                                .add_enabled(self.activity.is_none(), egui::Button::new("Open"))
                                .clicked()
                            {
                                self.load_workspace(entry.path.clone(), ui.ctx().clone());
                            }
                            if self.active_workspace.as_ref() == Some(&entry.path) {
                                ui.colored_label(theme::positive(self.dark_mode), "Active");
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }
        if let Some(health) = &self.health {
            ui.add_space(12.0);
            accessible_heading(ui, "Latest integrity check", 2);
            ui.label(if health.check.ok {
                "Database and referenced blobs passed verification."
            } else {
                "The workspace needs attention before further mutation."
            });
            for issue in &health.check.issues {
                ui.colored_label(
                    theme::error(self.dark_mode),
                    format!("{}: {}", issue.code, issue.message),
                );
            }
        }
    }

    fn show_diagnostics(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            "Diagnostics",
            "Body-free product and runtime information",
        );
        egui::Grid::new("product_diagnostics")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                diagnostic_row(ui, "Product", &self.product.product);
                diagnostic_row(ui, "Version", &self.product.version);
                diagnostic_row(ui, "Protocol", &self.product.protocol);
                diagnostic_row(ui, "Workspace format", &self.product.workspace_format);
                diagnostic_row(
                    ui,
                    "Target",
                    &format!("{}-{}", self.product.target_arch, self.product.target_os),
                );
                diagnostic_row(
                    ui,
                    "Text size",
                    &format!("{:.0}%", ui.ctx().zoom_factor() * 100.0),
                );
                diagnostic_row(
                    ui,
                    "Display scale",
                    &ui.ctx().native_pixels_per_point().map_or_else(
                        || "Not reported by the window system".to_owned(),
                        |scale| format!("{scale:.2} physical pixels per point"),
                    ),
                );
                diagnostic_row(
                    ui,
                    "Reduced motion",
                    if self.reduce_motion {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                );
            });
        ui.add_space(18.0);
        if ui
            .add_enabled(
                self.activity.is_none(),
                theme::primary_button("Run native self-check"),
            )
            .clicked()
        {
            self.dispatch("Running native self-check", ui.ctx().clone(), || {
                WorkerEvent::DoctorFinished(
                    Application::doctor().map_err(|error| error.to_string()),
                )
            });
        }
        if let Some(doctor) = &self.doctor {
            ui.add_space(16.0);
            accessible_heading(
                ui,
                if doctor.healthy {
                    "Native foundation healthy"
                } else {
                    "Native foundation needs attention"
                },
                2,
            );
            ui.label(format!(
                "{} embedded resources; renderer produced {} page(s) with {} warning(s)",
                doctor.embedded_resources, doctor.rendered_pages, doctor.render_warning_count
            ));
            ui.label("Python runtime: not required");
        }
        ui.add_space(24.0);
        ui.label(
            RichText::new(
                "Diagnostics intentionally omit job adverts, profile evidence, drafts, and provider payloads.",
            )
            .weak(),
        );
    }

    fn show_command_line(&mut self, ui: &mut egui::Ui) {
        self.show_command_line_content(ui);
    }

    fn show_command_line_content(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            "Command line",
            "Keep the terminal CLI aligned with this CanISend desktop release",
        );
        ui.label(
            "GUI actions and CLI commands share the Rust application layer and the same local \
             workspace. This page checks only CanISend product versions. It does not inspect \
             language runtimes or package managers, and upgrading the CLI never migrates workspace data.",
        );
        ui.add_space(16.0);

        let Some(status) = self.cli_status.clone() else {
            if self.activity.is_none() {
                if ui
                    .add(theme::primary_button("Check CLI installation"))
                    .clicked()
                {
                    self.refresh_cli_status(ui.ctx().clone());
                }
            } else {
                ui.spinner();
                ui.label("Checking CLI installation…");
            }
            return;
        };

        let (state_label, state_color) = cli_state_style(&status, self.dark_mode);
        egui::Frame::new()
            .fill(if self.dark_mode {
                Color32::from_rgb(38, 48, 52)
            } else {
                Color32::WHITE
            })
            .stroke(Stroke::new(1.0, theme::SLATE_300))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    accessible_heading(ui, "Terminal installation", 2);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.colored_label(state_color, RichText::new(state_label).strong());
                    });
                });
                ui.add_space(8.0);
                egui::Grid::new("cli_install_status")
                    .num_columns(2)
                    .spacing([24.0, 9.0])
                    .show(ui, |ui| {
                        diagnostic_row(ui, "Bundled version", &status.bundled_version);
                        diagnostic_row(
                            ui,
                            "Installed version",
                            status
                                .installed_version
                                .as_deref()
                                .unwrap_or(if status.installed {
                                    "Unknown (older version interface)"
                                } else {
                                    "Not installed"
                                }),
                        );
                        diagnostic_row(
                            ui,
                            "Bundled CLI",
                            &status
                                .source_path
                                .as_ref()
                                .map_or("Not found".to_owned(), |path| path.display().to_string()),
                        );
                        diagnostic_row(
                            ui,
                            "Install destination",
                            &status.destination.display().to_string(),
                        );
                        diagnostic_row(
                            ui,
                            "Current PATH resolves",
                            &status
                                .active_command
                                .as_ref()
                                .map_or("No canisend command found on PATH".to_owned(), |path| {
                                    path.display().to_string()
                                }),
                        );
                        diagnostic_row(
                            ui,
                            "Destination on current PATH",
                            if status.path_configured { "Yes" } else { "No" },
                        );
                    });
                ui.add_space(12.0);
                self.show_cli_state_guidance(ui, &status);
                ui.add_space(12.0);
                self.show_cli_actions(ui, &status);
            });

        ui.add_space(18.0);
        self.show_product_update_check(ui);
        ui.add_space(18.0);
        accessible_heading(ui, "Use from a terminal or agent host", 2);
        ui.label(
            "Open a new terminal after installation, then verify the native binary before using \
             the same workspace from Codex, Claude, or another local agent.",
        );
        command_copy_row(ui, "canisend version --json");
        command_copy_row(ui, "canisend --help");
        if !status.path_configured {
            ui.add_space(8.0);
            ui.colored_label(
                theme::warning(self.dark_mode),
                "The destination directory is not visible in this app's PATH. Add it to your \
                 shell profile, then open a new terminal.",
            );
            command_copy_row(ui, "export PATH=\"$HOME/.local/bin:$PATH\"");
        }
    }

    fn show_cli_state_guidance(&mut self, ui: &mut egui::Ui, status: &CliInstallStatus) {
        match status.state {
            CliInstallState::NotInstalled => {
                ui.label("No GUI-managed Rust CLI is installed at the destination.");
            }
            CliInstallState::Current if status.active_is_managed => {
                ui.colored_label(
                    theme::positive(self.dark_mode),
                    "The GUI-managed native CLI is the command currently resolved by PATH.",
                );
            }
            CliInstallState::Current => {
                ui.colored_label(
                    theme::warning(self.dark_mode),
                    "The native CLI is installed, but the terminal currently resolves another \
                     CanISend installation first.",
                );
            }
            CliInstallState::UpdateAvailable => {
                ui.label("The CLI installed by this GUI differs from the bundled release.");
            }
            CliInstallState::MigrationAvailable => {
                let guidance = match status.version_relation {
                    CliVersionRelation::Older => format!(
                        "CanISend {} is installed. Upgrade to {} with one click; the current \
                         executable will be preserved for rollback.",
                        status.installed_version.as_deref().unwrap_or(""),
                        status.bundled_version
                    ),
                    CliVersionRelation::Same => format!(
                        "CanISend {} is installed outside this GUI. Adopt the bundled copy to \
                         enable verified updates and rollback.",
                        status.bundled_version
                    ),
                    CliVersionRelation::Unknown => format!(
                        "An older CanISend command interface is installed but does not report a \
                         usable version. Migrate it to {} with one click; the current executable \
                         will be preserved for rollback.",
                        status.bundled_version
                    ),
                    CliVersionRelation::Newer => {
                        "A newer CanISend version is installed.".to_owned()
                    }
                };
                ui.colored_label(theme::warning(self.dark_mode), guidance);
            }
            CliInstallState::NewerInstalled => {
                ui.colored_label(
                    theme::positive(self.dark_mode),
                    format!(
                        "CanISend {} is newer than the bundled {} release. This GUI will not \
                         downgrade it.",
                        status.installed_version.as_deref().unwrap_or("Unknown"),
                        status.bundled_version
                    ),
                );
            }
            CliInstallState::Modified => {
                ui.colored_label(
                    theme::error(self.dark_mode),
                    "The managed binary or installation record changed outside the GUI. Move or \
                     repair it manually before continuing; CanISend will not overwrite it.",
                );
            }
            CliInstallState::SourceUnavailable => {
                ui.colored_label(
                    theme::error(self.dark_mode),
                    "This GUI build does not include a sibling or app-bundled canisend binary. \
                     Build/package both executables before installing.",
                );
            }
        }
        if status.previous_installation_preserved {
            ui.label(
                RichText::new(
                    "A previous installation is preserved and will be restored if you uninstall.",
                )
                .weak(),
            );
        }
    }

    fn show_cli_actions(&mut self, ui: &mut egui::Ui, status: &CliInstallStatus) {
        ui.horizontal(|ui| {
            let install_label = match status.state {
                CliInstallState::UpdateAvailable => "Update CLI",
                CliInstallState::MigrationAvailable
                    if status.version_relation == CliVersionRelation::Older =>
                {
                    "Upgrade installed CLI"
                }
                CliInstallState::MigrationAvailable => "Migrate installed CLI",
                CliInstallState::Current => "Reinstall CLI",
                _ => "Install CLI",
            };
            let can_install = self.activity.is_none()
                && self.cli_source.is_some()
                && !matches!(
                    status.state,
                    CliInstallState::Modified
                        | CliInstallState::NewerInstalled
                        | CliInstallState::SourceUnavailable
                );
            if ui
                .add_enabled(can_install, theme::primary_button(install_label))
                .on_disabled_hover_text(match status.state {
                    CliInstallState::Modified => {
                        "Externally modified managed installations are never overwritten"
                    }
                    CliInstallState::NewerInstalled => {
                        "A newer installed CanISend version is never downgraded"
                    }
                    CliInstallState::SourceUnavailable => {
                        "No bundled Rust CLI is available in this GUI build"
                    }
                    _ => "CLI installation is not currently available",
                })
                .clicked()
            {
                self.install_cli(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.activity.is_none()
                        && status.managed
                        && status.state != CliInstallState::Modified,
                    theme::destructive_button("Uninstall managed CLI"),
                )
                .on_disabled_hover_text("Only an unchanged GUI-managed CLI can be uninstalled")
                .clicked()
            {
                self.pending_confirmation = Some(PendingConfirmation::UninstallCli {
                    restores_previous: status.previous_installation_preserved,
                });
            }
            if ui
                .add_enabled(self.activity.is_none(), egui::Button::new("Refresh"))
                .clicked()
            {
                self.refresh_cli_status(ui.ctx().clone());
            }
        });
    }

    fn show_product_update_check(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(if self.dark_mode {
                Color32::from_rgb(38, 48, 52)
            } else {
                Color32::WHITE
            })
            .stroke(Stroke::new(1.0, theme::SLATE_300))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    accessible_heading(ui, "CanISend updates", 2);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                self.activity.is_none(),
                                egui::Button::new("Check for updates"),
                            )
                            .on_hover_text(
                                "Contact the public CanISend GitHub release endpoint once",
                            )
                            .clicked()
                        {
                            self.check_for_updates(ui.ctx().clone());
                        }
                    });
                });
                ui.label(format!(
                    "Current desktop and bundled CLI version: {}",
                    self.product.version
                ));
                ui.label(
                    RichText::new(
                        "Checks are manual and body-free. No workspace, job, profile, or document \
                         data is sent.",
                    )
                    .weak(),
                );
                if let Some(update) = &self.update_check {
                    ui.add_space(10.0);
                    if update.update_available {
                        ui.colored_label(
                            theme::warning(self.dark_mode),
                            RichText::new(format!(
                                "{} is available on the {} channel.",
                                update.latest_version, update.channel
                            ))
                            .strong(),
                        );
                        ui.label(
                            "Download the newer CanISend desktop release to update both the GUI \
                             and its bundled CLI. This preview does not download or run installers.",
                        );
                    } else {
                        ui.colored_label(
                            theme::positive(self.dark_mode),
                            RichText::new(format!(
                                "Up to date — latest compatible release is {}.",
                                update.latest_version
                            ))
                            .strong(),
                        );
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&update.release_name).weak());
                        if ui.small_button("Copy release link").clicked() {
                            ui.ctx().copy_text(update.release_url.clone());
                        }
                    });
                }
            });
    }

    fn refresh_cli_status(&mut self, ctx: egui::Context) {
        if self.activity.is_some() {
            return;
        }
        let source = self.cli_source.clone();
        let destination = self.cli_destination.clone();
        self.dispatch("Checking CLI installation", ctx, move || {
            WorkerEvent::CliStatusLoaded(
                Application::cli_install_status(source.as_deref(), &destination)
                    .map_err(|error| error.to_string()),
            )
        });
    }

    fn install_cli(&mut self, ctx: egui::Context) {
        let Some(source) = self.cli_source.clone() else {
            self.fail("No bundled CanISend CLI is available".to_owned());
            return;
        };
        let destination = self.cli_destination.clone();
        self.dispatch("Installing or upgrading CanISend CLI", ctx, move || {
            WorkerEvent::CliInstalled(
                Application::install_cli(
                    &source,
                    &destination,
                    true,
                    TerminalInstallConsent::granted_by_user(),
                )
                .map_err(|error| error.to_string()),
            )
        });
    }

    fn check_for_updates(&mut self, ctx: egui::Context) {
        self.dispatch("Checking for CanISend updates", ctx, move || {
            WorkerEvent::UpdateCheckFinished(
                Application::check_for_updates(NetworkFetchConsent::granted_by_user())
                    .map_err(|error| error.to_string()),
            )
        });
    }

    fn uninstall_cli(&mut self, ctx: egui::Context) {
        let source = self.cli_source.clone();
        let destination = self.cli_destination.clone();
        self.dispatch("Uninstalling managed CLI", ctx, move || {
            WorkerEvent::CliUninstalled(
                Application::uninstall_cli(
                    source.as_deref(),
                    &destination,
                    TerminalInstallConsent::granted_by_user(),
                )
                .map_err(|error| error.to_string()),
            )
        });
    }

    fn page_header(&mut self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                accessible_heading(ui, title, 1);
                ui.label(RichText::new(subtitle).weak());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.activity.is_some() {
                    ui.spinner();
                }
            });
        });
        ui.add_space(14.0);
    }

    fn open_job_form(&mut self) {
        self.show_job_form = true;
        self.pending_focus = Some(FocusTarget::JobTitle);
    }

    fn open_import_form(&mut self) {
        self.show_import_form = true;
        self.pending_focus = Some(FocusTarget::ImportKind);
    }

    fn open_workspace_form(&mut self) {
        self.show_workspace_form = true;
        self.pending_focus = Some(FocusTarget::WorkspaceAlias);
    }

    fn empty_workspace(&mut self, ui: &mut egui::Ui) {
        ui.add_space(36.0);
        ui.vertical_centered(|ui| {
            accessible_heading(ui, "Choose a local workspace", 2);
            ui.label("Create a new workspace or register an existing Rust v2 workspace.");
            ui.add_space(12.0);
            if ui
                .add(theme::primary_button("Open workspace manager"))
                .clicked()
            {
                self.page = Page::Workspaces;
            }
        });
    }

    fn check_active_workspace(&mut self, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch("Checking workspace integrity", ctx, move || {
            WorkerEvent::WorkspaceChecked(
                Application::check_workspace(&path).map_err(|error| error.to_string()),
            )
        });
    }

    fn backup_active_workspace(&mut self, ctx: egui::Context) {
        let Some(root) = self.active_workspace.clone() else {
            return;
        };
        let Some(destination) = rfd::FileDialog::new()
            .set_title("Choose a new backup directory")
            .pick_folder()
        else {
            return;
        };
        self.dispatch("Creating verified backup", ctx, move || {
            WorkerEvent::BackupCreated(
                Application::backup_workspace(&root, &destination)
                    .map_err(|error| error.to_string()),
            )
        });
    }

    fn start_selected_workflow(&mut self, ctx: egui::Context) {
        let (Some(path), Some(id)) = (self.active_workspace.clone(), self.selected_job_id.clone())
        else {
            return;
        };
        self.dispatch("Starting workflow", ctx, move || {
            WorkerEvent::WorkflowLoaded(
                Application::start_workflow(&path, &id).map_err(|error| error.to_string()),
            )
        });
    }

    fn archive_selected_job(&mut self, ctx: egui::Context) {
        let (Some(path), Some(id)) = (self.active_workspace.clone(), self.selected_job_id.clone())
        else {
            return;
        };
        self.dispatch("Archiving job", ctx, move || {
            WorkerEvent::JobArchived(
                Application::archive_job(&path, &id).map_err(|error| error.to_string()),
            )
        });
    }

    fn show_job_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.show_job_form {
            return;
        }
        let mut open = self.show_job_form;
        egui::Window::new("Add job")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .show(ui.ctx(), |ui| {
                let title_label = ui.label("Title");
                let title_response = ui
                    .add_enabled(
                        self.activity.is_none(),
                        egui::TextEdit::singleline(&mut self.job_form.title)
                            .hint_text("Lecturer in Economics")
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(title_label.id);
                if self.pending_focus == Some(FocusTarget::JobTitle) {
                    title_response.request_focus();
                    self.pending_focus = None;
                }
                if title_response.changed() {
                    self.job_form.error = None;
                }
                let institution_label = ui.label("Institution");
                if ui
                    .add_enabled(
                        self.activity.is_none(),
                        egui::TextEdit::singleline(&mut self.job_form.institution)
                            .hint_text("University X")
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(institution_label.id)
                    .changed()
                {
                    self.job_form.error = None;
                }
                if let Some(error) = &self.job_form.error {
                    accessible_error(ui, theme::error(self.dark_mode), error);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.activity.is_none(), theme::primary_button("Create job"))
                        .clicked()
                    {
                        match validate_job_form(&self.job_form.title, &self.job_form.institution) {
                            Ok(()) => {
                                if let Some(path) = self.active_workspace.clone() {
                                    let title = self.job_form.title.trim().to_owned();
                                    let institution = self.job_form.institution.trim().to_owned();
                                    self.dispatch("Creating job", ui.ctx().clone(), move || {
                                        WorkerEvent::JobCreated(
                                            Application::create_job(&path, &title, &institution)
                                                .map_err(|error| error.to_string()),
                                        )
                                    });
                                }
                            }
                            Err(error) => self.job_form.error = Some(error),
                        }
                    }
                    if ui
                        .add_enabled(self.activity.is_none(), egui::Button::new("Cancel"))
                        .clicked()
                    {
                        self.show_job_form = false;
                    }
                });
            });
        if self.activity.is_none() && ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            self.show_job_form = false;
        }
        self.show_job_form = self.activity.is_some() || (open && self.show_job_form);
    }

    fn show_import_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.show_import_form {
            return;
        }
        let mut open = self.show_import_form;
        egui::Window::new("Import job source")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ui.ctx(), |ui| {
                ui.add_enabled_ui(self.activity.is_none(), |ui| {
                    let file_response = ui.selectable_value(
                        &mut self.import_form.kind,
                        ImportKind::File,
                        "Local file",
                    );
                    if self.pending_focus == Some(FocusTarget::ImportKind) {
                        file_response.request_focus();
                        self.pending_focus = None;
                    }
                    let url_response = ui.selectable_value(
                        &mut self.import_form.kind,
                        ImportKind::Url,
                        "Public URL",
                    );
                    let file_changed = file_response.changed();
                    let url_changed = url_response.changed();
                    if file_changed || url_changed {
                        self.import_form.error = None;
                    }
                });
                ui.separator();
                match self.import_form.kind {
                    ImportKind::File => {
                        ui.label("Supported: Markdown, text, JSON, and text-based PDF.");
                        ui.horizontal(|ui| {
                            let path = self
                                .import_form
                                .file
                                .as_ref()
                                .map_or("No file selected".to_owned(), |path| {
                                    path.display().to_string()
                                });
                            ui.label(path);
                            if ui
                                .add_enabled(
                                    self.activity.is_none(),
                                    egui::Button::new("Choose file"),
                                )
                                .clicked()
                            {
                                self.import_form.file = rfd::FileDialog::new()
                                    .add_filter("Job sources", &["md", "txt", "json", "pdf"])
                                    .pick_file();
                                self.import_form.error = None;
                            }
                        });
                        if ui
                            .add_enabled(
                                self.activity.is_none(),
                                egui::Checkbox::new(
                                    &mut self.import_form.private_read_consent,
                                    "Allow CanISend to read and store this private local source",
                                ),
                            )
                            .changed()
                        {
                            self.import_form.error = None;
                        }
                    }
                    ImportKind::Url => {
                        ui.label("CanISend will fetch this user-supplied public HTTP(S) URL.");
                        let url_label = ui.label("Job source URL");
                        if ui
                            .add_enabled(
                                self.activity.is_none(),
                                egui::TextEdit::singleline(&mut self.import_form.url)
                                    .hint_text("https://jobs.example.edu/vacancy/123")
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(url_label.id)
                            .changed()
                        {
                            self.import_form.error = None;
                        }
                        if ui
                            .add_enabled(
                                self.activity.is_none(),
                                egui::Checkbox::new(
                                    &mut self.import_form.network_consent,
                                    "Allow this user-invoked network fetch",
                                ),
                            )
                            .changed()
                        {
                            self.import_form.error = None;
                        }
                    }
                }
                if let Some(error) = &self.import_form.error {
                    accessible_error(ui, theme::error(self.dark_mode), error);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.activity.is_none(), theme::primary_button("Import"))
                        .clicked()
                    {
                        self.import_selected_source(ui.ctx().clone());
                    }
                    if ui
                        .add_enabled(self.activity.is_none(), egui::Button::new("Cancel"))
                        .clicked()
                    {
                        self.show_import_form = false;
                    }
                });
            });
        if self.activity.is_none() && ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            self.show_import_form = false;
        }
        self.show_import_form = self.activity.is_some() || (open && self.show_import_form);
    }

    fn import_selected_source(&mut self, ctx: egui::Context) {
        let (Some(root), Some(job_id)) =
            (self.active_workspace.clone(), self.selected_job_id.clone())
        else {
            self.import_form.error = Some("No active job is selected".to_owned());
            return;
        };
        match self.import_form.kind {
            ImportKind::File => {
                if !self.import_form.private_read_consent {
                    self.import_form.error =
                        Some("Confirm private local source access before importing".to_owned());
                    return;
                }
                let Some(file) = self.import_form.file.clone() else {
                    self.import_form.error = Some("Choose a source file".to_owned());
                    return;
                };
                self.dispatch("Importing local source", ctx, move || {
                    WorkerEvent::SourceImported(
                        Application::import_local_job_source(
                            &root,
                            &job_id,
                            &file,
                            PrivateReadConsent::granted_by_user(),
                        )
                        .map_err(|error| error.to_string()),
                    )
                });
            }
            ImportKind::Url => {
                if !self.import_form.network_consent {
                    self.import_form.error =
                        Some("Confirm the user-invoked network fetch before importing".to_owned());
                    return;
                }
                let url = self.import_form.url.trim().to_owned();
                if url.is_empty() {
                    self.import_form.error = Some("Enter a public HTTP(S) URL".to_owned());
                    return;
                }
                self.dispatch("Fetching and importing URL", ctx, move || {
                    WorkerEvent::SourceImported(
                        Application::import_url_job_source(
                            &root,
                            &job_id,
                            &url,
                            NetworkFetchConsent::granted_by_user(),
                        )
                        .map_err(|error| error.to_string()),
                    )
                });
            }
        }
    }

    fn show_workspace_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.show_workspace_form {
            return;
        }
        let mut open = self.show_workspace_form;
        let title = if self.workspace_form.create_new {
            "Create workspace"
        } else {
            "Register existing workspace"
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ui.ctx(), |ui| {
                let alias_label = ui.label("Workspace name");
                let alias_response = ui
                    .add_enabled(
                        self.activity.is_none(),
                        egui::TextEdit::singleline(&mut self.workspace_form.alias)
                            .hint_text("Academic applications")
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(alias_label.id);
                if self.pending_focus == Some(FocusTarget::WorkspaceAlias) {
                    alias_response.request_focus();
                    self.pending_focus = None;
                }
                if alias_response.changed() {
                    self.workspace_form.error = None;
                }
                ui.label(if self.workspace_form.create_new {
                    "Choose a new or empty directory."
                } else {
                    "Choose a directory containing canisend.toml."
                });
                ui.horizontal(|ui| {
                    ui.label(
                        self.workspace_form
                            .path
                            .as_ref()
                            .map_or("No directory selected".to_owned(), |path| {
                                path.display().to_string()
                            }),
                    );
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new("Choose directory"),
                        )
                        .clicked()
                    {
                        self.workspace_form.path = rfd::FileDialog::new().pick_folder();
                        self.workspace_form.error = None;
                    }
                });
                if let Some(error) = &self.workspace_form.error {
                    accessible_error(ui, theme::error(self.dark_mode), error);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let label = if self.workspace_form.create_new {
                        "Create"
                    } else {
                        "Register"
                    };
                    if ui
                        .add_enabled(self.activity.is_none(), theme::primary_button(label))
                        .clicked()
                    {
                        self.submit_workspace_form(ui.ctx().clone());
                    }
                    if ui
                        .add_enabled(self.activity.is_none(), egui::Button::new("Cancel"))
                        .clicked()
                    {
                        self.show_workspace_form = false;
                    }
                });
            });
        if self.activity.is_none() && ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            self.show_workspace_form = false;
        }
        self.show_workspace_form = self.activity.is_some() || (open && self.show_workspace_form);
    }

    fn show_pending_confirmation(&mut self, ui: &mut egui::Ui) {
        let Some(pending) = self.pending_confirmation.clone() else {
            return;
        };
        let mut confirmed = false;
        let mut cancelled = false;
        let modal = egui::Modal::new(egui::Id::new("pending_confirmation")).show(ui.ctx(), |ui| {
            ui.set_width(420.0);
            match &pending {
                PendingConfirmation::ArchiveJob { title } => {
                    accessible_heading(ui, "Archive this job?", 1);
                    ui.label(
                        "The job will leave the active list. Its workspace records are retained, \
                             but this GUI preview does not yet provide an unarchive action.",
                    );
                    ui.label(RichText::new(title).strong());
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(theme::destructive_button("Confirm archive"))
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                }
                PendingConfirmation::UninstallCli { restores_previous } => {
                    accessible_heading(ui, "Uninstall the managed CLI?", 1);
                    ui.label(if *restores_previous {
                        "CanISend will verify the managed binary, remove it, and restore the \
                             previous installation. Workspace data is never removed."
                    } else {
                        "CanISend will verify and remove only the unchanged GUI-managed CLI. \
                             Workspace data is never removed."
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.add(theme::destructive_button("Uninstall CLI")).clicked() {
                            confirmed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                }
            }
        });
        if confirmed {
            self.pending_confirmation = None;
            match pending {
                PendingConfirmation::ArchiveJob { .. } => {
                    self.archive_selected_job(ui.ctx().clone());
                }
                PendingConfirmation::UninstallCli { .. } => {
                    self.uninstall_cli(ui.ctx().clone());
                }
            }
        } else if cancelled || modal.should_close() {
            self.pending_confirmation = None;
        }
    }

    fn submit_workspace_form(&mut self, ctx: egui::Context) {
        let alias = self.workspace_form.alias.trim().to_owned();
        if let Err(error) = validate_workspace_alias(&alias) {
            self.workspace_form.error = Some(error);
            return;
        }
        let Some(path) = self.workspace_form.path.clone() else {
            self.workspace_form.error = Some("Choose a directory".to_owned());
            return;
        };
        if self.workspace_form.create_new {
            self.dispatch("Creating workspace", ctx, move || {
                WorkerEvent::WorkspaceCreated {
                    alias,
                    result: Application::initialize_workspace(&path)
                        .map_err(|error| error.to_string()),
                }
            });
        } else {
            match self.registry.register(&alias, &path) {
                Ok(canonical) => {
                    self.active_workspace = Some(canonical.clone());
                    self.show_workspace_form = false;
                    self.workspace_form = WorkspaceForm::default();
                    self.save_registry();
                    self.load_workspace(canonical, ctx);
                }
                Err(error) => self.workspace_form.error = Some(error),
            }
        }
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
                        Page::Workspaces => self.show_workspaces(ui),
                        Page::CommandLine => self.show_command_line(ui),
                        Page::Diagnostics => self.show_diagnostics(ui),
                    });
            });
        set_accesskit_role(
            ui.ctx(),
            panel.response.id,
            egui::accesskit::Role::Main,
            Some(page_accessible_label(self.page)),
        );
        self.show_job_dialog(ui);
        self.show_import_dialog(ui);
        self.show_workspace_dialog(ui);
        self.show_pending_confirmation(ui);
    }
}

fn set_accesskit_role(
    ctx: &egui::Context,
    id: egui::Id,
    role: egui::accesskit::Role,
    label: Option<&str>,
) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(role);
        if let Some(label) = label {
            node.set_label(label);
        }
    });
}

fn accessible_heading(ui: &mut egui::Ui, text: &str, level: usize) -> egui::Response {
    let response = ui.heading(text);
    set_accesskit_role(ui.ctx(), response.id, egui::accesskit::Role::Heading, None);
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_level(level));
    response
}

fn accessible_live_region(ui: &mut egui::Ui, text: String, polite: bool) -> egui::Response {
    let response = ui.label(text);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(if polite {
            egui::accesskit::Role::Status
        } else {
            egui::accesskit::Role::Alert
        });
        node.set_live(if polite {
            egui::accesskit::Live::Polite
        } else {
            egui::accesskit::Live::Assertive
        });
    });
    response
}

fn accessible_error(ui: &mut egui::Ui, color: Color32, text: &str) -> egui::Response {
    let response = ui.colored_label(color, text);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Alert);
        node.set_live(egui::accesskit::Live::Assertive);
    });
    response
}

fn paint_focus_ring(ui: &egui::Ui, response: &egui::Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            6,
            Stroke::new(2.0, theme::AMBER_600),
            egui::StrokeKind::Inside,
        );
    }
}

fn keep_focused_visible(response: &egui::Response) {
    if response.has_focus() {
        response.scroll_to_me(Some(Align::Center));
    }
}

fn page_accessible_label(page: Page) -> &'static str {
    match page {
        Page::Overview => "Overview content",
        Page::Jobs => "Jobs content",
        Page::Workspaces => "Workspaces content",
        Page::CommandLine => "Command line content",
        Page::Diagnostics => "Diagnostics content",
    }
}

fn validate_job_form(title: &str, institution: &str) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("Job title is required".to_owned());
    }
    if institution.trim().is_empty() {
        return Err("Institution is required".to_owned());
    }
    if title.len() > 512 || institution.len() > 512 {
        return Err("Title and institution must each be at most 512 bytes".to_owned());
    }
    Ok(())
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: &str, help: &str) {
    egui::Frame::new()
        .fill(if ui.visuals().dark_mode {
            Color32::from_rgb(38, 48, 52)
        } else {
            Color32::WHITE
        })
        .stroke(Stroke::new(1.0, theme::SLATE_300))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(RichText::new(label).weak());
            ui.label(RichText::new(value).size(24.0).strong());
            ui.label(RichText::new(help).small().weak());
        });
}

fn workflow_timeline(ui: &mut egui::Ui, workflow: &WorkflowStatusData) {
    for state in &workflow.stages {
        ui.horizontal(|ui| {
            let (color, label) = stage_status_style(state.status, ui.visuals().dark_mode);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, color);
            ui.label(RichText::new(stage_label(state.stage)).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.colored_label(color, label);
            });
        });
        ui.add_space(5.0);
    }
    if !workflow.blockers.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("Blockers").strong());
        for blocker in &workflow.blockers {
            ui.label(format!("{}: {}", blocker.code, blocker.description));
        }
    }
}

fn stage_status_style(status: StageExecutionStatus, dark: bool) -> (Color32, &'static str) {
    match status {
        StageExecutionStatus::Complete => (theme::positive(dark), "Complete"),
        StageExecutionStatus::Ready => (theme::warning(dark), "Ready"),
        StageExecutionStatus::Running => (theme::info(dark), "Running"),
        StageExecutionStatus::AwaitingUser => (theme::warning(dark), "Awaiting user"),
        StageExecutionStatus::Blocked => (theme::neutral(dark), "Blocked"),
        StageExecutionStatus::Stale => (theme::error(dark), "Stale"),
    }
}

fn stage_label(stage: WorkflowStage) -> &'static str {
    match stage {
        WorkflowStage::Intake => "Intake",
        WorkflowStage::Parse => "Parse",
        WorkflowStage::Criteria => "Criteria",
        WorkflowStage::Evidence => "Evidence",
        WorkflowStage::Match => "Match",
        WorkflowStage::Plan => "Plan",
        WorkflowStage::Draft => "Draft",
        WorkflowStage::Review => "Review",
        WorkflowStage::Package => "Package",
        WorkflowStage::Render => "Render",
    }
}

fn diagnostic_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).strong());
    ui.add(egui::Label::new(value).truncate())
        .on_hover_text(value);
    ui.end_row();
}

fn cli_state_style(status: &CliInstallStatus, dark: bool) -> (&'static str, Color32) {
    match status.state {
        CliInstallState::NotInstalled => ("Not installed", theme::neutral(dark)),
        CliInstallState::Current if status.active_is_managed => ("Ready", theme::positive(dark)),
        CliInstallState::Current => ("Installed; not active", theme::warning(dark)),
        CliInstallState::UpdateAvailable => ("Update available", theme::warning(dark)),
        CliInstallState::MigrationAvailable => ("Migration available", theme::warning(dark)),
        CliInstallState::NewerInstalled => ("Newer version installed", theme::positive(dark)),
        CliInstallState::Modified => ("Needs attention", theme::error(dark)),
        CliInstallState::SourceUnavailable => ("CLI missing from package", theme::error(dark)),
    }
}

fn command_copy_row(ui: &mut egui::Ui, command: &str) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.monospace(command);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Copy").clicked() {
                        ui.ctx().copy_text(command.to_owned());
                    }
                });
            });
        });
    ui.add_space(6.0);
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_app::{Application, PrivateReadConsent};

    use super::{
        GuiPreferences, WorkspaceRegistry, accessible_error, accessible_heading,
        accessible_live_region, validate_job_form,
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
        assert!(validate_job_form("", "University X").is_err());
        assert!(validate_job_form("Lecturer", " ").is_err());
        assert!(validate_job_form("Lecturer", "University X").is_ok());
        assert!(validate_job_form(&"x".repeat(513), "University X").is_err());
    }

    #[test]
    fn accessibility_preferences_are_strict_and_round_trip() {
        let preferences = GuiPreferences {
            dark_mode: true,
            compact: false,
            reduce_motion: true,
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
