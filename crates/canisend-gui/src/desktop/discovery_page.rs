use canisend_app::{DiscoveryImportRequest, DiscoveryRefreshRequest};
use canisend_contracts::{
    DiscoveryFreshness, DiscoveryImportReport, DiscoveryLeadRecord, DiscoveryLeadStatus,
    DiscoveryMetadataValue, DiscoverySourceKind,
};

use super::*;

const DUPLICATE_SUGGESTION_LIMIT: usize = 5;

impl CanISendDesktop {
    pub(super) fn show_discovery(&mut self, ui: &mut egui::Ui) {
        self.page_header(
            ui,
            self.language.text("Discovery"),
            self.language.select(
                "Review bounded public and agent-provided leads before creating application jobs.",
                "先审阅有界的公开来源或 Agent 提供的线索，再创建申请职位。",
            ),
        );
        if self.active_workspace.is_none() {
            self.empty_workspace(ui);
            return;
        }

        ui.horizontal_wrapped(|ui| {
            for panel in DiscoveryPanel::ALL {
                let response = ui.selectable_value(
                    &mut self.discovery_panel,
                    panel,
                    panel.label(self.language),
                );
                paint_focus_ring(ui, &response);
                keep_focused_visible(&response);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.activity.is_none(),
                        egui::Button::new(self.language.text("Refresh")),
                    )
                    .clicked()
                {
                    self.refresh_discovery_workspace(ui.ctx().clone());
                }
            });
        });
        ui.separator();

        match self.discovery_panel {
            DiscoveryPanel::Leads => self.show_discovery_leads(ui),
            DiscoveryPanel::Sources => self.show_discovery_sources(ui),
            DiscoveryPanel::Import => self.show_discovery_import(ui),
            DiscoveryPanel::Refresh => self.show_discovery_refresh(ui),
        }
    }

    fn show_discovery_leads(&mut self, ui: &mut egui::Ui) {
        if let Some(action) = self.discovery_next_actions.first() {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(Stroke::new(1.0, theme::TEAL_700))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    accessible_heading(
                        ui,
                        self.language.select(
                            "Safe advert import is ready",
                            "可以安全导入招聘广告",
                        ),
                        2,
                    );
                    ui.label(self.language.select(
                        "Promotion created the job without fetching the advert. Run or copy this bounded next action when you are ready to import it.",
                        "提升操作只创建了职位，没有读取招聘广告。准备好后可运行或复制以下有界操作来导入。",
                    ));
                    ui.label(match self.language {
                        Language::English => action.description.as_str(),
                        Language::SimplifiedChinese => {
                            "通过安全的直接接收 URL 边界导入所选招聘广告"
                        }
                    });
                    command_copy_row(ui, &action.action, self.language);
                });
            ui.add_space(12.0);
        }

        if self.selected_discovery_lead.is_some() {
            self.show_discovery_lead_detail(ui);
            return;
        }

        ui.horizontal_wrapped(|ui| {
            let count = self
                .discovery_leads
                .as_ref()
                .map_or(0, |model| model.leads.len());
            accessible_heading(
                ui,
                &match self.language {
                    Language::English => format!("Discovery leads ({count})"),
                    Language::SimplifiedChinese => format!("发现线索（{count}）"),
                },
                2,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let history = ui.add_enabled(
                    self.activity.is_none(),
                    egui::Checkbox::new(
                        &mut self.discovery_include_history,
                        self.language.select(
                            "Include removed, expired, and promoted",
                            "包含已移除、已过期和已提升线索",
                        ),
                    ),
                );
                if history.changed() {
                    self.selected_discovery_lead = None;
                    self.discovery_suggestions = None;
                    self.refresh_discovery_workspace(ui.ctx().clone());
                }
            });
        });
        ui.label(self.language.select(
            "Active leads are shown by default. History is read-only and remains available for provenance.",
            "默认只显示活跃线索。历史记录为只读，并保留用于来源追踪。",
        ));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(self.language.text("Search"));
            ui.add(
                egui::TextEdit::singleline(&mut self.discovery_filter)
                    .hint_text(
                        self.language
                            .select("Title, organization, or location", "职位名称、机构或地点"),
                    )
                    .desired_width(360.0),
            );
        });
        ui.add_space(10.0);

        let filter = self.discovery_filter.trim().to_lowercase();
        let leads = self
            .discovery_leads
            .as_ref()
            .map(|model| {
                model
                    .leads
                    .iter()
                    .filter(|lead| {
                        filter.is_empty()
                            || lead.title.to_lowercase().contains(&filter)
                            || lead.organization.to_lowercase().contains(&filter)
                            || lead
                                .location
                                .as_deref()
                                .is_some_and(|value| value.to_lowercase().contains(&filter))
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if self.discovery_leads.is_none() {
            ui.label(self.language.select(
                "Load or refresh the discovery workspace to see leads.",
                "加载或刷新职位发现数据后即可查看线索。",
            ));
        } else if leads.is_empty() {
            ui.label(self.language.select(
                "No discovery leads match the current filter.",
                "没有符合当前筛选条件的发现线索。",
            ));
        } else {
            for lead in leads {
                self.discovery_lead_row(ui, &lead);
            }
        }
    }

    fn discovery_lead_row(&mut self, ui: &mut egui::Ui, lead: &DiscoveryLeadRecord) {
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
                        ui.label(RichText::new(&lead.title).strong().size(16.0));
                        ui.label(&lead.organization);
                        if let Some(location) = &lead.location {
                            ui.label(RichText::new(location).weak());
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(discovery_freshness_text(self.language, lead.freshness))
                                .strong(),
                        );
                        ui.label(discovery_status_text(self.language, lead.status));
                    });
                });
            })
            .response
            .interact(if self.activity.is_none() {
                Sense::click()
            } else {
                Sense::hover()
            });
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                self.activity.is_none(),
                match self.language {
                    Language::English => {
                        format!(
                            "Open discovery lead {} at {}",
                            lead.title, lead.organization
                        )
                    }
                    Language::SimplifiedChinese => {
                        format!("打开发现线索 {}（{}）", lead.title, lead.organization)
                    }
                },
            )
        });
        paint_focus_ring(ui, &response);
        keep_focused_visible(&response);
        if response.clicked() {
            self.load_discovery_lead(lead.id.to_string(), ui.ctx().clone());
        }
        ui.add_space(8.0);
    }

    fn show_discovery_lead_detail(&mut self, ui: &mut egui::Ui) {
        let Some(lead) = self.selected_discovery_lead.clone() else {
            return;
        };
        if ui
            .add_enabled(
                self.activity.is_none(),
                egui::Button::new(self.language.select("Back to leads", "返回线索列表")),
            )
            .clicked()
        {
            self.selected_discovery_lead = None;
            self.discovery_suggestions = None;
            return;
        }
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            accessible_heading(ui, &lead.title, 2);
            ui.label(RichText::new(discovery_status_text(self.language, lead.status)).strong());
            ui.label(
                RichText::new(discovery_freshness_text(self.language, lead.freshness)).strong(),
            );
        });
        ui.label(RichText::new(&lead.organization).size(16.0));
        if let Some(summary) = &lead.summary {
            ui.add_space(8.0);
            ui.label(summary);
        }
        ui.add_space(10.0);
        egui::Grid::new(("discovery_lead_detail", lead.id.as_str()))
            .num_columns(2)
            .spacing([24.0, 8.0])
            .show(ui, |ui| {
                diagnostic_row(
                    ui,
                    self.language.select("Lead ID", "线索 ID"),
                    lead.id.as_str(),
                );
                diagnostic_row(
                    ui,
                    self.language.select("Source ID", "来源 ID"),
                    lead.source_id.as_str(),
                );
                diagnostic_row(
                    ui,
                    self.language.text("Revision"),
                    &lead.revision.get().to_string(),
                );
                diagnostic_row(
                    ui,
                    self.language.select("First seen", "首次发现"),
                    lead.first_seen_at.as_str(),
                );
                diagnostic_row(
                    ui,
                    self.language.select("Last seen", "最近发现"),
                    lead.last_seen_at.as_str(),
                );
                diagnostic_row(
                    ui,
                    self.language.select("Location", "地点"),
                    lead.location.as_deref().unwrap_or("—"),
                );
                diagnostic_row(
                    ui,
                    self.language.select("Deadline", "截止日期"),
                    lead.deadline.as_deref().unwrap_or("—"),
                );
                diagnostic_row(ui, "URL", &lead.url);
                if let Some(job_id) = &lead.promoted_job_id {
                    diagnostic_row(
                        ui,
                        self.language.select("Promoted job", "已提升职位"),
                        job_id.as_str(),
                    );
                }
            });
        if !lead.metadata.is_empty() {
            ui.add_space(12.0);
            accessible_heading(ui, self.language.select("Public metadata", "公开元数据"), 3);
            for (key, value) in &lead.metadata {
                diagnostic_row(ui, key, &discovery_metadata_text(value));
            }
        }

        ui.add_space(16.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(
                        self.language
                            .select("Check possible duplicates", "检查可能的重复线索"),
                    ),
                )
                .clicked()
            {
                self.load_discovery_suggestions(lead.id.to_string(), ui.ctx().clone());
            }
            if lead.status == DiscoveryLeadStatus::Active
                && ui
                    .add_enabled(
                        self.activity.is_none(),
                        theme::primary_button(
                            self.language
                                .select("Promote to application job", "提升为申请职位"),
                        ),
                    )
                    .clicked()
            {
                self.pending_confirmation = Some(PendingConfirmation::PromoteDiscoveryLead {
                    lead_id: lead.id.to_string(),
                    title: lead.title.clone(),
                    organization: lead.organization.clone(),
                });
            }
        });
        ui.label(
            RichText::new(self.language.select(
                "Duplicate suggestions are advisory only. CanISend never merges leads automatically.",
                "重复建议仅供参考。CanISend 绝不会自动合并线索。",
            ))
            .weak(),
        );
        if let Some(suggestions) = &self.discovery_suggestions {
            ui.add_space(12.0);
            accessible_heading(
                ui,
                &match self.language {
                    Language::English => {
                        format!("Possible duplicates ({})", suggestions.suggestions.len())
                    }
                    Language::SimplifiedChinese => {
                        format!("可能的重复线索（{}）", suggestions.suggestions.len())
                    }
                },
                3,
            );
            if suggestions.suggestions.is_empty() {
                ui.label(self.language.select(
                    "No possible duplicates were found within the bounded comparison.",
                    "在有界比较范围内未发现可能的重复线索。",
                ));
            }
            for suggestion in &suggestions.suggestions {
                egui::Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&suggestion.lead.title).strong());
                        ui.label(&suggestion.lead.organization);
                        ui.label(match self.language {
                            Language::English => {
                                format!("Similarity: {}%", suggestion.similarity_percent)
                            }
                            Language::SimplifiedChinese => {
                                format!("相似度：{}%", suggestion.similarity_percent)
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        }
    }

    fn show_discovery_sources(&mut self, ui: &mut egui::Ui) {
        accessible_heading(
            ui,
            self.language
                .select("Compiled adapter catalog", "内置适配器目录"),
            2,
        );
        ui.label(self.language.select(
            "Every adapter applies a compiled item bound and the shared URL destination policy.",
            "每个适配器都执行内置条目上限和统一的 URL 目标策略。",
        ));
        ui.add_space(8.0);
        if let Some(catalog) = &self.discovery_adapters {
            if ui.available_width() < 720.0 {
                for adapter in &catalog.adapters {
                    egui::Frame::new()
                        .fill(ui.visuals().faint_bg_color)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(discovery_source_kind_text(
                                    self.language,
                                    adapter.kind,
                                ))
                                .strong(),
                            );
                            ui.label(match self.language {
                                Language::English => format!(
                                    "Public fetch · cursor {} · preserves history {} · limit {}",
                                    yes_no(self.language, adapter.supports_cursor),
                                    yes_no(self.language, adapter.preserves_removed),
                                    adapter.max_items_per_refresh
                                ),
                                Language::SimplifiedChinese => format!(
                                    "公开读取 · 游标 {} · 保留历史 {} · 上限 {}",
                                    yes_no(self.language, adapter.supports_cursor),
                                    yes_no(self.language, adapter.preserves_removed),
                                    adapter.max_items_per_refresh
                                ),
                            });
                        });
                    ui.add_space(6.0);
                }
            } else {
                egui::Grid::new("discovery_adapter_catalog")
                    .striped(true)
                    .num_columns(5)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new(self.language.select("Adapter", "适配器")).strong());
                        ui.label(RichText::new(self.language.select("Network", "网络")).strong());
                        ui.label(RichText::new(self.language.select("Cursor", "游标")).strong());
                        ui.label(RichText::new(self.language.select("History", "历史")).strong());
                        ui.label(
                            RichText::new(self.language.select("Item limit", "条目上限")).strong(),
                        );
                        ui.end_row();
                        for adapter in &catalog.adapters {
                            ui.label(discovery_source_kind_text(self.language, adapter.kind));
                            ui.label(self.language.select("Public fetch", "公开读取"));
                            ui.label(yes_no(self.language, adapter.supports_cursor));
                            ui.label(yes_no(self.language, adapter.preserves_removed));
                            ui.label(adapter.max_items_per_refresh.to_string());
                            ui.end_row();
                        }
                    });
            }
        } else {
            ui.label(self.language.select(
                "The adapter catalog has not been loaded.",
                "尚未加载适配器目录。",
            ));
        }

        ui.add_space(20.0);
        let count = self
            .discovery_sources
            .as_ref()
            .map_or(0, |model| model.sources.len());
        accessible_heading(
            ui,
            &match self.language {
                Language::English => format!("Workspace sources ({count})"),
                Language::SimplifiedChinese => format!("工作区来源（{count}）"),
            },
            2,
        );
        if let Some(model) = &self.discovery_sources {
            if model.sources.is_empty() {
                ui.label(self.language.select(
                    "No discovery source has been committed.",
                    "尚未提交任何发现来源。",
                ));
            }
            for source in &model.sources {
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
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&source.name).strong().size(16.0));
                            ui.label(discovery_source_kind_text(self.language, source.kind));
                            ui.label(if source.enabled {
                                self.language.select("Enabled", "已启用")
                            } else {
                                self.language.select("Disabled", "已停用")
                            });
                        });
                        egui::Grid::new(("discovery_source", source.id.as_str()))
                            .num_columns(2)
                            .spacing([24.0, 8.0])
                            .show(ui, |ui| {
                                diagnostic_row(
                                    ui,
                                    self.language.select("Source ID", "来源 ID"),
                                    source.id.as_str(),
                                );
                                diagnostic_row(
                                    ui,
                                    self.language.select("Endpoint", "端点"),
                                    source.endpoint.as_deref().unwrap_or("—"),
                                );
                                diagnostic_row(
                                    ui,
                                    self.language.select("Last refreshed", "最近刷新"),
                                    source
                                        .last_refreshed_at
                                        .as_ref()
                                        .map_or("—", |value| value.as_str()),
                                );
                                diagnostic_row(
                                    ui,
                                    self.language.select("Stale after", "过期阈值"),
                                    &match self.language {
                                        Language::English => {
                                            format!("{} seconds", source.policy.stale_after_seconds)
                                        }
                                        Language::SimplifiedChinese => {
                                            format!("{} 秒", source.policy.stale_after_seconds)
                                        }
                                    },
                                );
                                diagnostic_row(
                                    ui,
                                    self.language.select("Item limit", "条目上限"),
                                    &source.policy.max_items.to_string(),
                                );
                                diagnostic_row(
                                    ui,
                                    self.language.select("Missing items", "缺失条目"),
                                    if source.policy.mark_missing_as_removed {
                                        self.language.select("Mark removed", "标记为已移除")
                                    } else {
                                        self.language.select("Keep active", "保持活跃")
                                    },
                                );
                            });
                    });
                ui.add_space(8.0);
            }
        }
    }

    fn show_discovery_import(&mut self, ui: &mut egui::Ui) {
        accessible_heading(
            ui,
            self.language
                .select("Preview a local discovery batch", "预览本地发现批次"),
            2,
        );
        ui.label(self.language.select(
            "CSV and versioned JSON are supported. Host-agent mode requires versioned JSON. No workspace changes occur during preview.",
            "支持 CSV 和带版本的 JSON。宿主 Agent 模式必须使用带版本的 JSON。预览不会修改工作区。",
        ));
        ui.add_space(10.0);

        let previous_file = self.discovery_import_form.file.clone();
        ui.horizontal_wrapped(|ui| {
            ui.label(self.discovery_import_form.file.as_ref().map_or_else(
                || self.language.text("No file selected").to_owned(),
                |path| path.display().to_string(),
            ));
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(self.language.text("Choose file")),
                )
                .clicked()
                && let Some(path) = pick_discovery_batch_file()
            {
                self.discovery_import_form.file = Some(path);
            }
        });
        if self.discovery_import_form.file != previous_file {
            self.discovery_import_form.host_agent = false;
            if self
                .discovery_import_form
                .file
                .as_ref()
                .and_then(|path| path.extension())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            {
                self.discovery_import_form.source_name.clear();
                self.discovery_import_form.source_url.clear();
            }
            self.discovery_import_form.invalidate_preview();
        }
        let json_batch = self
            .discovery_import_form
            .file
            .as_ref()
            .and_then(|path| path.extension())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
        let host_agent = ui.add_enabled(
            self.activity.is_none() && json_batch,
            egui::Checkbox::new(
                &mut self.discovery_import_form.host_agent,
                self.language.select(
                    "Versioned host-agent JSON batch",
                    "带版本的宿主 Agent JSON 批次",
                ),
            ),
        );
        if host_agent.changed() {
            self.discovery_import_form.invalidate_preview();
        }
        if json_batch {
            ui.label(
                RichText::new(self.language.select(
                    "JSON declares its source name and URL inside the versioned batch.",
                    "JSON 会在带版本的批次内部声明来源名称和 URL。",
                ))
                .weak(),
            );
        } else {
            let source_name = ui.add_enabled(
                self.activity.is_none(),
                egui::TextEdit::singleline(&mut self.discovery_import_form.source_name)
                    .hint_text(self.language.select(
                        "CSV source name (file name if empty)",
                        "CSV 来源名称（留空则使用文件名）",
                    ))
                    .desired_width(420.0),
            );
            let source_url = ui.add_enabled(
                self.activity.is_none(),
                egui::TextEdit::singleline(&mut self.discovery_import_form.source_url)
                    .hint_text(
                        self.language
                            .select("Optional public source URL", "可选的公开来源 URL"),
                    )
                    .desired_width(420.0),
            );
            if source_name.changed() || source_url.changed() {
                self.discovery_import_form.invalidate_preview();
            }
        }
        ui.add_space(8.0);
        ui.add_enabled(
            self.activity.is_none(),
            egui::Checkbox::new(
                &mut self.discovery_import_form.private_read_consent,
                self.language.select(
                    "Allow this user-invoked read of the selected local batch",
                    "允许本次由用户发起的本地批次读取",
                ),
            ),
        );
        if ui
            .add_enabled(
                self.activity.is_none(),
                theme::primary_button(
                    self.language
                        .select("Preview and validate batch", "预览并验证批次"),
                ),
            )
            .clicked()
        {
            self.preview_discovery_import(ui.ctx().clone());
        }
        if let Some(error) = &self.discovery_import_form.error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }
        if let Some(report) = self.discovery_import_form.preview.clone() {
            ui.add_space(16.0);
            show_discovery_report(ui, self.language, &report);
            if ui
                .add_enabled(
                    self.activity.is_none() && report.batch.is_some(),
                    theme::primary_button(
                        self.language
                            .select("Commit reviewed batch", "提交已审阅批次"),
                    ),
                )
                .clicked()
            {
                self.commit_discovery_import(report, ui.ctx().clone());
            }
        }
    }

    fn show_discovery_refresh(&mut self, ui: &mut egui::Ui) {
        accessible_heading(
            ui,
            self.language
                .select("Preview a public-source refresh", "预览公开来源刷新"),
            2,
        );
        ui.label(self.language.select(
            "Refreshing performs a public network request under the URL destination policy. Review the normalized report before committing it.",
            "刷新会依照 URL 目标策略执行公开网络请求。提交前请先审阅规范化报告。",
        ));
        ui.add_space(10.0);

        let previous_adapter = self.discovery_refresh_form.adapter;
        let adapter_combo = egui::ComboBox::from_label(self.language.select("Adapter", "适配器"))
            .selected_text(network_adapter_text(
                self.language,
                self.discovery_refresh_form.adapter,
            ))
            .show_ui(ui, |ui| {
                for adapter in [
                    canisend_app::DiscoveryNetworkAdapter::RssAtom,
                    canisend_app::DiscoveryNetworkAdapter::JobsAcUk,
                    canisend_app::DiscoveryNetworkAdapter::Greenhouse,
                    canisend_app::DiscoveryNetworkAdapter::Lever,
                ] {
                    ui.selectable_value(
                        &mut self.discovery_refresh_form.adapter,
                        adapter,
                        network_adapter_text(self.language, adapter),
                    );
                }
            });
        keep_focused_visible(&adapter_combo.response);
        if self.discovery_refresh_form.adapter != previous_adapter {
            self.discovery_refresh_form.invalidate_preview();
        }
        if let Some(capability) = self.discovery_adapters.as_ref().and_then(|catalog| {
            let kind = self.discovery_refresh_form.adapter.source_kind();
            catalog.adapters.iter().find(|item| item.kind == kind)
        }) {
            ui.label(
                RichText::new(match self.language {
                    Language::English => format!(
                        "Compiled limit: {} items · cursor: {} · preserves removed: {}",
                        capability.max_items_per_refresh,
                        yes_no(self.language, capability.supports_cursor),
                        yes_no(self.language, capability.preserves_removed)
                    ),
                    Language::SimplifiedChinese => format!(
                        "内置上限：{} 条 · 游标：{} · 保留移除记录：{}",
                        capability.max_items_per_refresh,
                        yes_no(self.language, capability.supports_cursor),
                        yes_no(self.language, capability.preserves_removed)
                    ),
                })
                .weak(),
            );
        }
        let endpoint = ui.add_enabled(
            self.activity.is_none(),
            egui::TextEdit::singleline(&mut self.discovery_refresh_form.endpoint)
                .hint_text(
                    self.language
                        .select("Public HTTPS endpoint", "公开 HTTPS 端点"),
                )
                .desired_width(520.0),
        );
        let source_name = ui.add_enabled(
            self.activity.is_none(),
            egui::TextEdit::singleline(&mut self.discovery_refresh_form.source_name)
                .hint_text(self.language.select("Source name", "来源名称"))
                .desired_width(420.0),
        );
        let organization = ui.add_enabled(
            self.activity.is_none(),
            egui::TextEdit::singleline(&mut self.discovery_refresh_form.organization)
                .hint_text(
                    self.language
                        .select("Optional organization override", "可选的机构覆盖值"),
                )
                .desired_width(420.0),
        );
        if endpoint.changed() || source_name.changed() || organization.changed() {
            self.discovery_refresh_form.invalidate_preview();
        }
        ui.add_space(8.0);
        ui.add_enabled(
            self.activity.is_none(),
            egui::Checkbox::new(
                &mut self.discovery_refresh_form.network_consent,
                self.language.select(
                    "Allow this user-invoked public network refresh",
                    "允许本次由用户发起的公开网络刷新",
                ),
            ),
        );
        if ui
            .add_enabled(
                self.activity.is_none(),
                theme::primary_button(
                    self.language
                        .select("Fetch and preview refresh", "读取并预览刷新"),
                ),
            )
            .clicked()
        {
            self.preview_discovery_refresh(ui.ctx().clone());
        }
        if let Some(error) = &self.discovery_refresh_form.error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }
        if let Some(report) = self.discovery_refresh_form.preview.clone() {
            ui.add_space(16.0);
            show_discovery_report(ui, self.language, &report);
            if ui
                .add_enabled(
                    self.activity.is_none() && report.batch.is_some(),
                    theme::primary_button(
                        self.language
                            .select("Commit reviewed refresh", "提交已审阅刷新"),
                    ),
                )
                .clicked()
            {
                self.commit_discovery_refresh(report, ui.ctx().clone());
            }
        }
    }

    fn preview_discovery_import(&mut self, ctx: egui::Context) {
        if let Err(error) = validate_discovery_import_form(
            self.discovery_import_form.file.as_deref(),
            self.discovery_import_form.host_agent,
            self.discovery_import_form.private_read_consent,
            self.language,
        ) {
            self.discovery_import_form.error = Some(error);
            return;
        }
        let Some(path) = self.discovery_import_form.file.clone() else {
            return;
        };
        let json_batch = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
        let optional = |value: &str| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        };
        let (source_name, source_url) = if json_batch {
            (None, None)
        } else {
            (
                optional(&self.discovery_import_form.source_name),
                optional(&self.discovery_import_form.source_url),
            )
        };
        let request = DiscoveryImportRequest {
            path,
            source_name,
            source_url,
            host_agent: self.discovery_import_form.host_agent,
        };
        self.dispatch(
            self.language
                .select("Validating discovery batch", "正在验证发现批次"),
            ctx,
            WorkerRequest::PreviewDiscoveryImport { request },
        );
    }

    fn commit_discovery_import(&mut self, report: DiscoveryImportReport, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            self.discovery_import_form.error = Some(
                self.language
                    .text("No active workspace is selected")
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language
                .select("Committing discovery batch", "正在提交发现批次"),
            ctx,
            WorkerRequest::CommitDiscoveryImport {
                path,
                report,
                include_history: self.discovery_include_history,
            },
        );
    }

    fn preview_discovery_refresh(&mut self, ctx: egui::Context) {
        let endpoint = self.discovery_refresh_form.endpoint.trim();
        let source_name = self.discovery_refresh_form.source_name.trim();
        if let Err(error) = validate_discovery_refresh_form(
            endpoint,
            source_name,
            self.discovery_refresh_form.network_consent,
            self.language,
        ) {
            self.discovery_refresh_form.error = Some(error);
            return;
        }
        let organization = self.discovery_refresh_form.organization.trim();
        let request = DiscoveryRefreshRequest {
            adapter: self.discovery_refresh_form.adapter,
            endpoint: endpoint.to_owned(),
            source_name: source_name.to_owned(),
            organization: (!organization.is_empty()).then(|| organization.to_owned()),
        };
        self.dispatch(
            self.language
                .select("Fetching public discovery source", "正在读取公开发现来源"),
            ctx,
            WorkerRequest::PreviewDiscoveryRefresh { request },
        );
    }

    fn commit_discovery_refresh(&mut self, report: DiscoveryImportReport, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            self.discovery_refresh_form.error = Some(
                self.language
                    .text("No active workspace is selected")
                    .to_owned(),
            );
            return;
        };
        self.dispatch(
            self.language
                .select("Committing discovery refresh", "正在提交发现刷新"),
            ctx,
            WorkerRequest::CommitDiscoveryRefresh {
                path,
                report,
                include_history: self.discovery_include_history,
            },
        );
    }

    fn load_discovery_lead(&mut self, lead_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language
                .select("Loading discovery lead", "正在加载发现线索"),
            ctx,
            WorkerRequest::LoadDiscoveryLead { path, lead_id },
        );
    }

    fn load_discovery_suggestions(&mut self, lead_id: String, ctx: egui::Context) {
        let Some(path) = self.active_workspace.clone() else {
            return;
        };
        self.dispatch(
            self.language
                .select("Checking possible duplicates", "正在检查可能的重复线索"),
            ctx,
            WorkerRequest::LoadDiscoverySuggestions {
                path,
                lead_id,
                limit: DUPLICATE_SUGGESTION_LIMIT,
            },
        );
    }
}

