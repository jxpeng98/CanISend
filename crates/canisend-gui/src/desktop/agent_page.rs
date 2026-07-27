use super::*;

impl CanISendDesktop {
    pub(super) fn show_agent_integration(&mut self, ui: &mut egui::Ui) {
        let interaction_height = ui.spacing().interact_size.y.max(44.0);
        ui.spacing_mut().interact_size.y = interaction_height;
        self.page_header(
            ui,
            self.language.select("Agent integration", "Agent 集成"),
            self.language.select(
                "Inspect body-free Agent v2 state and export a verified host pack",
                "检查不含正文的 Agent v2 状态，并导出经过验证的宿主资源包",
            ),
        );
        ui.label(self.language.select(
            "This page never sends private source bodies, executes a host, or runs a general shell.",
            "此页面不会发送私有来源正文、执行 Agent 宿主或运行通用 Shell。",
        ));
        ui.add_space(12.0);

        self.show_agent_context_controls(ui);
        ui.add_space(14.0);

        if let Some(capabilities) = self.agent_form.capabilities.clone() {
            show_agent_capabilities(ui, self.language, self.dark_mode, &capabilities);
            ui.add_space(14.0);
        }
        if let Some(context) = self.agent_form.context.clone() {
            self.show_agent_context(ui, &context);
            ui.add_space(14.0);
        } else if self.activity.is_none() && self.agent_form.failure.is_none() {
            ui.label(
                RichText::new(self.language.select(
                    "Load context to inspect the current workspace or optional job.",
                    "加载上下文以检查当前工作区或可选职位。",
                ))
                .weak(),
            );
        }

        self.show_agent_pack_export(ui);

        if let Some(failure) = self.agent_form.failure.clone() {
            ui.add_space(12.0);
            show_agent_failure(ui, self.language, self.dark_mode, &failure);
        }
    }

