use canisend_app::ApplicationFailure;
use canisend_contracts::{ArtifactReference, ConsentScope, TaskDescriptor, TaskStatus};

use super::*;

impl CanISendDesktop {
    pub(super) fn show_task_panel(&mut self, ui: &mut egui::Ui, job_id: &str) {
        self.task_form.select_job(job_id);
        accessible_heading(ui, self.language.select("Agent task", "Agent 任务"), 2);
        ui.separator();
        ui.label(self.language.select(
            "Prepare one bounded Agent v2 task from the current workflow. CanISend freezes exact input revisions and validates the returned JSON before any commit.",
            "从当前工作流准备一个受限的 Agent v2 任务。CanISend 会冻结精确的输入修订版本，并在提交前验证返回的 JSON。",
        ));
        ui.add_space(8.0);

        let current = self.task_form.state.clone();
        let may_prepare_new = current
            .as_ref()
            .is_none_or(|state| state.status == TaskStatus::Committed);
        if may_prepare_new {
            self.show_task_prepare_controls(ui, job_id, current.is_some());
        }

        if let Some(state) = current {
            ui.add_space(12.0);
            self.show_task_descriptor(ui, &state);
            match state.status {
                TaskStatus::Prepared => {
                    ui.add_space(12.0);
                    self.show_task_export(ui, &state.descriptor);
                    ui.add_space(12.0);
                    self.show_task_completion(ui, &state.descriptor);
                }
                TaskStatus::Cancelled | TaskStatus::Stale => {
                    ui.add_space(12.0);
                    self.show_task_recovery(ui, &state);
                }
                TaskStatus::Committed => {}
            }
        }

        if let Some(failure) = &self.task_form.failure {
            ui.add_space(10.0);
            show_task_failure(ui, self.language, self.dark_mode, failure);
        }
    }

    fn show_task_prepare_controls(
        &mut self,
        ui: &mut egui::Ui,
        job_id: &str,
        after_committed_task: bool,
    ) {
        let operations = available_task_operations(self.workflow_controls.as_ref());
        if operations.is_empty() {
            ui.label(
                RichText::new(self.language.select(
                    "No Agent task is ready. Complete the current workflow decision or prerequisite first.",
                    "当前没有可准备的 Agent 任务。请先完成当前工作流决策或前置步骤。",
                ))
                .weak(),
            );
            return;
        }
        if after_committed_task {
            accessible_heading(
                ui,
                self.language
                    .select("Prepare the next task", "准备下一个任务"),
                3,
            );
        }
        if !operations.contains(&self.task_form.operation) {
            self.task_form.operation = operations[0];
        }
        let previous_operation = self.task_form.operation;
        let operation_combo =
            egui::ComboBox::from_label(self.language.select("Task operation", "任务操作"))
                .selected_text(task_operation_label(
                    self.task_form.operation,
                    self.language,
                ))
                .show_ui(ui, |ui| {
                    for operation in operations.iter().copied() {
                        ui.selectable_value(
                            &mut self.task_form.operation,
                            operation,
                            task_operation_label(operation, self.language),
                        );
                    }
                });
        keep_focused_visible(&operation_combo.response);

        let mut modes =
            available_task_modes(self.workflow_controls.as_ref(), self.task_form.operation);
        if modes.is_empty() {
            modes = vec![TaskExecutionMode::HostAgent];
        }
        if previous_operation != self.task_form.operation || !modes.contains(&self.task_form.mode) {
            self.task_form.mode = modes[0];
        }
        let mode_combo =
            egui::ComboBox::from_label(self.language.select("Execution mode", "执行模式"))
                .selected_text(task_mode_label(self.task_form.mode, self.language))
                .show_ui(ui, |ui| {
                    for mode in modes {
                        ui.selectable_value(
                            &mut self.task_form.mode,
                            mode,
                            task_mode_label(mode, self.language),
                        );
                    }
                });
        keep_focused_visible(&mode_combo.response);
        ui.label(
            RichText::new(self.language.select(
                "Options are limited to ready workflow stages. Plan assignment and exact source revisions are rechecked when the task is prepared.",
                "选项仅来自已就绪的工作流阶段。准备任务时还会重新检查计划分配和精确来源修订版本。",
            ))
            .weak(),
        );
        let prepare = ui.add_enabled(
            self.activity.is_none(),
            theme::next_button(self.language.select("Prepare task", "准备任务"))
                .min_size(egui::vec2(150.0, 44.0)),
        );
        paint_focus_ring(ui, &prepare);
        keep_focused_visible(&prepare);
        if self.pending_focus == Some(FocusTarget::TaskPrepare) {
            prepare.request_focus();
            self.pending_focus = None;
        }
        if prepare.clicked() {
            self.prepare_task(job_id, ui.ctx().clone());
        }
    }

