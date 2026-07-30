#![forbid(unsafe_code)]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
};

use canisend_app::{
    AgentHandoffRequest, AgentHost, Application, ApplicationError, ContentCatalogFilter,
    ContentSearchRequest, PrivateReadConsent, WorkspaceRegistry,
};
use canisend_store::StoreError;

const PRIVATE_SENTINEL: &str = "stage4mprivacytoken";

static NEXT: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
    files: Vec<PathBuf>,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = temporary_path(label);
        Application::initialize_workspace(&root).expect("initialize Stage 4M workspace");
        Self {
            root,
            files: Vec::new(),
        }
    }

    fn write_source(&mut self, label: &str, body: &str) -> PathBuf {
        let path = temporary_path(label).with_extension("md");
        fs::write(&path, body).expect("write Stage 4M source fixture");
        self.files.push(path.clone());
        path
    }

    fn create_sourced_job(&mut self) -> String {
        let source = self.write_source(
            "private-source",
            &format!(
                "# Lecturer in Economics\n\nTeach, research, and retain {PRIVATE_SENTINEL} locally.\n"
            ),
        );
        let job =
            Application::create_job(&self.root, "Lecturer in Economics", "Stage 4M University")
                .expect("create Stage 4M job")
                .data;
        Application::import_local_job_source(
            &self.root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import Stage 4M private source");
        job.id.to_string()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        for path in &self.files {
            let _ = fs::remove_file(path);
        }
    }
}

fn temporary_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "canisend-stage4m-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn catalog_rebuild_is_deterministic_ephemeral_and_concurrent() {
    let mut fixture = Fixture::new("catalog-rebuild");
    fixture.create_sourced_job();

    let expected_catalog =
        Application::content_catalog(&fixture.root, ContentCatalogFilter::default())
            .expect("build initial catalog")
            .data;
    let reopened_catalog =
        Application::content_catalog(&fixture.root, ContentCatalogFilter::default())
            .expect("rebuild catalog after reopening workspace")
            .data;
    assert_eq!(reopened_catalog, expected_catalog);
    assert_eq!(expected_catalog.total_entries, 2);

    let private_search = Application::search_content(
        &fixture.root,
        ContentSearchRequest {
            query: PRIVATE_SENTINEL.to_owned(),
            filter: ContentCatalogFilter::default(),
            include_private_bodies: true,
            limit: 10,
        },
        Some(PrivateReadConsent::granted_by_user()),
    )
    .expect("run explicitly consented private search")
    .data;
    assert_eq!(private_search.total_matches, 1);
    assert_eq!(private_search.index.private_body_entries, 1);
    assert!(private_search.results.iter().any(|result| {
        result
            .snippet
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(PRIVATE_SENTINEL))
    }));

    let metadata_only = Application::search_content(
        &fixture.root,
        ContentSearchRequest::metadata(PRIVATE_SENTINEL),
        None,
    )
    .expect("rebuild a metadata-only index")
    .data;
    assert_eq!(metadata_only.total_matches, 0);
    assert_eq!(metadata_only.index.private_body_entries, 0);
    assert_eq!(metadata_only.index.private_body_bytes, 0);
    assert!(
        metadata_only
            .results
            .iter()
            .all(|result| result.snippet.is_none())
    );

    thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                let root = fixture.root.clone();
                scope.spawn(move || {
                    let catalog =
                        Application::content_catalog(&root, ContentCatalogFilter::default())
                            .expect("concurrent catalog rebuild")
                            .data;
                    let search = Application::search_content(
                        &root,
                        ContentSearchRequest::metadata("Lecturer"),
                        None,
                    )
                    .expect("concurrent metadata search")
                    .data;
                    (catalog, search)
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            let (catalog, search) = handle.join().expect("concurrent reader thread");
            assert_eq!(catalog, expected_catalog);
            assert_eq!(search.index.private_body_entries, 0);
            assert!(search.total_matches > 0);
        }
    });

    let malformed = Application::search_content(
        &fixture.root,
        ContentSearchRequest::metadata("invalid\0query"),
        None,
    )
    .expect_err("control characters must be rejected");
    assert!(matches!(malformed, ApplicationError::InvalidInput(_)));

    let unbounded = Application::search_content(
        &fixture.root,
        ContentSearchRequest {
            query: "Lecturer".to_owned(),
            filter: ContentCatalogFilter::default(),
            include_private_bodies: false,
            limit: 101,
        },
        None,
    )
    .expect_err("unbounded result limits must be rejected");
    assert!(matches!(unbounded, ApplicationError::InvalidInput(_)));
}

