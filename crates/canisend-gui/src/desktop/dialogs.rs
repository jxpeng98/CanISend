use super::*;

impl CanISendDesktop {
    pub(super) fn page_header(&mut self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
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

    pub(super) fn open_job_form(&mut self) {
        self.show_job_form = true;
        self.pending_focus = Some(FocusTarget::JobTitle);
    }

    pub(super) fn open_import_form(&mut self) {
        self.show_import_form = true;
        self.pending_focus = Some(FocusTarget::ImportKind);
    }

    pub(super) fn open_workspace_form(&mut self) {
        self.show_workspace_form = true;
        self.pending_focus = Some(FocusTarget::WorkspaceAlias);
    }

    pub(super) fn empty_workspace(&mut self, ui: &mut egui::Ui) {
        ui.add_space(36.0);
        ui.vertical_centered(|ui| {
            accessible_heading(ui, self.language.text("Choose a local workspace"), 2);
            ui.label(
                self.language
                    .text("Create a new workspace or register an existing Rust v2 workspace."),
            );
            ui.add_space(12.0);
            if ui
                .add(theme::primary_button(
                    self.language.text("Open workspace manager"),
                ))
                .clicked()
            {
                self.page = Page::Workspaces;
            }
        });
    }

    pub(super) fn check_active_workspace(&mut self, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language.text("Checking workspace integrity"),
            ctx,
            WorkerRequest::CheckWorkspace { path },
        );
    }

    pub(super) fn backup_active_workspace(&mut self, ctx: egui::Context) {
        let Some(root) = self.active_workspace.clone() else {
            return;
        };
        let Some(destination) = pick_directory(Some(
            self.language
                .select("Choose a new backup directory", "选择新的备份目录"),
        )) else {
            return;
        };
        self.dispatch(
            self.language.text("Creating verified backup"),
            ctx,
            WorkerRequest::BackupWorkspace { root, destination },
        );
    }

    pub(super) fn start_selected_workflow(&mut self, ctx: egui::Context) {
        let (Some(path), Some(id)) = (self.active_workspace.clone(), self.selected_job_id.clone())
        else {
            return;
        };
        self.dispatch(
            self.language.text("Starting workflow"),
            ctx,
            WorkerRequest::StartWorkflow { path, id },
        );
    }

    pub(super) fn archive_selected_job(&mut self, ctx: egui::Context) {
        let (Some(path), Some(id)) = (self.active_workspace.clone(), self.selected_job_id.clone())
        else {
            return;
        };
        self.dispatch(
            self.language.text("Archiving job"),
            ctx,
            WorkerRequest::ArchiveJob { path, id },
        );
    }

