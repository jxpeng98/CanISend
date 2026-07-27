use super::*;

impl CanISendDesktop {
    pub(super) fn show_inspection_diagnostics(&mut self, ui: &mut egui::Ui) {
        ui.add_space(24.0);
        accessible_heading(
            ui,
            self.language.select("Schemas & resources", "Schema 与资源"),
            2,
        );
        ui.label(self.language.select(
            "Inspect verified public contracts and embedded assets without opening a workspace. Catalog metadata never contains job adverts, profile evidence, drafts, provider payloads, or credentials.",
            "无需打开工作区即可检查经过验证的公开契约和内置资源。目录元数据不会包含职位广告、个人资料证据、草稿、提供商载荷或凭据。",
        ));

        let load = ui.add_enabled(
            self.activity.is_none(),
            theme::primary_button(if self.catalog_form.catalog.is_some() {
                self.language
                    .select("Reload verified catalogs", "重新加载已验证目录")
            } else {
                self.language
                    .select("Load verified catalogs", "加载已验证目录")
            })
            .min_size(egui::vec2(190.0, 44.0)),
        );
        paint_focus_ring(ui, &load);
        keep_focused_visible(&load);
        if self.pending_focus == Some(FocusTarget::CatalogLoad) {
            load.request_focus();
            self.pending_focus = None;
        }
        if load.clicked() {
            self.catalog_form.failure = None;
            self.dispatch(
                self.language
                    .select("Loading verified catalogs", "正在加载已验证目录"),
                ui.ctx().clone(),
                WorkerRequest::LoadInspectionCatalog,
            );
        }

        if let Some(failure) = &self.catalog_form.failure {
            let message = format!("{}: {}", failure.code.as_str(), failure.message);
            accessible_error(ui, theme::error(self.dark_mode), &message);
        }

        if let Some(catalog) = self.catalog_form.catalog.clone() {
            ui.add_space(12.0);
            ui.label(
                RichText::new(self.language.select(
                    "Integrity verified against compiled SHA-256 metadata",
                    "已根据编译时 SHA-256 元数据完成完整性验证",
                ))
                .color(theme::positive(self.dark_mode))
                .strong(),
            );
            ui.label(match self.language {
                Language::English => format!(
                    "{} public schemas · {} embedded resources",
                    catalog.schemas.schemas.len(),
                    catalog.resources.len()
                ),
                Language::SimplifiedChinese => format!(
                    "{} 个公开 Schema · {} 个内置资源",
                    catalog.schemas.schemas.len(),
                    catalog.resources.len()
                ),
            });

            ui.horizontal_wrapped(|ui| {
                for panel in [CatalogPanel::Schemas, CatalogPanel::Resources] {
                    let selected = self.catalog_form.panel == panel;
                    let response = ui.add(
                        egui::Button::new(catalog_panel_label(panel, self.language))
                            .selected(selected)
                            .min_size(egui::vec2(140.0, 44.0)),
                    );
                    paint_focus_ring(ui, &response);
                    keep_focused_visible(&response);
                    if response.clicked() {
                        self.catalog_form.panel = panel;
                    }
                }
            });

            let filter = ui.add(
                egui::TextEdit::singleline(&mut self.catalog_form.filter)
                    .hint_text(self.language.select(
                        "Filter by ID, URI, kind, or path",
                        "按 ID、URI、类型或路径筛选",
                    ))
                    .desired_width(f32::INFINITY)
                    .min_size(egui::vec2(240.0, 44.0)),
            );
            paint_focus_ring(ui, &filter);
            keep_focused_visible(&filter);

            match self.catalog_form.panel {
                CatalogPanel::Schemas => {
                    show_schema_catalog(ui, self.language, &catalog, &self.catalog_form.filter);
                }
                CatalogPanel::Resources => {
                    show_resource_catalog(ui, self.language, &catalog, &self.catalog_form.filter);
                }
            }

            self.show_catalog_export(ui);
        }
    }

