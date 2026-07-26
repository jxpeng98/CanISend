use canisend_app::{ActionReceipt, CliInstallState, CliInstallStatus};
use canisend_contracts::{SourceKind, StageExecutionStatus, WorkflowStage, WorkflowStatusData};
use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke};

use crate::{i18n::Language, state::Page, theme};

pub(crate) fn set_accesskit_role(
    ctx: &egui::Context,
    id: egui::Id,
    role: egui::accesskit::Role,
    label: Option<&str>,
) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(role);
        if let Some(label) = label {
            node.set_label(label);
        }
    });
}

pub(crate) fn accessible_heading(ui: &mut egui::Ui, text: &str, level: usize) -> egui::Response {
    let response = ui.heading(text);
    set_accesskit_role(ui.ctx(), response.id, egui::accesskit::Role::Heading, None);
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_level(level));
    response
}

pub(crate) fn accessible_live_region(
    ui: &mut egui::Ui,
    text: String,
    polite: bool,
) -> egui::Response {
    let response = ui.label(text);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(if polite {
            egui::accesskit::Role::Status
        } else {
            egui::accesskit::Role::Alert
        });
        node.set_live(if polite {
            egui::accesskit::Live::Polite
        } else {
            egui::accesskit::Live::Assertive
        });
    });
    response
}

pub(crate) fn accessible_error(ui: &mut egui::Ui, color: Color32, text: &str) -> egui::Response {
    let response = ui.colored_label(color, text);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Alert);
        node.set_live(egui::accesskit::Live::Assertive);
    });
    response
}

pub(crate) fn paint_focus_ring(ui: &egui::Ui, response: &egui::Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            6,
            Stroke::new(2.0, theme::AMBER_600),
            egui::StrokeKind::Inside,
        );
    }
}

pub(crate) fn keep_focused_visible(response: &egui::Response) {
    if response.has_focus() {
        response.scroll_to_me(Some(Align::Center));
    }
}

pub(crate) fn localized_receipt_summary<T>(
    receipt: &ActionReceipt<T>,
    language: Language,
) -> String {
    if language == Language::English {
        return receipt.summary.clone();
    }
    match receipt.operation.as_str() {
        "workspace.init" => "工作区已创建",
        "workspace.status" => "工作区已打开",
        "workspace.check" => "工作区完整性检查已完成",
        "workspace.backup" => "经过验证的工作区备份已创建",
        "workspace.restore" => "工作区已从经过验证的备份恢复",
        "workspace.repair" => "工作区托管文件已修复",
        "job.create" => "职位已创建",
        "job.archive" => "职位已归档",
        "job.import" => "职位来源已导入",
        "workflow.start" => "工作流已启动",
        "workflow.status" => "工作流状态已更新",
        "cli.install" => "CanISend CLI 已安装或更新",
        "cli.uninstall" => "受管理的 CanISend CLI 已卸载",
        "product.update.check" => "CanISend 更新检查已完成",
        "product.doctor" => "原生自检已完成",
        _ => receipt.summary.as_str(),
    }
    .to_owned()
}

pub(crate) fn localized_workspace_alias_error(error: String, language: Language) -> String {
    if language == Language::English {
        return error;
    }
    match error.as_str() {
        "Workspace name is required" => "必须填写工作区名称".to_owned(),
        "Workspace name cannot start or end with whitespace" => {
            "工作区名称不能以空白字符开头或结尾".to_owned()
        }
        "Workspace name cannot contain control characters" => {
            "工作区名称不能包含控制字符".to_owned()
        }
        _ if error.starts_with("Workspace name must be at most") => {
            "工作区名称不能超过 128 字节".to_owned()
        }
        _ => error,
    }
}

pub(crate) fn page_accessible_label(page: Page, language: Language) -> &'static str {
    language.text(match page {
        Page::Overview => "Overview content",
        Page::Jobs => "Jobs content",
        Page::Workspaces => "Workspaces content",
        Page::CommandLine => "Command line content",
        Page::Diagnostics => "Diagnostics content",
    })
}

pub(crate) fn validate_job_form(
    title: &str,
    institution: &str,
    language: Language,
) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err(language.text("Job title is required").to_owned());
    }
    if institution.trim().is_empty() {
        return Err(language.text("Institution is required").to_owned());
    }
    if title.len() > 512 || institution.len() > 512 {
        return Err(language
            .text("Title and institution must each be at most 512 bytes")
            .to_owned());
    }
    Ok(())
}

