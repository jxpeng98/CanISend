use super::*;

impl CanISendDesktop {
    pub(super) fn show_render_workspace(&mut self, ui: &mut egui::Ui, job_id: &str) {
        self.render_form.select_job(job_id);
        accessible_heading(
            ui,
            self.language
                .select("Validated PDF rendering", "已验证 PDF 渲染"),
            2,
        );
        ui.label(self.language.select(
            "CanISend projects authoritative structured documents to trusted Typst, compiles inside the Rust process, validates each PDF, and freezes exact artifact metadata.",
            "CanISend 将权威结构化文档投影为可信 Typst，在 Rust 进程内编译、验证每个 PDF，并冻结精确工件元数据。",
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
                egui::Button::new(self.language.select("Load current render", "加载当前渲染"))
                    .min_size(egui::vec2(155.0, 44.0)),
            );
            if self.pending_focus == Some(FocusTarget::RenderBuild) {
                load.request_focus();
                self.pending_focus = None;
            }
            if load.clicked() {
                self.render_form.error = None;
                self.render_manifest(job_id.to_owned(), false, ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    self.activity.is_none(),
                    theme::primary_button(
                        self.language
                            .select("Build validated PDFs", "构建并验证 PDF"),
                    ),
                )
                .clicked()
            {
                self.render_form.error = None;
                self.render_manifest(job_id.to_owned(), true, ui.ctx().clone());
            }
        });
        ui.label(
            RichText::new(self.language.select(
                "Build is deterministic and idempotent for the exact current package revision.",
                "对于精确的当前申请包修订版本，构建是确定且幂等的。",
            ))
            .weak(),
        );
        if let Some(error) = &self.render_form.error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }

        let Some(manifest) = self.render_form.manifest.clone() else {
            ui.add_space(12.0);
            ui.label(
                RichText::new(self.language.select(
                    "No current render manifest is loaded.",
                    "尚未加载当前渲染清单。",
                ))
                .weak(),
            );
            return;
        };

        ui.add_space(18.0);
        show_render_manifest(ui, self.language, &manifest);
        ui.add_space(18.0);
        self.show_pdf_export(ui, job_id);
    }

    fn show_pdf_export(&mut self, ui: &mut egui::Ui, job_id: &str) {
        accessible_heading(
            ui,
            self.language.select("Private PDF export", "私密 PDF 导出"),
            3,
        );
        let destination_label = ui.label(
            self.language
                .select("Workspace-relative directory", "工作区相对目录"),
        );
        let destination = ui
            .add_enabled(
                self.activity.is_none(),
                egui::TextEdit::singleline(&mut self.render_form.export_destination)
                    .desired_width(f32::INFINITY)
                    .hint_text(format!("jobs/{job_id}/rendered")),
            )
            .labelled_by(destination_label.id);
        if destination.changed() {
            self.render_form.export = None;
            self.render_form.error = None;
        }
        let consent = ui.add_enabled(
            self.activity.is_none(),
            egui::Checkbox::new(
                &mut self.render_form.private_export_consent,
                self.language.select(
                    "Allow this user-invoked export of private PDF bodies",
                    "允许本次由用户发起的私密 PDF 正文导出",
                ),
            ),
        );
        if consent.changed() {
            self.render_form.error = None;
        }
        ui.label(
            RichText::new(self.language.select(
                "The destination must be new or empty and remain under jobs/JOB_ID/. Existing files are never overwritten.",
                "目标必须为新目录或空目录，并位于 jobs/JOB_ID/ 下。现有文件绝不会被覆盖。",
            ))
            .weak(),
        );
        let export = ui.add_enabled(
            self.activity.is_none(),
            theme::next_button(
                self.language
                    .select("Export validated PDFs", "导出已验证 PDF"),
            ),
        );
        if self.pending_focus == Some(FocusTarget::RenderExport) {
            export.request_focus();
            self.pending_focus = None;
        }
        if export.clicked() {
            if !self.render_form.private_export_consent {
                self.render_form.error = Some(
                    self.language
                        .select(
                            "Confirm private export consent before writing PDF bodies",
                            "写出 PDF 正文前请确认私密导出同意",
                        )
                        .to_owned(),
                );
                self.pending_focus = Some(FocusTarget::RenderExport);
            } else {
                match RenderExportRequest::try_new(
                    job_id,
                    self.render_form.export_destination.trim(),
                ) {
                    Ok(request) => {
                        self.render_form.error = None;
                        self.export_render(request, true, ui.ctx().clone());
                    }
                    Err(error) => {
                        self.render_form.error = Some(error.to_string());
                        self.pending_focus = Some(FocusTarget::RenderExport);
                    }
                }
            }
        }

        if let Some(export) = &self.render_form.export {
            ui.add_space(10.0);
            ui.colored_label(
                theme::positive(self.dark_mode),
                self.language
                    .select("Validated PDF export complete", "已完成验证 PDF 导出"),
            );
            egui::Grid::new(("render_export", export.destination.as_str()))
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    diagnostic_row(
                        ui,
                        self.language.select("Destination", "目标"),
                        export.destination.as_str(),
                    );
                    diagnostic_row(
                        ui,
                        self.language
                            .select("Submission performed", "已执行门户提交"),
                        if export.submission_performed {
                            self.language.select("Yes", "是")
                        } else {
                            self.language.select("No", "否")
                        },
                    );
                });
            for path in &export.files {
                ui.monospace(path.as_str());
            }
        }
    }
}

