use std::collections::BTreeSet;

use canisend_contracts::{
    ApplicationModelSnapshotV3, ContentRevisionReferenceV3, DeliverableDocumentV3,
    DeliverableKindId, DeliverableRecordV3, DeliverableStateV3, DeliverableTextRoleV3,
    PlanRecordV3, PlanStateV3, PlannedDeliverableDispositionV3, RequirementConfirmationV3,
};

use crate::WorkflowPackDeliverableDescriptor;

/// Required kinds belong to the confirmed Plan, independently of Pack minimum counts.
pub fn validate_required_plan_deliverables<'a>(
    plan: &PlanRecordV3,
    kinds: impl IntoIterator<Item = &'a DeliverableKindId>,
) -> Result<(), String> {
    if plan.state != PlanStateV3::Confirmed {
        return Err("the current Plan is not confirmed".to_owned());
    }
    let kinds = kinds.into_iter().collect::<BTreeSet<_>>();
    for kind in &kinds {
        if !plan.deliverables.iter().any(|item| {
            &item.kind == *kind && item.disposition != PlannedDeliverableDispositionV3::Omitted
        }) {
            return Err(format!(
                "Deliverable kind {kind} is absent or omitted in the current Plan"
            ));
        }
    }
    for item in &plan.deliverables {
        if item.disposition == PlannedDeliverableDispositionV3::Required
            && !kinds.contains(&item.kind)
        {
            return Err(format!(
                "required Plan Deliverable {} is missing",
                item.kind
            ));
        }
    }
    Ok(())
}

/// Dispatch only capabilities the kernel actually implements. Natural-language truth and
/// the appropriateness of Intent/NonFactual labels remain responsibilities of explicit review.
pub fn validate_deliverable_document_v3(
    descriptor: &WorkflowPackDeliverableDescriptor,
    document: &DeliverableDocumentV3,
    snapshot: &ApplicationModelSnapshotV3,
    deliverable: &DeliverableRecordV3,
    current_evidence: &[ContentRevisionReferenceV3],
    exporting: bool,
) -> Result<(), String> {
    document.validate().map_err(str::to_owned)?;
    for validator in descriptor.validators() {
        let capability = validator.capability().as_str();
        if !validator.parameters().is_empty()
            && (capability != "canisend.validator.placeholder-free"
                || validator.parameters().len() != 1
                || !validator
                    .parameters()
                    .contains_key("required_evidence_fields"))
        {
            return Err(format!(
                "validator {} has unsupported parameters",
                validator.id()
            ));
        }
        let mut missing_field = false;
        if let Some(value) = validator.parameters().get("required_evidence_fields") {
            let fields = value
                .as_array()
                .filter(|fields| fields.len() <= 64)
                .ok_or_else(|| "required evidence fields parameter is invalid".to_owned())?;
            for key in fields {
                let key = key
                    .as_str()
                    .ok_or_else(|| "required evidence field key is invalid".to_owned())?;
                canisend_contracts::WorkflowPackItemId::try_new(key)
                    .map_err(|_| "required evidence field key is invalid".to_owned())?;
                missing_field |= !document.fields.iter().any(|field| {
                    field.key.as_str() == key
                        && field.value.role == DeliverableTextRoleV3::EvidenceBound
                        && !field.value.evidence.is_empty()
                });
            }
        }
        let failed = match capability {
            "canisend.validator.evidence-traceability" => document.texts().any(|text| {
                text.evidence.iter().any(|reference| {
                    !current_evidence.contains(reference)
                        || !deliverable.evidence_inputs.iter().any(|input| {
                            input.id == reference.id && input.revision == reference.revision
                        })
                })
            }),
            "canisend.validator.unsupported-claims" => {
                document.texts().any(|text| match text.role {
                    DeliverableTextRoleV3::EvidenceBound => {
                        text.evidence.is_empty() || !text.requirements.is_empty()
                    }
                    DeliverableTextRoleV3::RequirementBound => {
                        text.requirements.is_empty() || !text.evidence.is_empty()
                    }
                    DeliverableTextRoleV3::Intent | DeliverableTextRoleV3::NonFactual => {
                        !text.evidence.is_empty() || !text.requirements.is_empty()
                    }
                })
            }
            "canisend.validator.citation-integrity" => document.texts().any(|text| {
                text.evidence
                    .iter()
                    .any(|reference| !current_evidence.contains(reference))
                    || text.requirements.iter().any(|reference| {
                        !snapshot.requirements.iter().any(|requirement| {
                            requirement.id == reference.id
                                && requirement.revision == reference.revision
                                && requirement.confirmation == RequirementConfirmationV3::Confirmed
                        })
                    })
            }),
            "canisend.validator.placeholder-free" => {
                missing_field
                    || !document.unresolved_fields.is_empty()
                    || has_placeholder(&deliverable.title)
                    || document.texts().any(|text| has_placeholder(&text.text))
                    || document
                        .sections
                        .iter()
                        .any(|section| section.heading.as_deref().is_some_and(has_placeholder))
            }
            "canisend.validator.review-complete" => {
                deliverable.state
                    != if exporting {
                        DeliverableStateV3::Approved
                    } else {
                        DeliverableStateV3::ReviewRequired
                    }
            }
            _ => return Err(format!("validator {} is not implemented", validator.id())),
        };
        if failed {
            return Err(format!(
                "Deliverable {} failed validator {}",
                deliverable.kind,
                validator.id()
            ));
        }
    }
    Ok(())
}