#[test]
fn stale_intake_preview_fails_and_assistance_rebuilds_from_current_revisions() {
    let mut fixture = Fixture::new("stale-assistance");
    let first_source = fixture.write_source("first-candidate", "# First reviewed advert\n");
    let second_source = fixture.write_source("second-candidate", "# Newer reviewed advert\n");
    let job = Application::create_job(&fixture.root, "Research Fellow", "Stage 4M University")
        .expect("create job")
        .data;

    let initial = Application::agent_assistance(&fixture.root, job.id.as_str())
        .expect("initial assistance")
        .data;
    assert_eq!(initial.dossier.job.revision.get(), 1);
    assert_eq!(initial.content.total_entries, 0);

    let prepared = Application::prepare_local_job_source(
        &fixture.root,
        job.id.as_str(),
        &first_source,
        PrivateReadConsent::granted_by_user(),
    )
    .expect("prepare first intake candidate");
    Application::import_local_job_source(
        &fixture.root,
        job.id.as_str(),
        &second_source,
        PrivateReadConsent::granted_by_user(),
    )
    .expect("commit competing source edit");

    let stale = Application::commit_prepared_job_source(prepared)
        .expect_err("stale intake preview must not overwrite the newer revision");
    assert!(matches!(
        stale,
        ApplicationError::Store(StoreError::DependencyConflict(_))
    ));

    let refreshed = Application::agent_assistance(&fixture.root, job.id.as_str())
        .expect("rebuild assistance after authoritative mutation")
        .data;
    assert_eq!(refreshed.dossier.job.revision.get(), 2);
    assert_eq!(refreshed.content.total_entries, 2);
    assert_ne!(
        refreshed.recommendation.next_action,
        initial.recommendation.next_action
    );

    let handoff = Application::prepare_agent_handoff(&AgentHandoffRequest {
        host: AgentHost::Codex,
        workspace: fixture.root.clone(),
        selected_job_id: Some(job.id.to_string()),
    })
    .expect("prepare current handoff")
    .data;
    assert_eq!(
        handoff
            .assistance
            .as_ref()
            .expect("job-scoped handoff assistance")
            .dossier
            .job
            .revision
            .get(),
        2
    );
}

#[test]
fn routine_coordination_and_diagnostics_never_serialize_private_bodies() {
    let mut fixture = Fixture::new("privacy-proof");
    let job_id = fixture.create_sourced_job();
    let mut registry = WorkspaceRegistry::default();
    registry
        .register("Stage 4M workspace", &fixture.root)
        .expect("register workspace shortcut");

    let serialized = serde_json::to_string(&serde_json::json!({
        "product": Application::product_summary(),
        "workspace_status": Application::workspace_status(&fixture.root)
            .expect("workspace status"),
        "workspace_check": Application::check_workspace(&fixture.root)
            .expect("workspace check"),
        "registry": registry,
        "jobs": Application::list_jobs(&fixture.root, false).expect("job list"),
        "job": Application::job_detail(&fixture.root, &job_id).expect("job detail"),
        "dossier": Application::application_dossier(&fixture.root, &job_id)
            .expect("dossier"),
        "dossiers": Application::list_application_dossiers(&fixture.root, false)
            .expect("dossier list"),
        "catalog": Application::content_catalog(
            &fixture.root,
            ContentCatalogFilter::default(),
        )
        .expect("catalog"),
        "routine_search": Application::search_content(
            &fixture.root,
            ContentSearchRequest::metadata("Lecturer"),
            None,
        )
        .expect("metadata search"),
        "agent_context": Application::agent_context(Some(&fixture.root), Some(&job_id))
            .expect("agent context"),
        "agent_assistance": Application::agent_assistance(&fixture.root, &job_id)
            .expect("agent assistance"),
        "handoff": Application::prepare_agent_handoff(&AgentHandoffRequest {
            host: AgentHost::Codex,
            workspace: fixture.root.clone(),
            selected_job_id: Some(job_id),
        })
        .expect("agent handoff"),
    }))
    .expect("serialize routine read models");

    assert!(
        !serialized.to_lowercase().contains(PRIVATE_SENTINEL),
        "private source bodies must remain behind explicit read consent"
    );
}
