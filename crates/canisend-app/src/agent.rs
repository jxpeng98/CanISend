use std::path::{Path, PathBuf};

use canisend_contracts::{
    AGENT_PROTOCOL, ActorKind, AgentContextBlocker, AgentContextData, CapabilitiesData, ErrorCode,
    ExecutionMode, NextAction, PrivacyClassification, RESOURCE_FORMAT, SemanticVersion,
    WORKSPACE_FORMAT,
};
use canisend_core::{CapabilityRegistry, StageRegistry};
use canisend_io::discovery_adapter_capabilities;
pub use canisend_resources::AgentHost;
use canisend_resources::{AgentPackExportData, export_agent_pack as export_embedded_agent_pack};
use canisend_store::{AgentContextService, StoreError, Workspace};
use serde::{Deserialize, Serialize};

use crate::{ActionReceipt, Application, ApplicationError, application::parse_entity_id};

pub type AgentCapabilitiesReadModel = CapabilitiesData;
pub type AgentContextReadModel = AgentContextData;
pub type AgentPackExportReadModel = AgentPackExportData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPackExportRequest {
    pub host: AgentHost,
    pub destination: PathBuf,
}

impl AgentPackExportRequest {
    #[must_use]
    pub fn new(host: AgentHost, destination: impl Into<PathBuf>) -> Self {
        Self {
            host,
            destination: destination.into(),
        }
    }
}

impl Application {
    pub fn agent_capabilities()
    -> Result<ActionReceipt<AgentCapabilitiesReadModel>, ApplicationError> {
        let data = AgentCapabilitiesReadModel {
            product_version: compiled_product_version()?,
            protocol: AGENT_PROTOCOL.to_owned(),
            workspace_format: WORKSPACE_FORMAT.to_owned(),
            resource_format: RESOURCE_FORMAT.to_owned(),
            capabilities: CapabilityRegistry::built_in(),
            stages: StageRegistry::built_in(),
            discovery_adapters: discovery_adapter_capabilities(),
            error_codes: ErrorCode::ALL
                .into_iter()
                .map(|code| code.as_str().to_owned())
                .collect(),
        };
        Ok(ActionReceipt::new(
            "agent.capabilities",
            "available",
            format!(
                "Loaded {} Agent v2 capability families",
                data.capabilities.len()
            ),
            data,
        ))
    }

    pub fn agent_context(
        root: Option<&Path>,
        selected_job_id: Option<&str>,
    ) -> Result<ActionReceipt<AgentContextReadModel>, ApplicationError> {
        let workspace = open_optional_workspace(root)?;
        let mut blockers = Vec::new();
        let mut next_actions = Vec::new();
        let mut workspace_summary = None;
        let mut selected_job = None;

        if let Some(workspace) = &workspace {
            let service = AgentContextService::new(&workspace.database);
            let summary = service.workspace_summary()?;
            if let Some(job_id) = selected_job_id {
                let job_id = parse_entity_id(job_id)?;
                let job = service.job_summary(&job_id)?;
                append_selected_job_guidance(&job, &mut blockers, &mut next_actions);
                selected_job = Some(job);
            } else {
                append_workspace_guidance(&summary, &mut blockers, &mut next_actions);
            }
            workspace_summary = Some(summary);
        } else {
            blockers.push(AgentContextBlocker {
                code: "workspace.not_selected".to_owned(),
                description: "No CanISend workspace was discovered or selected".to_owned(),
                subject_id: None,
            });
            next_actions.push(NextAction {
                action: "canisend --workspace PATH workspace init --json".to_owned(),
                description: "Initialize or explicitly select a workspace".to_owned(),
            });
        }

        let data = AgentContextReadModel {
            product_version: compiled_product_version()?,
            protocol: AGENT_PROTOCOL.to_owned(),
            workspace_format: WORKSPACE_FORMAT.to_owned(),
            resource_format: RESOURCE_FORMAT.to_owned(),
            actor: ActorKind::HostAgent,
            execution_mode: ExecutionMode::HostAgent,
            workspace_id: workspace_summary
                .as_ref()
                .map(|summary| summary.workspace_id.clone()),
            active_job_id: selected_job.as_ref().map(|job| job.id.clone()),
            workspace: workspace_summary,
            selected_job,
            supported_stages: StageRegistry::built_in(),
            blockers,
            next_actions,
            privacy: PrivacyClassification::Public,
        };
        let next_actions = data.next_actions.clone();
        Ok(ActionReceipt::new(
            "agent.context",
            "available",
            format!(
                "Loaded body-free Agent v2 context with {} blocker(s)",
                data.blockers.len()
            ),
            data,
        )
        .with_next_actions(next_actions))
    }

