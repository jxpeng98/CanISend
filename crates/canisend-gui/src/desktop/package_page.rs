use super::*;

impl CanISendDesktop {
    pub(super) fn show_package_workspace(&mut self, ui: &mut egui::Ui, job_id: &str) {
        self.package_form.select_job(job_id);
        accessible_heading(
            ui,
            self.language.select(
                "Package readiness and private export",
                "申请包就绪状态与私密导出",
            ),
            2,
        );
        ui.label(self.language.select(
            "Readiness is derived from exact plan, evidence, profile, document-set, and review revisions. Export creates managed local projections only; CanISend never submits to an application portal.",
            "就绪状态根据精确的计划、证据、个人资料、文档集合和审阅修订版本计算。导出只会创建受管理的本地投影；CanISend 绝不会向申请门户提交。",
        ));
        ui.label(
            RichText::new(
                self.language
                    .select("Submission performed: no", "已执行门户提交：否"),
            )
            .strong(),
        );
        ui.add_space(10.0);

        ui.horizontal_wrapped(|ui| {
            let load = ui.add_enabled(
                self.activity.is_none(),
                egui::Button::new(
                    self.language
                        .select("Load current manifest", "加载当前清单"),
                )
                .min_size(egui::vec2(160.0, 44.0)),
            );
            if self.pending_focus == Some(FocusTarget::PackageCheck) {
                load.request_focus();
                self.pending_focus = None;
            }
            if load.clicked() {
                self.package_form.error = None;
                self.check_package(job_id.to_owned(), true, ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::primary_button(
                        self.language
                            .select("Check package readiness", "检查申请包就绪状态"),
                    ),
                )
                .clicked()
            {
                self.package_form.error = None;
                self.check_package(job_id.to_owned(), false, ui.ctx().clone());
            }
        });
        if let Some(error) = &self.package_form.error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }

        let Some(manifest) = self.package_form.manifest.clone() else {
            ui.add_space(12.0);
            ui.label(
                RichText::new(self.language.select(
                    "No current package manifest is loaded. Run the deterministic readiness check after the Review stage is complete.",
                    "尚未加载当前申请包清单。请在审阅阶段完成后运行确定性就绪检查。",
                ))
                .weak(),
            );
            return;
        };

        ui.add_space(18.0);
        show_manifest(ui, self.language, self.dark_mode, &manifest);
        ui.add_space(18.0);
        self.show_private_projection_export(ui, job_id, manifest.readiness.state);
        ui.add_space(18.0);
        self.show_projection_reconciliation(ui, job_id);
    }