    fn show_task_descriptor(
        &mut self,
        ui: &mut egui::Ui,
        state: &canisend_contracts::TaskStateData,
    ) {
        let (status_color, status_text) =
            task_status_style(state.status, self.dark_mode, self.language);
        egui::Frame::new()
            .fill(if self.dark_mode {
                Color32::from_rgb(31, 45, 49)
            } else {
                Color32::WHITE
            })
            .stroke(Stroke::new(1.0, theme::SLATE_300))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    accessible_heading(
                        ui,
                        self.language.select("Current task", "当前任务"),
                        3,
                    );
                    ui.colored_label(status_color, RichText::new(status_text).strong());
                });
                task_metadata_row(
                    ui,
                    self.language.select("Task ID", "任务 ID"),
                    state.descriptor.id.as_str(),
                );
                task_metadata_row(
                    ui,
                    self.language.select("Operation", "操作"),
                    &format!(
                        "{} ({})",
                        task_operation_from_descriptor(&state.descriptor.operation)
                            .map_or(state.descriptor.operation.as_str(), |operation| {
                                task_operation_label(operation, self.language)
                            }),
                        state.descriptor.operation
                    ),
                );
                task_metadata_row(
                    ui,
                    self.language.select("Execution mode", "执行模式"),
                    execution_mode_label(state.descriptor.execution_mode, self.language),
                );
                task_metadata_row(
                    ui,
                    self.language.select("Lease expires", "租约到期时间"),
                    state.descriptor.lease.expires_at.as_str(),
                );
                task_metadata_row(
                    ui,
                    self.language.select("Job revision", "职位修订版本"),
                    &state.descriptor.job_revision.get().to_string(),
                );
                if let Some(profile_revision) = state.descriptor.profile_revision {
                    task_metadata_row(
                        ui,
                        self.language.select("Profile revision", "个人资料修订版本"),
                        &profile_revision.get().to_string(),
                    );
                }
                task_metadata_row(
                    ui,
                    self.language.select("Allowed output", "允许的输出"),
                    &format!("{:?}", state.descriptor.allowed_output_kind),
                );
                task_metadata_row(
                    ui,
                    self.language.select("Candidate schema", "候选 schema"),
                    &format!(
                        "{} · {}",
                        state.descriptor.candidate_schema.id,
                        state.descriptor.candidate_schema.version
                    ),
                );

                ui.add_space(8.0);
                ui.label(
                    RichText::new(self.language.select(
                        "Declared input artifacts",
                        "已声明的输入工件",
                    ))
                    .strong(),
                );
                for artifact in &state.descriptor.input_artifacts {
                    task_artifact_row(ui, artifact);
                }
                ui.label(
                    RichText::new(self.language.select(
                        "Only these exact revisions may be exported. Artifact bodies are not shown in this panel.",
                        "只能导出这些精确修订版本。本面板不会显示工件正文。",
                    ))
                    .weak(),
                );

                ui.add_space(8.0);
                ui.label(
                    RichText::new(self.language.select(
                        "Required consents",
                        "所需授权",
                    ))
                    .strong(),
                );
                for consent in &state.descriptor.required_consents {
                    ui.label(format!(
                        "• {} — {}",
                        consent_scope_label(consent.scope, self.language),
                        localized_consent_description(consent.scope, self.language)
                    ));
                    ui.label(
                        RichText::new(match self.language {
                            Language::English => {
                                format!("  Exact artifact scope: {}", consent.artifacts.len())
                            }
                            Language::SimplifiedChinese => {
                                format!("  精确工件范围：{}", consent.artifacts.len())
                            }
                        })
                        .weak(),
                    );
                }
                if let Some(result) = &state.result {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(self.language.select(
                            "Committed artifact",
                            "已提交工件",
                        ))
                        .strong()
                        .color(theme::positive(self.dark_mode)),
                    );
                    task_artifact_row(ui, result);
                }
            });
    }

    fn show_task_export(&mut self, ui: &mut egui::Ui, descriptor: &TaskDescriptor) {
        accessible_heading(
            ui,
            self.language.select("Export scoped inputs", "导出限定输入"),
            3,
        );
        ui.label(self.language.select(
            "Choose a new or empty directory. CanISend writes only the descriptor's declared artifacts and a digest-bound manifest.",
            "请选择一个新目录或空目录。CanISend 只会写入描述符中声明的工件和绑定摘要的清单。",
        ));
        ui.horizontal_wrapped(|ui| {
            let path = self.task_form.export_destination.as_ref().map_or_else(
                || {
                    self.language
                        .select("No directory selected", "未选择目录")
                        .to_owned()
                },
                |path| path.display().to_string(),
            );
            ui.add(egui::Label::new(path.clone()).truncate())
                .on_hover_text(path);
            let choose = ui.add_enabled(
                self.activity.is_none(),
                egui::Button::new(self.language.select("Choose directory", "选择目录"))
                    .min_size(egui::vec2(130.0, 44.0)),
            );
            paint_focus_ring(ui, &choose);
            if self.pending_focus == Some(FocusTarget::TaskExport) {
                choose.request_focus();
                self.pending_focus = None;
            }
            if choose.clicked() {
                self.task_form.export_destination = pick_directory(Some(self.language.select(
                    "Choose a new or empty task directory",
                    "选择新的或空的任务目录",
                )));
                self.task_form.exported = None;
                self.task_form.failure = None;
            }
        });
        let private = ui.checkbox(
            &mut self.task_form.private_read_consent,
            self.language.select(
                "Allow reading the exact private input revisions listed above",
                "允许读取上方列出的精确私有输入修订版本",
            ),
        );
        keep_focused_visible(&private);
        let provider_required = self.task_form.requires_provider_send();
        if provider_required {
            let provider = ui.checkbox(
                &mut self.task_form.provider_send_consent,
                self.language.select(
                    "Allow sending only this exact scope to the configured provider",
                    "允许仅将此精确范围发送给已配置的提供商",
                ),
            );
            keep_focused_visible(&provider);
        }
        let can_export = self.activity.is_none()
            && self.task_form.export_destination.is_some()
            && self.task_form.private_read_consent
            && (!provider_required || self.task_form.provider_send_consent);
        let export = ui
            .add_enabled(
                can_export,
                theme::primary_button(self.language.select("Export inputs", "导出输入"))
                    .min_size(egui::vec2(140.0, 44.0)),
            )
            .on_disabled_hover_text(self.language.select(
                "Choose a destination and confirm every required consent first",
                "请先选择目标目录并确认所有所需授权",
            ));
        paint_focus_ring(ui, &export);
        keep_focused_visible(&export);
        if export.clicked() {
            self.export_task_inputs(descriptor, ui.ctx().clone());
        }
        if let Some(exported) = &self.task_form.exported {
            ui.add_space(6.0);
            ui.colored_label(
                theme::positive(self.dark_mode),
                self.language
                    .select("Scoped export completed", "限定范围导出已完成"),
            );
            task_metadata_row(
                ui,
                self.language.select("Manifest SHA-256", "清单 SHA-256"),
                exported.manifest_sha256.as_str(),
            );
            for file in &exported.files {
                ui.label(format!(
                    "• {} — {:?} · r{}",
                    file.relative_path,
                    file.artifact.kind,
                    file.artifact.revision.get()
                ));
            }
        }
    }

    fn show_task_completion(&mut self, ui: &mut egui::Ui, descriptor: &TaskDescriptor) {
        accessible_heading(
            ui,
            self.language
                .select("Validate and commit completion", "验证并提交完成文件"),
            3,
        );
        ui.label(self.language.select(
            "Select one bounded canisend.task-completion/v2 JSON file. Preview validates schema, source spans, lease, job revision, and every input revision/hash without mutating the task.",
            "选择一个受限的 canisend.task-completion/v2 JSON 文件。预览会验证 schema、来源区间、租约、职位修订版本以及每个输入修订和哈希，且不会修改任务。",
        ));
        ui.horizontal_wrapped(|ui| {
            let path = self.task_form.completion_file.as_ref().map_or_else(
                || {
                    self.language
                        .select("No file selected", "未选择文件")
                        .to_owned()
                },
                |path| path.display().to_string(),
            );
            ui.add(egui::Label::new(path.clone()).truncate())
                .on_hover_text(path);
            let choose = ui.add_enabled(
                self.activity.is_none(),
                egui::Button::new(self.language.select("Choose JSON file", "选择 JSON 文件"))
                    .min_size(egui::vec2(130.0, 44.0)),
            );
            paint_focus_ring(ui, &choose);
            if self.pending_focus == Some(FocusTarget::TaskCompletionFile) {
                choose.request_focus();
                self.pending_focus = None;
            }
            if choose.clicked() {
                let selected = pick_task_completion_file();
                if selected != self.task_form.completion_file {
                    self.task_form.completion_file = selected;
                    self.task_form.completion_read_consent = false;
                    self.task_form.invalidate_completion_preview();
                }
            }
        });
        let consent = ui.checkbox(
            &mut self.task_form.completion_read_consent,
            self.language.select(
                "Allow reading this selected completion JSON for validation",
                "允许读取所选完成 JSON 以进行验证",
            ),
        );
        keep_focused_visible(&consent);
        if consent.changed() {
            self.task_form.invalidate_completion_preview();
        }
        let preview_enabled = self.activity.is_none()
            && self.task_form.completion_file.is_some()
            && self.task_form.completion_read_consent;
        let preview = ui
            .add_enabled(
                preview_enabled,
                egui::Button::new(self.language.select("Preview validation", "预览验证"))
                    .min_size(egui::vec2(145.0, 44.0)),
            )
            .on_disabled_hover_text(self.language.select(
                "Choose a JSON file and confirm read consent first",
                "请先选择 JSON 文件并确认读取授权",
            ));
        paint_focus_ring(ui, &preview);
        keep_focused_visible(&preview);
        if preview.clicked() {
            self.preview_task_completion(ui.ctx().clone());
        }

        if let Some(reviewed) = self.task_form.completion_preview.clone() {
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(if self.dark_mode {
                    Color32::from_rgb(26, 72, 65)
                } else {
                    theme::TEAL_100
                })
                .corner_radius(6)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(self.language.select(
                            "Validation passed — not yet committed",
                            "验证通过——尚未提交",
                        ))
                        .strong(),
                    );
                    task_metadata_row(
                        ui,
                        self.language.select("Task ID", "任务 ID"),
                        reviewed.request.task_id.as_str(),
                    );
                    task_metadata_row(
                        ui,
                        self.language.select("Lease ID", "租约 ID"),
                        reviewed.request.lease_id.as_str(),
                    );
                    task_metadata_row(
                        ui,
                        self.language.select("Expected job revision", "预期职位修订版本"),
                        &reviewed.request.expected_job_revision.get().to_string(),
                    );
                    ui.label(match self.language {
                        Language::English => format!(
                            "Exact input revisions reviewed: {}",
                            reviewed.request.expected_inputs.len()
                        ),
                        Language::SimplifiedChinese => format!(
                            "已审阅的精确输入修订版本：{}",
                            reviewed.request.expected_inputs.len()
                        ),
                    });
                    ui.label(
                        RichText::new(self.language.select(
                            "Commit uses this in-memory reviewed request even if the source file changes.",
                            "即使源文件随后发生变化，提交仍会使用此内存中已审阅的请求。",
                        ))
                        .weak(),
                    );
                });
            let commit = ui.add_enabled(
                self.activity.is_none(),
                theme::next_button(
                    self.language
                        .select("Commit reviewed completion", "提交已审阅的完成结果"),
                )
                .min_size(egui::vec2(200.0, 44.0)),
            );
            paint_focus_ring(ui, &commit);
            keep_focused_visible(&commit);
            if self.pending_focus == Some(FocusTarget::TaskCommit) {
                commit.request_focus();
                self.pending_focus = None;
            }
            if commit.clicked() {
                self.commit_task_completion(reviewed.request, ui.ctx().clone());
            }
        }

        if self.task_form.stale_detected {
            ui.colored_label(
                theme::warning(self.dark_mode),
                self.language.select(
                    "Stale task detected. Preview did not mutate it; cancel this lease, then prepare again against current revisions.",
                    "检测到任务已过期。预览未修改任务；请先取消此租约，再根据当前修订版本重新准备。",
                ),
            );
        }
        let cancel = ui.add_enabled(
            self.activity.is_none(),
            theme::destructive_button(
                self.language
                    .select("Cancel prepared task", "取消已准备的任务"),
            )
            .min_size(egui::vec2(170.0, 44.0)),
        );
        paint_focus_ring(ui, &cancel);
        keep_focused_visible(&cancel);
        if self.pending_focus == Some(FocusTarget::TaskCancel) {
            cancel.request_focus();
            self.pending_focus = None;
        }
        if cancel.clicked() {
            self.pending_confirmation = Some(PendingConfirmation::CancelTask {
                task_id: descriptor.id.to_string(),
                operation: descriptor.operation.clone(),
            });
        }
    }

    fn show_task_recovery(&mut self, ui: &mut egui::Ui, state: &canisend_contracts::TaskStateData) {
        let message = match state.status {
            TaskStatus::Cancelled => self.language.select(
                "This task is cancelled. Prepare again to create a new lease against current revisions.",
                "此任务已取消。重新准备会根据当前修订版本创建新租约。",
            ),
            TaskStatus::Stale => self.language.select(
                "This task is stale and cannot be reused. Prepare again against current revisions.",
                "此任务已过期，不能重复使用。请根据当前修订版本重新准备。",
            ),
            TaskStatus::Prepared | TaskStatus::Committed => return,
        };
        ui.colored_label(theme::warning(self.dark_mode), message);
        let prepare_again = ui.add_enabled(
            self.activity.is_none() && self.task_form.can_prepare_again(),
            theme::next_button(self.language.select("Prepare again", "重新准备"))
                .min_size(egui::vec2(150.0, 44.0)),
        );
        paint_focus_ring(ui, &prepare_again);
        keep_focused_visible(&prepare_again);
        if self.pending_focus == Some(FocusTarget::TaskPrepareAgain) {
            prepare_again.request_focus();
            self.pending_focus = None;
        }
        if prepare_again.clicked() {
            self.prepare_task_again(state.descriptor.id.as_str(), ui.ctx().clone());
        }
    }

    fn prepare_task(&mut self, job_id: &str, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        let request = match TaskPrepareRequest::try_new(
            job_id,
            self.task_form.operation,
            self.task_form.mode,
        ) {
            Ok(request) => request,
            Err(error) => {
                self.apply_task_failure(error.classify(), FocusTarget::TaskPrepare);
                return;
            }
        };
        self.task_form.failure = None;
        self.dispatch(
            self.language.select("Preparing task", "正在准备任务"),
            ctx,
            WorkerRequest::PrepareTask { path, request },
        );
    }

    fn export_task_inputs(&mut self, descriptor: &TaskDescriptor, ctx: egui::Context) {
        let (Some(path), Some(destination)) = (
            self.active_workspace.clone(),
            self.task_form.export_destination.clone(),
        ) else {
            return;
        };
        let request = TaskInputExportRequest {
            task_id: descriptor.id.clone(),
            destination,
        };
        self.task_form.failure = None;
        self.dispatch(
            self.language
                .select("Exporting scoped inputs", "正在导出限定输入"),
            ctx,
            WorkerRequest::ExportTaskInputs {
                path,
                request,
                private_read_consent: self.task_form.private_read_consent,
                provider_send_consent: self.task_form.provider_send_consent,
            },
        );
    }

    fn preview_task_completion(&mut self, ctx: egui::Context) {
        let (Some(path), Some(file)) = (
            self.active_workspace.clone(),
            self.task_form.completion_file.clone(),
        ) else {
            return;
        };
        self.task_form.failure = None;
        self.dispatch(
            self.language
                .select("Validating task completion", "正在验证任务完成文件"),
            ctx,
            WorkerRequest::PreviewTaskCompletion { path, file },
        );
    }

    fn commit_task_completion(
        &mut self,
        request: canisend_contracts::TaskCompletionRequest,
        ctx: egui::Context,
    ) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.task_form.failure = None;
        self.dispatch(
            self.language
                .select("Committing task completion", "正在提交任务完成结果"),
            ctx,
            WorkerRequest::CommitTaskCompletion { path, request },
        );
    }

    pub(super) fn cancel_task(&mut self, task_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.task_form.failure = None;
        self.dispatch(
            self.language.select("Cancelling task", "正在取消任务"),
            ctx,
            WorkerRequest::CancelTask { path, task_id },
        );
    }

    fn prepare_task_again(&mut self, task_id: &str, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.task_form.failure = None;
        self.dispatch(
            self.language
                .select("Preparing replacement task", "正在准备替代任务"),
            ctx,
            WorkerRequest::PrepareTaskAgain {
                path,
                task_id: task_id.to_owned(),
            },
        );
    }
}