fn show_render_manifest(
    ui: &mut egui::Ui,
    language: Language,
    manifest: &canisend_contracts::RenderManifestRecord,
) {
    let pages = manifest
        .documents
        .iter()
        .map(|document| u64::from(document.page_count))
        .sum::<u64>();
    let bytes = manifest
        .documents
        .iter()
        .map(|document| document.byte_count)
        .sum::<u64>();
    egui::Grid::new(("render_manifest", manifest.id.as_str()))
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            diagnostic_row(
                ui,
                language.select("Render manifest", "渲染清单"),
                manifest.id.as_str(),
            );
            diagnostic_row(
                ui,
                language.select("Revision", "修订"),
                &manifest.revision.get().to_string(),
            );
            diagnostic_row(
                ui,
                language.select("Package artifact", "申请包工件"),
                &artifact_label(&manifest.package_artifact),
            );
            diagnostic_row(
                ui,
                language.select("Rendered documents", "已渲染文档"),
                &manifest.documents.len().to_string(),
            );
            diagnostic_row(
                ui,
                language.select("PDF pages", "PDF 页数"),
                &pages.to_string(),
            );
            diagnostic_row(
                ui,
                language.select("PDF bytes", "PDF 字节数"),
                &bytes.to_string(),
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
    for document in &manifest.documents {
        ui.add_space(10.0);
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(Stroke::new(1.0, theme::SLATE_300))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(RichText::new(format!("{:?}", document.kind)).strong());
                egui::Grid::new(("render_document", document.pdf_artifact.id.as_str()))
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        diagnostic_row(
                            ui,
                            language.select("Document artifact", "文档工件"),
                            &artifact_label(&document.document_artifact),
                        );
                        diagnostic_row(
                            ui,
                            language.select("Typst artifact", "Typst 工件"),
                            &artifact_label(&document.typst_artifact),
                        );
                        diagnostic_row(
                            ui,
                            language.select("PDF artifact", "PDF 工件"),
                            &artifact_label(&document.pdf_artifact),
                        );
                        diagnostic_row(
                            ui,
                            language.select("Pages", "页数"),
                            &document.page_count.to_string(),
                        );
                        diagnostic_row(
                            ui,
                            language.select("Bytes", "字节"),
                            &document.byte_count.to_string(),
                        );
                        diagnostic_row(
                            ui,
                            language.select("Warnings", "警告数"),
                            &document.warning_count.to_string(),
                        );
                        diagnostic_row(
                            ui,
                            language.select("Elapsed milliseconds", "耗时（毫秒）"),
                            &document.elapsed_millis.to_string(),
                        );
                    });
            });
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

#[cfg(test)]
mod tests {
    use canisend_app::RenderExportRequest;

    const JOB_ID: &str = "019f2f55-7c00-7000-8000-000000000101";

    #[test]
    fn render_export_paths_remain_inside_the_selected_job() {
        assert!(RenderExportRequest::try_new(JOB_ID, &format!("jobs/{JOB_ID}/rendered")).is_ok());
        assert!(RenderExportRequest::try_new(JOB_ID, "jobs/other/rendered").is_err());
    }
}
