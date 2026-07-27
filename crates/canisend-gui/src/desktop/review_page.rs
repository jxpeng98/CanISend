use super::*;

impl CanISendDesktop {
    pub(super) fn show_review_workspace(&mut self, ui: &mut egui::Ui, job_id: &str) {
        self.review_form.select_job(job_id);

        accessible_heading(
            ui,
            self.language
                .select("Review findings and dispositions", "审阅发现与人工处置"),
            2,
        );
        ui.label(self.language.select(
            "Deterministic findings are read-only. Only findings marked for human review can be accepted as risk or dismissed, and every selected disposition requires a rationale.",
            "确定性发现为只读。只有标记为人工审阅的发现可以接受风险或驳回，并且每项选定的处置都必须填写理由。",
        ));
        ui.colored_label(
            theme::warning(self.dark_mode),
            self.language.select(
                "Review messages, targets, and rationales are private local application material.",
                "审阅信息、目标和处置理由属于本地私有申请材料。",
            ),
        );
        ui.add_space(10.0);

        let consent = ui.add_enabled(
            self.activity.is_none(),
            egui::Checkbox::new(
                &mut self.review_form.private_read_consent,
                self.language.select(
                    "Allow this user-invoked private review",
                    "允许本次由用户发起的私有审阅",
                ),
            ),
        );
        if consent.changed() {
            self.review_form.error = None;
            if !self.review_form.private_read_consent {
                self.review_form.clear_loaded_private_data();
            }
        }
        let load = ui.add_enabled(
            self.activity.is_none(),
            theme::primary_button(self.language.select("Load current review", "加载当前审阅")),
        );
        if self.pending_focus == Some(FocusTarget::ReviewLoad) {
            load.request_focus();
            self.pending_focus = None;
        }
        if load.clicked() {
            if self.review_form.private_read_consent {
                self.review_form.error = None;
                self.load_review(job_id.to_owned(), ui.ctx().clone());
            } else {
                self.review_form.error = Some(
                    self.language
                        .select(
                            "Confirm private review access before loading",
                            "加载前请确认允许访问私有审阅内容",
                        )
                        .to_owned(),
                );
                self.pending_focus = Some(FocusTarget::ReviewLoad);
            }
        }
        if let Some(error) = &self.review_form.error {
            accessible_error(ui, theme::error(self.dark_mode), error);
        }

        let Some(review) = self.review_form.current.clone() else {
            ui.add_space(12.0);
            ui.label(
                RichText::new(self.language.select(
                    "No private review findings are loaded.",
                    "尚未加载任何私有审阅发现。",
                ))
                .weak(),
            );
            return;
        };

        ui.add_space(18.0);
        self.show_review_summary(ui, &review);
        ui.add_space(18.0);
        accessible_heading(ui, self.language.select("Current findings", "当前发现"), 3);

        if review.findings.is_empty() {
            ui.colored_label(
                theme::positive(self.dark_mode),
                self.language
                    .select("No review findings.", "没有审阅发现。"),
            );
        }

        let language = self.language;
        let dark_mode = self.dark_mode;
        for finding in &review.findings {
            ui.add_space(10.0);
            let decision = self.review_form.candidate_mut(&finding.id);
            show_finding(ui, language, dark_mode, finding, decision);
        }

        let Some(candidate) = self.review_form.candidate.clone() else {
            return;
        };
        if candidate.decisions.is_empty() {
            ui.add_space(16.0);
            ui.label(self.language.select(
                "There are no human-review findings to disposition.",
                "没有需要人工处置的发现。",
            ));
            return;
        }

        ui.add_space(18.0);
        ui.separator();
        let confirmation = ui.add_enabled(
            self.activity.is_none(),
            egui::Checkbox::new(
                &mut self.review_form.downstream_effects_confirmed,
                self.language.select(
                    "I reviewed the selected dispositions and understand they change package readiness",
                    "我已审阅所选处置，并了解这些处置会改变申请包就绪状态",
                ),
            ),
        );
        paint_focus_ring(ui, &confirmation);

        let confirm = ui.add_enabled(
            self.activity.is_none(),
            theme::next_button(
                self.language
                    .select("Confirm selected dispositions", "确认所选处置"),
            ),
        );
        if self.pending_focus == Some(FocusTarget::ReviewConfirm) {
            confirm.request_focus();
            self.pending_focus = None;
        }
        if confirm.clicked() {
            if let Some(issue) = self.review_form.validation_issue() {
                self.review_form.error = Some(review_validation_message(self.language, issue));
                self.pending_focus = Some(FocusTarget::ReviewConfirm);
            } else {
                self.review_form.error = None;
                self.confirm_review(job_id.to_owned(), candidate, ui.ctx().clone());
            }
        }
    }