    fn show_private_projection_export(
        &mut self,
        ui: &mut egui::Ui,
        job_id: &str,
        readiness: canisend_contracts::ReadinessState,
    ) {
        use canisend_contracts::ReadinessState;

        accessible_heading(
            ui,
            self.language
                .select("Editable private projections", "可编辑私密投影"),
            3,
        );
        let ready = matches!(
            readiness,
            ReadinessState::ReadyToExport | ReadinessState::Exported
        );
        if !ready {
            ui.colored_label(
                theme::warning(self.dark_mode),
                self.language.select(
                    "Resolve every readiness reason before exporting application bodies.",
                    "请先解决全部就绪原因，再导出申请材料正文。",
                ),
            );
        }

        let destination_label = ui.label(
            self.language
                .select("Workspace-relative directory", "工作区相对目录"),
        );
        let destination = ui
            .add_enabled(
                self.activity.is_none() && ready,
                egui::TextEdit::singleline(&mut self.package_form.export_destination)
                    .desired_width(f32::INFINITY)
                    .hint_text(format!("jobs/{job_id}/application")),
            )
            .labelled_by(destination_label.id);
        if destination.changed() {
            self.package_form.error = None;
            self.package_form.export_receipt = None;
            self.package_form.reconciliation = None;
        }
        let consent = ui.add_enabled(
            self.activity.is_none() && ready,
            egui::Checkbox::new(
                &mut self.package_form.private_export_consent,
                self.language.select(
                    "Allow this user-invoked export of private application bodies",
                    "允许本次由用户发起的私密申请材料正文导出",
                ),
            ),
        );
        if consent.changed() {
            self.package_form.error = None;
        }
        ui.label(
            RichText::new(self.language.select(
                "Destination must be new or empty and remain under jobs/JOB_ID/. Existing files are never overwritten by export.",
                "目标必须为新目录或空目录，并位于 jobs/JOB_ID/ 下。导出绝不会覆盖现有文件。",
            ))
            .weak(),
        );

        ui.horizontal_wrapped(|ui| {
            let export = ui.add_enabled(
                self.activity.is_none() && ready,
                theme::next_button(
                    self.language
                        .select("Export private projections", "导出私密投影"),
                ),
            );
            if self.pending_focus == Some(FocusTarget::PackageExport) {
                export.request_focus();
                self.pending_focus = None;
            }
            if export.clicked() {
                if !self.package_form.private_export_consent {
                    self.package_form.error = Some(
                        self.language
                            .select(
                                "Confirm private export consent before writing application bodies",
                                "写出申请材料正文前请确认私密导出同意",
                            )
                            .to_owned(),
                    );
                    self.pending_focus = Some(FocusTarget::PackageExport);
                } else {
                    match PackageExportRequest::try_new(
                        job_id,
                        self.package_form.export_destination.trim(),
                    ) {
                        Ok(request) => {
                            self.package_form.error = None;
                            self.export_package(request, true, ui.ctx().clone());
                        }
                        Err(error) => {
                            self.package_form.error = Some(error.to_string());
                            self.pending_focus = Some(FocusTarget::PackageExport);
                        }
                    }
                }
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    egui::Button::new(
                        self.language
                            .select("Load current export receipt", "加载当前导出收据"),
                    )
                    .min_size(egui::vec2(185.0, 44.0)),
                )
                .clicked()
            {
                self.package_form.error = None;
                self.load_package_export(job_id.to_owned(), ui.ctx().clone());
            }
        });

        if let Some(receipt) = &self.package_form.export_receipt {
            ui.add_space(10.0);
            egui::Grid::new(("package_export", receipt.id.as_str()))
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    diagnostic_row(
                        ui,
                        self.language.select("Export receipt", "导出收据"),
                        receipt.id.as_str(),
                    );
                    diagnostic_row(
                        ui,
                        self.language.select("Package artifact", "申请包工件"),
                        &artifact_label(&receipt.package_artifact),
                    );
                    diagnostic_row(
                        ui,
                        self.language.select("Managed projections", "受管理投影"),
                        &receipt.projections.len().to_string(),
                    );
                    diagnostic_row(
                        ui,
                        self.language
                            .select("Submission performed", "已执行门户提交"),
                        if receipt.submission_performed {
                            self.language.select("Yes", "是")
                        } else {
                            self.language.select("No", "否")
                        },
                    );
                });
        }
    }

    fn show_projection_reconciliation(&mut self, ui: &mut egui::Ui, job_id: &str) {
        accessible_heading(
            ui,
            self.language
                .select("Projection reconciliation", "投影对账"),
            3,
        );
        ui.label(self.language.select(
            "Reconciliation observes generated files without importing edits into authoritative artifacts.",
            "对账只观察生成文件，不会把编辑内容导入权威工件。",
        ));
        let reconcile = ui.add_enabled(
            self.activity.is_none() && self.package_form.export_receipt.is_some(),
            egui::Button::new(
                self.language
                    .select("Reconcile managed files", "对账受管理文件"),
            )
            .min_size(egui::vec2(180.0, 44.0)),
        );
        if self.pending_focus == Some(FocusTarget::PackageReconcile) {
            reconcile.request_focus();
            self.pending_focus = None;
        }
        if reconcile.clicked() {
            self.package_form.error = None;
            self.reconcile_package(job_id.to_owned(), ui.ctx().clone());
        }

        let Some(records) = self.package_form.reconciliation.clone() else {
            return;
        };
        let edited = records
            .iter()
            .filter(|record| {
                record.projection.edit_status == canisend_contracts::ProjectionEditStatus::Edited
            })
            .count();
        let missing = records
            .iter()
            .filter(|record| {
                record.projection.edit_status == canisend_contracts::ProjectionEditStatus::Missing
            })
            .count();
        ui.label(match self.language {
            Language::English => {
                format!(
                    "{} managed · {edited} edited · {missing} missing",
                    records.len()
                )
            }
            Language::SimplifiedChinese => {
                format!(
                    "{} 个受管理文件 · {edited} 个已编辑 · {missing} 个缺失",
                    records.len()
                )
            }
        });

        let mut selected = None;
        for record in &records {
            let path = &record.projection.relative_path;
            let is_selected = self.package_form.selected_projection.as_ref() == Some(path);
            let label = format!(
                "{} · {:?} · {:?}",
                path, record.projection.kind, record.projection.edit_status
            );
            let response = ui.selectable_label(is_selected, label);
            paint_focus_ring(ui, &response);
            if response.clicked() {
                selected = Some(path.clone());
            }
        }
        if let Some(path) = selected {
            self.package_form.select_projection(path);
        }

        let Some(path) = self.package_form.selected_projection.clone() else {
            return;
        };
        let Some(record) = records
            .iter()
            .find(|record| record.projection.relative_path == path)
        else {
            return;
        };
        ui.add_space(10.0);
        show_projection_detail(ui, self.language, record);

        use canisend_contracts::ProjectionEditStatus;
        if record.projection.edit_status != ProjectionEditStatus::Current
            && ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::destructive_button(
                        self.language
                            .select("Replace from authoritative data", "用权威数据覆盖"),
                    ),
                )
                .clicked()
        {
            match ProjectionReplaceRequest::try_new(job_id, path.as_str()) {
                Ok(request) => {
                    self.pending_confirmation =
                        Some(PendingConfirmation::ReplaceProjection { request });
                }
                Err(error) => self.package_form.error = Some(error.to_string()),
            }
        }

        if record.projection.edit_status == ProjectionEditStatus::Edited {
            let copy_label = ui.label(
                self.language
                    .select("Preserved copy destination", "保留副本目标"),
            );
            ui.add_enabled(
                self.activity.is_none(),
                egui::TextEdit::singleline(&mut self.package_form.copy_destination)
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(copy_label.id);
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::primary_button(
                        self.language
                            .select("Copy edit as new, then restore", "复制编辑稿后恢复"),
                    ),
                )
                .clicked()
            {
                match ProjectionCopyAsNewRequest::try_new(
                    job_id,
                    path.as_str(),
                    self.package_form.copy_destination.trim(),
                ) {
                    Ok(request) => {
                        self.package_form.error = None;
                        self.copy_projection_as_new(request, ui.ctx().clone());
                    }
                    Err(error) => self.package_form.error = Some(error.to_string()),
                }
            }
            ui.label(
                RichText::new(self.language.select(
                    "The preserved copy is unmanaged and must be a new path. The managed file is then restored.",
                    "保留副本不受管理，且必须使用新路径；随后受管理文件会被恢复。",
                ))
                .weak(),
            );
        }
    }
}

