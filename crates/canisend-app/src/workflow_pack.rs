use canisend_core::{
    VerifiedWorkflowPackBundle, WorkflowPackByteLoader, WorkflowPackCapabilityRegistry,
    WorkflowPackOrigin, WorkflowPackRuntime,
};
use canisend_resources::{ACADEMIC_JOB_WORKFLOW_PACK_ID, academic_job_workflow_pack, verify};

use crate::ApplicationError;

pub fn built_in_academic_job_pack() -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    verify().map_err(ApplicationError::ResourceIntegrity)?;
    let embedded = academic_job_workflow_pack();
    if embedded.id() != ACADEMIC_JOB_WORKFLOW_PACK_ID {
        return Err(ApplicationError::ResourceIntegrity(
            "embedded academic workflow Pack identity is inconsistent".to_owned(),
        ));
    }
    let manifest_bytes = embedded.manifest_bytes();
    let verified = WorkflowPackByteLoader::verify(
        manifest_bytes,
        embedded.into_resources(),
        WorkflowPackOrigin::BuiltIn,
        &WorkflowPackRuntime::parse(env!("CARGO_PKG_VERSION"), "3.0.0-alpha.1", "3.0.0-alpha.1")
            .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?,
        &WorkflowPackCapabilityRegistry::built_in(),
    )
    .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    Ok(verified.into_bundle())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use canisend_contracts::{
        ArtifactKind, DocumentKind, WorkflowPackResourceKind, WorkflowPackStageOutput,
        WorkflowStage,
    };
    use canisend_core::{
        StageGraph, WorkflowPackDeliverableCatalogRuntime, WorkflowPackStageGraph,
    };

    use super::*;

    #[test]
    fn academic_pack_preserves_the_legacy_stage_graph_and_modes() {
        let pack = built_in_academic_job_pack().expect("verified academic Pack");
        let legacy = StageGraph::built_in().descriptors();
        let graph = WorkflowPackStageGraph::from_verified_bundle(&pack)
            .expect("compiled academic Pack graph");
        let packed = graph.descriptors();
        assert_eq!(packed.len(), WorkflowStage::ALL.len());
        for legacy in &legacy {
            let packed = packed
                .iter()
                .find(|descriptor| descriptor.local_id().as_str() == legacy.stage.as_str())
                .expect("legacy stage is declared by the academic Pack");
            assert_eq!(packed.local_id().as_str(), legacy.stage.as_str());
            assert_eq!(
                packed
                    .depends_on()
                    .iter()
                    .map(|stage| stage.local_id_str())
                    .collect::<BTreeSet<_>>(),
                legacy
                    .depends_on
                    .iter()
                    .map(|stage| stage.as_str())
                    .collect::<BTreeSet<_>>()
            );
            assert_eq!(packed.execution_modes(), legacy.execution_modes.as_slice());
            assert_eq!(packed.output(), pack_output(legacy.output_kind));
        }
        assert_eq!(
            graph.terminal_stage().local_id_str(),
            WorkflowStage::Render.as_str()
        );
    }

    #[test]
    fn academic_pack_owns_the_canonical_taxonomy_materials_and_resources() {
        let pack = built_in_academic_job_pack().expect("verified academic Pack");
        let manifest = pack.manifest();
        assert_eq!(pack.snapshot().origin(), &WorkflowPackOrigin::BuiltIn);
        assert_eq!(pack.snapshot().id().as_str(), ACADEMIC_JOB_WORKFLOW_PACK_ID);
        let categories = [
            "qualification",
            "teaching",
            "research",
            "communication",
            "leadership",
            "service",
            "employment",
            "other",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(
            manifest
                .requirements
                .categories
                .iter()
                .map(|category| category.id.as_str())
                .collect::<BTreeSet<_>>(),
            categories
        );
        assert_eq!(
            manifest
                .evidence
                .categories
                .iter()
                .map(|category| category.id.as_str())
                .collect::<BTreeSet<_>>(),
            categories
        );

        let catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(&pack)
            .expect("compiled Deliverable catalog");
        assert_eq!(
            catalog
                .descriptors()
                .iter()
                .map(|descriptor| descriptor.local_id().as_str())
                .collect::<Vec<_>>(),
            DocumentKind::ALL
                .iter()
                .map(|kind| document_kind_id(*kind))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            catalog
                .required_kinds()
                .iter()
                .map(|kind| kind.local_id_str())
                .collect::<Vec<_>>(),
            vec!["cover-letter", "cv"]
        );

        assert_eq!(
            manifest
                .resources
                .iter()
                .filter(|resource| resource.kind == WorkflowPackResourceKind::Prompt)
                .map(|resource| resource.id.as_str())
                .collect::<BTreeSet<_>>(),
            [
                "job-parse-prompt",
                "evidence-normalize-prompt",
                "evidence-match-prompt",
                "document-draft-prompt",
                "document-review-prompt",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            manifest
                .resources
                .iter()
                .filter(|resource| resource.kind == WorkflowPackResourceKind::Template)
                .map(|resource| resource.id.as_str())
                .collect::<BTreeSet<_>>(),
            ["modernpro-coverletter", "modernpro-cv"]
                .into_iter()
                .collect()
        );
        assert_eq!(
            catalog
                .descriptors()
                .iter()
                .map(|descriptor| {
                    (
                        descriptor.local_id().as_str(),
                        descriptor
                            .template()
                            .expect("academic Deliverable template")
                            .path()
                            .as_str(),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("cover-letter", "templates/modernpro-coverletter.typ"),
                ("research-statement", "templates/modernpro-coverletter.typ"),
                ("teaching-statement", "templates/modernpro-coverletter.typ"),
                ("cv", "templates/modernpro-cv.typ"),
            ])
        );
        assert_eq!(
            manifest
                .validation
                .definitions
                .iter()
                .map(|validator| (validator.id.as_str(), validator.capability.as_str()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("traceability", "canisend.validator.evidence-traceability"),
                (
                    "unsupported-claims",
                    "canisend.validator.unsupported-claims"
                ),
                ("placeholder-free", "canisend.validator.placeholder-free"),
                (
                    "citation-integrity",
                    "canisend.validator.citation-integrity"
                ),
                ("review-complete", "canisend.validator.review-complete"),
            ])
        );
        assert_eq!(
            manifest
                .capabilities
                .intake_adapters
                .iter()
                .map(|capability| capability.as_str())
                .collect::<BTreeSet<_>>(),
            [
                "canisend.intake.local-file",
                "canisend.intake.user-url",
                "canisend.intake.text-pdf",
                "canisend.discovery.rss-atom",
                "canisend.discovery.jobs-ac-uk",
                "canisend.discovery.greenhouse",
                "canisend.discovery.lever",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(pack.resources().len(), 7);
        assert!(
            manifest
                .locales
                .keys()
                .any(|locale| locale.as_str() == "en")
        );
        assert!(
            manifest
                .locales
                .keys()
                .any(|locale| locale.as_str() == "zh-Hans")
        );
    }

    const fn pack_output(kind: ArtifactKind) -> WorkflowPackStageOutput {
        match kind {
            ArtifactKind::SourceNormalizedText => WorkflowPackStageOutput::Sources,
            ArtifactKind::ParsedJob => WorkflowPackStageOutput::None,
            ArtifactKind::Criteria => WorkflowPackStageOutput::Requirements,
            ArtifactKind::EvidenceCatalog => WorkflowPackStageOutput::Evidence,
            ArtifactKind::EvidenceMatches => WorkflowPackStageOutput::Matches,
            ArtifactKind::ApplicationPlan => WorkflowPackStageOutput::Plan,
            ArtifactKind::DocumentSet => WorkflowPackStageOutput::Deliverables,
            ArtifactKind::ReviewFindings => WorkflowPackStageOutput::Review,
            ArtifactKind::PackageManifest => WorkflowPackStageOutput::Package,
            ArtifactKind::RenderManifest => WorkflowPackStageOutput::Render,
            _ => panic!("legacy academic graph contains an unexpected artifact kind"),
        }
    }

    const fn document_kind_id(kind: DocumentKind) -> &'static str {
        match kind {
            DocumentKind::CoverLetter => "cover-letter",
            DocumentKind::ResearchStatement => "research-statement",
            DocumentKind::TeachingStatement => "teaching-statement",
            DocumentKind::Cv => "cv",
        }
    }
}