    fn show_review_summary(
        &self,
        ui: &mut egui::Ui,
        review: &canisend_contracts::ReviewFindingsRecord,
    ) {
        use canisend_contracts::{FindingAuthority, FindingSeverity, FindingStatus};

        accessible_heading(ui, self.language.select("Review revision", "审阅修订"), 3);
        let deterministic_blockers = review
            .findings
            .iter()
            .filter(|finding| {
                finding.authority == FindingAuthority::Deterministic
                    && finding.severity == FindingSeverity::Blocker
                    && finding.status == FindingStatus::Open
            })
            .count();
        let pending_human = review
            .findings
            .iter()
            .filter(|finding| {
                finding.authority == FindingAuthority::HumanReview
                    && finding.status == FindingStatus::Open
            })
            .count();
        egui::Grid::new(("review_summary", review.id.as_str()))
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                diagnostic_row(
                    ui,
                    self.language.select("Review ID", "审阅 ID"),
                    review.id.as_str(),
                );
                diagnostic_row(
                    ui,
                    self.language.select("Revision", "修订"),
                    &review.revision.get().to_string(),
                );
                diagnostic_row(
                    ui,
                    self.language.select("Document set", "文档集合"),
                    &artifact_label(&review.document_set_artifact),
                );
                diagnostic_row(
                    ui,
                    self.language
                        .select("Deterministic blockers", "确定性阻断项"),
                    &deterministic_blockers.to_string(),
                );
                diagnostic_row(
                    ui,
                    self.language
                        .select("Pending human findings", "待处理人工发现"),
                    &pending_human.to_string(),
                );
            });
        if let Some(candidate) = &self.review_form.candidate {
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    self.language
                        .select("Exact review artifact", "精确审阅工件"),
                    artifact_label(&candidate.review_artifact)
                ))
                .monospace()
                .weak(),
            );
        }
    }
}

fn show_finding(
    ui: &mut egui::Ui,
    language: Language,
    dark_mode: bool,
    finding: &canisend_contracts::FindingRecord,
    decision: Option<&mut canisend_contracts::FindingDispositionCandidateRecord>,
) {
    use canisend_contracts::FindingAuthority;

    let is_human = finding.authority == FindingAuthority::HumanReview;
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(1.0, theme::SLATE_300))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&finding.code).strong());
                ui.label(authority_label(language, finding.authority));
                ui.label(format!(
                    "{} · {} · r{}",
                    severity_label(language, finding.severity),
                    status_label(language, finding.status),
                    finding.revision.get()
                ));
            });
            ui.add(egui::Label::new(&finding.message).wrap());
            ui.label(
                RichText::new(format!(
                    "{}: {:?}",
                    language.select("Target", "目标"),
                    finding.target
                ))
                .monospace()
                .weak(),
            );
            for related in &finding.related_targets {
                ui.label(
                    RichText::new(format!(
                        "{}: {related:?}",
                        language.select("Related", "相关目标")
                    ))
                    .weak(),
                );
            }
            if let Some(resolution) = &finding.suggested_resolution {
                ui.label(format!(
                    "{}: {resolution}",
                    language.select("Suggested resolution", "建议解决方式")
                ));
            }
            if let Some(reason) = &finding.disposition_reason {
                ui.label(format!(
                    "{}: {reason}",
                    language.select("Current rationale", "当前理由")
                ));
            }

            ui.add_space(8.0);
            if !is_human {
                ui.colored_label(
                    theme::warning(dark_mode),
                    language.select(
                        "Read-only deterministic finding · regenerate current documents to resolve it",
                        "只读确定性发现 · 请重新生成当前文档以解决",
                    ),
                );
                return;
            }

            let Some(decision) = decision else {
                accessible_error(
                    ui,
                    theme::error(dark_mode),
                    language.select(
                        "The exact human disposition candidate is unavailable",
                        "精确的人工处置候选项不可用",
                    ),
                );
                return;
            };
            ui.label(
                RichText::new(language.select("Human disposition", "人工处置")).strong(),
            );
            let before = decision.disposition;
            egui::ComboBox::from_id_salt(("review_disposition", finding.id.as_str()))
                .selected_text(disposition_label(language, decision.disposition))
                .show_ui(ui, |ui| {
                    for value in crate::state::finding_disposition_values() {
                        ui.selectable_value(
                            &mut decision.disposition,
                            value,
                            disposition_label(language, value),
                        );
                    }
                });
            if before != decision.disposition && decision.disposition.is_none() {
                decision.rationale = None;
            }
            if decision.disposition.is_some() && decision.rationale.is_none() {
                decision.rationale = Some(String::new());
            }
            let rationale = decision.rationale.get_or_insert_with(String::new);
            ui.add_enabled(
                decision.disposition.is_some(),
                egui::TextEdit::multiline(rationale)
                    .desired_rows(2)
                    .hint_text(language.select(
                        "Required rationale for this selected disposition",
                        "此项处置的必填理由",
                    )),
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

fn disposition_label(
    language: Language,
    disposition: Option<canisend_contracts::FindingDisposition>,
) -> &'static str {
    use canisend_contracts::FindingDisposition;
    language.select(
        match disposition {
            None => "No new disposition",
            Some(FindingDisposition::AcceptedRisk) => "Accept risk",
            Some(FindingDisposition::Dismissed) => "Dismiss",
        },
        match disposition {
            None => "不作新处置",
            Some(FindingDisposition::AcceptedRisk) => "接受风险",
            Some(FindingDisposition::Dismissed) => "驳回",
        },
    )
}