    fn show_catalog_export(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        accessible_heading(
            ui,
            self.language
                .select("Export verified public catalog", "导出经过验证的公开目录"),
            2,
        );
        ui.label(self.language.select(
            "Export writes every compiled public resource plus one integrity manifest to a new or empty directory. It does not export workspace data, overwrite files, launch an Agent host, or run a shell.",
            "导出会把所有编译进程序的公开资源和一份完整性清单写入新的或空的目录。它不会导出工作区数据、覆盖文件、启动 Agent 宿主或运行 Shell。",
        ));
        ui.horizontal_wrapped(|ui| {
            let path = self.catalog_form.destination.as_ref().map_or_else(
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
                    "Choose a new or empty public catalog directory",
                    "选择新的或空的公开目录导出位置",
                )))
            {
                self.catalog_form.select_destination(destination);
            }
        });

        if let Some(preview) = self.catalog_form.destination_preview {
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
        if let Some(issue) = self.catalog_form.destination_issue {
            accessible_error(
                ui,
                theme::error(self.dark_mode),
                catalog_destination_issue_label(issue, self.language),
            );
        }

        let export = ui
            .add_enabled(
                self.activity.is_none() && self.catalog_form.export_ready(),
                theme::primary_button(
                    self.language
                        .select("Export verified catalog", "导出经过验证的目录"),
                )
                .min_size(egui::vec2(200.0, 44.0)),
            )
            .on_disabled_hover_text(self.language.select(
                "Load the catalogs and choose a destination that is new or empty",
                "请先加载目录，并选择一个新的或空的目标目录",
            ));
        paint_focus_ring(ui, &export);
        keep_focused_visible(&export);
        if self.pending_focus == Some(FocusTarget::CatalogExport) {
            export.request_focus();
            self.pending_focus = None;
        }
        if export.clicked()
            && let Some(destination) = self.catalog_form.destination.clone()
        {
            self.catalog_form.exported = None;
            self.catalog_form.failure = None;
            self.dispatch(
                self.language.select(
                    "Exporting verified public catalog",
                    "正在导出经过验证的公开目录",
                ),
                ui.ctx().clone(),
                WorkerRequest::ExportResourceCatalog {
                    request: ResourceCatalogExportRequest::new(destination),
                },
            );
        }

        if let Some(exported) = &self.catalog_form.exported {
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
                            .select("Verified catalog exported", "经过验证的目录已导出"),
                        3,
                    );
                    diagnostic_row(
                        ui,
                        self.language.select("Manifest", "清单"),
                        &exported.manifest_path.display().to_string(),
                    );
                    diagnostic_row(
                        ui,
                        self.language.select("Resource count", "资源数量"),
                        &exported.manifest.files.len().to_string(),
                    );
                    ui.label(self.language.select(
                        "Workspace bodies exported: no · Agent host launched: no",
                        "已导出工作区正文：否 · 已启动 Agent 宿主：否",
                    ));
                    ui.collapsing(
                        self.language
                            .select("Exact exported files", "导出的精确文件"),
                        |ui| {
                            for file in &exported.manifest.files {
                                ui.add(
                                    egui::Label::new(format!(
                                        "{} — {} B · {}",
                                        file.path, file.size, file.resource_id
                                    ))
                                    .selectable(true)
                                    .wrap(),
                                );
                            }
                            ui.add(
                                egui::Label::new("canisend-resource-catalog.json").selectable(true),
                            );
                        },
                    );
                });
        }
    }
}

fn show_schema_catalog(
    ui: &mut egui::Ui,
    language: Language,
    catalog: &canisend_app::InspectionCatalogReadModel,
    filter: &str,
) {
    let filter = filter.trim().to_ascii_lowercase();
    let schemas = catalog
        .schemas
        .schemas
        .iter()
        .filter(|schema| {
            filter.is_empty()
                || schema.id.to_ascii_lowercase().contains(&filter)
                || schema.uri.to_ascii_lowercase().contains(&filter)
                || schema.resource_id.to_ascii_lowercase().contains(&filter)
        })
        .collect::<Vec<_>>();
    ui.label(match language {
        Language::English => format!(
            "Showing {} of {} schemas",
            schemas.len(),
            catalog.schemas.schemas.len()
        ),
        Language::SimplifiedChinese => {
            format!(
                "显示 {} / {} 个 Schema",
                schemas.len(),
                catalog.schemas.schemas.len()
            )
        }
    });
    for schema in schemas {
        ui.push_id(&schema.id, |ui| {
            egui::CollapsingHeader::new(&schema.id)
                .default_open(false)
                .show(ui, |ui| {
                    selectable_metadata_row(
                        ui,
                        language.select("Canonical URI", "规范 URI"),
                        &schema.uri,
                    );
                    selectable_metadata_row(
                        ui,
                        language.select("Version", "版本"),
                        schema.version.as_str(),
                    );
                    selectable_metadata_row(
                        ui,
                        language.select("Resource ID", "资源 ID"),
                        &schema.resource_id,
                    );
                    selectable_metadata_row(
                        ui,
                        language.select("Size", "大小"),
                        &format!("{} B", schema.size),
                    );
                    selectable_metadata_row(ui, "SHA-256", schema.sha256.as_str());
                });
        });
    }
    if catalog.schemas.schemas.is_empty() {
        ui.label(language.select("No public schemas are compiled.", "未编译任何公开 Schema。"));
    } else if !filter.is_empty()
        && catalog.schemas.schemas.iter().all(|schema| {
            !schema.id.to_ascii_lowercase().contains(&filter)
                && !schema.uri.to_ascii_lowercase().contains(&filter)
                && !schema.resource_id.to_ascii_lowercase().contains(&filter)
        })
    {
        ui.label(language.select(
            "No schemas match this filter.",
            "没有 Schema 匹配此筛选条件。",
        ));
    }
}

