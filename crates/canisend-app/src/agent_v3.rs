use std::path::{Path, PathBuf};

use canisend_contracts::{
    ActorKind, ApplicationId, ApplicationLifecycleV3, ApplicationPackBindingV3, DeliverableId,
    DeliverableKindId, DeliverableStateV3, EntityId, ExecutionMode, NextAction, OperationRegistry,
    OperationSurface, PrivacyClassification, RequirementConfirmationV3, Revision, SemanticVersion,
    Sha256Digest, WORKSPACE_V3_FORMAT,
};
use canisend_store::ApplicationFlowServiceV3;
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, AgentHost, Application, ApplicationError, ApplicationFlowCommitReadModelV3,
    ApplicationFlowComposeRequestV3, ApplicationFlowCreateRequestV3,
    ApplicationFlowExportReadModelV3, ApplicationFlowExportRequestV3, ApplicationFlowPlanRequestV3,
    ApplicationFlowReadModelV3, ApplicationFlowStageReadModelV3,
    GENERIC_APPLICATION_WORKFLOW_PACK_ID, PrivateExportConsent, application::open_workspace,
    built_in_generic_application_pack,
};

pub const AGENT_V3_PROTOCOL: &str = "canisend.agent/v3";

pub const CANISEND_MCP_V3_TOOLS: [&str; 9] = [
    "canisend_agent_v3_capabilities",
    "canisend_agent_v3_context",
    "canisend_application_approve",
    "canisend_application_compose",
    "canisend_application_create",
    "canisend_application_export",
    "canisend_application_plan",
    "canisend_application_review",
    "canisend_applications_list",
];

pub const CANISEND_MCP_V3_READ_ONLY_TOOLS: [&str; 4] = [
    "canisend_agent_v3_capabilities",
    "canisend_agent_v3_context",
    "canisend_application_review",
    "canisend_applications_list",
];