fn authority_label(
    language: Language,
    authority: canisend_contracts::FindingAuthority,
) -> &'static str {
    use canisend_contracts::FindingAuthority;
    language.select(
        match authority {
            FindingAuthority::Deterministic => "Deterministic",
            FindingAuthority::HumanReview => "Human review",
        },
        match authority {
            FindingAuthority::Deterministic => "确定性",
            FindingAuthority::HumanReview => "人工审阅",
        },
    )
}

fn severity_label(
    language: Language,
    severity: canisend_contracts::FindingSeverity,
) -> &'static str {
    use canisend_contracts::FindingSeverity;
    language.select(
        match severity {
            FindingSeverity::Info => "Info",
            FindingSeverity::Warning => "Warning",
            FindingSeverity::Blocker => "Blocker",
        },
        match severity {
            FindingSeverity::Info => "信息",
            FindingSeverity::Warning => "警告",
            FindingSeverity::Blocker => "阻断",
        },
    )
}

fn status_label(language: Language, status: canisend_contracts::FindingStatus) -> &'static str {
    use canisend_contracts::FindingStatus;
    language.select(
        match status {
            FindingStatus::Open => "Open",
            FindingStatus::AcceptedRisk => "Accepted risk",
            FindingStatus::Resolved => "Resolved",
            FindingStatus::Dismissed => "Dismissed",
        },
        match status {
            FindingStatus::Open => "待处理",
            FindingStatus::AcceptedRisk => "已接受风险",
            FindingStatus::Resolved => "已解决",
            FindingStatus::Dismissed => "已驳回",
        },
    )
}

fn review_validation_message(
    language: Language,
    issue: crate::state::ReviewValidationIssue,
) -> String {
    use crate::state::ReviewValidationIssue;
    language
        .select(
            match issue {
                ReviewValidationIssue::NoSelection => {
                    "Select at least one human-review disposition"
                }
                ReviewValidationIssue::MissingRationale => {
                    "Every selected disposition requires a non-empty rationale"
                }
                ReviewValidationIssue::DownstreamEffectsNotConfirmed => {
                    "Confirm the package-readiness effect before committing dispositions"
                }
            },
            match issue {
                ReviewValidationIssue::NoSelection => "请至少选择一项人工审阅处置",
                ReviewValidationIssue::MissingRationale => "每项选定的处置都必须填写非空理由",
                ReviewValidationIssue::DownstreamEffectsNotConfirmed => {
                    "提交处置前请确认其对申请包就绪状态的影响"
                }
            },
        )
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{disposition_label, review_validation_message};
    use crate::{i18n::Language, state::ReviewValidationIssue};

    #[test]
    fn review_labels_cover_both_supported_languages() {
        assert_eq!(
            disposition_label(Language::English, None),
            "No new disposition"
        );
        assert_eq!(
            disposition_label(Language::SimplifiedChinese, None),
            "不作新处置"
        );
        assert_eq!(
            review_validation_message(Language::English, ReviewValidationIssue::MissingRationale),
            "Every selected disposition requires a non-empty rationale"
        );
        assert_eq!(
            review_validation_message(
                Language::SimplifiedChinese,
                ReviewValidationIssue::MissingRationale
            ),
            "每项选定的处置都必须填写非空理由"
        );
    }
}
