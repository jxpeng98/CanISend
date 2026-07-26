use super::*;

impl CanISendDesktop {
    pub(super) fn show_top_bar(&mut self, ui: &mut egui::Ui) {
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
                    let workspace_label = ui.label(self.language.text("Workspace"));
                    let selected = self
                        .active_workspace
                        .as_ref()
                        .and_then(|path| {
                            self.registry
                                .entries
                                .iter()
                                .find(|entry| &entry.path == path)
                        })
                        .map_or(self.language.text("Choose a workspace"), |entry| {
                            entry.alias.as_str()
                        });
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
                                self.language.text("No workspace")
                            } else {
                                self.health.as_ref().map_or(
                                    self.language.text("Not checked"),
                                    |health| {
                                        if health.check.ok {
                                            self.language.text("Healthy")
                                        } else {
                                            self.language.text("Needs attention")
                                        }
                                    },
                                )
                            };
                            ui.label(RichText::new(health).color(if self.dark_mode {
                                theme::TEAL_100
                            } else {
                                theme::TEAL_700
                            }));
                            ui.label(RichText::new(self.language.text("Health")).weak());
                        });
                    }
                });
            });
        set_accesskit_role(
            ui.ctx(),
            panel.response.id,
            egui::accesskit::Role::Banner,
            Some(self.language.text("CanISend workspace header")),
        );
    }

    pub(super) fn show_navigation(&mut self, ui: &mut egui::Ui) {
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
                        for page in Page::ALL {
                            let selected = self.page == page;
                            let text = RichText::new(page.label(self.language)).size(15.0).color(
                                if selected {
                                    Color32::WHITE
                                } else {
                                    theme::neutral(self.dark_mode)
                                },
                            );
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
                        accessible_heading(ui, self.language.text("Accessibility & appearance"), 2);
                        let previous_language = self.language;
                        let language_combo =
                            egui::ComboBox::from_label(self.language.text("Language"))
                                .selected_text(self.language.native_name())
                                .show_ui(ui, |ui| {
                                    for language in Language::ALL {
                                        if ui
                                            .selectable_value(
                                                &mut self.language,
                                                language,
                                                language.native_name(),
                                            )
                                            .clicked()
                                        {
                                            ui.close();
                                        }
                                    }
                                });
                        keep_focused_visible(&language_combo.response);
                        if self.language != previous_language {
                            ui.ctx().request_repaint();
                        }
                        let dark_response =
                            ui.checkbox(&mut self.dark_mode, self.language.text("Dark appearance"));
                        keep_focused_visible(&dark_response);
                        if dark_response.changed() {
                            theme::apply(
                                ui.ctx(),
                                self.dark_mode,
                                self.compact,
                                self.reduce_motion,
                            );
                        }
                        let compact_response =
                            ui.checkbox(&mut self.compact, self.language.text("Compact density"));
                        keep_focused_visible(&compact_response);
                        if compact_response.changed() {
                            theme::apply(
                                ui.ctx(),
                                self.dark_mode,
                                self.compact,
                                self.reduce_motion,
                            );
                        }
                        let motion_response = ui
                            .checkbox(&mut self.reduce_motion, self.language.text("Reduce motion"));
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
                        let zoom_combo =
                            egui::ComboBox::from_label(self.language.text("Text size"))
                                .selected_text(format!("{:.0}%", zoom * 100.0))
                                .show_ui(ui, |ui| {
                                    for (factor, label) in [
                                        (1.0, "100%"),
                                        (1.25, "125%"),
                                        (1.5, "150%"),
                                        (2.0, "200%"),
                                    ] {
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
            Some(self.language.text("Primary navigation")),
        );
    }

    pub(super) fn show_status_bar(&mut self, ui: &mut egui::Ui) {
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
                        ui.label(RichText::new(self.language.text("Local workspace state")).weak());
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
            Some(self.language.text("Application status")),
        );
    }

    pub(super) fn show_notice(&mut self, ui: &mut egui::Ui) {
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
                            format!(
                                "{}: {message}",
                                if success {
                                    self.language.text("Completed")
                                } else {
                                    self.language.text("Needs attention")
                                }
                            ),
                            success,
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button(self.language.text("Dismiss")).clicked() {
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

    pub(super) fn show_overview(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            self.language.text("Overview"),
            self.language
                .text("Current local workspace and next actions"),
        );
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
        let health = self
            .health
            .as_ref()
            .map_or(self.language.text("Not checked"), |health| {
                if health.check.ok {
                    self.language.text("Healthy")
                } else {
                    self.language.text("Issues")
                }
            });
        if ui.available_width() >= 640.0 {
            ui.columns(3, |columns| {
                metric_card(
                    &mut columns[0],
                    self.language.text("Active jobs"),
                    &active_jobs,
                    self.language.text("Stored in this workspace"),
                );
                metric_card(
                    &mut columns[1],
                    self.language.text("Artifacts"),
                    &artifacts,
                    self.language.text("Revisioned local records"),
                );
                metric_card(
                    &mut columns[2],
                    self.language.text("Workspace health"),
                    health,
                    self.language.text("Run an integrity check regularly"),
                );
            });
        } else {
            metric_card(
                ui,
                self.language.text("Active jobs"),
                &active_jobs,
                self.language.text("Stored in this workspace"),
            );
            ui.add_space(8.0);
            metric_card(
                ui,
                self.language.text("Artifacts"),
                &artifacts,
                self.language.text("Revisioned local records"),
            );
            ui.add_space(8.0);
            metric_card(
                ui,
                self.language.text("Workspace health"),
                health,
                self.language.text("Run an integrity check regularly"),
            );
        }
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::next_button(self.language.text("Add job"))
                        .min_size(egui::vec2(120.0, 36.0)),
                )
                .clicked()
            {
                self.open_job_form();
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(self.language.text("Check workspace")),
                )
                .clicked()
            {
                self.check_active_workspace(ui.ctx().clone());
            }
            if ui.button(self.language.text("View all jobs")).clicked() {
                self.page = Page::Jobs;
            }
        });
        ui.add_space(22.0);
        accessible_heading(ui, self.language.text("Recently updated jobs"), 2);
        ui.separator();
        if self.jobs.is_empty() {
            ui.label(
                self.language
                    .text("No jobs yet. Add a job from a URL, PDF, Markdown, text, or JSON file."),
            );
        } else {
            let recent = self.jobs.iter().rev().take(5).cloned().collect::<Vec<_>>();
            for job in recent {
                self.job_row(ui, &job);
            }
        }
    }

    pub(super) fn show_jobs(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            self.language.text("Jobs"),
            self.language
                .text("Application records, supplied sources, and workflow state"),
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
            let search_label = ui.label(self.language.text("Search"));
            ui.add(
                egui::TextEdit::singleline(&mut self.job_filter)
                    .hint_text(self.language.text("Title or institution"))
                    .desired_width(280.0),
            )
            .labelled_by(search_label.id);
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Checkbox::new(
                        &mut self.include_archived,
                        self.language.text("Include archived"),
                    ),
                )
                .changed()
            {
                self.refresh_jobs(ui.ctx().clone());
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.activity.is_none(),
                        theme::primary_button(self.language.text("Add job")),
                    )
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
            ui.label(self.language.text("No jobs match the current filter."));
        }
        for job in visible {
            self.job_row(ui, &job);
        }
    }

    pub(super) fn job_row(&mut self, ui: &mut egui::Ui, job: &JobRecord) {
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
                        ui.label(format!(
                            "{} {}",
                            if self.language == Language::SimplifiedChinese {
                                "修订"
                            } else {
                                "Revision"
                            },
                            job.revision.get()
                        ));
                        if job.archived {
                            ui.colored_label(
                                theme::neutral(self.dark_mode),
                                self.language.text("Archived"),
                            );
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
            .on_hover_text(self.language.text("Open job"));
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                self.activity.is_none(),
                match self.language {
                    Language::English => {
                        format!("Open {} at {}", job.title, job.institution)
                    }
                    Language::SimplifiedChinese => {
                        format!("打开 {}（{}）", job.title, job.institution)
                    }
                },
            )
        });
        paint_focus_ring(ui, &response);
        if response.clicked() {
            self.selected_job_id = Some(job.id.to_string());
            self.load_job(job.id.to_string(), ui.ctx().clone());
        }
        ui.add_space(8.0);
    }

    pub(super) fn show_job_detail(&mut self, ui: &mut egui::Ui) {
        let Some(detail) = self.selected_job.clone() else {
            return;
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(self.language.text("Back to jobs")),
                )
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
                    theme::primary_button(self.language.text("Import source")),
                )
                .clicked()
            {
                self.open_import_form();
            }
            if detail.workflow.is_none()
                && ui
                    .add_enabled(
                        self.activity.is_none() && !detail.sources.is_empty(),
                        theme::next_button(self.language.text("Start workflow")),
                    )
                    .on_disabled_hover_text(self.language.text("Import at least one source first"))
                    .clicked()
            {
                self.start_selected_workflow(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.activity.is_none() && !detail.job.archived,
                    theme::destructive_button(self.language.text("Archive")),
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
            accessible_heading(&mut columns[0], self.language.text("Sources"), 2);
            columns[0].separator();
            if detail.sources.is_empty() {
                columns[0].label(self.language.text("No source has been imported."));
            }
            for source in &detail.sources {
                columns[0].label(
                    RichText::new(source_kind_label(source.kind, self.language))
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
            accessible_heading(&mut columns[1], self.language.text("Workflow"), 2);
            columns[1].separator();
            if let Some(workflow) = &detail.workflow {
                workflow_timeline(&mut columns[1], workflow, self.language);
            } else {
                columns[1].label(self.language.text("Workflow has not started."));
                columns[1].label(
                    self.language
                        .text("Import a source, then start the durable stage graph."),
                );
            }
            columns[1].add_space(12.0);
            columns[1].label(
                RichText::new(self.language.text("Alpha GUI scope"))
                    .strong()
                    .color(theme::warning(self.dark_mode)),
            );
            columns[1].label(self.language.text(
                "Stage begin/complete/rerun, criteria, evidence, documents, review, render, and export remain available through the CLI or Agent v2 until the Beta GUI.",
            ));
        });
    }

    pub(super) fn show_workspaces(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            self.language.text("Workspaces"),
            self.language
                .text("Local workspace registry, integrity, and backups"),
        );
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::primary_button(self.language.text("Create workspace")),
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
                    egui::Button::new(self.language.text("Register existing")),
                )
                .clicked()
            {
                self.workspace_form = WorkspaceForm::default();
                self.open_workspace_form();
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(self.language.text("Restore backup")),
                )
                .clicked()
            {
                self.restore_workspace_form = RestoreWorkspaceForm::default();
                self.show_restore_workspace_form = true;
                self.pending_focus = Some(FocusTarget::RestoreWorkspaceAlias);
            }
            if ui
                .add_enabled(
                    self.active_workspace.is_some() && self.activity.is_none(),
                    egui::Button::new(self.language.text("Check active")),
                )
                .clicked()
            {
                self.check_active_workspace(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.active_workspace.is_some() && self.activity.is_none(),
                    egui::Button::new(self.language.text("Back up active")),
                )
                .clicked()
            {
                self.backup_active_workspace(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.active_workspace.is_some() && self.activity.is_none(),
                    egui::Button::new(self.language.text("Repair active")),
                )
                .on_hover_text(
                    self.language.text(
                        "Rebuild missing or changed managed projections from verified records",
                    ),
                )
                .clicked()
                && let Some(path) = self.active_workspace.clone()
            {
                self.pending_confirmation = Some(PendingConfirmation::RepairWorkspace { path });
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
                                    theme::destructive_button(
                                        self.language.text("Remove from list"),
                                    ),
                                )
                                .on_hover_text(
                                    self.language.text("This does not delete workspace data"),
                                )
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
                                .add_enabled(
                                    self.activity.is_none(),
                                    egui::Button::new(self.language.text("Open")),
                                )
                                .clicked()
                            {
                                self.load_workspace(entry.path.clone(), ui.ctx().clone());
                            }
                            if self.active_workspace.as_ref() == Some(&entry.path) {
                                ui.colored_label(
                                    theme::positive(self.dark_mode),
                                    self.language.text("Active"),
                                );
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }
        if let Some(health) = &self.health {
            ui.add_space(12.0);
            accessible_heading(ui, self.language.text("Latest integrity check"), 2);
            ui.label(if health.check.ok {
                self.language
                    .text("Database and referenced blobs passed verification.")
            } else {
                self.language
                    .text("The workspace needs attention before further mutation.")
            });
            for issue in &health.check.issues {
                ui.colored_label(
                    theme::error(self.dark_mode),
                    format!("{}: {}", issue.code, issue.message),
                );
            }
        }
    }

    pub(super) fn show_diagnostics(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            self.language.text("Diagnostics"),
            self.language
                .text("Body-free product and runtime information"),
        );
        egui::Grid::new("product_diagnostics")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                diagnostic_row(ui, self.language.text("Product"), &self.product.product);
                diagnostic_row(ui, self.language.text("Version"), &self.product.version);
                diagnostic_row(ui, self.language.text("Protocol"), &self.product.protocol);
                diagnostic_row(
                    ui,
                    self.language.text("Workspace format"),
                    &self.product.workspace_format,
                );
                diagnostic_row(
                    ui,
                    self.language.text("Target"),
                    &format!("{}-{}", self.product.target_arch, self.product.target_os),
                );
                diagnostic_row(
                    ui,
                    self.language.text("Text size"),
                    &format!("{:.0}%", ui.ctx().zoom_factor() * 100.0),
                );
                diagnostic_row(
                    ui,
                    self.language.text("Display scale"),
                    &ui.ctx().native_pixels_per_point().map_or_else(
                        || {
                            self.language
                                .text("Not reported by the window system")
                                .to_owned()
                        },
                        |scale| {
                            format!(
                                "{scale:.2} {}",
                                self.language.text("physical pixels per point")
                            )
                        },
                    ),
                );
                diagnostic_row(
                    ui,
                    self.language.text("Reduced motion"),
                    if self.reduce_motion {
                        self.language.text("Enabled")
                    } else {
                        self.language.text("Disabled")
                    },
                );
                diagnostic_row(
                    ui,
                    self.language.text("Language"),
                    self.language.native_name(),
                );
                diagnostic_row(ui, self.language.text("Locale"), self.language.code());
                diagnostic_row(
                    ui,
                    self.language.text("CJK font"),
                    self.cjk_font_path
                        .unwrap_or(self.language.text("Not found")),
                );
            });
        ui.add_space(18.0);
        if ui
            .add_enabled(
                self.activity.is_none(),
                theme::primary_button(self.language.text("Run native self-check")),
            )
            .clicked()
        {
            self.dispatch(
                self.language.text("Running native self-check"),
                ui.ctx().clone(),
                WorkerRequest::Doctor,
            );
        }
        if let Some(doctor) = &self.doctor {
            ui.add_space(16.0);
            accessible_heading(
                ui,
                if doctor.healthy {
                    self.language.text("Native foundation healthy")
                } else {
                    self.language.text("Native foundation needs attention")
                },
                2,
            );
            ui.label(match self.language {
                Language::English => format!(
                    "{} embedded resources; renderer produced {} page(s) with {} warning(s)",
                    doctor.embedded_resources, doctor.rendered_pages, doctor.render_warning_count
                ),
                Language::SimplifiedChinese => format!(
                    "{} 个内置资源；渲染器生成 {} 页，产生 {} 条警告",
                    doctor.embedded_resources, doctor.rendered_pages, doctor.render_warning_count
                ),
            });
            ui.label(self.language.text("Python runtime: not required"));
        }
        ui.add_space(24.0);
        ui.label(
            RichText::new(
                self.language
                    .text("Diagnostics intentionally omit job adverts, profile evidence, drafts, and provider payloads."),
            )
            .weak(),
        );
    }

    pub(super) fn show_command_line(&mut self, ui: &mut egui::Ui) {
        self.show_command_line_content(ui);
    }

    pub(super) fn show_command_line_content(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            self.language.text("Command line"),
            self.language
                .text("Keep the terminal CLI aligned with this CanISend desktop release"),
        );
        ui.label(
            self.language.select(
                "GUI actions and CLI commands share the Rust application layer and the same local workspace. This page checks only CanISend product versions. It does not inspect language runtimes or package managers, and upgrading the CLI never migrates workspace data.",
                "GUI 操作和 CLI 命令共用 Rust 应用层及同一个本地工作区。此页面只检查 CanISend 产品版本，不检测语言运行时或包管理器；升级 CLI 也不会迁移工作区数据。",
            ),
        );
        ui.add_space(16.0);

        let Some(status) = self.cli_status.clone() else {
            if self.activity.is_none() {
                if ui
                    .add(theme::primary_button(
                        self.language.text("Check CLI installation"),
                    ))
                    .clicked()
                {
                    self.refresh_cli_status(ui.ctx().clone());
                }
            } else {
                ui.spinner();
                ui.label(self.language.text("Checking CLI installation…"));
            }
            return;
        };

        let (state_label, state_color) = cli_state_style(&status, self.dark_mode, self.language);
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
                    accessible_heading(ui, self.language.text("Terminal installation"), 2);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.colored_label(state_color, RichText::new(state_label).strong());
                    });
                });
                ui.add_space(8.0);
                egui::Grid::new("cli_install_status")
                    .num_columns(2)
                    .spacing([24.0, 9.0])
                    .show(ui, |ui| {
                        diagnostic_row(
                            ui,
                            self.language.text("Bundled version"),
                            &status.bundled_version,
                        );
                        diagnostic_row(
                            ui,
                            self.language.text("Installed version"),
                            status
                                .installed_version
                                .as_deref()
                                .unwrap_or(if status.installed {
                                    self.language.text("Unknown (older version interface)")
                                } else {
                                    self.language.text("Not installed")
                                }),
                        );
                        diagnostic_row(
                            ui,
                            self.language.text("Bundled CLI"),
                            &status
                                .source_path
                                .as_ref()
                                .map_or(self.language.text("Not found").to_owned(), |path| {
                                    path.display().to_string()
                                }),
                        );
                        diagnostic_row(
                            ui,
                            self.language.text("Install destination"),
                            &status.destination.display().to_string(),
                        );
                        diagnostic_row(
                            ui,
                            self.language.text("Current PATH resolves"),
                            &status.active_command.as_ref().map_or(
                                self.language
                                    .text("No canisend command found on PATH")
                                    .to_owned(),
                                |path| path.display().to_string(),
                            ),
                        );
                        diagnostic_row(
                            ui,
                            self.language.text("Destination on current PATH"),
                            if status.path_configured {
                                self.language.text("Yes")
                            } else {
                                self.language.text("No")
                            },
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
        accessible_heading(
            ui,
            self.language.text("Use from a terminal or agent host"),
            2,
        );
        ui.label(
            self.language.select(
                "Open a new terminal after installation, then verify the native binary before using the same workspace from Codex, Claude, or another local agent.",
                "安装后请打开新的终端并验证原生二进制文件，随后即可从 Codex、Claude 或其他本地 Agent 使用同一工作区。",
            ),
        );
        command_copy_row(ui, "canisend version --json", self.language);
        command_copy_row(ui, "canisend --help", self.language);
        if !status.path_configured {
            ui.add_space(8.0);
            ui.colored_label(
                theme::warning(self.dark_mode),
                self.language.select(
                    "The destination directory is not visible in this app's PATH. Add it to your shell profile, then open a new terminal.",
                    "此应用的 PATH 中没有安装目录。请将其加入 shell 配置文件，然后打开新的终端。",
                ),
            );
            command_copy_row(ui, "export PATH=\"$HOME/.local/bin:$PATH\"", self.language);
        }
    }

    pub(super) fn show_cli_state_guidance(&mut self, ui: &mut egui::Ui, status: &CliInstallStatus) {
        match status.state {
            CliInstallState::NotInstalled => {
                ui.label(
                    self.language
                        .text("No GUI-managed Rust CLI is installed at the destination."),
                );
            }
            CliInstallState::Current if status.active_is_managed => {
                ui.colored_label(
                    theme::positive(self.dark_mode),
                    self.language.text(
                        "The GUI-managed native CLI is the command currently resolved by PATH.",
                    ),
                );
            }
            CliInstallState::Current => {
                ui.colored_label(
                    theme::warning(self.dark_mode),
                    self.language.select(
                        "The native CLI is installed, but the terminal currently resolves another CanISend installation first.",
                        "原生 CLI 已安装，但终端当前优先解析到另一个 CanISend 安装。",
                    ),
                );
            }
            CliInstallState::UpdateAvailable => {
                ui.label(
                    self.language
                        .text("The CLI installed by this GUI differs from the bundled release."),
                );
            }
            CliInstallState::MigrationAvailable => {
                let guidance = match (self.language, status.version_relation) {
                    (Language::English, CliVersionRelation::Older) => format!(
                        "CanISend {} is installed. Upgrade to {} with one click; the current \
                         executable will be preserved for rollback.",
                        status.installed_version.as_deref().unwrap_or(""),
                        status.bundled_version
                    ),
                    (Language::English, CliVersionRelation::Same) => format!(
                        "CanISend {} is installed outside this GUI. Adopt the bundled copy to \
                         enable verified updates and rollback.",
                        status.bundled_version
                    ),
                    (Language::English, CliVersionRelation::Unknown) => format!(
                        "An older CanISend command interface is installed but does not report a \
                         usable version. Migrate it to {} with one click; the current executable \
                         will be preserved for rollback.",
                        status.bundled_version
                    ),
                    (Language::English, CliVersionRelation::Newer) => {
                        "A newer CanISend version is installed.".to_owned()
                    }
                    (Language::SimplifiedChinese, CliVersionRelation::Older) => format!(
                        "已安装 CanISend {}。可一键升级到 {}；当前可执行文件会保留以便回滚。",
                        status.installed_version.as_deref().unwrap_or(""),
                        status.bundled_version
                    ),
                    (Language::SimplifiedChinese, CliVersionRelation::Same) => format!(
                        "CanISend {} 安装在 GUI 管理范围之外。采用内置副本后可启用经过验证的更新和回滚。",
                        status.bundled_version
                    ),
                    (Language::SimplifiedChinese, CliVersionRelation::Unknown) => format!(
                        "旧版 CanISend 接口无法报告可用版本。可一键迁移到 {}；当前可执行文件会保留以便回滚。",
                        status.bundled_version
                    ),
                    (Language::SimplifiedChinese, CliVersionRelation::Newer) => {
                        "已安装较新的 CanISend 版本。".to_owned()
                    }
                };
                ui.colored_label(theme::warning(self.dark_mode), guidance);
            }
            CliInstallState::NewerInstalled => {
                ui.colored_label(
                    theme::positive(self.dark_mode),
                    match self.language {
                        Language::English => format!(
                            "CanISend {} is newer than the bundled {} release. This GUI will not downgrade it.",
                            status.installed_version.as_deref().unwrap_or("Unknown"),
                            status.bundled_version
                        ),
                        Language::SimplifiedChinese => format!(
                            "CanISend {} 比内置的 {} 版本更新。此 GUI 不会执行降级。",
                            status.installed_version.as_deref().unwrap_or("未知"),
                            status.bundled_version
                        ),
                    },
                );
            }
            CliInstallState::Modified => {
                ui.colored_label(
                    theme::error(self.dark_mode),
                    self.language.select(
                        "The managed binary or installation record changed outside the GUI. Move or repair it manually before continuing; CanISend will not overwrite it.",
                        "受管理的二进制文件或安装记录已在 GUI 之外发生更改。请先手动移动或修复；CanISend 不会覆盖它。",
                    ),
                );
            }
            CliInstallState::SourceUnavailable => {
                ui.colored_label(
                    theme::error(self.dark_mode),
                    self.language.select(
                        "This GUI build does not include a sibling or app-bundled canisend binary. Build/package both executables before installing.",
                        "此 GUI 构建不包含同目录或 App 内置的 canisend 二进制文件。请先构建并打包两个可执行文件。",
                    ),
                );
            }
        }
        if status.previous_installation_preserved {
            ui.label(
                RichText::new(self.language.text(
                    "A previous installation is preserved and will be restored if you uninstall.",
                ))
                .weak(),
            );
        }
    }

    pub(super) fn show_cli_actions(&mut self, ui: &mut egui::Ui, status: &CliInstallStatus) {
        ui.horizontal(|ui| {
            let install_label = match status.state {
                CliInstallState::UpdateAvailable => self.language.text("Update CLI"),
                CliInstallState::MigrationAvailable
                    if status.version_relation == CliVersionRelation::Older =>
                {
                    self.language.text("Upgrade installed CLI")
                }
                CliInstallState::MigrationAvailable => self.language.text("Migrate installed CLI"),
                CliInstallState::Current => self.language.text("Reinstall CLI"),
                _ => self.language.text("Install CLI"),
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
                    CliInstallState::Modified => self.language.select(
                        "Externally modified managed installations are never overwritten",
                        "绝不会覆盖在外部修改过的受管理安装",
                    ),
                    CliInstallState::NewerInstalled => self.language.select(
                        "A newer installed CanISend version is never downgraded",
                        "绝不会降级较新的 CanISend 安装",
                    ),
                    CliInstallState::SourceUnavailable => self.language.select(
                        "No bundled Rust CLI is available in this GUI build",
                        "此 GUI 构建中没有可用的内置 Rust CLI",
                    ),
                    _ => self.language.select(
                        "CLI installation is not currently available",
                        "当前无法安装 CLI",
                    ),
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
                    theme::destructive_button(self.language.text("Uninstall managed CLI")),
                )
                .on_disabled_hover_text(self.language.select(
                    "Only an unchanged GUI-managed CLI can be uninstalled",
                    "只能卸载未经修改、由 GUI 管理的 CLI",
                ))
                .clicked()
            {
                self.pending_confirmation = Some(PendingConfirmation::UninstallCli {
                    restores_previous: status.previous_installation_preserved,
                });
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(self.language.text("Refresh")),
                )
                .clicked()
            {
                self.refresh_cli_status(ui.ctx().clone());
            }
        });
    }

    pub(super) fn show_product_update_check(&mut self, ui: &mut egui::Ui) {
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
                    accessible_heading(ui, self.language.text("CanISend updates"), 2);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                self.activity.is_none(),
                                egui::Button::new(self.language.text("Check for updates")),
                            )
                            .on_hover_text(self.language.select(
                                "Contact the public CanISend GitHub release endpoint once",
                                "仅访问一次公开的 CanISend GitHub Releases 接口",
                            ))
                            .clicked()
                        {
                            self.check_for_updates(ui.ctx().clone());
                        }
                    });
                });
                ui.label(format!(
                    "{}{}",
                    self.language.select(
                        "Current desktop and bundled CLI version: ",
                        "当前桌面端及内置 CLI 版本："
                    ),
                    self.product.version,
                ));
                ui.label(
                    RichText::new(
                        self.language.select(
                            "Checks are manual and body-free. No workspace, job, profile, or document data is sent.",
                            "更新检查仅由用户手动触发，且不包含正文。不会发送工作区、职位、个人资料或文档数据。",
                        ),
                    )
                    .weak(),
                );
                if let Some(update) = &self.update_check {
                    ui.add_space(10.0);
                    if update.update_available {
                        ui.colored_label(
                            theme::warning(self.dark_mode),
                            RichText::new(match self.language {
                                Language::English => format!(
                                    "{} is available on the {} channel.",
                                    update.latest_version, update.channel
                                ),
                                Language::SimplifiedChinese => format!(
                                    "{} 已在 {} 渠道发布。",
                                    update.latest_version, update.channel
                                ),
                            })
                            .strong(),
                        );
                        ui.label(
                            self.language.select(
                                "Download the newer CanISend desktop release to update both the GUI and its bundled CLI. This preview does not download or run installers.",
                                "请下载新版 CanISend 桌面应用，以同时更新 GUI 和内置 CLI。当前预览版不会下载或运行安装程序。",
                            ),
                        );
                    } else {
                        ui.colored_label(
                            theme::positive(self.dark_mode),
                            RichText::new(match self.language {
                                Language::English => format!(
                                    "Up to date — latest compatible release is {}.",
                                    update.latest_version
                                ),
                                Language::SimplifiedChinese => format!(
                                    "已是最新版本——最新兼容版本为 {}。",
                                    update.latest_version
                                ),
                            })
                            .strong(),
                        );
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&update.release_name).weak());
                        if ui
                            .small_button(self.language.text("Copy release link"))
                            .clicked()
                        {
                            ui.ctx().copy_text(update.release_url.clone());
                        }
                    });
                }
            });
    }

    pub(super) fn refresh_cli_status(&mut self, ctx: egui::Context) {
        if self.activity.is_some() {
            return;
        }
        let source = self.cli_source.clone();
        let destination = self.cli_destination.clone();
        self.dispatch(
            self.language.text("Checking CLI installation"),
            ctx,
            WorkerRequest::LoadCliStatus {
                source,
                destination,
            },
        );
    }

    pub(super) fn install_cli(&mut self, ctx: egui::Context) {
        let Some(source) = self.cli_source.clone() else {
            self.fail(
                self.language
                    .text("No bundled CanISend CLI is available")
                    .to_owned(),
            );
            return;
        };
        let destination = self.cli_destination.clone();
        self.dispatch(
            self.language.text("Installing or upgrading CanISend CLI"),
            ctx,
            WorkerRequest::InstallCli {
                source,
                destination,
                replace_existing: true,
            },
        );
    }

    pub(super) fn check_for_updates(&mut self, ctx: egui::Context) {
        self.dispatch(
            self.language.text("Checking for CanISend updates"),
            ctx,
            WorkerRequest::CheckForUpdates,
        );
    }

    pub(super) fn uninstall_cli(&mut self, ctx: egui::Context) {
        let source = self.cli_source.clone();
        let destination = self.cli_destination.clone();
        self.dispatch(
            self.language.text("Uninstalling managed CLI"),
            ctx,
            WorkerRequest::UninstallCli {
                source,
                destination,
            },
        );
    }
}