pub(crate) fn metric_card(ui: &mut egui::Ui, label: &str, value: &str, help: &str) {
    egui::Frame::new()
        .fill(if ui.visuals().dark_mode {
            Color32::from_rgb(38, 48, 52)
        } else {
            Color32::WHITE
        })
        .stroke(Stroke::new(1.0, theme::SLATE_300))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(RichText::new(label).weak());
            ui.label(RichText::new(value).size(24.0).strong());
            ui.label(RichText::new(help).small().weak());
        });
}

pub(crate) fn workflow_timeline(
    ui: &mut egui::Ui,
    workflow: &WorkflowStatusData,
    language: Language,
) {
    for state in &workflow.stages {
        ui.horizontal(|ui| {
            let (color, label) = stage_status_style(state.status, ui.visuals().dark_mode, language);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, color);
            ui.label(RichText::new(stage_label(state.stage, language)).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.colored_label(color, label);
            });
        });
        ui.add_space(5.0);
    }
    if !workflow.blockers.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new(language.text("Blockers")).strong());
        for blocker in &workflow.blockers {
            ui.label(format!("{}: {}", blocker.code, blocker.description));
        }
    }
}

pub(crate) fn stage_status_style(
    status: StageExecutionStatus,
    dark: bool,
    language: Language,
) -> (Color32, &'static str) {
    match status {
        StageExecutionStatus::Complete => (theme::positive(dark), language.text("Complete")),
        StageExecutionStatus::Ready => (theme::warning(dark), language.text("Ready")),
        StageExecutionStatus::Running => (theme::info(dark), language.text("Running")),
        StageExecutionStatus::AwaitingUser => {
            (theme::warning(dark), language.text("Awaiting user"))
        }
        StageExecutionStatus::Blocked => (theme::neutral(dark), language.text("Blocked")),
        StageExecutionStatus::Stale => (theme::error(dark), language.text("Stale")),
    }
}

pub(crate) fn stage_label(stage: WorkflowStage, language: Language) -> &'static str {
    language.text(match stage {
        WorkflowStage::Intake => "Intake",
        WorkflowStage::Parse => "Parse",
        WorkflowStage::Criteria => "Criteria",
        WorkflowStage::Evidence => "Evidence",
        WorkflowStage::Match => "Match",
        WorkflowStage::Plan => "Plan",
        WorkflowStage::Draft => "Draft",
        WorkflowStage::Review => "Review",
        WorkflowStage::Package => "Package",
        WorkflowStage::Render => "Render",
    })
}

pub(crate) fn source_kind_label(kind: SourceKind, language: Language) -> &'static str {
    language.select(
        match kind {
            SourceKind::LocalFile => "Local file",
            SourceKind::UserUrl => "User URL",
            SourceKind::DiscoveryLead => "Discovery lead",
            SourceKind::ManualText => "Manual text",
        },
        match kind {
            SourceKind::LocalFile => "本地文件",
            SourceKind::UserUrl => "用户 URL",
            SourceKind::DiscoveryLead => "发现的职位线索",
            SourceKind::ManualText => "手动文本",
        },
    )
}

pub(crate) fn diagnostic_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).strong());
    ui.add(egui::Label::new(value).truncate())
        .on_hover_text(value);
    ui.end_row();
}

pub(crate) fn cli_state_style(
    status: &CliInstallStatus,
    dark: bool,
    language: Language,
) -> (&'static str, Color32) {
    match status.state {
        CliInstallState::NotInstalled => (language.text("Not installed"), theme::neutral(dark)),
        CliInstallState::Current if status.active_is_managed => {
            (language.text("Ready"), theme::positive(dark))
        }
        CliInstallState::Current => (language.text("Installed; not active"), theme::warning(dark)),
        CliInstallState::UpdateAvailable => {
            (language.text("Update available"), theme::warning(dark))
        }
        CliInstallState::MigrationAvailable => {
            (language.text("Migration available"), theme::warning(dark))
        }
        CliInstallState::NewerInstalled => (
            language.text("Newer version installed"),
            theme::positive(dark),
        ),
        CliInstallState::Modified => (language.text("Needs attention"), theme::error(dark)),
        CliInstallState::SourceUnavailable => (
            language.text("CLI missing from package"),
            theme::error(dark),
        ),
    }
}

pub(crate) fn command_copy_row(ui: &mut egui::Ui, command: &str, language: Language) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.monospace(command);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button(language.text("Copy")).clicked() {
                        ui.ctx().copy_text(command.to_owned());
                    }
                });
            });
        });
    ui.add_space(6.0);
}