fn show_discovery_report(ui: &mut egui::Ui, language: Language, report: &DiscoveryImportReport) {
    accessible_heading(ui, language.select("Validated preview", "已验证预览"), 3);
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(match language {
                Language::English => format!("Accepted: {}", report.accepted),
                Language::SimplifiedChinese => format!("已接受：{}", report.accepted),
            })
            .strong(),
        );
        ui.label(
            RichText::new(match language {
                Language::English => format!("Rejected: {}", report.rejected),
                Language::SimplifiedChinese => format!("已拒绝：{}", report.rejected),
            })
            .strong(),
        );
        ui.label(language.select("No workspace changes yet", "尚未修改工作区"));
    });
    if let Some(batch) = &report.batch {
        egui::Grid::new("discovery_report_batch")
            .num_columns(2)
            .spacing([24.0, 8.0])
            .show(ui, |ui| {
                diagnostic_row(
                    ui,
                    language.select("Source kind", "来源类型"),
                    discovery_source_kind_text(language, batch.source_kind),
                );
                diagnostic_row(
                    ui,
                    language.select("Source name", "来源名称"),
                    &batch.source_name,
                );
                diagnostic_row(
                    ui,
                    language.select("Source URL", "来源 URL"),
                    batch.source_url.as_deref().unwrap_or("—"),
                );
                diagnostic_row(
                    ui,
                    language.select("Observed at", "发现时间"),
                    batch.observed_at.as_str(),
                );
            });
    }
    if !report.diagnostics.is_empty() {
        ui.add_space(10.0);
        accessible_heading(ui, language.select("Diagnostics", "诊断"), 4);
        for diagnostic in &report.diagnostics {
            ui.label(format!(
                "{} {} · {} · {}",
                language.select("Row", "行"),
                diagnostic.row,
                diagnostic.code,
                diagnostic.message
            ));
        }
    }
}

