use super::*;

impl CanISendDesktop {
    pub(super) fn show_application_plan_workflow(&mut self, ui: &mut egui::Ui, job_id: &str) {
        if self.plan_review_form.job_id.as_deref() != Some(job_id) {
            self.plan_review_form = PlanReviewForm {
                job_id: Some(job_id.to_owned()),
                ..PlanReviewForm::default()
            };
        }

        accessible_heading(ui, self.language.text("Application plan"), 2);
        ui.separator();
        ui.label(self.language.select(
            "Review the revision-bound matches, choose Apply, Hold, or Skip, and define the application strategy and document requirements. CanISend uses Hold as the safe default.",
            "请审阅与修订版本绑定的匹配结果，选择申请、暂缓或跳过，并确定申请策略和文档要求。CanISend 默认使用较安全的“暂缓”。",
        ));
        ui.add_space(8.0);
        if ui
            .add_enabled(
                self.activity.is_none(),
                egui::Checkbox::new(
                    &mut self.plan_review_form.private_read_consent,
                    self.language
                        .text("Allow this user-invoked private plan review"),
                ),
            )
            .changed()
        {
            self.plan_review_form.error = None;
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::primary_button(self.language.text("Load editable plan")),
                )
                .clicked()
            {
                self.load_plan_candidate(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(self.language.text("Load confirmed plan")),
                )
                .clicked()
            {
                self.load_current_plan(ui.ctx().clone());
            }
        });
        if let Some(error) = &self.plan_review_form.error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }

        let Some(mut candidate) = self.plan_review_form.candidate.clone() else {
            ui.add_space(8.0);
            ui.label(
                RichText::new(self.language.select(
                    "Complete the evidence-match task before loading an editable plan. A confirmed plan is available only after the user has saved a decision.",
                    "请先完成 evidence-match 任务，再加载可编辑计划。只有用户保存过申请决定后，才会存在已确认计划。",
                ))
                .weak(),
            );
            return;
        };

        ui.add_space(16.0);
        ui.horizontal_wrapped(|ui| {
            accessible_heading(ui, self.language.text("Application plan candidate"), 3);
            if let Some(current) = &self.plan_review_form.current {
                ui.label(
                    RichText::new(match self.language {
                        Language::English => {
                            format!("Current confirmed revision {}", current.revision.get())
                        }
                        Language::SimplifiedChinese => {
                            format!("当前已确认修订 {}", current.revision.get())
                        }
                    })
                    .strong()
                    .color(theme::positive(self.dark_mode)),
                );
            }
        });
        ui.colored_label(
            theme::warning(self.dark_mode),
            self.language.select(
                "Private strategy, risk, blocker, and document-planning content is visible below and remains local.",
                "下方会显示私有的策略、风险、阻塞项和文档计划内容，数据只保留在本地。",
            ),
        );
        diagnostic_row(
            ui,
            self.language.text("Matches artifact"),
            &format!(
                "{} · r{} · SHA-256 {}",
                candidate.matches_artifact.id,
                candidate.matches_artifact.revision.get(),
                candidate.matches_artifact.sha256
            ),
        );

        let mut changed = false;
        let previous_decision = candidate.decision;
        ui.add_enabled_ui(self.activity.is_none(), |ui| {
            egui::ComboBox::from_label(self.language.text("Decision"))
                .selected_text(application_decision_text(self.language, candidate.decision))
                .show_ui(ui, |ui| {
                    for decision in [
                        ApplicationDecision::Apply,
                        ApplicationDecision::Hold,
                        ApplicationDecision::Skip,
                    ] {
                        ui.selectable_value(
                            &mut candidate.decision,
                            decision,
                            application_decision_text(self.language, decision),
                        );
                    }
                });
        });
        if candidate.decision != previous_decision {
            if candidate.decision == ApplicationDecision::Skip {
                for document in &mut candidate.documents {
                    document.requirement = DocumentRequirement::Omitted;
                    document.executor = None;
                }
            }
            changed = true;
        }
        if candidate.decision == ApplicationDecision::Apply
            && candidate
                .blockers
                .iter()
                .any(|blocker| blocker.severity == PlanBlockerSeverity::Blocking)
        {
            accessible_error(
                ui,
                theme::error(self.dark_mode),
                self.language
                    .text("Resolve blocking evidence gaps before choosing Apply"),
            );
        }

        ui.add_space(14.0);
        accessible_heading(ui, self.language.text("Application strategy"), 3);
        let positioning_label = ui.label(self.language.text("Positioning"));
        if ui
            .add_enabled(
                self.activity.is_none(),
                egui::TextEdit::multiline(&mut candidate.strategy.positioning)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(positioning_label.id)
            .changed()
        {
            changed = true;
        }

        accessible_heading(ui, self.language.text("Priorities"), 4);
        if edit_text_list(
            ui,
            &mut candidate.strategy.priorities,
            self.language.text("Priority"),
            self.language.text("Add priority"),
            self.language.text("Remove"),
            self.activity.is_none(),
            false,
        ) {
            changed = true;
        }
        accessible_heading(ui, self.language.text("Risks"), 4);
        if edit_text_list(
            ui,
            &mut candidate.strategy.risks,
            self.language.text("Risk"),
            self.language.text("Add risk"),
            self.language.text("Remove"),
            self.activity.is_none(),
            true,
        ) {
            changed = true;
        }

        ui.add_space(14.0);
        accessible_heading(ui, self.language.text("Document plan"), 3);
        for document in &mut candidate.documents {
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(if self.dark_mode {
                    Color32::from_rgb(38, 48, 52)
                } else {
                    Color32::WHITE
                })
                .stroke(Stroke::new(1.0, theme::SLATE_300))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    accessible_heading(ui, document_kind_text(self.language, document.kind), 4);
                    let previous_requirement = document.requirement;
                    ui.add_enabled_ui(self.activity.is_none(), |ui| {
                        egui::ComboBox::from_label(self.language.text("Requirement"))
                            .selected_text(document_requirement_text(
                                self.language,
                                document.requirement,
                            ))
                            .show_ui(ui, |ui| {
                                for requirement in [
                                    DocumentRequirement::Required,
                                    DocumentRequirement::Optional,
                                    DocumentRequirement::Omitted,
                                ] {
                                    ui.selectable_value(
                                        &mut document.requirement,
                                        requirement,
                                        document_requirement_text(self.language, requirement),
                                    );
                                }
                            });
                    });
                    if document.requirement != previous_requirement {
                        document.executor = if document.requirement == DocumentRequirement::Omitted
                        {
                            None
                        } else {
                            Some(ExecutionMode::HostAgent)
                        };
                        changed = true;
                    }

                    if document.requirement == DocumentRequirement::Omitted {
                        ui.label(
                            RichText::new(
                                self.language
                                    .text("Omitted documents do not have an executor."),
                            )
                            .weak(),
                        );
                    } else {
                        let previous_executor = document.executor;
                        let mut executor = document.executor.unwrap_or(ExecutionMode::HostAgent);
                        ui.add_enabled_ui(self.activity.is_none(), |ui| {
                            egui::ComboBox::from_label(self.language.text("Executor"))
                                .selected_text(execution_mode_label(executor, self.language))
                                .show_ui(ui, |ui| {
                                    for mode in [
                                        ExecutionMode::HostAgent,
                                        ExecutionMode::ConfiguredProvider,
                                    ] {
                                        ui.selectable_value(
                                            &mut executor,
                                            mode,
                                            execution_mode_label(mode, self.language),
                                        );
                                    }
                                });
                        });
                        document.executor = Some(executor);
                        if document.executor != previous_executor {
                            changed = true;
                        }
                    }

                    let rationale_label = ui.label(self.language.text("Document rationale"));
                    if ui
                        .add_enabled(
                            self.activity.is_none(),
                            egui::TextEdit::multiline(&mut document.rationale)
                                .desired_rows(2)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(rationale_label.id)
                        .changed()
                    {
                        changed = true;
                    }
                    accessible_heading(ui, self.language.text("Constraints"), 5);
                    if edit_text_list(
                        ui,
                        &mut document.constraints,
                        self.language.text("Constraint"),
                        self.language.text("Add constraint"),
                        self.language.text("Remove"),
                        self.activity.is_none(),
                        true,
                    ) {
                        changed = true;
                    }
                });
        }

        ui.add_space(14.0);
        accessible_heading(ui, self.language.text("Derived blockers"), 3);
        ui.label(
            RichText::new(self.language.select(
                "Blockers are derived from the current criteria and evidence matches. They cannot be edited here.",
                "阻塞项由当前职位条件和证据匹配结果派生，不能在此处编辑。",
            ))
            .weak(),
        );
        if candidate.blockers.is_empty() {
            ui.label(self.language.text("No derived blockers."));
        }
        for blocker in &candidate.blockers {
            let color = match blocker.severity {
                PlanBlockerSeverity::Blocking => theme::error(self.dark_mode),
                PlanBlockerSeverity::Warning => theme::warning(self.dark_mode),
            };
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(Stroke::new(1.0, theme::SLATE_300))
                .corner_radius(5)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            color,
                            RichText::new(plan_blocker_severity_text(
                                self.language,
                                blocker.severity,
                            ))
                            .strong(),
                        );
                        ui.monospace(&blocker.code);
                    });
                    ui.label(&blocker.description);
                    diagnostic_row(
                        ui,
                        self.language.text("Criterion ID"),
                        blocker.criterion.id.as_str(),
                    );
                });
            ui.add_space(6.0);
        }

        if changed {
            self.plan_review_form.decision_confirmed = false;
            self.plan_review_form.error = None;
        }
        self.plan_review_form.candidate = Some(candidate.clone());

        ui.add_space(14.0);
        ui.colored_label(
            theme::warning(self.dark_mode),
            self.language.select(
                "Confirming this revision records your application decision and may reset or block draft, review, package, and render work. Existing records remain available but may no longer be current.",
                "确认此修订版本会记录你的申请决定，并可能重置或阻止起草、审阅、打包和渲染工作。已有记录仍会保留，但可能不再是当前版本。",
            ),
        );
        if ui
            .add_enabled(
                self.activity.is_none(),
                egui::Checkbox::new(
                    &mut self.plan_review_form.decision_confirmed,
                    self.language
                        .text("I confirm this application decision and its downstream effects"),
                ),
            )
            .changed()
        {
            self.plan_review_form.error = None;
        }
        if ui
            .add_enabled(
                self.activity.is_none(),
                theme::primary_button(self.language.text("Confirm application plan")),
            )
            .clicked()
        {
            self.confirm_application_plan(candidate, ui.ctx().clone());
        }
    }

    pub(super) fn load_plan_candidate(&mut self, ctx: egui::Context) {
        let Some((path, job_id)) = self.plan_review_subject() else {
            return;
        };
        self.plan_review_form.candidate = None;
        self.plan_review_form.current = None;
        self.plan_review_form.decision_confirmed = false;
        self.plan_review_form.error = None;
        self.dispatch(
            self.language.text("Loading application plan candidate"),
            ctx,
            WorkerRequest::LoadPlanCandidate { path, job_id },
        );
    }

    pub(super) fn load_current_plan(&mut self, ctx: egui::Context) {
        let Some((path, job_id)) = self.plan_review_subject() else {
            return;
        };
        self.plan_review_form.candidate = None;
        self.plan_review_form.current = None;
        self.plan_review_form.decision_confirmed = false;
        self.plan_review_form.error = None;
        self.dispatch(
            self.language.text("Loading current application plan"),
            ctx,
            WorkerRequest::LoadCurrentPlan { path, job_id },
        );
    }

    pub(super) fn confirm_application_plan(
        &mut self,
        candidate: ApplicationPlanCandidate,
        ctx: egui::Context,
    ) {
        if let Err(error) = validate_plan_review(
            &candidate,
            self.plan_review_form.decision_confirmed,
            self.language,
        ) {
            self.plan_review_form.error = Some(error);
            return;
        }
        let (Some(path), Some(job_id)) = (
            self.active_workspace.clone(),
            self.plan_review_form.job_id.clone(),
        ) else {
            self.plan_review_form.error = Some(
                self.language
                    .text("No active workspace or job is selected")
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language.text("Confirming application plan"),
            ctx,
            WorkerRequest::ConfirmPlan {
                path,
                job_id,
                candidate,
            },
        );
    }

    fn plan_review_subject(&mut self) -> Option<(PathBuf, String)> {
        let (Some(path), Some(job_id)) = (
            self.active_workspace.clone(),
            self.plan_review_form.job_id.clone(),
        ) else {
            self.plan_review_form.error = Some(
                self.language
                    .text("No active workspace or job is selected")
                    .to_owned(),
            );
            return None;
        };
        if !self.plan_review_form.private_read_consent {
            self.plan_review_form.error = Some(
                self.language
                    .text("Confirm private plan access before loading")
                    .to_owned(),
            );
            return None;
        }
        Some((path, job_id))
    }
}

