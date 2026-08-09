use std::path::Path;

use canisend_contracts::{
    ApplicationPackBindingV3, CompatibilityAuthority, CompatibilityNotice, CompatibilitySurface,
    EntityId, NextAction,
};
use canisend_store::{
    LegacyCompatibilityAuthority, LegacyCompatibilityContextV3, LegacyCompatibilityService,
    StoreError,
};
use serde_json::json;

use crate::{ApplicationError, application::open_workspace, built_in_academic_job_pack};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyCompatibilityAccess {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyCompatibilityOperation {
    AgentCapabilities,
    AgentContext,
    JobList,
    JobShow,
    JobCreate,
    JobArchive,
    JobImport,
    JobIntakePreview,
    JobIntakeCommit,
    ProfileSources,
    TaskShow,
    TaskLatest,
    TaskPrepare,
    TaskInputs,
    TaskCompletionPreview,
    TaskCompletionCommit,
    TaskCancel,
    TaskPrepareAgain,
    WorkflowStatus,
}

impl LegacyCompatibilityOperation {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 19] = [
        Self::AgentCapabilities,
        Self::AgentContext,
        Self::JobList,
        Self::JobShow,
        Self::JobCreate,
        Self::JobArchive,
        Self::JobImport,
        Self::JobIntakePreview,
        Self::JobIntakeCommit,
        Self::ProfileSources,
        Self::TaskShow,
        Self::TaskLatest,
        Self::TaskPrepare,
        Self::TaskInputs,
        Self::TaskCompletionPreview,
        Self::TaskCompletionCommit,
        Self::TaskCancel,
        Self::TaskPrepareAgain,
        Self::WorkflowStatus,
    ];

    #[cfg(test)]
    pub(crate) const REGISTERED_ALIASES: [Self; 6] = [
        Self::AgentCapabilities,
        Self::AgentContext,
        Self::JobList,
        Self::JobShow,
        Self::TaskLatest,
        Self::WorkflowStatus,
    ];

    #[cfg(test)]
    pub(crate) const RETIRED_REGISTERED_ALIASES: [Self; 11] = [
        Self::JobCreate,
        Self::JobArchive,
        Self::JobIntakePreview,
        Self::JobIntakeCommit,
        Self::ProfileSources,
        Self::TaskPrepare,
        Self::TaskInputs,
        Self::TaskCompletionPreview,
        Self::TaskCompletionCommit,
        Self::TaskCancel,
        Self::TaskPrepareAgain,
    ];

    pub(crate) const fn legacy(self) -> &'static str {
        match self {
            Self::AgentCapabilities => "agent.capabilities",
            Self::AgentContext => "agent.context",
            Self::JobList => "job.list",
            Self::JobShow => "job.show",
            Self::JobCreate => "job.create",
            Self::JobArchive => "job.archive",
            Self::JobImport => "job.import",
            Self::JobIntakePreview => "job.intake.preview",
            Self::JobIntakeCommit => "job.intake.commit",
            Self::ProfileSources => "profile.source.list",
            Self::TaskShow => "task.show",
            Self::TaskLatest => "task.latest",
            Self::TaskPrepare => "task.prepare",
            Self::TaskInputs => "task.inputs",
            Self::TaskCompletionPreview => "task.complete.preview",
            Self::TaskCompletionCommit => "task.complete",
            Self::TaskCancel => "task.cancel",
            Self::TaskPrepareAgain => "task.prepare-again",
            Self::WorkflowStatus => "workflow.status",
        }
    }

    pub(crate) const fn canonical(self) -> &'static str {
        match self {
            Self::AgentCapabilities => "agent-v3.capabilities",
            Self::AgentContext => "agent-v3.context",
            Self::JobList => "application.list",
            Self::JobShow => "application.show",
            Self::JobCreate => "application.create",
            Self::JobArchive => "application.archive",
            Self::JobImport | Self::JobIntakeCommit => "opportunity.intake.commit",
            Self::JobIntakePreview => "opportunity.intake.preview",
            Self::ProfileSources => "profile-source.list",
            Self::TaskShow => "agent-v3.task.show",
            Self::TaskLatest => "agent-v3.task.latest",
            Self::TaskPrepare => "agent-v3.task.prepare",
            Self::TaskInputs => "agent-v3.task.inputs.export",
            Self::TaskCompletionPreview => "agent-v3.task.completion.preview",
            Self::TaskCompletionCommit => "agent-v3.task.completion.commit",
            Self::TaskCancel => "agent-v3.task.cancel",
            Self::TaskPrepareAgain => "agent-v3.task.prepare-again",
            Self::WorkflowStatus => "application.workflow.status",
        }
    }
}