fn show_manifest(
    ui: &mut egui::Ui,
    language: Language,
    dark_mode: bool,
    manifest: &canisend_contracts::PackageManifestRecord,
) {
    let color = match manifest.readiness.state {
        canisend_contracts::ReadinessState::ReadyToExport
        | canisend_contracts::ReadinessState::Exported => theme::positive(dark_mode),
        canisend_contracts::ReadinessState::Blocked
        | canisend_contracts::ReadinessState::NeedsReview => theme::warning(dark_mode),
    };
    ui.colored_label(
        color,
        RichText::new(format!(
            "{}: {:?}",
            language.select("Readiness", "就绪状态"),
            manifest.readiness.state
        ))
        .strong(),
    );
    egui::Grid::new(("package_manifest", manifest.id.as_str()))
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            diagnostic_row(
                ui,
                language.select("Manifest ID", "清单 ID"),
                manifest.id.as_str(),
            );
            diagnostic_row(
                ui,
                language.select("Revision", "修订"),
                &manifest.revision.get().to_string(),
            );
            diagnostic_row(
                ui,
                language.select("Documents", "文档"),
                &manifest.documents.len().to_string(),
            );
            diagnostic_row(
                ui,
                language.select("Review artifact", "审阅工件"),
                &artifact_label(&manifest.review_artifact),
            );
            diagnostic_row(
                ui,
                language.select("Submission performed", "已执行门户提交"),
                if manifest.submission_performed {
                    language.select("Yes", "是")
                } else {
                    language.select("No", "否")
                },
            );
        });
    if manifest.readiness.reasons.is_empty() {
        ui.label(language.select("No readiness blockers.", "没有就绪阻断原因。"));
    }
    for reason in &manifest.readiness.reasons {
        ui.label(format!(
            "• {:?}{}{}",
            reason.code,
            reason
                .document_kind
                .map_or_else(String::new, |kind| format!(" · {kind:?}")),
            reason
                .finding_id
                .as_ref()
                .map_or_else(String::new, |id| format!(" · finding {id}"))
        ));
        if let Some(artifact) = &reason.artifact {
            ui.monospace(artifact_label(artifact));
        }
    }
}

