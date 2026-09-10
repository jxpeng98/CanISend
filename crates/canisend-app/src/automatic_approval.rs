use std::{
    path::Path,
    time::{Duration, Instant},
};

use canisend_contracts::{ApplicationId, ContentRevisionReferenceV3};
use canisend_store::{EvidenceService, ProfileService};
use serde::Serialize;

use crate::{Application, ApplicationError, ApprovalKind, ApprovalScope};

pub const AUTOMATIC_APPROVAL_TTL: Duration = Duration::from_secs(60 * 60);

/// A standing user grant covers one Application, never an entire Workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AutomaticApprovalScopeV4 {
    pub workspace: ApprovalScope,
    pub application_id: ApplicationId,
}

impl AutomaticApprovalScopeV4 {
    /// Identify the exact imported Profile Source without returning private Evidence bodies.
    pub fn profile_source_for_evidence(
        &self,
        evidence: &ContentRevisionReferenceV3,
    ) -> Result<Option<ContentRevisionReferenceV3>, ApplicationError> {
        let mut workspace = crate::application::open_workspace_v4(&self.workspace.workspace)?;
        let normalized = EvidenceService::new(&mut workspace.database, &workspace.blobs)
            .source_reference_v4(evidence)?;
        Ok(
            ProfileService::new(&mut workspace.database, &workspace.blobs)
                .list_sources()?
                .into_iter()
                .find(|source| source.normalized_text == normalized)
                .map(|source| ContentRevisionReferenceV3 {
                    id: source.id,
                    revision: source.revision,
                    sha256: source.original.sha256,
                }),
        )
    }

    /// Newly selected Sources need individual authorization before joining routine work.
    pub fn includes_requirement_source(
        &self,
        source: &ContentRevisionReferenceV3,
    ) -> Result<bool, ApplicationError> {
        let model = Application::application_model_v4(
            &self.workspace.workspace,
            self.application_id.as_str(),
        )?
        .data;
        Ok(model
            .snapshot
            .requirements
            .iter()
            .any(|requirement| requirement.source_span.content == *source))
    }

