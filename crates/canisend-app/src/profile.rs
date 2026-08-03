use std::path::{Path, PathBuf};

use canisend_contracts::{
    ActorKind, EvidenceCatalogRecord, PrivacyClassification, ProfileSourceKind, ProfileSourceRecord,
};
use canisend_io::{LocalTextKind, read_local_text};
use canisend_store::{EvidenceService, NewProfileSource, ProfileService};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
    compatibility::{
        LegacyCompatibilityAccess, LegacyCompatibilityOperation, workspace_compatibility_notice,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSourceListReadModel {
    pub workspace: PathBuf,
    pub profile_revision: u64,
    pub sources: Vec<ProfileSourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSourceImportReadModel {
    pub profile_revision: u64,
    pub source: ProfileSourceRecord,
}

pub type ProfileInitializationReadModel = ProfileSourceImportReadModel;

impl Application {
    pub fn initialize_profile(
        root: &Path,
        markdown: &str,
        sensitivity: PrivacyClassification,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ProfileInitializationReadModel>, ApplicationError> {
        if !matches!(
            sensitivity,
            PrivacyClassification::Public | PrivacyClassification::PrivateLocal
        ) {
            return Err(ApplicationError::InvalidInput(
                "profile source sensitivity must be public or private-local".to_owned(),
            ));
        }
        let normalized = normalize_profile_markdown(markdown)?;
        let mut workspace = open_workspace(root)?;
        let mut service = ProfileService::new(&mut workspace.database, &workspace.blobs);
        if !service.list_sources()?.is_empty() {
            return Err(ApplicationError::InvalidInput(
                "profile initialization is only available before the first source is added"
                    .to_owned(),
            ));
        }
        let source = service.import_source(
            NewProfileSource {
                kind: ProfileSourceKind::Markdown,
                original_bytes: markdown.as_bytes().to_vec(),
                normalized_text: normalized,
                content_type: "text/markdown; charset=utf-8".to_owned(),
                sensitivity,
            },
            ActorKind::User,
        )?;
        let profile_revision = service.revision()?;
        let artifacts = [source.original.clone(), source.normalized_text.clone()];
        Ok(ActionReceipt::new(
            "profile.initialize",
            "initialized",
            "Initialized the local profile with a reviewed Markdown source",
            ProfileInitializationReadModel {
                profile_revision,
                source,
            },
        )
        .with_artifacts(artifacts))
    }

    pub fn import_profile_source(
        root: &Path,
        path: &Path,
        sensitivity: PrivacyClassification,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ProfileSourceImportReadModel>, ApplicationError> {
        if !matches!(
            sensitivity,
            PrivacyClassification::Public | PrivacyClassification::PrivateLocal
        ) {
            return Err(ApplicationError::InvalidInput(
                "profile source sensitivity must be public or private-local".to_owned(),
            ));
        }
        let document = read_local_text(path)?;
        let kind = match document.kind {
            LocalTextKind::Markdown => ProfileSourceKind::Markdown,
            LocalTextKind::PlainText => ProfileSourceKind::PlainText,
            LocalTextKind::Json => ProfileSourceKind::Json,
        };
        let mut workspace = open_workspace(root)?;
        let mut service = ProfileService::new(&mut workspace.database, &workspace.blobs);
        let source = service.import_source(
            NewProfileSource {
                kind,
                original_bytes: document.original_bytes,
                normalized_text: document.normalized_text,
                content_type: document.content_type.to_owned(),
                sensitivity,
            },
            ActorKind::User,
        )?;
        let profile_revision = service.revision()?;
        let artifacts = [source.original.clone(), source.normalized_text.clone()];
        Ok(ActionReceipt::new(
            "profile.source.add",
            "imported",
            format!("Imported {:?} profile source", source.kind),
            ProfileSourceImportReadModel {
                profile_revision,
                source,
            },
        )
        .with_artifacts(artifacts))
    }

    pub fn list_profile_sources(
        root: &Path,
    ) -> Result<ActionReceipt<ProfileSourceListReadModel>, ApplicationError> {
        let compatibility = workspace_compatibility_notice(
            root,
            LegacyCompatibilityOperation::ProfileSources,
            LegacyCompatibilityAccess::Read,
        )?;
        let mut workspace = open_workspace(root)?;
        let service = ProfileService::new(&mut workspace.database, &workspace.blobs);
        let sources = service.list_sources()?;
        let profile_revision = service.revision()?;
        Ok(ActionReceipt::new(
            "profile.source.list",
            "available",
            format!("Loaded {} profile source(s)", sources.len()),
            ProfileSourceListReadModel {
                workspace: workspace.paths.root,
                profile_revision,
                sources,
            },
        )
        .with_compatibility(compatibility))
    }

    pub fn profile_source(
        root: &Path,
        source_id: &str,
    ) -> Result<ActionReceipt<ProfileSourceRecord>, ApplicationError> {
        let source_id = parse_entity_id(source_id)?;
        let mut workspace = open_workspace(root)?;
        let source = ProfileService::new(&mut workspace.database, &workspace.blobs)
            .get_source(&source_id)?;
        Ok(ActionReceipt::new(
            "profile.source.show",
            "available",
            format!("Loaded {:?} profile source", source.kind),
            source,
        ))
    }

    pub fn proposed_profile_evidence(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<EvidenceCatalogRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let catalog =
            EvidenceService::new(&mut workspace.database, &workspace.blobs).proposed(&job_id)?;
        Ok(evidence_receipt(
            "profile.evidence.proposed",
            "available",
            "Loaded evidence proposal",
            catalog,
        ))
    }

    pub fn profile_evidence_template(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<EvidenceCatalogRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let catalog =
            EvidenceService::new(&mut workspace.database, &workspace.blobs).template(&job_id)?;
        Ok(evidence_receipt(
            "profile.evidence.export",
            "available",
            "Prepared evidence candidate",
            catalog,
        ))
    }

    pub fn confirmed_profile_evidence(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<EvidenceCatalogRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let catalog =
            EvidenceService::new(&mut workspace.database, &workspace.blobs).confirmed(&job_id)?;
        Ok(evidence_receipt(
            "profile.evidence.show",
            "available",
            "Loaded confirmed evidence",
            catalog,
        ))
    }

    pub fn confirm_profile_evidence(
        root: &Path,
        job_id: &str,
        candidate: &Value,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<EvidenceCatalogRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let artifact = EvidenceService::new(&mut workspace.database, &workspace.blobs)
            .confirm(&job_id, candidate)?;
        let catalog =
            EvidenceService::new(&mut workspace.database, &workspace.blobs).confirmed(&job_id)?;
        Ok(evidence_receipt(
            "profile.evidence.confirm",
            "confirmed",
            "Confirmed profile evidence",
            catalog,
        )
        .with_artifacts([artifact]))
    }
}

fn normalize_profile_markdown(markdown: &str) -> Result<String, ApplicationError> {
    const MAX_PROFILE_MARKDOWN_BYTES: usize = 256 * 1024;

    if markdown.is_empty() || markdown.len() > MAX_PROFILE_MARKDOWN_BYTES {
        return Err(ApplicationError::InvalidInput(format!(
            "profile Markdown must contain 1 to {MAX_PROFILE_MARKDOWN_BYTES} bytes"
        )));
    }
    if markdown
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(ApplicationError::InvalidInput(
            "profile Markdown contains unsupported control characters".to_owned(),
        ));
    }
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    if normalized
        .lines()
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
    {
        return Err(ApplicationError::InvalidInput(
            "add at least one reviewed profile detail below the section headings".to_owned(),
        ));
    }
    Ok(format!("{}\n", normalized.trim_end()))
}

fn evidence_receipt(
    operation: &'static str,
    status: &'static str,
    summary: &'static str,
    catalog: EvidenceCatalogRecord,
) -> ActionReceipt<EvidenceCatalogRecord> {
    let count = catalog.items.len();
    ActionReceipt::new(
        operation,
        status,
        format!("{summary}: {count} item(s)"),
        catalog,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{ErrorCode, PrivacyClassification};
    use serde_json::json;

    use super::Application;
    use crate::{ApplicationError, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-profile-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn profile_source_receipts_are_body_free_and_revision_bound() {
        let root = temporary_root("source");
        let source_path = temporary_root("private").with_extension("md");
        let sentinel = "PRIVATE-PROFILE-SENTINEL-DO-NOT-LEAK";
        fs::write(&source_path, format!("# Profile\n\n{sentinel}\n")).expect("write source");
        Application::initialize_workspace(&root).expect("initialize workspace");

        let imported = Application::import_profile_source(
            &root,
            &source_path,
            PrivacyClassification::PrivateLocal,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import profile source");
        assert_eq!(imported.operation, "profile.source.add");
        assert_eq!(imported.data.profile_revision, 1);
        assert_eq!(imported.artifacts.len(), 2);

        let listed = Application::list_profile_sources(&root).expect("list profile sources");
        assert_eq!(listed.data.profile_revision, 1);
        assert_eq!(listed.data.sources.len(), 1);
        let shown = Application::profile_source(&root, imported.data.source.id.as_str())
            .expect("show profile source");

        for value in [
            serde_json::to_string(&imported).expect("serialize import receipt"),
            serde_json::to_string(&listed).expect("serialize list receipt"),
            serde_json::to_string(&shown).expect("serialize show receipt"),
        ] {
            assert!(!value.contains(sentinel));
        }

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source_path).expect("remove source");
    }

    #[test]
    fn profile_initialization_creates_one_private_markdown_source() {
        let root = temporary_root("initialize");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let initialized = Application::initialize_profile(
            &root,
            "# Academic profile\n\nResearch economist.\n",
            PrivacyClassification::PrivateLocal,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("initialize profile");
        assert_eq!(initialized.operation, "profile.initialize");
        assert_eq!(initialized.data.profile_revision, 1);
        assert_eq!(
            initialized.data.source.kind,
            canisend_contracts::ProfileSourceKind::Markdown
        );
        assert!(
            Application::initialize_profile(
                &root,
                "# Second\n\nDuplicate initialization.\n",
                PrivacyClassification::PrivateLocal,
                PrivateReadConsent::granted_by_user(),
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("remove workspace");
    }

    #[test]
    fn profile_ids_are_validated_before_workspace_access() {
        let missing = temporary_root("missing");
        for error in [
            Application::profile_source(&missing, "not-a-uuid")
                .expect_err("invalid source ID must fail"),
            Application::proposed_profile_evidence(
                &missing,
                "not-a-uuid",
                PrivateReadConsent::granted_by_user(),
            )
            .expect_err("invalid job ID must fail"),
        ] {
            assert!(matches!(error, ApplicationError::InvalidEntityId(_)));
        }
    }

    #[test]
    fn malformed_evidence_candidate_fails_without_profile_mutation() {
        let root = temporary_root("candidate");
        let source_path = temporary_root("candidate-source").with_extension("txt");
        fs::write(&source_path, "Confirmed teaching experience.").expect("write source");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer", "University X")
            .expect("create job")
            .data;
        Application::import_profile_source(
            &root,
            &source_path,
            PrivacyClassification::PrivateLocal,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import profile source");

        let error = Application::confirm_profile_evidence(
            &root,
            job.id.as_str(),
            &json!({"unexpected": true}),
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("malformed candidate must fail");
        assert_eq!(error.classify().code, ErrorCode::CandidateSchemaInvalid);
        let listed = Application::list_profile_sources(&root).expect("list sources");
        assert_eq!(listed.data.profile_revision, 1);
        assert_eq!(listed.data.sources.len(), 1);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source_path).expect("remove source");
    }
}
