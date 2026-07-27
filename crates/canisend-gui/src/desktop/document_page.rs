use super::*;

impl CanISendDesktop {
    pub(super) fn show_job_panel_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for panel in JobPanel::ALL {
                let selected = self.job_panel == panel;
                let response = ui.add(
                    egui::Button::new(panel.label(self.language))
                        .selected(selected)
                        .min_size(egui::vec2(132.0, 44.0)),
                );
                paint_focus_ring(ui, &response);
                keep_focused_visible(&response);
                if response.clicked() {
                    self.job_panel = panel;
                }
            }
        });
    }

    pub(super) fn show_document_workspace(&mut self, ui: &mut egui::Ui, job_id: &str) {
        self.document_form.select_job(job_id);

        accessible_heading(
            ui,
            self.language
                .select("Structured application documents", "结构化申请文档"),
            2,
        );
        ui.label(self.language.select(
            "Inspect the exact revision-bound drafts accepted by the durable workflow. Draft creation and replacement remain Agent-task operations.",
            "检查耐久工作流接收的、与精确修订版本绑定的草稿。创建或替换草稿仍通过 Agent 任务完成。",
        ));
        ui.colored_label(
            theme::warning(self.dark_mode),
            self.language.select(
                "Document titles, sections, claims, and placeholders are private local application material.",
                "文档标题、章节、陈述和占位符属于本地私有申请材料。",
            ),
        );
        ui.add_space(10.0);

        let consent = ui.add_enabled(
            self.activity.is_none(),
            egui::Checkbox::new(
                &mut self.document_form.private_read_consent,
                self.language.select(
                    "Allow this user-invoked private document review",
                    "允许本次由用户发起的私有申请文档审阅",
                ),
            ),
        );
        if consent.changed() {
            self.document_form.error = None;
            if !self.document_form.private_read_consent {
                self.document_form.clear_loaded_private_data();
            }
        }
        let load = ui.add_enabled(
            self.activity.is_none(),
            theme::primary_button(
                self.language
                    .select("Load current documents", "加载当前申请文档"),
            ),
        );
        if self.pending_focus == Some(FocusTarget::DocumentLoad) {
            load.request_focus();
            self.pending_focus = None;
        }
        if load.clicked() {
            if self.document_form.private_read_consent {
                self.document_form.error = None;
                self.load_documents(job_id.to_owned(), ui.ctx().clone());
            } else {
                self.document_form.error = Some(
                    self.language
                        .select(
                            "Confirm private document access before loading",
                            "加载前请确认允许访问私有申请文档",
                        )
                        .to_owned(),
                );
                self.pending_focus = Some(FocusTarget::DocumentLoad);
            }
        }
        if let Some(error) = &self.document_form.error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }

        let Some(documents) = self.document_form.documents.clone() else {
            ui.add_space(12.0);
            ui.label(
                RichText::new(self.language.select(
                    "No private document bodies are loaded.",
                    "尚未加载任何私有申请文档正文。",
                ))
                .weak(),
            );
            return;
        };

        ui.add_space(18.0);
        self.show_accepted_document_set(ui, job_id);
        ui.add_space(18.0);
        accessible_heading(
            ui,
            self.language
                .select("Current document members", "当前文档成员"),
            3,
        );
        ui.label(match self.language {
            Language::English => format!("{} structured draft(s)", documents.len()),
            Language::SimplifiedChinese => format!("{} 份结构化草稿", documents.len()),
        });

        if documents.is_empty() {
            ui.label(self.language.select(
                "No current structured draft exists. Prepare the next document task from Workflow.",
                "尚无当前结构化草稿。请从“工作流”准备下一项文档任务。",
            ));
            if ui
                .add(theme::next_button(
                    self.language
                        .select("Open workflow tasks", "打开工作流任务"),
                ))
                .clicked()
            {
                self.job_panel = JobPanel::Workflow;
                self.pending_focus = Some(FocusTarget::TaskPrepare);
            }
            return;
        }

        for document in documents {
            ui.add_space(10.0);
            let header = format!(
                "{} · {} · r{}",
                document_kind_label(self.language, document.kind),
                document.title,
                document.revision.get()
            );
            egui::CollapsingHeader::new(header)
                .id_salt(("document", document.id.as_str(), document.revision.get()))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new(("document_metadata", document.id.as_str()))
                        .num_columns(2)
                        .spacing([16.0, 6.0])
                        .show(ui, |ui| {
                            diagnostic_row(
                                ui,
                                self.language.select("Document ID", "文档 ID"),
                                document.id.as_str(),
                            );
                            diagnostic_row(
                                ui,
                                self.language.select("Plan artifact", "申请计划工件"),
                                &artifact_label(&document.plan_artifact),
                            );
                            diagnostic_row(
                                ui,
                                self.language.select("Planned document", "计划文档"),
                                &format!(
                                    "{}@{}",
                                    document.planned_document.id,
                                    document.planned_document.revision.get()
                                ),
                            );
                            diagnostic_row(
                                ui,
                                self.language.select("Generated by", "生成方式"),
                                execution_mode_label(
                                    document.generation.execution_mode,
                                    self.language,
                                ),
                            );
                            diagnostic_row(
                                ui,
                                self.language.select("Task", "任务"),
                                document.generation.task_id.as_str(),
                            );
                            diagnostic_row(
                                ui,
                                self.language.select("Prompt resource", "提示资源"),
                                &document.generation.prompt_resource_id,
                            );
                        });

                    ui.add_space(12.0);
                    accessible_heading(ui, self.language.select("Sections", "章节"), 4);
                    for (index, section) in document.sections.iter().enumerate() {
                        egui::Frame::new()
                            .fill(ui.visuals().faint_bg_color)
                            .stroke(Stroke::new(1.0, theme::SLATE_300))
                            .corner_radius(6)
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} {} · {} · r{}",
                                            self.language.select("Section", "章节"),
                                            index + 1,
                                            section_kind_label(self.language, section.kind),
                                            section.revision.get()
                                        ))
                                        .strong(),
                                    );
                                    if let Some(heading) = &section.heading {
                                        ui.label(heading);
                                    }
                                });
                                ui.add(egui::Label::new(&section.body).wrap());
                                if !section.claims.is_empty() {
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new(self.language.select("Claims", "陈述"))
                                            .strong(),
                                    );
                                    for claim in &section.claims {
                                        ui.label(format!(
                                            "• {} · {} · r{}",
                                            claim_classification_label(
                                                self.language,
                                                claim.classification
                                            ),
                                            claim.text,
                                            claim.revision.get()
                                        ));
                                        for citation in &claim.citations {
                                            ui.label(
                                                RichText::new(format!(
                                                    "  ↳ {} · {}",
                                                    citation_target_label(&citation.target),
                                                    citation.purpose
                                                ))
                                                .weak(),
                                            );
                                        }
                                    }
                                }
                            });
                        ui.add_space(8.0);
                    }

                    accessible_heading(ui, self.language.select("Placeholders", "占位符"), 4);
                    if document.placeholders.is_empty() {
                        ui.label(self.language.select("No placeholders.", "没有占位符。"));
                    }
                    for placeholder in &document.placeholders {
                        let state = if placeholder.resolution.is_some() {
                            self.language.select("Resolved", "已解决")
                        } else if placeholder.required {
                            self.language
                                .select("Required and unresolved", "必填且尚未解决")
                        } else {
                            self.language
                                .select("Optional and unresolved", "可选且尚未解决")
                        };
                        ui.label(RichText::new(format!("{} · {state}", placeholder.key)).strong());
                        ui.label(&placeholder.instruction);
                        if let Some(resolution) = &placeholder.resolution {
                            ui.label(resolution);
                        }
                    }
                });
        }
    }

    fn show_accepted_document_set(&mut self, ui: &mut egui::Ui, job_id: &str) {
        accessible_heading(
            ui,
            self.language
                .select("Accepted document set", "已接收的文档集合"),
            3,
        );
        if let Some(set) = &self.document_form.accepted_set {
            ui.colored_label(
                theme::positive(self.dark_mode),
                self.language.select(
                    "Complete · every member matches the exact current document head",
                    "已完成 · 每个成员都与当前文档头的精确修订版本一致",
                ),
            );
            egui::Grid::new(("document_set", set.id.as_str()))
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    diagnostic_row(
                        ui,
                        self.language.select("Set ID", "集合 ID"),
                        set.id.as_str(),
                    );
                    diagnostic_row(
                        ui,
                        self.language.select("Set revision", "集合修订"),
                        &set.revision.get().to_string(),
                    );
                    diagnostic_row(
                        ui,
                        self.language.select("Plan artifact", "申请计划工件"),
                        &artifact_label(&set.plan_artifact),
                    );
                    diagnostic_row(
                        ui,
                        self.language.select("Members", "成员"),
                        &set.documents.len().to_string(),
                    );
                });
            for member in &set.documents {
                ui.monospace(artifact_label(member));
            }
            return;
        }

        ui.colored_label(
            theme::warning(self.dark_mode),
            self.language.select(
                "The Draft stage has not accepted a complete current document set.",
                "草稿阶段尚未接收完整的当前文档集合。",
            ),
        );
        if let Some(blocker) = &self.document_form.acceptance_blocker {
            ui.label(blocker);
        }
        ui.label(self.language.select(
            "Create every required draft through the revision-bound Agent task panel. CanISend assembles the accepted set only after all current members satisfy the plan.",
            "请通过与修订版本绑定的 Agent 任务面板创建所有必需草稿。只有当前全部成员满足申请计划后，CanISend 才会组装已接收集合。",
        ));
        if let Some(controls) = &self.workflow_controls {
            for action in controls.status.next_actions.iter().take(3) {
                command_copy_row(ui, &action.action, self.language);
            }
        }
        if ui
            .add(theme::next_button(
                self.language
                    .select("Open workflow tasks", "打开工作流任务"),
            ))
            .clicked()
        {
            self.job_panel = JobPanel::Workflow;
            self.pending_focus = Some(FocusTarget::TaskPrepare);
        }
        ui.label(
            RichText::new(match self.language {
                Language::English => format!("Job {job_id}"),
                Language::SimplifiedChinese => format!("职位 {job_id}"),
            })
            .weak(),
        );
    }
}