    pub(super) fn show_job_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.show_job_form {
            return;
        }
        let mut open = self.show_job_form;
        egui::Window::new(self.language.text("Add job"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .show(ui.ctx(), |ui| {
                let title_label = ui.label(self.language.text("Title"));
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
                let institution_label = ui.label(self.language.text("Institution"));
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
                        .add_enabled(
                            self.activity.is_none(),
                            theme::primary_button(self.language.text("Create job")),
                        )
                        .clicked()
                    {
                        match validate_job_form(
                            &self.job_form.title,
                            &self.job_form.institution,
                            self.language,
                        ) {
                            Ok(()) => {
                                if let Some(path) = self.active_workspace.clone() {
                                    let title = self.job_form.title.trim().to_owned();
                                    let institution = self.job_form.institution.trim().to_owned();
                                    self.dispatch(
                                        self.language.text("Creating job"),
                                        ui.ctx().clone(),
                                        WorkerRequest::CreateJob {
                                            path,
                                            title,
                                            institution,
                                        },
                                    );
                                }
                            }
                            Err(error) => self.job_form.error = Some(error),
                        }
                    }
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new(self.language.text("Cancel")),
                        )
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

    pub(super) fn show_import_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.show_import_form {
            return;
        }
        let mut open = self.show_import_form;
        egui::Window::new(self.language.text("Import job source"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ui.ctx(), |ui| {
                ui.add_enabled_ui(self.activity.is_none(), |ui| {
                    let file_response = ui.selectable_value(
                        &mut self.import_form.kind,
                        ImportKind::File,
                        self.language.text("Local file"),
                    );
                    if self.pending_focus == Some(FocusTarget::ImportKind) {
                        file_response.request_focus();
                        self.pending_focus = None;
                    }
                    let url_response = ui.selectable_value(
                        &mut self.import_form.kind,
                        ImportKind::Url,
                        self.language.text("Public URL"),
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
                        ui.label(
                            self.language
                                .text("Supported: Markdown, text, JSON, and text-based PDF."),
                        );
                        ui.horizontal(|ui| {
                            let path = self
                                .import_form
                                .file
                                .as_ref()
                                .map_or(self.language.text("No file selected").to_owned(), |path| {
                                    path.display().to_string()
                                });
                            ui.label(path);
                            if ui
                                .add_enabled(
                                    self.activity.is_none(),
                                    egui::Button::new(self.language.text("Choose file")),
                                )
                                .clicked()
                            {
                                self.import_form.file = pick_job_source_file();
                                self.import_form.error = None;
                            }
                        });
                        if ui
                            .add_enabled(
                                self.activity.is_none(),
                                egui::Checkbox::new(
                                    &mut self.import_form.private_read_consent,
                                    self.language.text(
                                        "Allow CanISend to read and store this private local source",
                                    ),
                                ),
                            )
                            .changed()
                        {
                            self.import_form.error = None;
                        }
                    }
                    ImportKind::Url => {
                        ui.label(
                            self.language.text(
                                "CanISend will fetch this user-supplied public HTTP(S) URL.",
                            ),
                        );
                        let url_label = ui.label(self.language.text("Job source URL"));
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
                                    self.language
                                        .text("Allow this user-invoked network fetch"),
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
                        .add_enabled(
                            self.activity.is_none(),
                            theme::primary_button(self.language.text("Import")),
                        )
                        .clicked()
                    {
                        self.import_selected_source(ui.ctx().clone());
                    }
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new(self.language.text("Cancel")),
                        )
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

    pub(super) fn import_selected_source(&mut self, ctx: egui::Context) {
        let (Some(root), Some(job_id)) =
            (self.active_workspace.clone(), self.selected_job_id.clone())
        else {
            self.import_form.error =
                Some(self.language.text("No active job is selected").to_owned());
            return;
        };
        match self.import_form.kind {
            ImportKind::File => {
                if !self.import_form.private_read_consent {
                    self.import_form.error = Some(
                        self.language
                            .text("Confirm private local source access before importing")
                            .to_owned(),
                    );
                    return;
                }
                let Some(file) = self.import_form.file.clone() else {
                    self.import_form.error =
                        Some(self.language.text("Choose a source file").to_owned());
                    return;
                };
                self.dispatch(
                    self.language.text("Importing local source"),
                    ctx,
                    WorkerRequest::ImportLocalSource {
                        path: root,
                        id: job_id,
                        source: file,
                    },
                );
            }
            ImportKind::Url => {
                if !self.import_form.network_consent {
                    self.import_form.error = Some(
                        self.language
                            .text("Confirm the user-invoked network fetch before importing")
                            .to_owned(),
                    );
                    return;
                }
                let url = self.import_form.url.trim().to_owned();
                if url.is_empty() {
                    self.import_form.error =
                        Some(self.language.text("Enter a public HTTP(S) URL").to_owned());
                    return;
                }
                self.dispatch(
                    self.language.text("Fetching and importing URL"),
                    ctx,
                    WorkerRequest::ImportUrlSource {
                        path: root,
                        id: job_id,
                        url,
                    },
                );
            }
        }
    }

    pub(super) fn show_workspace_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.show_workspace_form {
            return;
        }
        let mut open = self.show_workspace_form;
        let title = if self.workspace_form.create_new {
            self.language.text("Create workspace")
        } else {
            self.language.text("Register existing workspace")
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ui.ctx(), |ui| {
                let alias_label = ui.label(self.language.text("Workspace name"));
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
                    self.language.text("Choose a new or empty directory.")
                } else {
                    self.language
                        .text("Choose a directory containing canisend.toml.")
                });
                ui.horizontal(|ui| {
                    ui.label(self.workspace_form.path.as_ref().map_or(
                        self.language.text("No directory selected").to_owned(),
                        |path| path.display().to_string(),
                    ));
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new(self.language.text("Choose directory")),
                        )
                        .clicked()
                    {
                        self.workspace_form.path = pick_directory(None);
                        self.workspace_form.error = None;
                    }
                });
                if let Some(error) = &self.workspace_form.error {
                    accessible_error(ui, theme::error(self.dark_mode), error);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let label = if self.workspace_form.create_new {
                        self.language.text("Create")
                    } else {
                        self.language.text("Register")
                    };
                    if ui
                        .add_enabled(self.activity.is_none(), theme::primary_button(label))
                        .clicked()
                    {
                        self.submit_workspace_form(ui.ctx().clone());
                    }
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new(self.language.text("Cancel")),
                        )
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

    pub(super) fn show_restore_workspace_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.show_restore_workspace_form {
            return;
        }
        let mut open = self.show_restore_workspace_form;
        egui::Window::new(self.language.text("Restore workspace backup"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(600.0)
            .show(ui.ctx(), |ui| {
                ui.label(self.language.text(
                    "Restore verifies the backup before creating a separate workspace directory.",
                ));
                ui.add_space(8.0);

                let alias_label = ui.label(self.language.text("Workspace name"));
                let alias_response = ui
                    .add_enabled(
                        self.activity.is_none(),
                        egui::TextEdit::singleline(&mut self.restore_workspace_form.alias)
                            .hint_text("Recovered applications")
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(alias_label.id);
                if self.pending_focus == Some(FocusTarget::RestoreWorkspaceAlias) {
                    alias_response.request_focus();
                    self.pending_focus = None;
                }
                if alias_response.changed() {
                    self.restore_workspace_form.error = None;
                }

                ui.label(self.language.text("Verified backup directory"));
                ui.horizontal(|ui| {
                    ui.label(self.restore_workspace_form.backup.as_ref().map_or(
                        self.language.text("No directory selected").to_owned(),
                        |path| path.display().to_string(),
                    ));
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new(self.language.text("Choose backup")),
                        )
                        .clicked()
                    {
                        self.restore_workspace_form.backup =
                            pick_directory(Some(self.language.text("Choose a verified backup")));
                        self.restore_workspace_form.error = None;
                    }
                });

                ui.label(self.language.text("New workspace destination"));
                ui.horizontal(|ui| {
                    ui.label(self.restore_workspace_form.destination.as_ref().map_or(
                        self.language.text("No directory selected").to_owned(),
                        |path| path.display().to_string(),
                    ));
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new(self.language.text("Choose destination")),
                        )
                        .clicked()
                    {
                        self.restore_workspace_form.destination = pick_directory(Some(
                            self.language.text("Choose a new or empty destination"),
                        ));
                        self.restore_workspace_form.error = None;
                    }
                });
                ui.label(
                    RichText::new(
                        self.language
                            .text("The destination must be new or empty and is never overwritten."),
                    )
                    .weak(),
                );

                if let Some(error) = &self.restore_workspace_form.error {
                    accessible_error(ui, theme::error(self.dark_mode), error);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            theme::primary_button(self.language.text("Review restore")),
                        )
                        .clicked()
                    {
                        self.submit_restore_workspace_form();
                    }
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::Button::new(self.language.text("Cancel")),
                        )
                        .clicked()
                    {
                        self.show_restore_workspace_form = false;
                    }
                });
            });
        if self.activity.is_none() && ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            self.show_restore_workspace_form = false;
        }
        self.show_restore_workspace_form =
            self.activity.is_some() || (open && self.show_restore_workspace_form);
    }

    pub(super) fn show_pending_confirmation(&mut self, ui: &mut egui::Ui) {
        let Some(pending) = self.pending_confirmation.clone() else {
            return;
        };
        let mut confirmed = false;
        let mut cancelled = false;
        let modal = egui::Modal::new(egui::Id::new("pending_confirmation")).show(ui.ctx(), |ui| {
            ui.set_width(420.0);
            match &pending {
                PendingConfirmation::ArchiveJob { title } => {
                    accessible_heading(ui, self.language.text("Archive this job?"), 1);
                    ui.label(
                        self.language.select(
                            "The job will leave the active list. Its workspace records are retained, but this GUI preview does not yet provide an unarchive action.",
                            "此职位将从活跃列表中移除，但工作区记录会保留。当前 GUI 预览版尚未提供取消归档操作。",
                        ),
                    );
                    ui.label(RichText::new(title).strong());
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(theme::destructive_button(
                                self.language.text("Confirm archive"),
                            ))
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui.button(self.language.text("Cancel")).clicked() {
                            cancelled = true;
                        }
                    });
                }
                PendingConfirmation::RestoreWorkspace {
                    alias,
                    backup,
                    destination,
                } => {
                    accessible_heading(
                        ui,
                        self.language.text("Restore this workspace backup?"),
                        1,
                    );
                    ui.label(self.language.text(
                        "CanISend will verify the backup and create a separate workspace. The source backup is not changed.",
                    ));
                    ui.add_space(6.0);
                    ui.label(format!("{}: {alias}", self.language.text("Workspace name")));
                    ui.label(format!(
                        "{}: {}",
                        self.language.text("Verified backup directory"),
                        backup.display()
                    ));
                    ui.label(format!(
                        "{}: {}",
                        self.language.text("New workspace destination"),
                        destination.display()
                    ));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(theme::primary_button(
                                self.language.text("Confirm restore"),
                            ))
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui.button(self.language.text("Cancel")).clicked() {
                            cancelled = true;
                        }
                    });
                }
                PendingConfirmation::RepairWorkspace { path } => {
                    accessible_heading(
                        ui,
                        self.language.text("Repair the active workspace?"),
                        1,
                    );
                    ui.label(self.language.text(
                        "CanISend will rebuild managed projections from verified workspace records, then run an integrity check. User-edited files are protected by the workspace repair policy.",
                    ));
                    ui.label(RichText::new(path.display().to_string()).strong());
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(theme::primary_button(
                                self.language.text("Confirm repair"),
                            ))
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui.button(self.language.text("Cancel")).clicked() {
                            cancelled = true;
                        }
                    });
                }
                PendingConfirmation::UninstallCli { restores_previous } => {
                    accessible_heading(
                        ui,
                        self.language.text("Uninstall the managed CLI?"),
                        1,
                    );
                    ui.label(if *restores_previous {
                        self.language.select(
                            "CanISend will verify the managed binary, remove it, and restore the previous installation. Workspace data is never removed.",
                            "CanISend 将验证并移除受管理的二进制文件，然后恢复先前的安装。工作区数据绝不会被删除。",
                        )
                    } else {
                        self.language.select(
                            "CanISend will verify and remove only the unchanged GUI-managed CLI. Workspace data is never removed.",
                            "CanISend 只会验证并移除未经修改、由 GUI 管理的 CLI。工作区数据绝不会被删除。",
                        )
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(theme::destructive_button(
                                self.language.text("Uninstall CLI"),
                            ))
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui.button(self.language.text("Cancel")).clicked() {
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
                PendingConfirmation::RestoreWorkspace {
                    alias,
                    backup,
                    destination,
                } => {
                    self.dispatch(
                        self.language.text("Restoring verified workspace backup"),
                        ui.ctx().clone(),
                        WorkerRequest::RestoreWorkspace {
                            alias,
                            backup,
                            destination,
                        },
                    );
                }
                PendingConfirmation::RepairWorkspace { path } => {
                    self.dispatch(
                        self.language.text("Repairing managed workspace files"),
                        ui.ctx().clone(),
                        WorkerRequest::RepairWorkspace { path },
                    );
                }
                PendingConfirmation::UninstallCli { .. } => {
                    self.uninstall_cli(ui.ctx().clone());
                }
            }
        } else if cancelled || modal.should_close() {
            self.pending_confirmation = None;
        }
    }

    pub(super) fn submit_workspace_form(&mut self, ctx: egui::Context) {
        let alias = self.workspace_form.alias.trim().to_owned();
        if let Err(error) = validate_workspace_alias(&alias) {
            self.workspace_form.error = Some(localized_workspace_alias_error(error, self.language));
            return;
        }
        let Some(path) = self.workspace_form.path.clone() else {
            self.workspace_form.error = Some(self.language.text("Choose a directory").to_owned());
            return;
        };
        if self.workspace_form.create_new {
            self.dispatch(
                self.language.text("Creating workspace"),
                ctx,
                WorkerRequest::CreateWorkspace { alias, path },
            );
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

    fn submit_restore_workspace_form(&mut self) {
        let alias = self.restore_workspace_form.alias.trim().to_owned();
        if let Err(error) = validate_workspace_alias(&alias) {
            self.restore_workspace_form.error =
                Some(localized_workspace_alias_error(error, self.language));
            return;
        }
        let Some(backup) = self.restore_workspace_form.backup.clone() else {
            self.restore_workspace_form.error =
                Some(self.language.text("Choose a backup directory").to_owned());
            return;
        };
        let Some(destination) = self.restore_workspace_form.destination.clone() else {
            self.restore_workspace_form.error = Some(
                self.language
                    .text("Choose a destination directory")
                    .to_owned(),
            );
            return;
        };
        if backup == destination {
            self.restore_workspace_form.error = Some(
                self.language
                    .text("Backup and destination directories must be different")
                    .to_owned(),
            );
            return;
        }
        self.restore_workspace_form.error = None;
        self.pending_confirmation = Some(PendingConfirmation::RestoreWorkspace {
            alias,
            backup,
            destination,
        });
    }
}
