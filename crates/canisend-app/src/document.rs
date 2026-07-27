use std::path::Path;

use canisend_contracts::{DocumentKind, DocumentRecord, DocumentSetRecord};
use canisend_store::{DocumentService, StoreError};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentWorkspaceReadModel {
    pub documents: Vec<DocumentRecord>,
    pub accepted_set: Option<DocumentSetRecord>,
    pub acceptance_blocker: Option<String>,
}

impl Application {
    pub fn current_documents(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<Vec<DocumentRecord>>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let workspace = open_workspace(root)?;
        let documents =
            DocumentService::new(&workspace.database, &workspace.blobs).list(&job_id)?;
        let count = documents.len();
        Ok(ActionReceipt::new(
            "document.list",
            "available",
            format!("Loaded {count} current structured document(s)"),
            documents,
        ))
    }

    pub fn current_document(
        root: &Path,
        job_id: &str,
        kind: DocumentKind,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<DocumentRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let workspace = open_workspace(root)?;
        let document =
            DocumentService::new(&workspace.database, &workspace.blobs).current(&job_id, kind)?;
        let sections = document.sections.len();
        let placeholders = document.placeholders.len();
        Ok(ActionReceipt::new(
            "document.show",
            "available",
            format!(
                "Loaded current {} document: {sections} section(s), {placeholders} placeholder(s)",
                document_kind_name(kind)
            ),
            document,
        ))
    }

    pub fn current_document_set(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<DocumentSetRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let workspace = open_workspace(root)?;
        let set = DocumentService::new(&workspace.database, &workspace.blobs).set(&job_id)?;
        let count = set.documents.len();
        let artifacts = set.documents.clone();
        Ok(ActionReceipt::new(
            "document.set",
            "complete",
            format!("Loaded accepted document set with {count} current member(s)"),
            set,
        )
        .with_artifacts(artifacts))
    }

    pub fn document_workspace(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<DocumentWorkspaceReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let workspace = open_workspace(root)?;
        let service = DocumentService::new(&workspace.database, &workspace.blobs);
        let documents = service.list(&job_id)?;
        let (accepted_set, acceptance_blocker) = match service.set(&job_id) {
            Ok(set) => (Some(set), None),
            Err(StoreError::WorkflowConflict(message)) => (None, Some(message)),
            Err(error) => return Err(error.into()),
        };
        let status = if accepted_set.is_some() {
            "complete"
        } else if documents.is_empty() {
            "empty"
        } else {
            "in-progress"
        };
        let count = documents.len();
        Ok(ActionReceipt::new(
            "document.workspace",
            status,
            format!("Loaded document workspace with {count} current draft(s)"),
            DocumentWorkspaceReadModel {
                documents,
                accepted_set,
                acceptance_blocker,
            },
        ))
    }
}

fn document_kind_name(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::CoverLetter => "cover-letter",
        DocumentKind::ResearchStatement => "research-statement",
        DocumentKind::TeachingStatement => "teaching-statement",
        DocumentKind::Cv => "cv",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use canisend_contracts::DocumentKind;

    use crate::{Application, ApplicationError, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-document-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn document_ids_are_validated_before_workspace_access() {
        let missing = temporary_root("missing");
        let consent = PrivateReadConsent::granted_by_user();
        for error in [
            Application::current_documents(&missing, "not-a-uuid", consent)
                .expect_err("invalid document list job ID"),
            Application::current_document(
                &missing,
                "not-a-uuid",
                DocumentKind::CoverLetter,
                consent,
            )
            .expect_err("invalid document job ID"),
            Application::current_document_set(&missing, "not-a-uuid")
                .expect_err("invalid document set job ID"),
        ] {
            assert!(matches!(error, ApplicationError::InvalidEntityId(_)));
        }
    }

    #[test]
    fn document_reads_do_not_create_workspace_state() {
        let root = temporary_root("empty");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer", "University")
            .expect("create job")
            .data;
        let before = Application::workspace_status(&root)
            .expect("status before")
            .data
            .status
            .artifact_count;

        assert!(
            Application::current_documents(
                &root,
                job.id.as_str(),
                PrivateReadConsent::granted_by_user(),
            )
            .expect("empty document list")
            .data
            .is_empty()
        );
        let document_workspace = Application::document_workspace(
            &root,
            job.id.as_str(),
            PrivateReadConsent::granted_by_user(),
        )
        .expect("empty document workspace")
        .data;
        assert!(document_workspace.documents.is_empty());
        assert!(document_workspace.accepted_set.is_none());
        assert!(document_workspace.acceptance_blocker.is_some());
        assert!(Application::current_document_set(&root, job.id.as_str()).is_err());

        let after = Application::workspace_status(&root)
            .expect("status after")
            .data
            .status
            .artifact_count;
        assert_eq!(before, after);
        std::fs::remove_dir_all(root).expect("remove workspace");
    }
}