    fn show_agent_context_controls(&mut self, ui: &mut egui::Ui) {
        accessible_heading(ui, self.language.select("Context scope", "上下文范围"), 2);
        ui.label(self.language.select(
            "Choose no job for a workspace summary, or select one active job for bounded guidance.",
            "不选择职位可查看工作区摘要，或选择一个活跃职位以获取限定范围的指引。",
        ));

        let mut selected_job_id = self.agent_form.selected_job_id.clone();
        let selected_label = selected_job_id
            .as_deref()
            .and_then(|job_id| self.jobs.iter().find(|job| job.id.as_str() == job_id))
            .map_or_else(
                || {
                    self.language
                        .select("Workspace overview", "工作区概览")
                        .to_owned()
                },
                |job| format!("{} — {}", job.title, job.institution),
            );
        let job_combo = ui
            .add_enabled_ui(self.activity.is_none(), |ui| {
                egui::ComboBox::from_label(self.language.select("Optional job", "可选职位"))
                    .selected_text(selected_label)
                    .width(ui.available_width().clamp(120.0, 360.0))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut selected_job_id,
                            None,
                            self.language.select("Workspace overview", "工作区概览"),
                        );
                        for job in self.jobs.iter().filter(|job| !job.archived) {
                            ui.selectable_value(
                                &mut selected_job_id,
                                Some(job.id.to_string()),
                                format!("{} — {}", job.title, job.institution),
                            );
                        }
                    })
            })
            .inner;
        paint_focus_ring(ui, &job_combo.response);
        keep_focused_visible(&job_combo.response);

        if selected_job_id != self.agent_form.selected_job_id {
            self.agent_form.select_job(selected_job_id);
            self.load_agent_context(ui.ctx().clone());
        }

        let refresh = ui.add_enabled(
            self.activity.is_none(),
            egui::Button::new(
                self.language
                    .select("Refresh body-free context", "刷新不含正文的上下文"),
            )
            .min_size(egui::vec2(210.0, 44.0)),
        );
        paint_focus_ring(ui, &refresh);
        keep_focused_visible(&refresh);
        if self.pending_focus == Some(FocusTarget::AgentContextRefresh) {
            refresh.request_focus();
            self.pending_focus = None;
        }
        if refresh.clicked() {
            self.load_agent_integration(ui.ctx().clone());
        }
    }

    fn show_agent_context(
        &mut self,
        ui: &mut egui::Ui,
        context: &canisend_app::AgentContextReadModel,
    ) {
        accessible_heading(
            ui,
            self.language
                .select("Body-free context", "不含正文的上下文"),
            2,
        );
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
                agent_metadata_row(ui, "Protocol", &context.protocol);
                agent_metadata_row(
                    ui,
                    self.language.select("Privacy", "隐私级别"),
                    &format!("{:?}", context.privacy),
                );
                if let Some(workspace) = &context.workspace {
                    agent_metadata_row(
                        ui,
                        self.language.select("Workspace ID", "工作区 ID"),
                        workspace.workspace_id.as_str(),
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Active jobs", "活跃职位"),
                        &workspace.active_job_count.to_string(),
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Open tasks", "开放任务"),
                        &workspace.open_task_count.to_string(),
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Stale artifacts", "过期工件"),
                        &workspace.stale_artifact_count.to_string(),
                    );
                }
                if let Some(job) = &context.selected_job {
                    ui.add_space(8.0);
                    accessible_heading(ui, self.language.select("Selected job", "已选职位"), 3);
                    agent_metadata_row(
                        ui,
                        self.language.select("Job", "职位"),
                        &format!("{} — {}", job.title, job.institution),
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Job ID", "职位 ID"),
                        job.id.as_str(),
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Source count", "来源数量"),
                        &job.source_count.to_string(),
                    );
                }
            });

        ui.add_space(10.0);
        accessible_heading(ui, self.language.select("Blockers", "阻塞项"), 3);
        if context.blockers.is_empty() {
            ui.colored_label(
                theme::positive(self.dark_mode),
                self.language.select("No blockers reported", "没有阻塞项"),
            );
        } else {
            for blocker in &context.blockers {
                let text = blocker.subject_id.as_ref().map_or_else(
                    || format!("{} — {}", blocker.code, blocker.description),
                    |subject| format!("{} — {} [{}]", blocker.code, blocker.description, subject),
                );
                if agent_copy_row(
                    ui,
                    self.language,
                    &text,
                    self.language.select("Copy blocker", "复制阻塞项"),
                ) {
                    self.notice = Some((
                        true,
                        self.language
                            .select("Blocker copied", "阻塞项已复制")
                            .to_owned(),
                    ));
                }
            }
        }

        ui.add_space(10.0);
        accessible_heading(ui, self.language.select("Next actions", "后续操作"), 3);
        if context.next_actions.is_empty() {
            ui.label(
                RichText::new(
                    self.language
                        .select("No next action reported", "没有后续操作"),
                )
                .weak(),
            );
        } else {
            for action in &context.next_actions {
                ui.label(&action.description);
                if agent_copy_row(
                    ui,
                    self.language,
                    &action.action,
                    self.language.select("Copy action", "复制操作"),
                ) {
                    self.notice = Some((
                        true,
                        self.language
                            .select("Bounded action copied", "限定操作已复制")
                            .to_owned(),
                    ));
                }
            }
        }
    }

    fn show_agent_pack_export(&mut self, ui: &mut egui::Ui) {
        accessible_heading(
            ui,
            self.language
                .select("Export host resource pack", "导出宿主资源包"),
            2,
        );
        ui.label(self.language.select(
            "Choose one host and preview a new or empty destination. Export writes verified guides, prompts, schemas, examples, and one manifest; it does not launch the host.",
            "选择一个宿主并预览新的或空的目标目录。导出只会写入经过验证的指南、提示词、Schema、示例和一份清单，不会启动宿主。",
        ));

        let mut host = self.agent_form.host;
        let host_combo = ui
            .add_enabled_ui(self.activity.is_none(), |ui| {
                egui::ComboBox::from_label(self.language.select("Agent host", "Agent 宿主"))
                    .selected_text(agent_host_label(host))
                    .show_ui(ui, |ui| {
                        for candidate in [AgentHost::Codex, AgentHost::Claude, AgentHost::Generic] {
                            ui.selectable_value(&mut host, candidate, agent_host_label(candidate));
                        }
                    })
            })
            .inner;
        paint_focus_ring(ui, &host_combo.response);
        keep_focused_visible(&host_combo.response);
        self.agent_form.select_host(host);

        ui.horizontal_wrapped(|ui| {
            let path = self.agent_form.destination.as_ref().map_or_else(
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
                    .min_size(egui::vec2(140.0, 44.0)),
            );
            paint_focus_ring(ui, &choose);
            keep_focused_visible(&choose);
            if choose.clicked()
                && let Some(destination) = pick_directory(Some(self.language.select(
                    "Choose a new or empty Agent pack directory",
                    "选择新的或空的 Agent 资源包目录",
                )))
            {
                self.agent_form.select_destination(destination);
            }
        });

        if let Some(preview) = self.agent_form.destination_preview {
            ui.colored_label(
                theme::positive(self.dark_mode),
                match preview {
                    AgentDestinationPreview::New => self.language.select(
                        "Preview: destination will be created",
                        "预览：将创建目标目录",
                    ),
                    AgentDestinationPreview::Empty => self
                        .language
                        .select("Preview: selected directory is empty", "预览：所选目录为空"),
                },
            );
        }
        if let Some(issue) = self.agent_form.destination_issue {
            accessible_error(
                ui,
                theme::error(self.dark_mode),
                agent_destination_issue_label(issue, self.language),
            );
        }

        let export = ui
            .add_enabled(
                self.activity.is_none() && self.agent_form.export_ready(),
                theme::primary_button(
                    self.language
                        .select("Export verified pack", "导出经过验证的资源包"),
                )
                .min_size(egui::vec2(190.0, 44.0)),
            )
            .on_disabled_hover_text(self.language.select(
                "Choose a destination that is new or empty",
                "请选择一个新的或空的目标目录",
            ));
        paint_focus_ring(ui, &export);
        keep_focused_visible(&export);
        if self.pending_focus == Some(FocusTarget::AgentExport) {
            export.request_focus();
            self.pending_focus = None;
        }
        if export.clicked()
            && let Some(destination) = self.agent_form.destination.clone()
        {
            self.agent_form.exported = None;
            self.agent_form.failure = None;
            self.dispatch(
                self.language.select(
                    "Exporting verified Agent pack",
                    "正在导出经过验证的 Agent 资源包",
                ),
                ui.ctx().clone(),
                WorkerRequest::ExportAgentPack {
                    request: AgentPackExportRequest::new(self.agent_form.host, destination),
                },
            );
        }

        if let Some(exported) = self.agent_form.exported.clone() {
            ui.add_space(10.0);
            egui::Frame::new()
                .fill(if self.dark_mode {
                    Color32::from_rgb(26, 72, 65)
                } else {
                    theme::TEAL_100
                })
                .stroke(Stroke::new(1.0, theme::TEAL_600))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    accessible_heading(
                        ui,
                        self.language
                            .select("Verified pack exported", "经过验证的资源包已导出"),
                        3,
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Host", "宿主"),
                        agent_host_label(exported.manifest.host),
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Manifest", "清单"),
                        &exported.manifest_path.display().to_string(),
                    );
                    agent_metadata_row(
                        ui,
                        self.language.select("Resource count", "资源数量"),
                        &exported.manifest.files.len().to_string(),
                    );
                    ui.collapsing(
                        self.language
                            .select("Exact exported files", "导出的精确文件"),
                        |ui| {
                            for file in &exported.manifest.files {
                                ui.label(format!(
                                    "• {} — {} B · {}",
                                    file.path, file.size, file.resource_id
                                ));
                            }
                            ui.label("• canisend-agent-pack.json");
                        },
                    );
                });
        }
    }
}