pub const CANISEND_MCP_V3_GUARDED_WRITE_TOOLS: [&str; 5] = [
    "canisend_application_approve",
    "canisend_application_compose",
    "canisend_application_create",
    "canisend_application_export",
    "canisend_application_plan",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3OperationReadModel {
    pub id: String,
    pub mcp_tool: String,
    pub execution_mode: ExecutionMode,
    pub mutating: bool,
    pub requires_host_approval: bool,
    pub reads_private_content: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3CapabilitiesReadModel {
    pub product_version: SemanticVersion,
    pub protocol: String,
    pub workspace_format: String,
    pub pack: ApplicationPackBindingV3,
    pub operations: Vec<AgentV3OperationReadModel>,
    pub submission_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3DeliverableSummaryReadModel {
    pub id: DeliverableId,
    pub kind: DeliverableKindId,
    pub state: DeliverableStateV3,
    pub revision: Revision,
    pub content_sha256: Option<Sha256Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3ApplicationSummaryReadModel {
    pub id: ApplicationId,
    pub revision: Revision,
    pub snapshot_sha256: Sha256Digest,
    pub pack: ApplicationPackBindingV3,
    pub lifecycle: ApplicationLifecycleV3,
    pub requirement_count: usize,
    pub confirmed_requirement_count: usize,
    pub plan_revision: Option<Revision>,
    pub deliverables: Vec<AgentV3DeliverableSummaryReadModel>,
    pub stages: Vec<ApplicationFlowStageReadModelV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3ContextBlockerReadModel {
    pub code: String,
    pub description: String,
    pub application_id: Option<ApplicationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3ContextReadModel {
    pub product_version: SemanticVersion,
    pub protocol: String,
    pub workspace_format: String,
    pub workspace_id: EntityId,
    pub pack: ApplicationPackBindingV3,
    pub actor: ActorKind,
    pub execution_mode: ExecutionMode,
    pub applications: Vec<AgentV3ApplicationSummaryReadModel>,
    pub selected_application: Option<AgentV3ApplicationSummaryReadModel>,
    pub blockers: Vec<AgentV3ContextBlockerReadModel>,
    pub next_actions: Vec<NextAction>,
    pub privacy: PrivacyClassification,
    pub submission_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3HandoffRequest {
    pub host: AgentHost,
    pub workspace: PathBuf,
    pub selected_application_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentV3HandoffReadModel {
    pub host: AgentHost,
    pub workspace: PathBuf,
    pub selected_application_id: Option<String>,
    pub launch_command: String,
    pub start_command: String,
    pub bootstrap_prompt: String,
    pub recommended_integration: String,
    pub session_authority: String,
    pub state_authority: String,
    pub context: AgentV3ContextReadModel,
}

impl Application {
    pub fn agent_v3_capabilities()
    -> Result<ActionReceipt<AgentV3CapabilitiesReadModel>, ApplicationError> {
        let pack = exact_generic_pack_binding()?;
        let data = AgentV3CapabilitiesReadModel {
            product_version: compiled_product_version()?,
            protocol: AGENT_V3_PROTOCOL.to_owned(),
            workspace_format: WORKSPACE_V3_FORMAT.to_owned(),
            pack,
            operations: agent_v3_operations()?,
            submission_supported: false,
        };
        Ok(ActionReceipt::new(
            "agent-v3.capabilities",
            "available",
            format!(
                "Loaded {} canonical Agent v3 operations",
                data.operations.len()
            ),
            data,
        ))
    }

    pub fn agent_v3_context(
        workspace_root: &Path,
        selected_application_id: Option<&str>,
    ) -> Result<ActionReceipt<AgentV3ContextReadModel>, ApplicationError> {
        let workspace_status = Self::workspace_status(workspace_root)?.data;
        if workspace_status.pack_id != GENERIC_APPLICATION_WORKFLOW_PACK_ID {
            return Err(ApplicationError::CompatibilityUnavailable {
                message: "Agent v3 generic operations require the exact generic Application Pack"
                    .to_owned(),
                details: serde_json::json!({
                    "actual_pack_id": workspace_status.pack_id,
                    "required_pack_id": GENERIC_APPLICATION_WORKFLOW_PACK_ID,
                }),
                remediation: NextAction {
                    action: "select a generic Application workspace".to_owned(),
                    description: "Create a new workspace with the generic Pack; use Agent v2 only for the bounded academic compatibility surface".to_owned(),
                },
            });
        }

        let pack = exact_generic_pack_binding()?;
        let stored = Self::list_application_models_v3(workspace_root)?.data;
        let mut applications = Vec::with_capacity(stored.len());
        for model in stored {
            if model.snapshot.pack != pack {
                return Err(ApplicationError::CompatibilityUnavailable {
                    message: "Application Pack binding differs from the verified Agent v3 Pack"
                        .to_owned(),
                    details: serde_json::json!({
                        "application_id": model.snapshot.application.id,
                        "actual_pack": model.snapshot.pack,
                        "required_pack": pack,
                    }),
                    remediation: NextAction {
                        action: "stop and inspect Pack migration status".to_owned(),
                        description: "Do not continue an Application under a different Pack identity, version, or digest".to_owned(),
                    },
                });
            }
            let flow = Self::generic_application_flow_v3(
                workspace_root,
                model.snapshot.application.id.as_str(),
            )?
            .data;
            applications.push(body_free_summary(flow));
        }

        let selected_application = selected_application_id
            .map(|value| {
                let id = ApplicationId::try_new(value)
                    .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
                applications
                    .iter()
                    .find(|application| application.id == id)
                    .cloned()
                    .ok_or_else(|| {
                        ApplicationError::InvalidInput(format!(
                            "Application {id} is not present in this workspace"
                        ))
                    })
            })
            .transpose()?;
        let (blockers, next_actions) =
            context_guidance(&applications, selected_application.as_ref());
        let data = AgentV3ContextReadModel {
            product_version: compiled_product_version()?,
            protocol: AGENT_V3_PROTOCOL.to_owned(),
            workspace_format: workspace_status.status.workspace_format,
            workspace_id: workspace_status.status.workspace_id,
            pack,
            actor: ActorKind::HostAgent,
            execution_mode: ExecutionMode::HostAgent,
            applications,
            selected_application,
            blockers,
            next_actions,
            privacy: PrivacyClassification::Public,
            submission_supported: false,
        };
        let next_actions = data.next_actions.clone();
        Ok(ActionReceipt::new(
            "agent-v3.context",
            "available",
            format!(
                "Loaded body-free Agent v3 context with {} Application(s)",
                data.applications.len()
            ),
            data,
        )
        .with_next_actions(next_actions))
    }

    pub fn agent_v3_create_application(
        workspace_root: &Path,
        request: ApplicationFlowCreateRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        require_generic_workspace(workspace_root)?;
        let pack = built_in_generic_application_pack()?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create_with_actor(&pack, request, ActorKind::HostAgent)?;
        Ok(ActionReceipt::new(
            "application.create",
            "created",
            "Created a Pack-bound Application through the guarded Agent v3 facade",
            result,
        ))
    }

    pub fn agent_v3_plan_application(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowPlanRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        require_generic_workspace(workspace_root)?;
        Self::plan_generic_application_v3(workspace_root, application_id, request)
    }

    pub fn agent_v3_compose_application(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowComposeRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        require_generic_workspace(workspace_root)?;
        let application_id = ApplicationId::try_new(application_id)
            .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
        let pack = built_in_generic_application_pack()?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .compose_with_actor(&pack, &application_id, request, ActorKind::HostAgent)?;
        Ok(ActionReceipt::new(
            "application.compose",
            "review-required",
            "Committed Agent-composed Deliverables for explicit private review",
            result,
        ))
    }

    pub fn agent_v3_approve_application(
        workspace_root: &Path,
        application_id: &str,
        request: crate::ApplicationFlowApproveRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        require_generic_workspace(workspace_root)?;
        Self::approve_generic_application_v3(workspace_root, application_id, request)
    }

    pub fn agent_v3_export_application(
        workspace_root: &Path,
        request: ApplicationFlowExportRequestV3,
        consent: Option<PrivateExportConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, ApplicationError> {
        require_generic_workspace(workspace_root)?;
        Self::export_generic_application_v3(workspace_root, request, consent)
    }

    pub fn prepare_agent_v3_handoff(
        request: &AgentV3HandoffRequest,
    ) -> Result<ActionReceipt<AgentV3HandoffReadModel>, ApplicationError> {
        let workspace = Self::workspace_status(&request.workspace)?.data.path;
        let context =
            Self::agent_v3_context(&workspace, request.selected_application_id.as_deref())?.data;
        let quoted_workspace = shell_quote_path(&workspace)?;
        let (host_label, launch_command) = match request.host {
            AgentHost::Codex => ("Codex", format!("cd -- {quoted_workspace} && codex")),
            AgentHost::Claude => ("Claude", format!("cd -- {quoted_workspace} && claude")),
            AgentHost::Generic => (
                "the selected Agent host",
                format!("cd -- {quoted_workspace}"),
            ),
        };
        let scope = request.selected_application_id.as_deref().map_or_else(
            || "a new or existing generic Application".to_owned(),
            |id| format!("generic Application {id}"),
        );
        let bootstrap_prompt = format!(
            "Continue {scope} with the CanISend Agent v3 MCP operations. CanISend is the state authority and {host_label} is the session authority. Start with canisend_agent_v3_context, follow its exact next_actions, and keep routine context body-free. Request explicit consent before canisend_application_review or export, and request explicit user approval before Plan confirmation or the single-use review-token approval commit. Preserve the exact Pack binding and expected revision, never edit .canisend directly, and never upload or submit an Application."
        );
        let start_command = match request.host {
            AgentHost::Codex => format!(
                "cd -- {quoted_workspace} && codex {}",
                shell_quote_text(&bootstrap_prompt)
            ),
            AgentHost::Claude => format!(
                "cd -- {quoted_workspace} && claude {}",
                shell_quote_text(&bootstrap_prompt)
            ),
            AgentHost::Generic => launch_command.clone(),
        };
        let data = AgentV3HandoffReadModel {
            host: request.host,
            workspace,
            selected_application_id: request.selected_application_id.clone(),
            launch_command,
            start_command,
            bootstrap_prompt,
            recommended_integration: "mcp-agent-v3".to_owned(),
            session_authority: request.host.as_str().to_owned(),
            state_authority: "canisend".to_owned(),
            context,
        };
        Ok(ActionReceipt::new(
            "agent-v3.handoff.prepare",
            "prepared",
            format!("Prepared a body-free Agent v3 handoff for {scope}"),
            data,
        ))
    }
}

fn exact_generic_pack_binding() -> Result<ApplicationPackBindingV3, ApplicationError> {
    let pack = built_in_generic_application_pack()?;
    Ok(ApplicationPackBindingV3 {
        id: pack.manifest().id.clone(),
        version: pack.manifest().version.clone(),
        content_digest: pack.manifest().content_digest.clone(),
    })
}

fn require_generic_workspace(root: &Path) -> Result<(), ApplicationError> {
    let status = Application::workspace_status(root)?.data;
    if status.pack_id == GENERIC_APPLICATION_WORKFLOW_PACK_ID {
        return Ok(());
    }
    Err(ApplicationError::CompatibilityUnavailable {
        message: "Agent v3 mutation is unavailable for this Workspace Pack".to_owned(),
        details: serde_json::json!({
            "actual_pack_id": status.pack_id,
            "required_pack_id": GENERIC_APPLICATION_WORKFLOW_PACK_ID,
        }),
        remediation: NextAction {
            action: "use the matching Pack surface".to_owned(),
            description: "Generic Agent v3 writes never mutate the academic compatibility Pack"
                .to_owned(),
        },
    })
}

fn body_free_summary(flow: ApplicationFlowReadModelV3) -> AgentV3ApplicationSummaryReadModel {
    let snapshot = flow.stored.snapshot;
    AgentV3ApplicationSummaryReadModel {
        id: snapshot.application.id,
        revision: snapshot.application.revision,
        snapshot_sha256: flow.stored.snapshot_sha256,
        pack: snapshot.pack,
        lifecycle: snapshot.application.lifecycle,
        requirement_count: snapshot.requirements.len(),
        confirmed_requirement_count: snapshot
            .requirements
            .iter()
            .filter(|requirement| requirement.confirmation == RequirementConfirmationV3::Confirmed)
            .count(),
        plan_revision: snapshot.plan.as_ref().map(|plan| plan.revision),
        deliverables: snapshot
            .deliverables
            .into_iter()
            .map(|deliverable| AgentV3DeliverableSummaryReadModel {
                id: deliverable.id,
                kind: deliverable.kind,
                state: deliverable.state,
                revision: deliverable.revision,
                content_sha256: deliverable.content.map(|content| content.sha256),
            })
            .collect(),
        stages: flow.stages,
    }
}

fn context_guidance(
    applications: &[AgentV3ApplicationSummaryReadModel],
    selected: Option<&AgentV3ApplicationSummaryReadModel>,
) -> (Vec<AgentV3ContextBlockerReadModel>, Vec<NextAction>) {
    let mut blockers = Vec::new();
    let mut next_actions = Vec::new();
    let Some(selected) = selected else {
        if applications.is_empty() {
            next_actions.push(NextAction {
                action: "canisend_application_create".to_owned(),
                description: "Create one Pack-bound Application from reviewed source text and exact UTF-8 Requirement spans".to_owned(),
            });
        } else {
            blockers.push(AgentV3ContextBlockerReadModel {
                code: "application.not_selected".to_owned(),
                description:
                    "Select one Application ID before continuing its revision-bound workflow"
                        .to_owned(),
                application_id: None,
            });
            next_actions.push(NextAction {
                action: "canisend_agent_v3_context".to_owned(),
                description: "Call context again with one application_id returned by canisend_applications_list".to_owned(),
            });
        }
        return (blockers, next_actions);
    };

    if selected.plan_revision.is_none() {
        next_actions.push(NextAction {
            action: "canisend_application_plan".to_owned(),
            description: format!(
                "Ask the user to confirm Requirements and the Plan against Application revision {}",
                selected.revision.get()
            ),
        });
    } else if selected.deliverables.is_empty() {
        next_actions.push(NextAction {
            action: "canisend_application_compose".to_owned(),
            description: format!(
                "Compose the exact Pack-qualified Deliverables against Application revision {}",
                selected.revision.get()
            ),
        });
    } else if selected
        .deliverables
        .iter()
        .all(|deliverable| deliverable.state == DeliverableStateV3::Approved)
    {
        next_actions.push(NextAction {
            action: "canisend_application_export".to_owned(),
            description: format!(
                "After explicit private-export consent, render a local package for Application revision {}; submission remains unsupported",
                selected.revision.get()
            ),
        });
    } else if selected
        .deliverables
        .iter()
        .all(|deliverable| deliverable.state == DeliverableStateV3::ReviewRequired)
    {
        next_actions.push(NextAction {
            action: "canisend_application_review".to_owned(),
            description: format!(
                "After explicit private-read consent, review the exact Deliverable bodies at Application revision {} and use the returned single-use approval token",
                selected.revision.get()
            ),
        });
    } else {
        blockers.push(AgentV3ContextBlockerReadModel {
            code: "application.deliverables_inconsistent".to_owned(),
            description: "Deliverable states are mixed or stale; refresh context and recover before any approval or export".to_owned(),
            application_id: Some(selected.id.clone()),
        });
        next_actions.push(NextAction {
            action: "canisend_agent_v3_context".to_owned(),
            description:
                "Reload the authoritative head and inspect its exact revision and Pack binding"
                    .to_owned(),
        });
    }
    (blockers, next_actions)
}

fn agent_v3_operations() -> Result<Vec<AgentV3OperationReadModel>, ApplicationError> {
    let registry = OperationRegistry::built_in().map_err(|error| {
        ApplicationError::ResourceIntegrity(format!(
            "built-in operation registry failed validation: {error}"
        ))
    })?;
    let bindings = registry.resolved_bindings().map_err(|error| {
        ApplicationError::ResourceIntegrity(format!(
            "built-in operation bindings failed validation: {error}"
        ))
    })?;
    [
        (
            "agent-v3.capabilities",
            ExecutionMode::Deterministic,
            false,
            false,
            false,
        ),
        (
            "agent-v3.context",
            ExecutionMode::Deterministic,
            false,
            false,
            false,
        ),
        (
            "application.list",
            ExecutionMode::Deterministic,
            false,
            false,
            false,
        ),
        (
            "application.create",
            ExecutionMode::HostAgent,
            true,
            true,
            false,
        ),
        (
            "application.plan",
            ExecutionMode::UserDecision,
            true,
            true,
            false,
        ),
        (
            "application.compose",
            ExecutionMode::HostAgent,
            true,
            true,
            false,
        ),
        (
            "application.review",
            ExecutionMode::UserDecision,
            false,
            false,
            true,
        ),
        (
            "application.approve",
            ExecutionMode::UserDecision,
            true,
            true,
            false,
        ),
        (
            "application.export",
            ExecutionMode::Deterministic,
            true,
            true,
            true,
        ),
    ]
    .into_iter()
    .map(
        |(id, execution_mode, mutating, requires_host_approval, reads_private_content)| {
            let mcp_tool = bindings
                .iter()
                .find(|binding| {
                    binding.surface == OperationSurface::Mcp && binding.operation.as_str() == id
                })
                .map(|binding| binding.leaf.clone())
                .ok_or_else(|| {
                    ApplicationError::ResourceIntegrity(format!(
                        "canonical Agent v3 operation {id} has no exact MCP binding"
                    ))
                })?;
            Ok(AgentV3OperationReadModel {
                id: id.to_owned(),
                mcp_tool,
                execution_mode,
                mutating,
                requires_host_approval,
                reads_private_content,
            })
        },
    )
    .collect()
}

fn compiled_product_version() -> Result<SemanticVersion, ApplicationError> {
    SemanticVersion::try_new(env!("CARGO_PKG_VERSION")).map_err(|error| {
        ApplicationError::ResourceIntegrity(format!("compiled product version is invalid: {error}"))
    })
}

fn shell_quote_path(path: &Path) -> Result<String, ApplicationError> {
    let value = path.to_str().ok_or_else(|| {
        ApplicationError::InvalidInput("Agent handoff requires a UTF-8 workspace path".to_owned())
    })?;
    Ok(shell_quote_text(value))
}

fn shell_quote_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use canisend_contracts::{
        ApplicationFieldValueV3, PlannedDeliverableDispositionV3, RequirementPriorityV3,
        WorkflowPackItemId,
    };

    use super::*;
    use crate::{
        ApplicationFlowDeliverableDraftV3, ApplicationFlowPlannedDeliverableV3,
        ApplicationFlowRequirementDraftV3,
    };

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-agent-v3-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item")
    }

    fn create_request() -> ApplicationFlowCreateRequestV3 {
        let source = "Provide one primary narrative.";
        ApplicationFlowCreateRequestV3 {
            title: "Synthetic neutral Application".to_owned(),
            opportunity_metadata: BTreeMap::from([(
                item("organization"),
                ApplicationFieldValueV3::ShortText("Example Organization".to_owned()),
            )]),
            application_metadata: BTreeMap::new(),
            source_text: source.to_owned(),
            requirements: vec![ApplicationFlowRequirementDraftV3 {
                category: item("format"),
                statement: source.to_owned(),
                priority: RequirementPriorityV3::Mandatory,
                start_byte: 0,
                end_byte: source.len() as u64,
            }],
        }
    }

    #[test]
    fn context_is_body_free_and_routes_exact_next_actions() {
        let root = root("context");
        Application::initialize_workspace_v3(&root).expect("workspace");
        let empty = Application::agent_v3_context(&root, None)
            .expect("empty context")
            .data;
        assert_eq!(empty.next_actions[0].action, "canisend_application_create");
        assert!(!empty.submission_supported);

        let created = Application::agent_v3_create_application(&root, create_request())
            .expect("create")
            .data;
        let id = created.stored.snapshot.application.id.to_string();
        let selected = Application::agent_v3_context(&root, Some(&id))
            .expect("selected context")
            .data;
        assert_eq!(selected.next_actions[0].action, "canisend_application_plan");
        let encoded = serde_json::to_string(&selected).expect("context JSON");
        assert!(!encoded.contains("Provide one primary narrative"));
        assert!(!encoded.contains("Synthetic neutral Application"));
        assert!(!encoded.contains("Example Organization"));

        let history = Application::application_model_history_v3(&root, &id)
            .expect("history")
            .data;
        assert_eq!(history[0].actor, ActorKind::HostAgent);
        fs::remove_dir_all(root).expect("remove workspace");
    }

    #[test]
    fn agent_compose_records_host_actor_while_user_decisions_remain_user_owned() {
        let root = root("actors");
        Application::initialize_workspace_v3(&root).expect("workspace");
        let created = Application::agent_v3_create_application(&root, create_request())
            .expect("create")
            .data;
        let id = created.stored.snapshot.application.id.to_string();
        let planned = Application::agent_v3_plan_application(
            &root,
            &id,
            ApplicationFlowPlanRequestV3 {
                expected_revision: Revision::try_new(1).expect("revision"),
                decision: item("proceed"),
                deliverables: vec![ApplicationFlowPlannedDeliverableV3 {
                    kind: item("primary-document"),
                    disposition: PlannedDeliverableDispositionV3::Required,
                    rationale: "Required by the reviewed source".to_owned(),
                    constraints: Vec::new(),
                    execution_mode: Some(ExecutionMode::HostAgent),
                }],
            },
        )
        .expect("plan")
        .data;
        Application::agent_v3_compose_application(
            &root,
            &id,
            ApplicationFlowComposeRequestV3 {
                expected_revision: planned.commit.stored.snapshot.application.revision,
                deliverables: vec![ApplicationFlowDeliverableDraftV3 {
                    kind: item("primary-document"),
                    title: "Primary document".to_owned(),
                    media_type: "text/markdown".to_owned(),
                    content: "Reviewed evidence-bound content.".to_owned(),
                }],
            },
        )
        .expect("compose");
        let history = Application::application_model_history_v3(&root, &id)
            .expect("history")
            .data;
        assert_eq!(history[0].actor, ActorKind::HostAgent);
        assert_eq!(history[1].actor, ActorKind::User);
        assert_eq!(history[2].actor, ActorKind::HostAgent);
        fs::remove_dir_all(root).expect("remove workspace");
    }

    #[test]
    fn handoffs_for_codex_and_claude_are_generic_and_submission_safe() {
        let root = root("handoff");
        Application::initialize_workspace_v3(&root).expect("workspace");
        for host in [AgentHost::Codex, AgentHost::Claude] {
            let handoff = Application::prepare_agent_v3_handoff(&AgentV3HandoffRequest {
                host,
                workspace: root.clone(),
                selected_application_id: None,
            })
            .expect("handoff")
            .data;
            assert!(handoff.bootstrap_prompt.contains("Agent v3"));
            assert!(handoff.bootstrap_prompt.contains("never upload or submit"));
            assert!(!handoff.bootstrap_prompt.to_lowercase().contains("academic"));
            assert!(!handoff.bootstrap_prompt.to_lowercase().contains("job"));
        }
        fs::remove_dir_all(root).expect("remove workspace");
    }

    #[test]
    fn generic_agent_v3_fails_closed_on_academic_workspace() {
        let root = root("wrong-pack");
        Application::initialize_workspace(&root).expect("academic workspace");
        let error = Application::agent_v3_context(&root, None).expect_err("wrong Pack");
        assert!(matches!(
            error,
            ApplicationError::CompatibilityUnavailable { .. }
        ));
        fs::remove_dir_all(root).expect("remove workspace");
    }
}