fn task_operation_label(operation: TaskOperation, language: Language) -> &'static str {
    match operation {
        TaskOperation::JobParse => language.select("Parse job advert", "解析职位广告"),
        TaskOperation::EvidenceNormalize => {
            language.select("Normalize profile evidence", "规范化个人资料证据")
        }
        TaskOperation::EvidenceMatch => language.select("Match evidence", "匹配证据"),
        TaskOperation::CoverLetterDraft => language.select("Draft cover letter", "起草求职信"),
        TaskOperation::ResearchStatementDraft => {
            language.select("Draft research statement", "起草研究陈述")
        }
        TaskOperation::TeachingStatementDraft => {
            language.select("Draft teaching statement", "起草教学陈述")
        }
        TaskOperation::CvDraft => language.select("Draft CV", "起草简历"),
        TaskOperation::DocumentReview => language.select("Review documents", "审阅文档"),
    }
}

fn task_operation_from_descriptor(operation: &str) -> Option<TaskOperation> {
    TaskOperation::ALL
        .into_iter()
        .find(|candidate| match candidate {
            TaskOperation::JobParse => operation == "job.parse",
            TaskOperation::EvidenceNormalize => operation == "profile.evidence.normalize",
            TaskOperation::EvidenceMatch => operation == "evidence.match",
            TaskOperation::CoverLetterDraft => operation == "document.draft.cover-letter",
            TaskOperation::ResearchStatementDraft => {
                operation == "document.draft.research-statement"
            }
            TaskOperation::TeachingStatementDraft => {
                operation == "document.draft.teaching-statement"
            }
            TaskOperation::CvDraft => operation == "document.draft.cv",
            TaskOperation::DocumentReview => operation == "document.review",
        })
}