fn artifact_label(reference: &canisend_contracts::ArtifactReference) -> String {
    format!(
        "{:?} · {}@{} · {}",
        reference.kind,
        reference.id,
        reference.revision.get(),
        reference.sha256
    )
}

fn document_kind_label(language: Language, kind: DocumentKind) -> &'static str {
    language.select(
        match kind {
            DocumentKind::CoverLetter => "Cover letter",
            DocumentKind::ResearchStatement => "Research statement",
            DocumentKind::TeachingStatement => "Teaching statement",
            DocumentKind::Cv => "CV",
        },
        match kind {
            DocumentKind::CoverLetter => "求职信",
            DocumentKind::ResearchStatement => "研究陈述",
            DocumentKind::TeachingStatement => "教学陈述",
            DocumentKind::Cv => "简历",
        },
    )
}

fn section_kind_label(
    language: Language,
    kind: canisend_contracts::DocumentSectionKind,
) -> &'static str {
    use canisend_contracts::DocumentSectionKind;
    language.select(
        match kind {
            DocumentSectionKind::Opening => "Opening",
            DocumentSectionKind::Fit => "Fit",
            DocumentSectionKind::Research => "Research",
            DocumentSectionKind::Teaching => "Teaching",
            DocumentSectionKind::Service => "Service",
            DocumentSectionKind::Experience => "Experience",
            DocumentSectionKind::Education => "Education",
            DocumentSectionKind::Publications => "Publications",
            DocumentSectionKind::Skills => "Skills",
            DocumentSectionKind::Closing => "Closing",
            DocumentSectionKind::Other => "Other",
        },
        match kind {
            DocumentSectionKind::Opening => "开篇",
            DocumentSectionKind::Fit => "匹配",
            DocumentSectionKind::Research => "研究",
            DocumentSectionKind::Teaching => "教学",
            DocumentSectionKind::Service => "服务",
            DocumentSectionKind::Experience => "经历",
            DocumentSectionKind::Education => "教育",
            DocumentSectionKind::Publications => "出版物",
            DocumentSectionKind::Skills => "技能",
            DocumentSectionKind::Closing => "结尾",
            DocumentSectionKind::Other => "其他",
        },
    )
}