fn show_agent_capabilities(
    ui: &mut egui::Ui,
    language: Language,
    dark_mode: bool,
    capabilities: &canisend_app::AgentCapabilitiesReadModel,
) {
    accessible_heading(
        ui,
        language.select("Agent v2 capabilities", "Agent v2 能力"),
        2,
    );
    egui::Frame::new()
        .fill(if dark_mode {
            Color32::from_rgb(31, 45, 49)
        } else {
            Color32::WHITE
        })
        .stroke(Stroke::new(1.0, theme::SLATE_300))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            agent_metadata_row(ui, "Protocol", &capabilities.protocol);
            agent_metadata_row(
                ui,
                language.select("Product version", "产品版本"),
                capabilities.product_version.as_ref(),
            );
            agent_metadata_row(
                ui,
                language.select("Capability families", "能力类别"),
                &capabilities.capabilities.len().to_string(),
            );
            agent_metadata_row(
                ui,
                language.select("Workflow stages", "工作流阶段"),
                &capabilities.stages.len().to_string(),
            );
            agent_metadata_row(
                ui,
                language.select("Discovery adapters", "发现适配器"),
                &capabilities.discovery_adapters.len().to_string(),
            );
            ui.collapsing(
                language.select("Inspect capability IDs", "检查能力 ID"),
                |ui| {
                    for capability in &capabilities.capabilities {
                        ui.label(format!(
                            "• {} · {} · {:?}",
                            capability.id, capability.version, capability.status
                        ));
                    }
                },
            );
            ui.collapsing(
                language.select("Inspect stage support", "检查阶段支持"),
                |ui| {
                    for stage in &capabilities.stages {
                        ui.label(format!(
                            "• {} · {:?} · {:?}",
                            stage.id, stage.status, stage.execution_modes
                        ));
                    }
                },
            );
            ui.collapsing(
                language.select("Inspect discovery adapters", "检查发现适配器"),
                |ui| {
                    for adapter in &capabilities.discovery_adapters {
                        ui.label(format!(
                            "• {:?} · network={} · cursor={} · max={}",
                            adapter.kind,
                            adapter.network,
                            adapter.supports_cursor,
                            adapter.max_items_per_refresh
                        ));
                    }
                },
            );
        });
}

