#![forbid(unsafe_code)]

mod cli_bridge;
mod registry;
mod theme;

use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
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
use registry::{WorkspaceRegistry, default_registry_path};

const APP_ID: &str = "io.github.jxpeng98.canisend";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Overview,
    Jobs,
    Workspaces,
    CommandLine,
    Diagnostics,
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
    activity: Option<Activity>,
    receiver: Option<Receiver<WorkerEvent>>,
    notice: Option<(bool, String)>,
    registry_error: Option<String>,
}

impl CanISendDesktop {
    fn new(creation: &eframe::CreationContext<'_>) -> Self {
        let registry_path = default_registry_path();
        let (registry, registry_error) = match WorkspaceRegistry::load(&registry_path) {
            Ok(registry) => (registry, None),
            Err(error) => (WorkspaceRegistry::default(), Some(error)),
        };
        let active_workspace = registry.default_path.clone();
        let dark_mode = creation.egui_ctx.theme() == egui::Theme::Dark;
        theme::apply(&creation.egui_ctx, dark_mode, false);
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
            compact: false,
            activity: None,
            receiver: None,
            notice: None,
            registry_error,
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
        let event = self
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(event) = event {
            self.receiver = None;
            self.activity = None;
            self.apply_worker_event(event, ctx);
        } else if self.activity.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
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
                    self.notice = None;
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
        if let Err(error) = self.registry.save(&self.registry_path) {
            self.registry_error = Some(error);
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
        egui::Panel::top("top_bar")
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
                    ui.label(RichText::new("CanISend").size(22.0).strong().color(
                        if self.dark_mode {
                            theme::TEAL_100
                        } else {
                            theme::TEAL_700
                        },
                    ));
                    ui.add_space(24.0);
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
                    egui::ComboBox::from_id_salt("workspace_switcher")
                        .selected_text(selected)
                        .width(260.0)
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
                    if let Some(path) = chosen {
                        self.load_workspace(path, ui.ctx().clone());
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let health = self.health.as_ref().map_or("Not checked", |health| {
                            if health.check.ok {
                                "Healthy"
                            } else {
                                "Needs attention"
                            }
                        });
                        ui.label(RichText::new(health).color(if self.dark_mode {
                            theme::TEAL_100
                        } else {
                            theme::TEAL_700
                        }));
                        ui.label(RichText::new("Workspace").weak());
                    });
                });
            });
    }

    fn show_navigation(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("navigation")
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
                ui.add_space(8.0);
                let mut refresh_cli = false;
                for (page, label) in Page::ALL {
                    let selected = self.page == page;
                    let button = egui::Button::new(RichText::new(label).size(15.0))
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
                    if ui.add(button).clicked() {
                        self.page = page;
                        refresh_cli = page == Page::CommandLine;
                    }
                }
                if refresh_cli {
                    self.refresh_cli_status(ui.ctx().clone());
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.separator();
                    if ui.checkbox(&mut self.compact, "Compact density").changed() {
                        theme::apply(ui.ctx(), self.dark_mode, self.compact);
                    }
                    if ui
                        .checkbox(&mut self.dark_mode, "Dark appearance")
                        .changed()
                    {
                        theme::apply(ui.ctx(), self.dark_mode, self.compact);
                    }
                });
            });
    }

    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status_bar")
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
                        ui.label(if success {
                            "Completed:"
                        } else {
                            "Needs attention:"
                        });
                        ui.label(&message);
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
            ui.colored_label(theme::RED_600, error);
        }
    }

    fn show_overview(&mut self, ui: &mut egui::Ui) {
        self.page_header(ui, "Overview", "Current local workspace and next actions");
        let Some(workspace) = &self.workspace else {
            self.empty_workspace(ui);
            return;
        };
        ui.columns(3, |columns| {
            metric_card(
                &mut columns[0],
                "Active jobs",
                &self
                    .jobs
                    .iter()
                    .filter(|job| !job.archived)
                    .count()
                    .to_string(),
                "Stored in this workspace",
            );
            metric_card(
                &mut columns[1],
                "Artifacts",
                &workspace.status.artifact_count.to_string(),
                "Revisioned local records",
            );
            metric_card(
                &mut columns[2],
                "Workspace health",
                self.health.as_ref().map_or("Not checked", |health| {
                    if health.check.ok { "Healthy" } else { "Issues" }
                }),
                "Run an integrity check regularly",
            );
        });
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::next_button("Add job").min_size(egui::vec2(120.0, 36.0)),
                )
                .clicked()
            {
                self.show_job_form = true;
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
        ui.heading("Recently updated jobs");
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
            ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.job_filter)
                    .hint_text("Title or institution")
                    .desired_width(280.0),
            );
            if ui
                .checkbox(&mut self.include_archived, "Include archived")
                .changed()
            {
                self.refresh_jobs(ui.ctx().clone());
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(self.activity.is_none(), theme::primary_button("Add job"))
                    .clicked()
                {
                    self.show_job_form = true;
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
        egui::ScrollArea::vertical().show(ui, |ui| {
            for job in visible {
                self.job_row(ui, &job);
            }
        });
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
                            ui.colored_label(theme::SLATE_700, "Archived");
                        }
                    });
                });
            })
            .response
            .interact(Sense::click())
            .on_hover_text("Open job");
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
            if ui.button("Back to jobs").clicked() {
                self.selected_job = None;
                self.selected_job_id = None;
                return;
            }
            ui.separator();
            ui.label(RichText::new(&detail.job.title).size(20.0).strong());
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
                self.show_import_form = true;
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
                self.archive_selected_job(ui.ctx().clone());
            }
        });
        ui.add_space(18.0);
        ui.columns(2, |columns| {
            columns[0].heading("Sources");
            columns[0].separator();
            if detail.sources.is_empty() {
                columns[0].label("No source has been imported.");
            }
            for source in &detail.sources {
                columns[0].label(
                    RichText::new(format!("{:?}", source.kind))
                        .strong()
                        .color(theme::TEAL_600),
                );
                columns[0].label(&source.content_type);
                if let Some(url) = &source.final_url {
                    columns[0].label(url);
                }
                columns[0].label(RichText::new(source.retrieved_at.as_str()).weak());
                columns[0].add_space(8.0);
            }
            columns[1].heading("Workflow");
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
                self.show_workspace_form = true;
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new("Register existing"),
                )
                .clicked()
            {
                self.workspace_form = WorkspaceForm::default();
                self.show_workspace_form = true;
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
                                .add(theme::destructive_button("Remove from list"))
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
                            if ui.button("Open").clicked() {
                                self.load_workspace(entry.path.clone(), ui.ctx().clone());
                            }
                            if self.active_workspace.as_ref() == Some(&entry.path) {
                                ui.colored_label(theme::TEAL_600, "Active");
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }
        if let Some(health) = &self.health {
            ui.add_space(12.0);
            ui.heading("Latest integrity check");
            ui.label(if health.check.ok {
                "Database and referenced blobs passed verification."
            } else {
                "The workspace needs attention before further mutation."
            });
            for issue in &health.check.issues {
                ui.colored_label(theme::RED_600, format!("{}: {}", issue.code, issue.message));
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
            ui.heading(if doctor.healthy {
                "Native foundation healthy"
            } else {
                "Native foundation needs attention"
            });
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
        egui::ScrollArea::vertical()
            .id_salt("command_line_page")
            .show(ui, |ui| self.show_command_line_content(ui));
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

        let (state_label, state_color) = cli_state_style(&status);
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
                    ui.heading("Terminal installation");
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
        ui.heading("Use from a terminal or agent host");
        ui.label(
            "Open a new terminal after installation, then verify the native binary before using \
             the same workspace from Codex, Claude, or another local agent.",
        );
        command_copy_row(ui, "canisend version --json");
        command_copy_row(ui, "canisend --help");
        if !status.path_configured {
            ui.add_space(8.0);
            ui.colored_label(
                theme::AMBER_600,
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
                    theme::TEAL_600,
                    "The GUI-managed native CLI is the command currently resolved by PATH.",
                );
            }
            CliInstallState::Current => {
                ui.colored_label(
                    theme::AMBER_600,
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
                ui.colored_label(theme::AMBER_600, guidance);
            }
            CliInstallState::NewerInstalled => {
                ui.colored_label(
                    theme::TEAL_600,
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
                    theme::RED_600,
                    "The managed binary or installation record changed outside the GUI. Move or \
                     repair it manually before continuing; CanISend will not overwrite it.",
                );
            }
            CliInstallState::SourceUnavailable => {
                ui.colored_label(
                    theme::RED_600,
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
                self.uninstall_cli(ui.ctx().clone());
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
                    ui.heading("CanISend updates");
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
                            theme::AMBER_600,
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
                            theme::TEAL_600,
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
                ui.heading(title);
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

    fn empty_workspace(&mut self, ui: &mut egui::Ui) {
        ui.add_space(36.0);
        ui.vertical_centered(|ui| {
            ui.heading("Choose a local workspace");
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
                ui.label("Title");
                ui.add(
                    egui::TextEdit::singleline(&mut self.job_form.title)
                        .hint_text("Lecturer in Economics")
                        .desired_width(f32::INFINITY),
                );
                ui.label("Institution");
                ui.add(
                    egui::TextEdit::singleline(&mut self.job_form.institution)
                        .hint_text("University X")
                        .desired_width(f32::INFINITY),
                );
                if let Some(error) = &self.job_form.error {
                    ui.colored_label(theme::RED_600, error);
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
                    if ui.button("Cancel").clicked() {
                        self.show_job_form = false;
                    }
                });
            });
        self.show_job_form = open && self.show_job_form;
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
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.import_form.kind, ImportKind::File, "Local file");
                    ui.selectable_value(&mut self.import_form.kind, ImportKind::Url, "Public URL");
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
                            if ui.button("Choose file").clicked() {
                                self.import_form.file = rfd::FileDialog::new()
                                    .add_filter("Job sources", &["md", "txt", "json", "pdf"])
                                    .pick_file();
                            }
                        });
                        ui.checkbox(
                            &mut self.import_form.private_read_consent,
                            "Allow CanISend to read and store this private local source",
                        );
                    }
                    ImportKind::Url => {
                        ui.label("CanISend will fetch this user-supplied public HTTP(S) URL.");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.import_form.url)
                                .hint_text("https://jobs.example.edu/vacancy/123")
                                .desired_width(f32::INFINITY),
                        );
                        ui.checkbox(
                            &mut self.import_form.network_consent,
                            "Allow this user-invoked network fetch",
                        );
                    }
                }
                if let Some(error) = &self.import_form.error {
                    ui.colored_label(theme::RED_600, error);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.activity.is_none(), theme::primary_button("Import"))
                        .clicked()
                    {
                        self.import_selected_source(ui.ctx().clone());
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_import_form = false;
                    }
                });
            });
        self.show_import_form = open && self.show_import_form;
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
                ui.label("Workspace name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.workspace_form.alias)
                        .hint_text("Academic applications")
                        .desired_width(f32::INFINITY),
                );
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
                    if ui.button("Choose directory").clicked() {
                        self.workspace_form.path = rfd::FileDialog::new().pick_folder();
                    }
                });
                if let Some(error) = &self.workspace_form.error {
                    ui.colored_label(theme::RED_600, error);
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
                    if ui.button("Cancel").clicked() {
                        self.show_workspace_form = false;
                    }
                });
            });
        self.show_workspace_form = open && self.show_workspace_form;
    }

    fn submit_workspace_form(&mut self, ctx: egui::Context) {
        let alias = self.workspace_form.alias.trim().to_owned();
        if alias.is_empty() {
            self.workspace_form.error = Some("Workspace name is required".to_owned());
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
                theme::apply(ctx, self.dark_mode, true);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show_top_bar(ui);
        self.show_status_bar(ui);
        self.show_navigation(ui);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new().inner_margin(egui::Margin::same(if self.compact {
                    16
                } else {
                    24
                })),
            )
            .show(ui, |ui| {
                self.show_notice(ui);
                match self.page {
                    Page::Overview => self.show_overview(ui),
                    Page::Jobs => self.show_jobs(ui),
                    Page::Workspaces => self.show_workspaces(ui),
                    Page::CommandLine => self.show_command_line(ui),
                    Page::Diagnostics => self.show_diagnostics(ui),
                }
            });
        self.show_job_dialog(ui);
        self.show_import_dialog(ui);
        self.show_workspace_dialog(ui);
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
            let (color, label) = stage_status_style(state.status);
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

fn stage_status_style(status: StageExecutionStatus) -> (Color32, &'static str) {
    match status {
        StageExecutionStatus::Complete => (theme::TEAL_600, "Complete"),
        StageExecutionStatus::Ready => (theme::AMBER_600, "Ready"),
        StageExecutionStatus::Running => (Color32::from_rgb(37, 99, 235), "Running"),
        StageExecutionStatus::AwaitingUser => (theme::AMBER_600, "Awaiting user"),
        StageExecutionStatus::Blocked => (theme::SLATE_700, "Blocked"),
        StageExecutionStatus::Stale => (theme::RED_600, "Stale"),
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
    ui.label(value);
    ui.end_row();
}

fn cli_state_style(status: &CliInstallStatus) -> (&'static str, Color32) {
    match status.state {
        CliInstallState::NotInstalled => ("Not installed", theme::SLATE_700),
        CliInstallState::Current if status.active_is_managed => ("Ready", theme::TEAL_600),
        CliInstallState::Current => ("Installed; not active", theme::AMBER_600),
        CliInstallState::UpdateAvailable => ("Update available", theme::AMBER_600),
        CliInstallState::MigrationAvailable => ("Migration available", theme::AMBER_600),
        CliInstallState::NewerInstalled => ("Newer version installed", theme::TEAL_600),
        CliInstallState::Modified => ("Needs attention", theme::RED_600),
        CliInstallState::SourceUnavailable => ("CLI missing from package", theme::RED_600),
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
    use super::validate_job_form;

    #[test]
    fn job_form_requires_both_bounded_labels() {
        assert!(validate_job_form("", "University X").is_err());
        assert!(validate_job_form("Lecturer", " ").is_err());
        assert!(validate_job_form("Lecturer", "University X").is_ok());
        assert!(validate_job_form(&"x".repeat(513), "University X").is_err());
    }
}