fn task_mode_label(mode: TaskExecutionMode, language: Language) -> &'static str {
    match mode {
        TaskExecutionMode::HostAgent => language.select("Host agent", "宿主 Agent"),
        TaskExecutionMode::ConfiguredProvider => {
            language.select("Configured provider", "已配置的提供商")
        }
    }
}

fn task_status_style(
    status: TaskStatus,
    dark: bool,
    language: Language,
) -> (Color32, &'static str) {
    match status {
        TaskStatus::Prepared => (theme::info(dark), language.select("Prepared", "已准备")),
        TaskStatus::Committed => (
            theme::positive(dark),
            language.select("Committed", "已提交"),
        ),
        TaskStatus::Cancelled => (theme::neutral(dark), language.select("Cancelled", "已取消")),
        TaskStatus::Stale => (theme::error(dark), language.select("Stale", "已过期")),
    }
}

fn consent_scope_label(scope: ConsentScope, language: Language) -> &'static str {
    match scope {
        ConsentScope::ReadPrivateInputs => language.select("Read private inputs", "读取私有输入"),
        ConsentScope::SendToConfiguredProvider => {
            language.select("Send to configured provider", "发送到已配置的提供商")
        }
        ConsentScope::FetchUserSuppliedUrl => {
            language.select("Fetch user-supplied URL", "读取用户提供的 URL")
        }
        ConsentScope::ExportPrivateArtifacts => {
            language.select("Export private artifacts", "导出私有工件")
        }
        ConsentScope::UseSystemFonts => language.select("Use system fonts", "使用系统字体"),
    }
}

