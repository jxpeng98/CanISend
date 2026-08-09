use std::path::PathBuf;

use canisend_app::{
    ApprovalBinding, ApprovalBroker, ApprovalBrokerError, ApprovalDisposition, ApprovalGrant,
    ApprovalKind, ApprovalLease, ApprovalScope, WorkflowRerunRequest,
};
use canisend_contracts::DiscoveryImportReport;

use crate::application_intake::PreparedApplicationIntakeV4;
use crate::commands::DesktopCommandError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopDiscoveryKind {
    Import,
    Refresh,
}

#[derive(Debug, Clone)]
pub(crate) enum DesktopPendingApproval {
    ApplicationIntake(Box<PreparedApplicationIntakeV4>),
    Discovery {
        workspace: PathBuf,
        kind: DesktopDiscoveryKind,
        report: Box<DiscoveryImportReport>,
    },
    JobIntake,
    WorkflowRerun {
        workspace: PathBuf,
        request: WorkflowRerunRequest,
    },
}

#[derive(Debug, Default)]
pub(crate) struct DesktopApprovalStore {
    broker: ApprovalBroker<DesktopPendingApproval>,
}

impl DesktopApprovalStore {
    pub(crate) fn insert(
        &self,
        binding: ApprovalBinding,
        pending: DesktopPendingApproval,
    ) -> Result<ApprovalLease, DesktopCommandError> {
        self.broker
            .insert(binding, pending)
            .map_err(DesktopCommandError::approval)
    }

    pub(crate) fn take(
        &self,
        token: &str,
        kind: ApprovalKind,
        scope: &ApprovalScope,
    ) -> Result<ApprovalGrant<DesktopPendingApproval>, DesktopCommandError> {
        self.broker
            .take(token, kind, scope)
            .map_err(DesktopCommandError::approval)
    }

    pub(crate) fn resolve(
        &self,
        grant: ApprovalGrant<DesktopPendingApproval>,
        disposition: ApprovalDisposition,
    ) -> Result<(), DesktopCommandError> {
        self.broker
            .resolve(grant, disposition)
            .map_err(DesktopCommandError::approval)
    }

    pub(crate) fn discard(
        &self,
        token: &str,
        kind: ApprovalKind,
        scope: &ApprovalScope,
    ) -> Result<(), DesktopCommandError> {
        self.broker
            .discard(token, kind, scope)
            .map_err(DesktopCommandError::approval)
    }
}

pub(crate) fn lease_fields(lease: ApprovalLease) -> (String, u64, u64) {
    (
        lease.token,
        lease.expires_at_unix_ms,
        lease.remaining_ttl_seconds,
    )
}

pub(crate) fn approval_error_code(error: &ApprovalBrokerError) -> &'static str {
    match error {
        ApprovalBrokerError::InvalidConfiguration(_) => "approval.invalid-configuration",
        ApprovalBrokerError::Unavailable => "approval.unavailable",
        ApprovalBrokerError::TokenGeneration(_) | ApprovalBrokerError::TokenCollision => {
            "approval.token-generation-failed"
        }
        ApprovalBrokerError::CapacityFull { .. } => "approval.capacity-full",
        ApprovalBrokerError::MalformedToken => "approval.token-malformed",
        ApprovalBrokerError::Missing => "approval.missing-or-replayed",
        ApprovalBrokerError::Expired => "approval.expired",
        ApprovalBrokerError::WrongKind { .. } => "approval.wrong-kind",
        ApprovalBrokerError::WrongContext => "approval.wrong-context",
        ApprovalBrokerError::RestoreCollision => "approval.restore-collision",
    }
}