pub(crate) fn static_compatibility_notice(
    operation: LegacyCompatibilityOperation,
) -> Result<CompatibilityNotice, ApplicationError> {
    Ok(notice(
        operation,
        CompatibilityAuthority::StaticAcademic,
        academic_pack_binding()?,
    ))
}

pub(crate) fn workspace_compatibility_notice(
    root: &Path,
    operation: LegacyCompatibilityOperation,
    access: LegacyCompatibilityAccess,
) -> Result<CompatibilityNotice, ApplicationError> {
    let workspace = open_workspace(root)?;
    let context = LegacyCompatibilityService::new(&workspace.database).workspace_context()?;
    validate_context(operation, access, context)
}

pub(crate) fn job_compatibility_notice(
    root: &Path,
    operation: LegacyCompatibilityOperation,
    access: LegacyCompatibilityAccess,
    job_id: &EntityId,
) -> Result<CompatibilityNotice, ApplicationError> {
    let workspace = open_workspace(root)?;
    let context = match LegacyCompatibilityService::new(&workspace.database).job_context(job_id) {
        Ok(context) => context,
        Err(StoreError::ApplicationModelConflict(_)) => {
            return Err(unavailable(
                operation,
                "legacy-application-unmapped",
                "The legacy Job has no deterministic Workspace v3 Application mapping",
                Vec::new(),
            ));
        }
        Err(error) => return Err(error.into()),
    };
    validate_context(operation, access, context)
}

pub(crate) fn task_compatibility_notice(
    root: &Path,
    operation: LegacyCompatibilityOperation,
    access: LegacyCompatibilityAccess,
    task_id: &EntityId,
) -> Result<CompatibilityNotice, ApplicationError> {
    let workspace = open_workspace(root)?;
    let context = match LegacyCompatibilityService::new(&workspace.database).task_context(task_id) {
        Ok(context) => context,
        Err(StoreError::ApplicationModelConflict(_)) => {
            return Err(unavailable(
                operation,
                "legacy-application-unmapped",
                "The legacy task has no deterministic Workspace v3 Application mapping",
                Vec::new(),
            ));
        }
        Err(error) => return Err(error.into()),
    };
    validate_context(operation, access, context)
}

fn validate_context(
    operation: LegacyCompatibilityOperation,
    access: LegacyCompatibilityAccess,
    context: LegacyCompatibilityContextV3,
) -> Result<CompatibilityNotice, ApplicationError> {
    let expected = academic_pack_binding()?;
    match context.authority {
        LegacyCompatibilityAuthority::WorkspaceV2 => Ok(notice(
            operation,
            CompatibilityAuthority::WorkspaceV2ImplicitAcademic,
            expected,
        )),
        LegacyCompatibilityAuthority::WorkspaceV3 => {
            if context.bindings.is_empty() {
                return Err(unavailable(
                    operation,
                    "workspace-v3-pack-unbound",
                    "Workspace v3 has no exact legacy Application-to-Pack binding",
                    Vec::new(),
                ));
            }
            let detected = context
                .bindings
                .iter()
                .map(|binding| binding.pack.clone())
                .collect::<Vec<_>>();
            if detected.iter().any(|binding| binding != &expected) {
                return Err(unavailable(
                    operation,
                    "unsupported-pack",
                    "Agent v2 and job compatibility is available only for the exact built-in academic Pack",
                    detected,
                ));
            }
            if context
                .bindings
                .iter()
                .any(|binding| binding.legacy_job_id.is_none())
            {
                return Err(unavailable(
                    operation,
                    "legacy-application-unmapped",
                    "Workspace v3 contains an Application that has no deterministic legacy Job mapping",
                    detected,
                ));
            }
            if access == LegacyCompatibilityAccess::Write {
                return Err(unavailable(
                    operation,
                    "workspace-v3-read-only",
                    "Legacy writes are disabled after Workspace v3 authority is activated",
                    detected,
                ));
            }
            Ok(notice(
                operation,
                CompatibilityAuthority::WorkspaceV3AcademicReadOnly,
                expected,
            ))
        }
        LegacyCompatibilityAuthority::WorkspaceV4 => Err(unavailable(
            operation,
            "workspace-v4-legacy-surface-retired",
            "Legacy Agent, Job, Task, and Workflow compatibility is not supported in Workspace v4",
            Vec::new(),
        )),
    }
}