fn localized_consent_description(scope: ConsentScope, language: Language) -> &'static str {
    match scope {
        ConsentScope::ReadPrivateInputs => language.select(
            "Read only the exact artifact revisions declared by this task",
            "仅读取此任务声明的精确工件修订版本",
        ),
        ConsentScope::SendToConfiguredProvider => language.select(
            "Send only the exact declared scope after separate approval",
            "仅在单独授权后发送精确声明的范围",
        ),
        ConsentScope::FetchUserSuppliedUrl => {
            language.select("Fetch only the user-supplied URL", "仅读取用户提供的 URL")
        }
        ConsentScope::ExportPrivateArtifacts => language.select(
            "Export only declared private artifacts",
            "仅导出已声明的私有工件",
        ),
        ConsentScope::UseSystemFonts => language.select(
            "Use explicitly approved system fonts",
            "使用明确授权的系统字体",
        ),
    }
}

fn task_metadata_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{label}:")).strong());
        ui.monospace(value);
    });
}

fn task_artifact_row(ui: &mut egui::Ui, artifact: &ArtifactReference) {
    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "• {:?} · {} · r{}",
            artifact.kind,
            artifact.id,
            artifact.revision.get()
        ));
        ui.monospace(artifact.sha256.as_str());
    });
}

fn show_task_failure(
    ui: &mut egui::Ui,
    language: Language,
    dark_mode: bool,
    failure: &ApplicationFailure,
) {
    accessible_error(
        ui,
        theme::error(dark_mode),
        &format!(
            "{} [{}]: {}",
            language.select("Task action failed", "任务操作失败"),
            failure.code.as_str(),
            failure.message
        ),
    );
    if let Some(violations) = failure
        .details
        .as_ref()
        .and_then(serde_json::Value::as_array)
        && !violations.is_empty()
    {
        ui.label(RichText::new(language.select("Field-level validation", "字段级验证")).strong());
        for violation in violations {
            let pointer = violation
                .get("json_pointer")
                .and_then(serde_json::Value::as_str)
                .filter(|pointer| !pointer.is_empty())
                .unwrap_or("/");
            let code = violation
                .get("code")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("candidate.invalid");
            let message = violation
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| {
                    language.select("Candidate validation failed", "候选内容验证失败")
                });
            ui.group(|ui| {
                task_metadata_row(ui, language.select("JSON pointer", "JSON 指针"), pointer);
                task_metadata_row(ui, language.select("Code", "代码"), code);
                ui.label(message);
            });
        }
    }
    if let Some(remediation) = &failure.remediation {
        ui.label(
            RichText::new(format!(
                "{}: {} — {}",
                language.select("Recovery", "恢复操作"),
                remediation.action,
                remediation.description
            ))
            .weak(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        consent_scope_label, task_operation_from_descriptor, task_operation_label,
        task_status_style,
    };
    use crate::{i18n::Language, theme};
    use canisend_app::TaskOperation;
    use canisend_contracts::{ConsentScope, TaskStatus};

    #[test]
    fn task_labels_are_bilingual_and_status_is_never_color_only() {
        assert_eq!(
            task_operation_label(TaskOperation::DocumentReview, Language::SimplifiedChinese),
            "审阅文档"
        );
        assert_eq!(
            task_operation_from_descriptor("document.draft.cv"),
            Some(TaskOperation::CvDraft)
        );
        assert_eq!(
            consent_scope_label(
                ConsentScope::SendToConfiguredProvider,
                Language::SimplifiedChinese
            ),
            "发送到已配置的提供商"
        );
        let (color, label) =
            task_status_style(TaskStatus::Stale, false, Language::SimplifiedChinese);
        assert_eq!(label, "已过期");
        assert_eq!(color, theme::error(false));
    }
}