    pub fn for_application(
        root: &Path,
        application_id: &ApplicationId,
    ) -> Result<Self, ApplicationError> {
        let model = Application::application_model_v4(root, application_id.as_str())?.data;
        let status = Application::workspace_status_v4(root)?.data;
        let workspace = ApprovalScope {
            workspace: status.path,
            workspace_id: status.status.workspace_id,
            pack: model.snapshot.pack,
        };
        Ok(Self {
            workspace,
            application_id: application_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AutomaticApprovalActionV4 {
    PrivateApplicationRead,
    Mutation(ApprovalKind),
}

impl AutomaticApprovalActionV4 {
    /// Profile/Evidence operations additionally require an explicitly granted Source revision.
    #[must_use]
    pub fn is_routine(self) -> bool {
        match self {
            Self::PrivateApplicationRead => true,
            Self::Mutation(kind) => matches!(
                kind,
                ApprovalKind::ApplicationRequirementExtraction
                    | ApprovalKind::ApplicationRequirementRevision
                    | ApprovalKind::ApplicationSourceRevision
                    | ApprovalKind::ApplicationRequirementConfirmation
                    | ApprovalKind::ApplicationPlanProposal
                    | ApprovalKind::ApplicationPlanConfirmation
                    | ApprovalKind::DeliverableDraft
                    | ApprovalKind::DeliverableRevision
                    | ApprovalKind::ReviewDisposition
                    | ApprovalKind::EvidenceConfirmation
                    | ApprovalKind::EvidenceAssociation
                    | ApprovalKind::ProfileAssociation
            ),
        }
    }
}

/// Process-local delegation, separate from single-use mutation preview tokens.
#[derive(Debug, Default)]
pub struct AutomaticApprovalSessionV4 {
    grant: Option<(AutomaticApprovalScopeV4, Instant)>,
    profile_sources: Vec<ContentRevisionReferenceV3>,
}

impl AutomaticApprovalSessionV4 {
    /// Call only after a trusted UI has accepted the standing grant's explicit scope.
    pub fn grant_by_user(&mut self, scope: AutomaticApprovalScopeV4) {
        self.grant = Some((scope, Instant::now() + AUTOMATIC_APPROVAL_TTL));
        self.profile_sources.clear();
    }

    pub fn include_profile_source_by_user(&mut self, source: ContentRevisionReferenceV3) {
        if self.grant.is_some() && !self.profile_sources.contains(&source) {
            self.profile_sources.push(source);
        }
    }

    #[must_use]
    pub fn profile_sources(&self) -> &[ContentRevisionReferenceV3] {
        &self.profile_sources
    }

    pub fn revoke(&mut self) {
        self.grant = None;
        self.profile_sources.clear();
    }

    /// A scope switch or expiry permanently clears the grant, even if revisited later.
    pub fn remaining(&mut self, scope: &AutomaticApprovalScopeV4) -> Option<Duration> {
        if let Some((granted, expires)) = &self.grant
            && granted == scope
            && let Some(remaining) = expires.checked_duration_since(Instant::now())
            && !remaining.is_zero()
        {
            return Some(remaining);
        }
        self.revoke();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canisend_contracts::{ApplicationPackBindingV3, EntityId, Sha256Digest, WorkflowPackId};

    #[test]
    fn delegation_is_opt_in_scoped_expiring_and_excludes_sensitive_operations() {
        let scope = AutomaticApprovalScopeV4 {
            workspace: ApprovalScope {
                workspace: "/fixture/workspace".into(),
                workspace_id: EntityId::try_new("019f2f55-7c00-7000-8000-000000000001").unwrap(),
                pack: ApplicationPackBindingV3 {
                    id: WorkflowPackId::try_new("org.canisend.generic-application").unwrap(),
                    version: "1.0.0".parse().unwrap(),
                    content_digest: Sha256Digest::try_new("1".repeat(64)).unwrap(),
                },
            },
            application_id: ApplicationId::try_new("019f2f55-7c00-7000-8000-000000000002").unwrap(),
        };
        let mut session = AutomaticApprovalSessionV4::default();
        assert!(session.remaining(&scope).is_none());
        session.grant_by_user(scope.clone());
        let source = ContentRevisionReferenceV3 {
            id: EntityId::try_new("019f2f55-7c00-7000-8000-000000000005").unwrap(),
            revision: canisend_contracts::Revision::try_new(1).unwrap(),
            sha256: Sha256Digest::try_new("3".repeat(64)).unwrap(),
        };
        assert!(session.profile_sources().is_empty());
        let expiry = session.grant.as_ref().unwrap().1;
        session.include_profile_source_by_user(source.clone());
        session.include_profile_source_by_user(source.clone());
        assert_eq!(session.profile_sources(), std::slice::from_ref(&source));
        assert_eq!(session.grant.as_ref().unwrap().1, expiry);
        let mut revised_source = source;
        revised_source.sha256 = Sha256Digest::try_new("4".repeat(64)).unwrap();
        assert!(!session.profile_sources().contains(&revised_source));
        assert!(session.remaining(&scope).unwrap() <= AUTOMATIC_APPROVAL_TTL);
        let mut changed = scope.clone();
        changed.application_id =
            ApplicationId::try_new("019f2f55-7c00-7000-8000-000000000003").unwrap();
        assert!(session.remaining(&changed).is_none());
        assert!(session.profile_sources().is_empty());
        assert!(session.remaining(&scope).is_none());
        changed = scope.clone();
        changed.workspace.workspace_id =
            EntityId::try_new("019f2f55-7c00-7000-8000-000000000004").unwrap();
        session.grant_by_user(scope.clone());
        assert!(session.remaining(&changed).is_none());
        changed = scope.clone();
        changed.workspace.pack.content_digest = Sha256Digest::try_new("2".repeat(64)).unwrap();
        session.grant_by_user(scope.clone());
        assert!(session.remaining(&changed).is_none());
        session.grant = Some((scope.clone(), Instant::now()));
        assert!(session.remaining(&scope).is_none());
        session.grant_by_user(scope.clone());
        session.revoke();
        assert!(session.remaining(&scope).is_none());
        assert!(AutomaticApprovalActionV4::PrivateApplicationRead.is_routine());
        assert!(AutomaticApprovalActionV4::Mutation(ApprovalKind::DeliverableDraft).is_routine());
        for kind in [
            ApprovalKind::EvidenceConfirmation,
            ApprovalKind::EvidenceAssociation,
            ApprovalKind::ProfileAssociation,
        ] {
            assert!(AutomaticApprovalActionV4::Mutation(kind).is_routine());
        }
        for kind in [
            ApprovalKind::ExportPrepare,
            ApprovalKind::DiscoveryImport,
            ApprovalKind::WorkflowRerun,
        ] {
            assert!(!AutomaticApprovalActionV4::Mutation(kind).is_routine());
        }
    }
}