fn show_resource_catalog(
    ui: &mut egui::Ui,
    language: Language,
    catalog: &canisend_app::InspectionCatalogReadModel,
    filter: &str,
) {
    let filter = filter.trim().to_ascii_lowercase();
    let resources = catalog
        .resources
        .iter()
        .filter(|resource| {
            filter.is_empty()
                || resource.entry.id.to_ascii_lowercase().contains(&filter)
                || resource.entry.kind.to_ascii_lowercase().contains(&filter)
                || resource.path.to_ascii_lowercase().contains(&filter)
        })
        .collect::<Vec<_>>();
    ui.label(match language {
        Language::English => format!(
            "Showing {} of {} resources",
            resources.len(),
            catalog.resources.len()
        ),
        Language::SimplifiedChinese => {
            format!(
                "显示 {} / {} 个资源",
                resources.len(),
                catalog.resources.len()
            )
        }
    });
    for resource in resources {
        ui.push_id(&resource.entry.id, |ui| {
            egui::CollapsingHeader::new(&resource.entry.id)
                .default_open(false)
                .show(ui, |ui| {
                    selectable_metadata_row(
                        ui,
                        language.select("Kind", "类型"),
                        resource_kind_label(&resource.entry.kind, language),
                    );
                    selectable_metadata_row(
                        ui,
                        language.select("Version", "版本"),
                        resource.entry.version.as_str(),
                    );
                    selectable_metadata_row(
                        ui,
                        language.select("Embedded path", "内置路径"),
                        &resource.path,
                    );
                    selectable_metadata_row(
                        ui,
                        language.select("Size", "大小"),
                        &format!("{} B", resource.entry.size),
                    );
                    selectable_metadata_row(ui, "SHA-256", resource.entry.sha256.as_str());
                });
        });
    }
    if catalog.resources.is_empty() {
        ui.label(language.select(
            "No embedded resources are compiled.",
            "未编译任何内置资源。",
        ));
    } else if !filter.is_empty()
        && catalog.resources.iter().all(|resource| {
            !resource.entry.id.to_ascii_lowercase().contains(&filter)
                && !resource.entry.kind.to_ascii_lowercase().contains(&filter)
                && !resource.path.to_ascii_lowercase().contains(&filter)
        })
    {
        ui.label(language.select(
            "No resources match this filter.",
            "没有资源匹配此筛选条件。",
        ));
    }
}

fn selectable_metadata_row(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Grid::new(label)
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            ui.label(RichText::new(label).strong());
            ui.add(egui::Label::new(value).selectable(true).wrap());
            ui.end_row();
        });
}

fn catalog_panel_label(panel: CatalogPanel, language: Language) -> &'static str {
    match panel {
        CatalogPanel::Schemas => language.select("Schemas", "Schema"),
        CatalogPanel::Resources => language.select("Resources", "资源"),
    }
}

fn resource_kind_label(kind: &str, language: Language) -> &str {
    if language == Language::English {
        return kind;
    }
    match kind {
        "agent" => "Agent 指南",
        "example" => "示例",
        "prompt" => "提示词",
        "schema" => "Schema",
        "template" => "模板",
        _ => kind,
    }
}

fn catalog_destination_issue_label(
    issue: AgentDestinationIssue,
    language: Language,
) -> &'static str {
    match issue {
        AgentDestinationIssue::InsideWorkspace => language.select(
            "The destination cannot be inside .canisend",
            "目标目录不能位于 .canisend 内",
        ),
        AgentDestinationIssue::Symlink => language.select(
            "The destination or its parent cannot be a symbolic link",
            "目标目录或其父目录不能是符号链接",
        ),
        AgentDestinationIssue::NotDirectory => {
            language.select("The destination is not a directory", "目标位置不是目录")
        }
        AgentDestinationIssue::NotEmpty => language.select(
            "The destination must be new or empty",
            "目标目录必须是新的或空的",
        ),
        AgentDestinationIssue::MissingParent => language.select(
            "Choose a destination whose parent directory exists",
            "请选择父目录已经存在的目标位置",
        ),
        AgentDestinationIssue::Unreadable => language.select(
            "The destination cannot be inspected safely",
            "无法安全检查目标目录",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CatalogPanel, Language, catalog_destination_issue_label, catalog_panel_label,
        resource_kind_label,
    };
    use crate::state::AgentDestinationIssue;

    #[test]
    fn diagnostics_catalog_labels_cover_both_supported_languages() {
        assert_eq!(
            catalog_panel_label(CatalogPanel::Schemas, Language::English),
            "Schemas"
        );
        assert_eq!(
            catalog_panel_label(CatalogPanel::Resources, Language::SimplifiedChinese),
            "资源"
        );
        assert_eq!(
            resource_kind_label("prompt", Language::SimplifiedChinese),
            "提示词"
        );
        assert_eq!(
            catalog_destination_issue_label(
                AgentDestinationIssue::NotEmpty,
                Language::SimplifiedChinese,
            ),
            "目标目录必须是新的或空的"
        );
    }
}
