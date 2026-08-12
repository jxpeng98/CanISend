use std::path::Path;

use canisend_contracts::{
    EntityId, NextAction, PackageExportManifestRecord, PackageManifestRecord, ProjectionEditStatus,
    ProjectionReconcileRecord, ReadinessState, SafeRelativePath,
};
use canisend_io::EmbeddedTypstCompiler;
use canisend_store::{PackageService, ProjectionService, StoreError};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateExportConsent,
    application::{open_workspace, parse_entity_id},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageExportRequest {
    pub job_id: EntityId,
    pub destination: SafeRelativePath,
}

impl PackageExportRequest {
    pub fn try_new(job_id: &str, destination: &str) -> Result<Self, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let destination = parse_job_path(&job_id, destination)?;
        Ok(Self {
            job_id,
            destination,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionReplaceRequest {
    pub job_id: EntityId,
    pub path: SafeRelativePath,
}

impl ProjectionReplaceRequest {
    pub fn try_new(job_id: &str, path: &str) -> Result<Self, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let path = parse_job_path(&job_id, path)?;
        Ok(Self { job_id, path })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionCopyAsNewRequest {
    pub job_id: EntityId,
    pub path: SafeRelativePath,
    pub destination: SafeRelativePath,
}

impl ProjectionCopyAsNewRequest {
    pub fn try_new(job_id: &str, path: &str, destination: &str) -> Result<Self, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let path = parse_job_path(&job_id, path)?;
        let destination = parse_job_path(&job_id, destination)?;
        if path == destination {
            return Err(ApplicationError::InvalidInput(
                "copy destination must differ from the managed projection".to_owned(),
            ));
        }
        Ok(Self {
            job_id,
            path,
            destination,
        })
    }
}

impl Application {
    pub fn check_package(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<PackageManifestRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let artifact =
            PackageService::new(&mut workspace.database, &workspace.blobs).check(&job_id)?;
        let manifest =
            PackageService::new(&mut workspace.database, &workspace.blobs).current(&job_id)?;
        Ok(
            package_receipt("package.check", "Checked package readiness", manifest)
                .with_artifacts([artifact]),
        )
    }

    pub fn current_package(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<PackageManifestRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let manifest =
            PackageService::new(&mut workspace.database, &workspace.blobs).current(&job_id)?;
        Ok(package_receipt(
            "package.show",
            "Loaded current package manifest",
            manifest,
        ))
    }

    pub fn export_package(
        root: &Path,
        request: PackageExportRequest,
        consent: Option<PrivateExportConsent>,
    ) -> Result<ActionReceipt<PackageExportManifestRecord>, ApplicationError> {
        if consent.is_none() {
            return Err(private_export_consent_required(
                "The operation writes editable application material bodies under jobs/JOB_ID/",
            ));
        }
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let mut executor = EmbeddedTypstCompiler::new();
        let (artifact, receipt) =
            ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .export(&request.job_id, &request.destination, &mut executor)?;
        let count = receipt.projections.len();
        Ok(ActionReceipt::new(
            "package.export",
            "exported",
            format!(
                "Exported {count} managed projection(s) under {}; submission performed: no",
                request.destination
            ),
            receipt,
        )
        .with_artifacts([artifact]))
    }

    pub fn current_package_export(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<PackageExportManifestRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let (artifact, receipt) =
            ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .current(&job_id)?;
        let count = receipt.projections.len();
        Ok(ActionReceipt::new(
            "package.exports",
            "available",
            format!("Loaded current export receipt with {count} managed projection(s)"),
            receipt,
        )
        .with_artifacts([artifact]))
    }

    pub fn reconcile_package_projections(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<Vec<ProjectionReconcileRecord>>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let records =
            ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .reconcile(&job_id)?;
        let edited = records
            .iter()
            .filter(|record| record.projection.edit_status == ProjectionEditStatus::Edited)
            .count();
        let missing = records
            .iter()
            .filter(|record| record.projection.edit_status == ProjectionEditStatus::Missing)
            .count();
        let status = if edited == 0 && missing == 0 {
            "current"
        } else {
            "attention-required"
        };
        let count = records.len();
        Ok(ActionReceipt::new(
            "package.reconcile",
            status,
            format!(
                "Reconciled {count} managed projection(s): {edited} edited, {missing} missing; \
                 authoritative structured artifacts changed: no"
            ),
            records,
        ))
    }

    pub fn replace_package_projection(
        root: &Path,
        request: ProjectionReplaceRequest,
    ) -> Result<ActionReceipt<ProjectionReconcileRecord>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let mut executor = EmbeddedTypstCompiler::new();
        let record =
            ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .replace(&request.job_id, &request.path, &mut executor)?;
        Ok(ActionReceipt::new(
            "package.replace",
            "replaced",
            format!(
                "Restored managed projection {}; user edit preserved: no; authoritative \
                 structured artifacts changed: no",
                request.path
            ),
            record,
        ))
    }

    pub fn copy_package_projection_as_new(
        root: &Path,
        request: ProjectionCopyAsNewRequest,
    ) -> Result<ActionReceipt<ProjectionReconcileRecord>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let mut executor = EmbeddedTypstCompiler::new();
        let record =
            ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .copy_as_new(
                    &request.job_id,
                    &request.path,
                    &request.destination,
                    &mut executor,
                )?;
        Ok(ActionReceipt::new(
            "package.copy-as-new",
            "preserved-and-restored",
            format!(
                "Preserved user edit as {} and restored {}; authoritative structured artifacts \
                 changed: no",
                request.destination, request.path
            ),
            record,
        ))
    }
}

fn package_receipt(
    operation: &'static str,
    summary: &'static str,
    manifest: PackageManifestRecord,
) -> ActionReceipt<PackageManifestRecord> {
    let status = readiness_status(manifest.readiness.state);
    let reasons = manifest.readiness.reasons.len();
    ActionReceipt::new(
        operation,
        status,
        format!("{summary}: {reasons} readiness reason(s); submission performed: no"),
        manifest,
    )
}

const fn readiness_status(state: ReadinessState) -> &'static str {
    match state {
        ReadinessState::Blocked => "blocked",
        ReadinessState::NeedsReview => "needs-review",
        ReadinessState::ReadyToExport => "ready-to-export",
        ReadinessState::Exported => "exported",
    }
}

