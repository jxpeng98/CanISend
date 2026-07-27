use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_app::{
    AgentHost, AgentPackExportRequest, DiscoveryImportRequest, TaskExecutionMode, TaskOperation,
    TaskPrepareRequest,
};
use canisend_contracts::{DiscoveryLeadStatus, PrivacyClassification, TaskStatus};

use crate::worker::{WorkerEvent, WorkerRequest, execute};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temporary_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "canisend-gui-stage4d-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn discovery_task_and_agent_state_persist_across_workspace_reopen() {
    let root = temporary_root("workspace");
    let batch = temporary_root("discovery").with_extension("json");
    let advert = temporary_root("advert").with_extension("txt");
    let agent_pack = temporary_root("agent-pack");
    let private_sentinel = "STAGE4D-PRIVATE-ADVERT-BODY";

    std::fs::write(
        &batch,
        serde_json::to_vec_pretty(&serde_json::json!({
            "source_kind": "host-agent",
            "source_name": "Stage 4D reviewed leads",
            "source_url": null,
            "cursor": "stage4d-1",
            "observed_at": "2026-07-27T00:00:00Z",
            "leads": [{
                "external_id": "stage4d-lecturer",
                "title": "Lecturer in Economics",
                "organization": "University X",
                "location": "London",
                "deadline": "2026-09-30",
                "url": "https://example.edu/jobs/stage4d-lecturer",
                "summary": "Synthetic local qualification fixture",
                "metadata": {}
            }]
        }))
        .expect("serialize discovery fixture"),
    )
    .expect("write discovery fixture");
    std::fs::write(
        &advert,
        format!(
            "# Lecturer in Economics\n\n{private_sentinel}\nTeach economics and publish research.\n"
        ),
    )
    .expect("write private advert fixture");

    assert!(matches!(
        execute(WorkerRequest::CreateWorkspace {
            alias: "Stage 4D fixture".to_owned(),
            path: root.clone(),
        }),
        WorkerEvent::WorkspaceCreated { result: Ok(_), .. }
    ));
    let report = match execute(WorkerRequest::PreviewDiscoveryImport {
        request: DiscoveryImportRequest {
            path: batch.clone(),
            source_name: None,
            source_url: None,
            host_agent: true,
        },
    }) {
        WorkerEvent::DiscoveryImportPreviewed(Ok(receipt)) => {
            assert_eq!(receipt.data.accepted, 1);
            receipt.data
        }
        event => panic!("unexpected discovery preview event: {event:?}"),
    };
    let lead_id = match execute(WorkerRequest::CommitDiscoveryImport {
        path: root.clone(),
        report,
        include_history: true,
    }) {
        WorkerEvent::DiscoveryImportCommitted(Ok(committed)) => {
            committed
                .discovery
                .expect("reload committed discovery")
                .leads
                .leads
                .into_iter()
                .next()
                .expect("committed lead")
                .id
        }
        event => panic!("unexpected discovery commit event: {event:?}"),
    };
    let job = match execute(WorkerRequest::PromoteDiscoveryLead {
        path: root.clone(),
        lead_id: lead_id.to_string(),
        include_history: true,
        include_archived_jobs: false,
    }) {
        WorkerEvent::DiscoveryLeadPromoted(Ok(promoted)) => promoted.receipt.data.job,
        event => panic!("unexpected discovery promotion event: {event:?}"),
    };
    assert!(matches!(
        execute(WorkerRequest::ImportLocalSource {
            path: root.clone(),
            id: job.id.to_string(),
            source: advert.clone(),
        }),
        WorkerEvent::SourceImported(Ok(_))
    ));
    assert!(matches!(
        execute(WorkerRequest::StartWorkflow {
            path: root.clone(),
            id: job.id.to_string(),
        }),
        WorkerEvent::WorkflowLoaded(Ok(_))
    ));
    let prepared = match execute(WorkerRequest::PrepareTask {
        path: root.clone(),
        request: TaskPrepareRequest {
            job_id: job.id.clone(),
            operation: TaskOperation::JobParse,
            mode: TaskExecutionMode::HostAgent,
        },
    }) {
        WorkerEvent::TaskPrepared(Ok(receipt)) => receipt.data,
        event => panic!("unexpected task prepare event: {event:?}"),
    };

    let initial_context = load_agent_context(&root, job.id.as_str());
    assert_eq!(initial_context.privacy, PrivacyClassification::Public);
    assert_eq!(
        initial_context
            .workspace
            .as_ref()
            .map(|workspace| workspace.open_task_count),
        Some(1)
    );
    assert_eq!(
        initial_context
            .selected_job
            .as_ref()
            .map(|selected| selected.source_count),
        Some(1)
    );
    assert!(initial_context.blockers.is_empty());
    assert_body_free(&initial_context, private_sentinel);

    assert!(matches!(
        execute(WorkerRequest::LoadWorkspace { path: root.clone() }),
        WorkerEvent::WorkspaceLoaded(Ok(_))
    ));
    match execute(WorkerRequest::LoadDiscoveryWorkspace {
        path: root.clone(),
        include_history: true,
    }) {
        WorkerEvent::DiscoveryWorkspaceLoaded(Ok(discovery)) => {
            assert_eq!(discovery.sources.sources.len(), 1);
            assert_eq!(discovery.leads.leads.len(), 1);
            assert_eq!(discovery.leads.leads[0].id, lead_id);
            assert_eq!(
                discovery.leads.leads[0].status,
                DiscoveryLeadStatus::Promoted
            );
            assert_eq!(
                discovery.leads.leads[0].promoted_job_id.as_ref(),
                Some(&job.id)
            );
        }
        event => panic!("unexpected reopened discovery event: {event:?}"),
    }
    match execute(WorkerRequest::LoadJobs {
        path: root.clone(),
        include_archived: false,
    }) {
        WorkerEvent::JobsLoaded(Ok(receipt)) => {
            assert_eq!(receipt.data.jobs.len(), 1);
            assert_eq!(receipt.data.jobs[0].id, job.id);
        }
        event => panic!("unexpected reopened jobs event: {event:?}"),
    }
    match execute(WorkerRequest::LoadLatestTask {
        path: root.clone(),
        job_id: job.id.to_string(),
    }) {
        WorkerEvent::LatestTaskLoaded {
            result: Ok(receipt),
            ..
        } => {
            let reopened = receipt.data.expect("reopened prepared task");
            assert_eq!(reopened.descriptor.id, prepared.id);
            assert_eq!(reopened.status, TaskStatus::Prepared);
        }
        event => panic!("unexpected reopened task event: {event:?}"),
    }

    let reopened_context = load_agent_context(&root, job.id.as_str());
    assert_eq!(reopened_context, initial_context);
    assert_body_free(&reopened_context, private_sentinel);
    match execute(WorkerRequest::ExportAgentPack {
        request: AgentPackExportRequest::new(AgentHost::Generic, &agent_pack),
    }) {
        WorkerEvent::AgentPackExported {
            result: Ok(receipt),
            ..
        } => {
            assert_eq!(receipt.data.manifest.host, AgentHost::Generic);
            assert_eq!(receipt.data.manifest.files.len(), 31);
            assert!(receipt.data.manifest_path.is_file());
        }
        event => panic!("unexpected Agent pack export event: {event:?}"),
    }

    std::fs::remove_dir_all(root).expect("remove Stage 4D workspace");
    std::fs::remove_dir_all(agent_pack).expect("remove Stage 4D Agent pack");
    std::fs::remove_file(batch).expect("remove Stage 4D discovery fixture");
    std::fs::remove_file(advert).expect("remove Stage 4D advert fixture");
}

fn load_agent_context(root: &std::path::Path, job_id: &str) -> canisend_app::AgentContextReadModel {
    match execute(WorkerRequest::LoadAgentContext {
        root: Some(root.to_path_buf()),
        selected_job_id: Some(job_id.to_owned()),
    }) {
        WorkerEvent::AgentContextLoaded {
            selected_job_id,
            result: Ok(receipt),
        } => {
            assert_eq!(selected_job_id.as_deref(), Some(job_id));
            receipt.data
        }
        event => panic!("unexpected Agent context event: {event:?}"),
    }
}

fn assert_body_free(context: &canisend_app::AgentContextReadModel, private_sentinel: &str) {
    let serialized = serde_json::to_string(context).expect("serialize public Agent context");
    assert!(!serialized.contains(private_sentinel));
    assert!(!serialized.contains("normalized_text"));
    assert!(!serialized.contains("\"body\""));
}