fn discovery_source_kind_text(language: Language, kind: DiscoverySourceKind) -> &'static str {
    match kind {
        DiscoverySourceKind::Csv => "CSV",
        DiscoverySourceKind::Json => "JSON",
        DiscoverySourceKind::HostAgent => language.select("Host agent", "宿主 Agent"),
        DiscoverySourceKind::RssAtom => "RSS / Atom",
        DiscoverySourceKind::JobsAcUk => "jobs.ac.uk",
        DiscoverySourceKind::Greenhouse => "Greenhouse",
        DiscoverySourceKind::Lever => "Lever",
    }
}

fn network_adapter_text(
    language: Language,
    adapter: canisend_app::DiscoveryNetworkAdapter,
) -> &'static str {
    discovery_source_kind_text(language, adapter.source_kind())
}

fn discovery_status_text(language: Language, status: DiscoveryLeadStatus) -> &'static str {
    match status {
        DiscoveryLeadStatus::Active => language.select("Active", "活跃"),
        DiscoveryLeadStatus::Removed => language.select("Removed", "已移除"),
        DiscoveryLeadStatus::Expired => language.select("Expired", "已过期"),
        DiscoveryLeadStatus::Promoted => language.select("Promoted", "已提升"),
    }
}

fn discovery_freshness_text(language: Language, freshness: DiscoveryFreshness) -> &'static str {
    match freshness {
        DiscoveryFreshness::Current => language.select("Current", "当前"),
        DiscoveryFreshness::Stale => language.select("Stale", "已过期"),
        DiscoveryFreshness::Unknown => language.select("Freshness unknown", "新鲜度未知"),
    }
}

fn discovery_metadata_text(value: &DiscoveryMetadataValue) -> String {
    match value {
        DiscoveryMetadataValue::Text(value) => value.clone(),
        DiscoveryMetadataValue::Integer(value) => value.to_string(),
        DiscoveryMetadataValue::Boolean(value) => value.to_string(),
        DiscoveryMetadataValue::Json(value) => value.to_string(),
    }
}

fn yes_no(language: Language, value: bool) -> &'static str {
    if value {
        language.select("Yes", "是")
    } else {
        language.select("No", "否")
    }
}