fn has_placeholder(text: &str) -> bool {
    let upper = text.to_ascii_uppercase();
    upper.contains("[TODO") || upper.contains("[TBD") || upper.contains("[PLACEHOLDER")
}

#[cfg(test)]
mod tests {
    use super::*;
    use canisend_contracts::{
        ApplicationId, ApplicationPackBindingV3, PlanId, PlannedDeliverableV3, Revision,
        SemanticVersion, Sha256Digest, WorkflowPackId, WorkflowPackItemId,
    };

    #[test]
    fn required_optional_omitted_and_stale_plan_selections_are_distinct() {
        let pack_id = WorkflowPackId::try_new("org.canisend.plan-test").unwrap();
        let kind = |value| {
            DeliverableKindId::from_parts(&pack_id, &WorkflowPackItemId::try_new(value).unwrap())
        };
        let primary = kind("primary");
        let optional = kind("optional");
        let omitted = kind("omitted");
        let mut plan = PlanRecordV3 {
            id: PlanId::try_new("0190a541-6de8-7000-8000-000000000001").unwrap(),
            application_id: ApplicationId::try_new("0190a541-6de8-7000-8000-000000000002").unwrap(),
            pack: ApplicationPackBindingV3 {
                id: pack_id.clone(),
                version: SemanticVersion::try_new("1.0.0").unwrap(),
                content_digest: Sha256Digest::try_new("a".repeat(64)).unwrap(),
            },
            state: PlanStateV3::Confirmed,
            decision: None,
            requirement_inputs: vec![],
            deliverables: [
                (primary.clone(), PlannedDeliverableDispositionV3::Required),
                (optional.clone(), PlannedDeliverableDispositionV3::Optional),
                (omitted.clone(), PlannedDeliverableDispositionV3::Omitted),
            ]
            .into_iter()
            .map(|(kind, disposition)| PlannedDeliverableV3 {
                kind,
                disposition,
                rationale: "Fixture selection".to_owned(),
                constraints: vec![],
                execution_mode: None,
            })
            .collect(),
            blockers: vec![],
            decided_by: None,
            decided_at: None,
            revision: Revision::try_new(1).unwrap(),
        };
        assert!(validate_required_plan_deliverables(&plan, [&primary]).is_ok());
        assert!(validate_required_plan_deliverables(&plan, [&primary, &optional]).is_ok());
        assert!(
            validate_required_plan_deliverables(&plan, [&optional])
                .unwrap_err()
                .contains("missing")
        );
        assert!(
            validate_required_plan_deliverables(&plan, [&primary, &omitted])
                .unwrap_err()
                .contains("omitted")
        );
        plan.state = PlanStateV3::Stale;
        assert!(validate_required_plan_deliverables(&plan, [&primary]).is_err());
    }
}