fn show_projection_detail(
    ui: &mut egui::Ui,
    language: Language,
    record: &canisend_contracts::ProjectionReconcileRecord,
) {
    egui::Grid::new((
        "projection_detail",
        record.projection.relative_path.as_str(),
    ))
    .num_columns(2)
    .spacing([16.0, 6.0])
    .show(ui, |ui| {
        diagnostic_row(
            ui,
            language.select("Path", "路径"),
            record.projection.relative_path.as_str(),
        );
        diagnostic_row(
            ui,
            language.select("Edit status", "编辑状态"),
            &format!("{:?}", record.projection.edit_status),
        );
        diagnostic_row(
            ui,
            language.select("Generated SHA-256", "生成 SHA-256"),
            record.projection.generated_sha256.as_str(),
        );
        diagnostic_row(
            ui,
            language.select("Observed SHA-256", "观察到的 SHA-256"),
            record
                .projection
                .observed_sha256
                .as_ref()
                .map_or("—", canisend_contracts::Sha256Digest::as_str),
        );
        diagnostic_row(
            ui,
            language.select("Authoritative changed", "权威工件已改变"),
            if record.authoritative_changed {
                language.select("Yes", "是")
            } else {
                language.select("No", "否")
            },
        );
    });
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

#[cfg(test)]
mod tests {
    use canisend_app::{PackageExportRequest, ProjectionCopyAsNewRequest};

    const JOB_ID: &str = "019f2f55-7c00-7000-8000-000000000101";

    #[test]
    fn package_paths_are_job_scoped_and_copy_as_new_is_distinct() {
        assert!(
            PackageExportRequest::try_new(JOB_ID, &format!("jobs/{JOB_ID}/application")).is_ok()
        );
        assert!(PackageExportRequest::try_new(JOB_ID, "jobs/other/application").is_err());
        assert!(
            ProjectionCopyAsNewRequest::try_new(
                JOB_ID,
                &format!("jobs/{JOB_ID}/application/cover-letter.md"),
                &format!("jobs/{JOB_ID}/application/cover-letter-edited-copy.md"),
            )
            .is_ok()
        );
        assert!(
            ProjectionCopyAsNewRequest::try_new(
                JOB_ID,
                &format!("jobs/{JOB_ID}/application/cover-letter.md"),
                &format!("jobs/{JOB_ID}/application/cover-letter.md"),
            )
            .is_err()
        );
    }
}