fn show_agent_failure(
    ui: &mut egui::Ui,
    language: Language,
    dark_mode: bool,
    failure: &ApplicationFailure,
) {
    accessible_heading(
        ui,
        language.select("Agent action needs attention", "Agent 操作需要处理"),
        3,
    );
    accessible_error(ui, theme::error(dark_mode), &failure.message);
    agent_metadata_row(
        ui,
        language.select("Error code", "错误代码"),
        failure.code.as_str(),
    );
    if let Some(remediation) = &failure.remediation {
        ui.label(&remediation.description);
        agent_copy_row(
            ui,
            language,
            &remediation.action,
            language.select("Copy remediation", "复制修复操作"),
        );
    }
}

fn agent_copy_row(ui: &mut egui::Ui, language: Language, value: &str, copy_label: &str) -> bool {
    let mut copied = false;
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new(value).wrap());
        let copy = ui
            .add(egui::Button::new(copy_label).min_size(egui::vec2(110.0, 44.0)))
            .on_hover_text(
                language.select("Copy this exact public text", "复制这段精确的公开文本"),
            );
        paint_focus_ring(ui, &copy);
        keep_focused_visible(&copy);
        if copy.clicked() {
            ui.ctx().copy_text(value.to_owned());
            copied = true;
        }
    });
    copied
}

fn agent_metadata_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{label}:")).strong());
        ui.label(value);
    });
}

fn agent_host_label(host: AgentHost) -> &'static str {
    match host {
        AgentHost::Codex => "Codex",
        AgentHost::Claude => "Claude",
        AgentHost::Generic => "Generic",
    }
}

fn agent_destination_issue_label(issue: AgentDestinationIssue, language: Language) -> &'static str {
    match issue {
        AgentDestinationIssue::InsideWorkspace => language.select(
            "The Agent pack cannot be exported inside a .canisend directory.",
            "不能在 .canisend 目录内导出 Agent 资源包。",
        ),
        AgentDestinationIssue::Symlink => language.select(
            "The destination or its parent cannot be a symbolic link.",
            "目标目录或其父目录不能是符号链接。",
        ),
        AgentDestinationIssue::NotDirectory => language.select(
            "The selected destination is not a directory.",
            "所选目标不是目录。",
        ),
        AgentDestinationIssue::NotEmpty => language.select(
            "The selected directory is not empty. Choose a new or empty directory.",
            "所选目录不为空。请选择新的或空的目录。",
        ),
        AgentDestinationIssue::MissingParent => language.select(
            "The destination parent is unavailable.",
            "目标目录的父目录不可用。",
        ),
        AgentDestinationIssue::Unreadable => language.select(
            "The destination could not be inspected safely.",
            "无法安全检查目标目录。",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{agent_destination_issue_label, agent_host_label};
    use crate::{
        i18n::Language,
        state::{AgentDestinationIssue, AgentIntegrationForm},
    };
    use canisend_app::AgentHost;

    #[test]
    fn agent_page_labels_cover_both_supported_languages_and_hosts() {
        assert_eq!(agent_host_label(AgentHost::Codex), "Codex");
        assert_eq!(agent_host_label(AgentHost::Claude), "Claude");
        assert_eq!(agent_host_label(AgentHost::Generic), "Generic");
        assert_eq!(
            agent_destination_issue_label(
                AgentDestinationIssue::NotEmpty,
                Language::SimplifiedChinese
            ),
            "所选目录不为空。请选择新的或空的目录。"
        );
        assert_eq!(
            agent_destination_issue_label(AgentDestinationIssue::NotEmpty, Language::English),
            "The selected directory is not empty. Choose a new or empty directory."
        );
    }

    #[test]
    fn changing_host_invalidates_only_the_previous_export() {
        let mut form = AgentIntegrationForm::default();
        form.select_host(AgentHost::Claude);
        assert_eq!(form.host, AgentHost::Claude);
        assert!(form.exported.is_none());
        assert!(form.context.is_none());
    }
}