#[allow(clippy::too_many_arguments)]
fn edit_text_list(
    ui: &mut egui::Ui,
    values: &mut Vec<String>,
    item_label: &str,
    add_label: &str,
    remove_label: &str,
    enabled: bool,
    allow_empty: bool,
) -> bool {
    let mut changed = false;
    let mut remove = None;
    let values_len = values.len();
    for (index, value) in values.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let label = ui.label(format!("{item_label} {}", index + 1));
            if ui
                .add_enabled(
                    enabled,
                    egui::TextEdit::singleline(value).desired_width(f32::INFINITY),
                )
                .labelled_by(label.id)
                .changed()
            {
                changed = true;
            }
            if ui
                .add_enabled(
                    enabled && (allow_empty || values_len > 1),
                    egui::Button::new(remove_label),
                )
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        values.remove(index);
        changed = true;
    }
    if ui
        .add_enabled(enabled, egui::Button::new(add_label))
        .clicked()
    {
        values.push(String::new());
        changed = true;
    }
    changed
}

fn application_decision_text(language: Language, decision: ApplicationDecision) -> &'static str {
    language.text(match decision {
        ApplicationDecision::Apply => "Apply",
        ApplicationDecision::Hold => "Hold",
        ApplicationDecision::Skip => "Skip",
    })
}

fn document_kind_text(language: Language, kind: DocumentKind) -> &'static str {
    language.text(match kind {
        DocumentKind::CoverLetter => "Cover letter",
        DocumentKind::ResearchStatement => "Research statement",
        DocumentKind::TeachingStatement => "Teaching statement",
        DocumentKind::Cv => "CV",
    })
}

fn document_requirement_text(language: Language, requirement: DocumentRequirement) -> &'static str {
    language.text(match requirement {
        DocumentRequirement::Required => "Required",
        DocumentRequirement::Optional => "Optional",
        DocumentRequirement::Omitted => "Omitted",
    })
}

fn plan_blocker_severity_text(language: Language, severity: PlanBlockerSeverity) -> &'static str {
    language.text(match severity {
        PlanBlockerSeverity::Blocking => "Blocking",
        PlanBlockerSeverity::Warning => "Warning",
    })
}