pub(crate) fn parse_job_path(
    job_id: &EntityId,
    value: &str,
) -> Result<SafeRelativePath, ApplicationError> {
    let path = SafeRelativePath::try_new(value)
        .map_err(|error| ApplicationError::Store(StoreError::from(error)))?;
    let expected = format!("jobs/{job_id}/");
    if !path.as_str().starts_with(&expected) {
        return Err(ApplicationError::Store(StoreError::ProjectionPathRejected));
    }
    Ok(path)
}

pub(crate) fn private_export_consent_required(description: &str) -> ApplicationError {
    ApplicationError::ConsentRequired {
        message: "export-private-artifacts consent must be explicitly confirmed".to_owned(),
        remediation: NextAction {
            action: "obtain user approval, then repeat the scoped private export".to_owned(),
            description: description.to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use canisend_contracts::ErrorCode;

    use crate::{
        Application, PackageExportRequest, PrivateExportConsent, ProjectionCopyAsNewRequest,
        ProjectionReplaceRequest,
    };

    const JOB_ID: &str = "019f2f55-7c00-7000-8000-000000000101";

    #[test]
    fn package_requests_reject_paths_outside_the_job_tree() {
        for error in [
            PackageExportRequest::try_new(JOB_ID, "jobs/other/application")
                .expect_err("outside export path"),
            ProjectionReplaceRequest::try_new(JOB_ID, "other/application/cover-letter.md")
                .expect_err("outside managed path"),
            ProjectionCopyAsNewRequest::try_new(
                JOB_ID,
                &format!("jobs/{JOB_ID}/application/cover-letter.md"),
                "jobs/other/cover-letter-edited.md",
            )
            .expect_err("outside copy path"),
        ] {
            assert_eq!(error.classify().code, ErrorCode::InputPathRejected);
        }
    }

    #[test]
    fn private_export_consent_is_required_before_workspace_access() {
        let missing = std::env::temp_dir().join(format!(
            "canisend-app-package-consent-{}",
            std::process::id()
        ));
        let request = PackageExportRequest::try_new(JOB_ID, &format!("jobs/{JOB_ID}/application"))
            .expect("valid export request");
        let error = Application::export_package(&missing, request.clone(), None)
            .expect_err("private export without consent");
        assert_eq!(error.classify().code, ErrorCode::ConsentRequired);
        assert!(!missing.exists());

        let error = Application::export_package(
            &missing,
            request,
            Some(PrivateExportConsent::granted_by_user()),
        )
        .expect_err("missing workspace with consent");
        assert_eq!(error.classify().code, ErrorCode::WorkspaceNotFound);
    }
}
