use canisend_contracts::{ErrorCode, NextAction};
use canisend_io::{EmbeddedRenderError, IoAdapterError};
use canisend_resources::ResourceError;
use canisend_store::StoreError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("{0}")]
    Store(#[from] StoreError),
    #[error("{0}")]
    Input(#[from] IoAdapterError),
    #[error("{0}")]
    Render(#[from] EmbeddedRenderError),
    #[error("invalid entity ID: {0}")]
    InvalidEntityId(String),
    #[error("unknown public schema: {0}")]
    SchemaNotFound(String),
    #[error("input is invalid: {0}")]
    InvalidInput(String),
    #[error("{message}")]
    ConsentRequired {
        message: String,
        remediation: NextAction,
    },
    #[error("embedded resources failed verification: {0}")]
    ResourceIntegrity(String),
    #[error("{0}")]
    ResourceExport(#[from] ResourceError),
    #[error("CLI installation failed: {0}")]
    CliInstall(String),
    #[error("update check failed: {0}")]
    UpdateCheck(String),
    #[error("{message}")]
    CompatibilityUnavailable {
        message: String,
        details: Value,
        remediation: NextAction,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFailure {
    pub status: String,
    pub code: ErrorCode,
    pub retryable: bool,
    pub message: String,
    pub details: Option<Value>,
    pub remediation: Option<NextAction>,
}

impl ApplicationError {
    #[must_use]
    pub fn classify(&self) -> ApplicationFailure {
        let (status, code, retryable, details, remediation) = match self {
            Self::Store(error) => classify_store(error),
            Self::Input(error) => classify_input(error),
            Self::Render(error) => classify_render(error),
            Self::InvalidEntityId(_) | Self::InvalidInput(_) => {
                ("invalid", ErrorCode::InputInvalid, false, None, None)
            }
            Self::SchemaNotFound(_) => ("not-found", ErrorCode::SchemaNotFound, false, None, None),
            Self::ConsentRequired { remediation, .. } => (
                "consent-required",
                ErrorCode::ConsentRequired,
                false,
                None,
                Some(remediation.clone()),
            ),
            Self::ResourceIntegrity(_) => (
                "integrity-failed",
                ErrorCode::ResourcesIntegrityFailed,
                false,
                None,
                None,
            ),
            Self::ResourceExport(error) => classify_resource(error),
            Self::CliInstall(_) => (
                "install-failed",
                ErrorCode::InputPathRejected,
                false,
                None,
                None,
            ),
            Self::UpdateCheck(_) => (
                "fetch-failed",
                ErrorCode::ExternalIoFailed,
                true,
                None,
                None,
            ),
            Self::CompatibilityUnavailable {
                details,
                remediation,
                ..
            } => (
                "compatibility-unavailable",
                ErrorCode::CompatibilityUnavailable,
                false,
                Some(details.clone()),
                Some(remediation.clone()),
            ),
        };
        ApplicationFailure {
            status: status.to_owned(),
            code,
            retryable,
            message: match self {
                Self::InvalidEntityId(message)
                | Self::SchemaNotFound(message)
                | Self::InvalidInput(message)
                | Self::ResourceIntegrity(message)
                | Self::CliInstall(message)
                | Self::UpdateCheck(message) => message.clone(),
                Self::CompatibilityUnavailable { message, .. } => message.clone(),
                Self::ConsentRequired { message, .. } => message.clone(),
                Self::Store(_) | Self::Input(_) | Self::Render(_) | Self::ResourceExport(_) => {
                    self.to_string()
                }
            },
            details,
            remediation,
        }
    }
}

fn classify_resource(error: &ResourceError) -> Classification {
    match error {
        ResourceError::UnknownId(_) => {
            ("not-found", ErrorCode::ResourceNotFound, false, None, None)
        }
        ResourceError::InvalidSelection(_) => {
            ("invalid", ErrorCode::InputInvalid, false, None, None)
        }
        ResourceError::Integrity(_) => (
            "integrity-failed",
            ErrorCode::ResourcesIntegrityFailed,
            false,
            None,
            None,
        ),
        ResourceError::UnsafeExportPath(_)
        | ResourceError::ManagedSkillModified(_)
        | ResourceError::UnmanagedSkillFiles(_) => {
            ("invalid", ErrorCode::InputPathRejected, false, None, None)
        }
        ResourceError::ExportIo { .. } => {
            ("io-failed", ErrorCode::ExternalIoFailed, true, None, None)
        }
    }
}

type Classification = (
    &'static str,
    ErrorCode,
    bool,
    Option<Value>,
    Option<NextAction>,
);

fn classify_input(error: &IoAdapterError) -> Classification {
    let (status, code, retryable) = match error {
        IoAdapterError::PdfEncrypted => ("invalid", ErrorCode::PdfEncrypted, false),
        IoAdapterError::PdfMalformed(_) | IoAdapterError::PdfPageLimit { .. } => {
            ("invalid", ErrorCode::PdfMalformed, false)
        }
        IoAdapterError::PdfTextUnavailable => {
            ("text-unavailable", ErrorCode::PdfTextUnavailable, false)
        }
        IoAdapterError::Io { .. } => ("io-failed", ErrorCode::ExternalIoFailed, true),
        IoAdapterError::Http(_)
        | IoAdapterError::ResponseRead(_)
        | IoAdapterError::DnsResolution(_)
        | IoAdapterError::HttpStatus(_) => ("fetch-failed", ErrorCode::ExternalIoFailed, true),
        IoAdapterError::UnsafeLocalFile(_) | IoAdapterError::UnsupportedLocalType(_) => {
            ("invalid", ErrorCode::InputPathRejected, false)
        }
        IoAdapterError::InputTooLarge { .. }
        | IoAdapterError::InvalidTextEncoding
        | IoAdapterError::UnsafeTextControlCharacter
        | IoAdapterError::TextUnavailable
        | IoAdapterError::InvalidUrl(_)
        | IoAdapterError::UrlPolicy(_)
        | IoAdapterError::InvalidRedirect(_)
        | IoAdapterError::UnsupportedContentType(_)
        | IoAdapterError::Html(_)
        | IoAdapterError::PdfTimeBudget
        | IoAdapterError::DiscoveryInput(_)
        | IoAdapterError::CandidateInput(_) => ("invalid", ErrorCode::InputInvalid, false),
    };
    let remediation = match error {
        IoAdapterError::PdfTextUnavailable => Some(NextAction {
            action: "provide a text-based PDF, Markdown, or plain-text advert".to_owned(),
            description:
                "CanISend does not run OCR; extract and review scanned text with a trusted tool before importing"
                    .to_owned(),
        }),
        IoAdapterError::PdfEncrypted => Some(NextAction {
            action: "decrypt the PDF or request an unencrypted advert".to_owned(),
            description: "CanISend never guesses, stores, or transmits PDF passwords".to_owned(),
        }),
        _ => None,
    };
    (status, code, retryable, None, remediation)
}

fn classify_render(error: &EmbeddedRenderError) -> Classification {
    let code = match error {
        EmbeddedRenderError::EncryptedPdf => ErrorCode::PdfEncrypted,
        EmbeddedRenderError::InvalidPdf | EmbeddedRenderError::PageCountInvalid => {
            ErrorCode::PdfMalformed
        }
        EmbeddedRenderError::SourceTooLarge { .. }
        | EmbeddedRenderError::CompileFailed { .. }
        | EmbeddedRenderError::PdfExportFailed { .. }
        | EmbeddedRenderError::PdfTooLarge { .. }
        | EmbeddedRenderError::TimeBudgetExceeded { .. } => ErrorCode::InternalInvariantFailed,
    };
    ("render-failed", code, false, None, None)
}

#[allow(clippy::too_many_lines)]
fn classify_store(error: &StoreError) -> Classification {
    let (status, code, retryable) = match error {
        StoreError::WorkspaceNotFound(_) => ("not-found", ErrorCode::WorkspaceNotFound, false),
        StoreError::JobNotFound(_) => ("not-found", ErrorCode::JobNotFound, false),
        StoreError::JobArchived(_) => ("archived", ErrorCode::JobArchived, false),
        StoreError::ApplicationModelNotFound(_) => {
            ("not-found", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::ApplicationModelUnavailable | StoreError::ApplicationModelConflict(_) => {
            ("conflict", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::ApplicationAssociationNotFound(_) => {
            ("not-found", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::ApplicationAssociationConflict(_) => {
            ("conflict", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::ApplicationAssociationConsentRequired(_) => {
            ("consent-required", ErrorCode::ConsentRequired, false)
        }
        StoreError::WorkspaceMigrationConflict(_) => {
            ("conflict", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::WorkspaceVersionUnsupported { .. } => {
            ("upgrade-required", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::WorkspaceFormatUnsupported { .. } => (
            "compatibility-unavailable",
            ErrorCode::CompatibilityUnavailable,
            false,
        ),
        StoreError::ApplicationModelIntegrity(_) | StoreError::WorkspaceMigrationIntegrity(_) => (
            "integrity-failed",
            ErrorCode::InternalInvariantFailed,
            false,
        ),
        StoreError::ProfileSourceNotFound(_) => {
            ("not-found", ErrorCode::ProfileSourceNotFound, false)
        }
        StoreError::DiscoverySourceNotFound(_) => {
            ("not-found", ErrorCode::DiscoverySourceNotFound, false)
        }
        StoreError::DiscoveryLeadNotFound(_) => {
            ("not-found", ErrorCode::DiscoveryLeadNotFound, false)
        }
        StoreError::DiscoveryConflict(_) => ("conflict", ErrorCode::DiscoveryConflict, false),
        StoreError::TaskNotFound(_) => ("not-found", ErrorCode::TaskNotFound, false),
        StoreError::TaskStale(_) => ("stale", ErrorCode::TaskStale, true),
        StoreError::TaskConflict(_) => ("conflict", ErrorCode::TaskConflict, false),
        StoreError::WorkflowNotFound(_) => ("not-found", ErrorCode::WorkflowNotFound, false),
        StoreError::WorkflowConflict(_) | StoreError::TemplateFieldsUnresolved { .. } => {
            ("conflict", ErrorCode::WorkflowConflict, false)
        }
        StoreError::EmbeddedRender(render) => return classify_render(render),
        StoreError::CandidateStructural(_) => {
            ("validation-failed", ErrorCode::CandidateSchemaInvalid, true)
        }
        StoreError::CandidateSemantic(_) => (
            "validation-failed",
            ErrorCode::CandidateSemanticInvalid,
            true,
        ),
        StoreError::WorkspaceExists(_)
        | StoreError::Sqlite(_)
        | StoreError::DependencyConflict(_)
        | StoreError::ArtifactNotFound(_)
        | StoreError::ProjectionEdited(_)
        | StoreError::ProjectionUnmanagedConflict(_)
        | StoreError::ProjectionNotFound(_) => ("conflict", ErrorCode::WorkspaceConflict, false),
        StoreError::UnsafePath(_)
        | StoreError::NotDirectory(_)
        | StoreError::ProjectionPathRejected
        | StoreError::BlobTooLarge { .. }
        | StoreError::ConfigDecode(_)
        | StoreError::BackupInvalid(_) => ("invalid", ErrorCode::InputPathRejected, false),
        StoreError::InvalidInput(_) => ("invalid", ErrorCode::InputInvalid, false),
        StoreError::Io { .. } | StoreError::BlobMissing(_) => {
            ("io-failed", ErrorCode::ExternalIoFailed, true)
        }
        StoreError::BlobDigestMismatch { .. } | StoreError::BlobCollision(_) => {
            ("integrity-failed", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::ConfigEncode(_)
        | StoreError::Json(_)
        | StoreError::Contract(_)
        | StoreError::Random(_)
        | StoreError::Clock
        | StoreError::TypstProjectionInvariant
        | StoreError::Invariant(_) => (
            "invariant-failed",
            ErrorCode::InternalInvariantFailed,
            false,
        ),
    };
    let (details, remediation) = match error {
        StoreError::CandidateStructural(violations) | StoreError::CandidateSemantic(violations) => (
            serde_json::to_value(violations).ok(),
            Some(NextAction {
                action: "correct the candidate JSON and retry the same leased task".to_owned(),
                description:
                    "Use each violation's JSON pointer and stable code; no task state was committed"
                        .to_owned(),
            }),
        ),
        StoreError::TaskStale(_) => (
            None,
            Some(NextAction {
                action: "prepare the task again for the current job revision".to_owned(),
                description:
                    "A lease expired or a declared input changed; do not reuse the old candidate"
                        .to_owned(),
            }),
        ),
        StoreError::WorkspaceNotFound(_) => (
            None,
            Some(NextAction {
                action: "choose or initialize a CanISend workspace".to_owned(),
                description:
                    "Choose a new workspace directory, or select an existing canisend.toml"
                        .to_owned(),
            }),
        ),
        StoreError::WorkspaceVersionUnsupported { .. } => (
            None,
            Some(NextAction {
                action: "upgrade CanISend or restore the verified pre-upgrade backup to a new path"
                    .to_owned(),
                description:
                    "Do not modify the newer Workspace or attempt an in-place database downgrade"
                        .to_owned(),
            }),
        ),
        StoreError::WorkspaceFormatUnsupported { found, required } => (
            Some(serde_json::json!({
                "found": found,
                "required": required,
            })),
            Some(NextAction {
                action: "initialize a clean Workspace v4".to_owned(),
                description: "Choose a new or empty directory; compatibility detection does not open, migrate, or mutate the unsupported Workspace".to_owned(),
            }),
        ),
        StoreError::ApplicationAssociationConsentRequired(_) => (
            None,
            Some(NextAction {
                action: "grant the exact requested association consent".to_owned(),
                description: "Review the private Workspace resource and approve its explicit use by the selected Application"
                    .to_owned(),
            }),
        ),
        _ => (None, None),
    };
    (status, code, retryable, details, remediation)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use canisend_contracts::ErrorCode;
    use canisend_io::IoAdapterError;
    use canisend_resources::ResourceError;
    use canisend_store::StoreError;

    use super::ApplicationError;

    #[test]
    fn application_failures_use_stable_adapter_neutral_classes() {
        let missing =
            ApplicationError::from(StoreError::WorkspaceNotFound(PathBuf::from("/missing")))
                .classify();
        assert_eq!(missing.code, ErrorCode::WorkspaceNotFound);
        assert_eq!(missing.status, "not-found");
        assert!(!missing.retryable);
        assert!(missing.remediation.is_some());

        let future = ApplicationError::from(StoreError::WorkspaceVersionUnsupported {
            found: 15,
            supported: 14,
        })
        .classify();
        assert_eq!(future.code, ErrorCode::WorkspaceConflict);
        assert_eq!(future.status, "upgrade-required");
        assert!(!future.retryable);
        assert!(
            future
                .message
                .contains("restore a verified pre-upgrade backup")
        );
        assert_eq!(
            future.remediation.expect("restore remediation").action,
            "upgrade CanISend or restore the verified pre-upgrade backup to a new path"
        );

        let pdf = ApplicationError::from(IoAdapterError::PdfTextUnavailable).classify();
        assert_eq!(pdf.code, ErrorCode::PdfTextUnavailable);
        assert_eq!(pdf.status, "text-unavailable");
        assert!(pdf.remediation.is_some());

        let unsafe_export =
            ApplicationError::from(ResourceError::UnsafeExportPath(PathBuf::from("/unsafe")))
                .classify();
        assert_eq!(unsafe_export.code, ErrorCode::InputPathRejected);
        assert_eq!(unsafe_export.status, "invalid");
        assert!(!unsafe_export.retryable);
    }
}