fn claim_classification_label(
    language: Language,
    classification: canisend_contracts::ClaimClassification,
) -> &'static str {
    use canisend_contracts::ClaimClassification;
    language.select(
        match classification {
            ClaimClassification::ApplicantFact => "Applicant fact",
            ClaimClassification::JobRequirement => "Job requirement",
            ClaimClassification::UserIntent => "User intent",
            ClaimClassification::NonFactual => "Non-factual",
        },
        match classification {
            ClaimClassification::ApplicantFact => "申请人事实",
            ClaimClassification::JobRequirement => "职位要求",
            ClaimClassification::UserIntent => "用户意图",
            ClaimClassification::NonFactual => "非事实性陈述",
        },
    )
}

fn citation_target_label(target: &canisend_contracts::CitationTarget) -> String {
    match target {
        canisend_contracts::CitationTarget::Evidence { evidence } => {
            format!("evidence {}@{}", evidence.id, evidence.revision.get())
        }
        canisend_contracts::CitationTarget::Criterion { criterion } => {
            format!("criterion {}@{}", criterion.id, criterion.revision.get())
        }
    }
}

#[cfg(test)]
mod tests {
    use canisend_contracts::DocumentKind;

    use super::document_kind_label;
    use crate::i18n::Language;

    #[test]
    fn document_workspace_labels_cover_both_supported_languages() {
        assert_eq!(
            document_kind_label(Language::English, DocumentKind::CoverLetter),
            "Cover letter"
        );
        assert_eq!(
            document_kind_label(Language::SimplifiedChinese, DocumentKind::CoverLetter),
            "求职信"
        );
    }
}