    pub fn export_agent_assets(
        request: &AgentPackExportRequest,
    ) -> Result<ActionReceipt<AgentPackExportReadModel>, ApplicationError> {
        canisend_resources::verify().map_err(ApplicationError::ResourceIntegrity)?;
        let exported = export_embedded_agent_pack(request.host, &request.destination)?;
        Ok(ActionReceipt::new(
            "agent.assets.export",
            "exported",
            format!(
                "Exported {} Agent v2 resources for {}",
                exported.manifest.files.len(),
                request.host.as_str()
            ),
            exported,
        ))
    }
}

fn compiled_product_version() -> Result<SemanticVersion, ApplicationError> {
    SemanticVersion::try_new(env!("CARGO_PKG_VERSION")).map_err(|error| {
        ApplicationError::ResourceIntegrity(format!("compiled product version is invalid: {error}"))
    })
}

fn open_optional_workspace(root: Option<&Path>) -> Result<Option<Workspace>, ApplicationError> {
    match Workspace::open(root) {
        Ok(workspace) => Ok(Some(workspace)),
        Err(StoreError::WorkspaceNotFound(_)) if root.is_none() => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn append_selected_job_guidance(
    job: &canisend_contracts::AgentJobSummary,
    blockers: &mut Vec<AgentContextBlocker>,
    next_actions: &mut Vec<NextAction>,
) {
    if job.archived {
        blockers.push(AgentContextBlocker {
            code: "job.archived".to_owned(),
            description: "The selected job is archived".to_owned(),
            subject_id: Some(job.id.clone()),
        });
    } else if job.source_count == 0 {
        blockers.push(AgentContextBlocker {
            code: "job.source_missing".to_owned(),
            description: "The selected job has no imported advert source".to_owned(),
            subject_id: Some(job.id.clone()),
        });
        next_actions.push(NextAction {
            action: format!("canisend job import {} --file PATH", job.id),
            description: "Import a local advert, PDF, or use --url before preparing work"
                .to_owned(),
        });
    } else {
        next_actions.push(NextAction {
            action: format!("canisend workflow start --job {} --json", job.id),
            description: "Start or resume the durable application stage graph".to_owned(),
        });
    }
}

fn append_workspace_guidance(
    summary: &canisend_contracts::AgentWorkspaceSummary,
    blockers: &mut Vec<AgentContextBlocker>,
    next_actions: &mut Vec<NextAction>,
) {
    if summary.active_job_count > 0 {
        blockers.push(AgentContextBlocker {
            code: "job.not_selected".to_owned(),
            description: "Select an active job with agent context --job JOB_ID".to_owned(),
            subject_id: None,
        });
        next_actions.push(NextAction {
            action: "canisend job list --json".to_owned(),
            description: "Choose one active job for the next workflow operation".to_owned(),
        });
    } else if summary.active_lead_count > 0 {
        blockers.push(AgentContextBlocker {
            code: "job.missing".to_owned(),
            description: "Promote a discovery lead before preparing application work".to_owned(),
            subject_id: None,
        });
        next_actions.push(NextAction {
            action: "canisend discovery list --json".to_owned(),
            description: "Select and promote an active discovery lead".to_owned(),
        });
    } else {
        blockers.push(AgentContextBlocker {
            code: "job.missing".to_owned(),
            description: "Create or discover a job before preparing application work".to_owned(),
            subject_id: None,
        });
        next_actions.push(NextAction {
            action: "canisend job create --title TITLE --institution INSTITUTION --json".to_owned(),
            description: "Create a direct-intake job or import discovery leads".to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{ErrorCode, PrivacyClassification};
    use sha2::{Digest, Sha256};

    use super::{
        AgentCapabilitiesReadModel, AgentContextReadModel, AgentHost, AgentPackExportReadModel,
        AgentPackExportRequest,
    };
    use crate::{ActionReceipt, Application, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-agent-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn agent_facade_is_typed_body_free_and_exports_verified_host_packs() {
        let capabilities = Application::agent_capabilities().expect("capabilities");
        assert_eq!(capabilities.operation, "agent.capabilities");
        assert_eq!(capabilities.data.protocol, "canisend.agent/v2");
        assert!(
            capabilities
                .data
                .capabilities
                .iter()
                .any(|capability| capability.id == "agent.context")
        );
        let capabilities_round_trip: ActionReceipt<AgentCapabilitiesReadModel> =
            serde_json::from_slice(
                &serde_json::to_vec(&capabilities).expect("encode capabilities receipt"),
            )
            .expect("decode capabilities receipt");
        assert_eq!(capabilities_round_trip, capabilities);

        let root = temporary_root("workspace");
        let source = temporary_root("private-source").with_extension("txt");
        let sentinel = "PRIVATE-AGENT-CONTEXT-SENTINEL";
        fs::write(&source, sentinel).expect("write source");
        Application::initialize_workspace(&root).expect("workspace");
        let job = Application::create_job(&root, "Lecturer", "University X")
            .expect("job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("source");

        let unselected = Application::agent_context(Some(&root), None).expect("unselected context");
        assert_eq!(unselected.data.blockers[0].code, "job.not_selected");
        let selected = Application::agent_context(Some(&root), Some(job.id.as_str()))
            .expect("selected context");
        assert_eq!(selected.operation, "agent.context");
        assert_eq!(selected.data.active_job_id.as_ref(), Some(&job.id));
        assert_eq!(selected.data.privacy, PrivacyClassification::Public);
        assert_eq!(selected.next_actions, selected.data.next_actions);
        let encoded = serde_json::to_string(&selected).expect("context JSON");
        assert!(!encoded.contains(sentinel));
        assert!(!encoded.contains("normalized_text"));
        assert!(!encoded.contains("original"));
        let selected_round_trip: ActionReceipt<AgentContextReadModel> =
            serde_json::from_str(&encoded).expect("decode context receipt");
        assert_eq!(selected_round_trip, selected);

        let pack_parent = temporary_root("packs");
        fs::create_dir(&pack_parent).expect("pack parent");
        for host in [AgentHost::Codex, AgentHost::Claude, AgentHost::Generic] {
            let destination = pack_parent.join(host.as_str());
            if host == AgentHost::Claude {
                fs::create_dir(&destination).expect("existing empty destination");
            }
            let request = AgentPackExportRequest::new(host, &destination);
            let exported = Application::export_agent_assets(&request).expect("export pack");
            assert_eq!(exported.operation, "agent.assets.export");
            assert_eq!(exported.data.manifest.host, host);
            assert_eq!(exported.data.manifest.files.len(), 31);
            let exported_round_trip: ActionReceipt<AgentPackExportReadModel> =
                serde_json::from_slice(
                    &serde_json::to_vec(&exported).expect("encode export receipt"),
                )
                .expect("decode export receipt");
            assert_eq!(exported_round_trip, exported);
            for file in &exported.data.manifest.files {
                let bytes = fs::read(destination.join(&file.path)).expect("exported resource");
                assert_eq!(bytes.len(), file.size);
                assert_eq!(hex::encode(Sha256::digest(bytes)), file.sha256);
            }
            let failure = Application::export_agent_assets(&request)
                .expect_err("existing pack must not be overwritten")
                .classify();
            assert_eq!(failure.code, ErrorCode::InputPathRejected);
        }
        let internal_destination = pack_parent.join(".canisend/pack");
        let failure = Application::export_agent_assets(&AgentPackExportRequest::new(
            AgentHost::Generic,
            &internal_destination,
        ))
        .expect_err("internal destination must fail")
        .classify();
        assert_eq!(failure.code, ErrorCode::InputPathRejected);
        assert!(!internal_destination.exists());

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
        fs::remove_dir_all(pack_parent).expect("remove packs");
    }
}