fn notice(
    operation: LegacyCompatibilityOperation,
    authority: CompatibilityAuthority,
    pack: ApplicationPackBindingV3,
) -> CompatibilityNotice {
    CompatibilityNotice {
        surface: CompatibilitySurface::AgentV2,
        deprecated: true,
        legacy_operation: operation.legacy().to_owned(),
        canonical_v3_operation: operation.canonical().to_owned(),
        authority,
        pack,
    }
}

fn academic_pack_binding() -> Result<ApplicationPackBindingV3, ApplicationError> {
    let pack = built_in_academic_job_pack()?;
    Ok(ApplicationPackBindingV3 {
        id: pack.snapshot().id().clone(),
        version: pack.snapshot().version().clone(),
        content_digest: pack.snapshot().content_digest().clone(),
    })
}

fn unavailable(
    operation: LegacyCompatibilityOperation,
    reason: &'static str,
    message: &'static str,
    detected_packs: Vec<ApplicationPackBindingV3>,
) -> ApplicationError {
    ApplicationError::CompatibilityUnavailable {
        message: message.to_owned(),
        details: json!({
            "surface": "agent-v2",
            "deprecated": true,
            "legacy_operation": operation.legacy(),
            "canonical_v3_operation": operation.canonical(),
            "reason": reason,
            "detected_packs": detected_packs,
            "workspace_mutated": false
        }),
        remediation: NextAction {
            action: operation.canonical().to_owned(),
            description: "Use the canonical v3 operation with an explicit Application and verified Pack binding"
                .to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ApplicationId, ApplicationPackBindingV3, CompatibilityAuthority, EntityId,
        OperationRegistry, SemanticVersion, Sha256Digest, WorkflowPackId,
    };
    use canisend_store::{
        LegacyApplicationBindingV3, LegacyCompatibilityAuthority, LegacyCompatibilityContextV3,
    };

    use crate::{Application, WorkspaceV3MigrationRequest};

    use super::{
        LegacyCompatibilityAccess, LegacyCompatibilityOperation, academic_pack_binding,
        validate_context,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn compatibility_registry_is_total_for_exported_aliases_and_unambiguous() {
        let registry = OperationRegistry::built_in().expect("typed operation registry");
        let mut legacy = BTreeSet::new();
        for operation in LegacyCompatibilityOperation::REGISTERED_ALIASES {
            assert!(legacy.insert(operation.legacy()));
            assert!(!operation.canonical().is_empty());
            assert_ne!(operation.legacy(), operation.canonical());
            let registered = registry
                .compatibility_alias(operation.legacy())
                .expect("legacy alias is registered");
            assert_eq!(
                registered.canonical_operation.as_str(),
                operation.canonical()
            );
        }
        assert_eq!(legacy.len(), registry.compatibility_aliases.len());
        for retired_or_internal in LegacyCompatibilityOperation::RETIRED_REGISTERED_ALIASES
            .into_iter()
            .chain([
                LegacyCompatibilityOperation::JobImport,
                LegacyCompatibilityOperation::TaskShow,
            ])
        {
            assert!(
                registry
                    .compatibility_alias(retired_or_internal.legacy())
                    .is_none(),
                "retired or internal legacy alias must not be registered: {}",
                retired_or_internal.legacy()
            );
        }
    }

    #[test]
    fn generic_pack_context_fails_closed_with_canonical_remediation() {
        let context = LegacyCompatibilityContextV3 {
            authority: LegacyCompatibilityAuthority::WorkspaceV3,
            bindings: vec![LegacyApplicationBindingV3 {
                legacy_job_id: Some(entity(1)),
                application_id: ApplicationId::try_new(entity(2).to_string())
                    .expect("Application ID"),
                pack: ApplicationPackBindingV3 {
                    id: WorkflowPackId::try_new("org.example.generic-application")
                        .expect("Pack ID"),
                    version: SemanticVersion::try_new("1.0.0").expect("version"),
                    content_digest: Sha256Digest::try_new("b".repeat(64)).expect("digest"),
                },
            }],
        };
        let error = validate_context(
            LegacyCompatibilityOperation::JobShow,
            LegacyCompatibilityAccess::Read,
            context,
        )
        .expect_err("generic Pack must fail closed");
        let failure = error.classify();
        assert_eq!(
            failure.code,
            canisend_contracts::ErrorCode::CompatibilityUnavailable
        );
        assert_eq!(
            failure
                .details
                .as_ref()
                .and_then(|details| details.get("reason"))
                .and_then(serde_json::Value::as_str),
            Some("unsupported-pack")
        );
        assert_eq!(
            failure.remediation.expect("remediation").action,
            "application.show"
        );
    }

    #[test]
    fn every_legacy_operation_fails_closed_on_workspace_v4() {
        for operation in LegacyCompatibilityOperation::ALL {
            for access in [
                LegacyCompatibilityAccess::Read,
                LegacyCompatibilityAccess::Write,
            ] {
                let error = validate_context(
                    operation,
                    access,
                    LegacyCompatibilityContextV3 {
                        authority: LegacyCompatibilityAuthority::WorkspaceV4,
                        bindings: Vec::new(),
                    },
                )
                .expect_err("Workspace v4 must reject every legacy operation")
                .classify();
                assert_eq!(
                    error.code,
                    canisend_contracts::ErrorCode::CompatibilityUnavailable
                );
                let details = error.details.expect("body-free compatibility details");
                assert_eq!(details["reason"], "workspace-v4-legacy-surface-retired");
                assert_eq!(details["workspace_mutated"], false);
                assert_eq!(details["detected_packs"], serde_json::json!([]));
            }
        }
    }

    #[test]
    fn migrated_academic_reads_pass_and_legacy_writes_do_not_mutate() {
        let root = temporary_root("migrated-academic");
        let backup = temporary_root("migrated-academic-backup");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&backup);
        Application::initialize_workspace(&root).expect("workspace");
        let created = Application::create_job(&root, "Research Fellow", "Example University")
            .expect("legacy Job")
            .data;
        let preview = Application::preview_workspace_v3_migration(&root)
            .expect("migration preview")
            .data;
        Application::migrate_workspace_v3(
            &root,
            WorkspaceV3MigrationRequest {
                expected_plan_sha256: preview.migration_plan_sha256,
                backup_destination: backup.clone(),
            },
        )
        .expect("migration");

        let shown = Application::job_detail(&root, created.id.as_str())
            .expect("academic compatibility read");
        assert_eq!(
            shown.compatibility.expect("compatibility").authority,
            CompatibilityAuthority::WorkspaceV3AcademicReadOnly
        );
        assert!(!shown.data.job.archived);

        let error = Application::archive_job(&root, created.id.as_str())
            .expect_err("legacy write must fail before mutation")
            .classify();
        assert_eq!(
            error.code,
            canisend_contracts::ErrorCode::CompatibilityUnavailable
        );
        assert_eq!(
            error
                .details
                .as_ref()
                .and_then(|details| details.get("workspace_mutated"))
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert!(
            !Application::job_detail(&root, created.id.as_str())
                .expect("unchanged Job")
                .data
                .job
                .archived
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(backup);
    }

    #[test]
    fn exact_academic_pack_is_required_not_only_the_pack_id() {
        let mut mismatched = academic_pack_binding().expect("academic binding");
        mismatched.content_digest = Sha256Digest::try_new("c".repeat(64)).expect("digest");
        let context = LegacyCompatibilityContextV3 {
            authority: LegacyCompatibilityAuthority::WorkspaceV3,
            bindings: vec![LegacyApplicationBindingV3 {
                legacy_job_id: Some(entity(3)),
                application_id: ApplicationId::try_new(entity(4).to_string())
                    .expect("Application ID"),
                pack: mismatched,
            }],
        };
        assert!(
            validate_context(
                LegacyCompatibilityOperation::JobShow,
                LegacyCompatibilityAccess::Read,
                context,
            )
            .is_err()
        );
    }

    fn entity(suffix: u16) -> EntityId {
        EntityId::try_new(format!("019f4000-0000-7000-8000-{suffix:012}")).expect("Entity ID")
    }

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-compatibility-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
