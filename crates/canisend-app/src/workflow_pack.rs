use std::path::Path;

use canisend_contracts::WorkflowPackManifest;
use canisend_core::{
    VerifiedWorkflowPackBundle, WorkflowPackByteLoader, WorkflowPackCapabilityRegistry,
    WorkflowPackOrigin, WorkflowPackRegistry, WorkflowPackRuntime,
};
use canisend_resources::{
    ACADEMIC_JOB_WORKFLOW_PACK_ID, EmbeddedWorkflowPack, GENERIC_APPLICATION_WORKFLOW_PACK_ID,
    academic_job_workflow_pack, generic_application_workflow_pack, verify,
};

use crate::{ActionReceipt, Application, ApplicationError};

impl Application {
    pub fn application_pack_manifest_v4(
        workspace_root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<WorkflowPackManifest>, ApplicationError> {
        let stored = Self::application_model_v4(workspace_root, application_id)?.data;
        let binding = &stored.snapshot.pack;
        let registry = built_in_workflow_pack_registry()?;
        let pack = registry
            .resolve_exact(&binding.id, &binding.version, &binding.content_digest)
            .map_err(|error| {
                ApplicationError::ResourceIntegrity(format!(
                    "Application references an unavailable or substituted workflow Pack: {error}"
                ))
            })?;
        Ok(ActionReceipt::new(
            "application.pack.show",
            "current",
            "Loaded the complete verified Manifest for the Application's exact Pack binding",
            pack.manifest().clone(),
        ))
    }
}

pub fn built_in_academic_job_pack() -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    verify().map_err(ApplicationError::ResourceIntegrity)?;
    load_built_in_pack(ACADEMIC_JOB_WORKFLOW_PACK_ID, academic_job_workflow_pack())
}

pub fn built_in_generic_application_pack() -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    verify().map_err(ApplicationError::ResourceIntegrity)?;
    load_built_in_pack(
        GENERIC_APPLICATION_WORKFLOW_PACK_ID,
        generic_application_workflow_pack(),
    )
}

pub fn built_in_workflow_pack_registry() -> Result<WorkflowPackRegistry, ApplicationError> {
    verify().map_err(ApplicationError::ResourceIntegrity)?;
    let mut registry = WorkflowPackRegistry::new();
    for (expected_id, embedded) in [
        (ACADEMIC_JOB_WORKFLOW_PACK_ID, academic_job_workflow_pack()),
        (
            GENERIC_APPLICATION_WORKFLOW_PACK_ID,
            generic_application_workflow_pack(),
        ),
    ] {
        registry
            .insert(load_built_in_pack(expected_id, embedded)?)
            .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    }
    for pack in historical_academic_packs()? {
        registry
            .insert(pack)
            .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    }
    Ok(registry)
}

fn historical_academic_packs() -> Result<Vec<VerifiedWorkflowPackBundle>, ApplicationError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ArchivedPack {
        manifest: String,
        resources: std::collections::BTreeMap<canisend_contracts::SafeRelativePath, String>,
    }
    let history: Vec<ArchivedPack> =
        serde_json::from_slice(canisend_resources::ACADEMIC_JOB_WORKFLOW_PACK_HISTORY)
            .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    history
        .into_iter()
        .map(|pack| {
            let verified = WorkflowPackByteLoader::verify(
                pack.manifest.as_bytes(),
                pack.resources
                    .into_iter()
                    .map(|(path, body)| (path, body.into_bytes()))
                    .collect(),
                WorkflowPackOrigin::BuiltIn,
                &WorkflowPackRuntime::parse(
                    env!("CARGO_PKG_VERSION"),
                    "3.0.0-alpha.1",
                    "3.0.0-alpha.1",
                )
                .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?,
                &WorkflowPackCapabilityRegistry::built_in(),
            )
            .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
            let bundle = verified.into_bundle();
            if bundle.snapshot().id().as_str() != ACADEMIC_JOB_WORKFLOW_PACK_ID {
                return Err(ApplicationError::ResourceIntegrity(
                    "historical Pack identity differs".to_owned(),
                ));
            }
            Ok(bundle)
        })
        .collect()
}

fn load_built_in_pack(
    expected_id: &str,
    embedded: EmbeddedWorkflowPack,
) -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    if embedded.id() != expected_id {
        return Err(ApplicationError::ResourceIntegrity(format!(
            "embedded workflow Pack identity is inconsistent: expected {expected_id}"
        )));
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
        ApplicationFieldValueV3, ArtifactKind, DocumentKind, RequirementPriorityV3, Sha256Digest,
        WorkflowPackId, WorkflowPackItemId, WorkflowPackResourceKind, WorkflowPackStageOutput,
        WorkflowStage,
    };
    use canisend_core::{
        StageGraph, WorkflowPackDeliverableCatalogRuntime, WorkflowPackHostLocale,
        WorkflowPackLocalizationRuntime, WorkflowPackStageGraph,
        calculate_workflow_pack_content_digest,
    };

    use super::*;

    #[test]
    fn application_pack_manifest_returns_complete_exact_catalogs_without_mutation() {
        let root = std::env::temp_dir().join(format!(
            "canisend-application-pack-manifest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("current time")
                .as_nanos()
        ));
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        for (pack_id, required, count) in [
            (ACADEMIC_JOB_WORKFLOW_PACK_ID, vec!["cover-letter", "cv"], 4),
            (
                GENERIC_APPLICATION_WORKFLOW_PACK_ID,
                vec!["primary-document"],
                2,
            ),
        ] {
            let registry = built_in_workflow_pack_registry().expect("registry");
            let pack = if pack_id == ACADEMIC_JOB_WORKFLOW_PACK_ID {
                built_in_academic_job_pack()
            } else {
                built_in_generic_application_pack()
            }
            .expect("verified Pack");
            let source = "Provide supported material.";
            let created = Application::create_application_flow_v4(
                &root,
                crate::ApplicationFlowCreateRequestV4 {
                    pack_id: WorkflowPackId::try_new(pack_id).expect("Pack ID"),
                    application: crate::ApplicationFlowCreateRequestV3 {
                        title: "Synthetic catalog inspection".to_owned(),
                        opportunity_metadata: if pack_id == ACADEMIC_JOB_WORKFLOW_PACK_ID {
                            BTreeMap::from([(
                                WorkflowPackItemId::try_new("institution").expect("field ID"),
                                ApplicationFieldValueV3::ShortText("Example University".to_owned()),
                            )])
                        } else {
                            BTreeMap::new()
                        },
                        application_metadata: Default::default(),
                        source_text: source.to_owned(),
                        requirements: vec![crate::ApplicationFlowRequirementDraftV3 {
                            category: pack.manifest().requirements.categories[0].id.clone(),
                            statement: source.to_owned(),
                            priority: RequirementPriorityV3::Mandatory,
                            start_byte: 0,
                            end_byte: source.len() as u64,
                        }],
                    },
                },
            )
            .expect("create Application")
            .data
            .stored;
            let id = created.snapshot.application.id.as_str();
            let manifest = Application::application_pack_manifest_v4(&root, id)
                .expect("complete catalog")
                .data;
            assert_eq!(&manifest, pack.manifest());
            assert_eq!(manifest.deliverables.kinds.len(), count);
            assert_eq!(
                manifest
                    .deliverables
                    .kinds
                    .iter()
                    .filter(|kind| kind.minimum > 0)
                    .map(|kind| kind.id.as_str())
                    .collect::<Vec<_>>(),
                required
            );
            assert!(
                manifest
                    .deliverables
                    .kinds
                    .iter()
                    .all(|kind| kind.maximum >= kind.minimum)
            );
            assert_eq!(
                Application::application_model_v4(&root, id)
                    .expect("unchanged")
                    .data,
                created
            );
            let binding = &created.snapshot.pack;
            assert!(
                registry
                    .resolve_exact(
                        &binding.id,
                        &binding.version,
                        &Sha256Digest::try_new("0".repeat(64)).expect("digest"),
                    )
                    .is_err()
            );
        }
        std::fs::remove_dir_all(root).expect("remove fixture");
    }

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

    #[test]
    fn generic_pack_digest_matches_its_canonical_embedded_bundle() {
        let embedded = generic_application_workflow_pack();
        let manifest: WorkflowPackManifest =
            serde_json::from_slice(embedded.manifest_bytes()).expect("typed generic Pack Manifest");
        let actual = calculate_workflow_pack_content_digest(&manifest, embedded.resources())
            .expect("generic Pack digest");

        assert_eq!(manifest.content_digest, actual);
    }

    #[test]
    fn generic_pack_is_domain_neutral_configurable_and_bilingual() {
        let pack = built_in_generic_application_pack().expect("verified generic Pack");
        let manifest = pack.manifest();
        assert_eq!(
            pack.snapshot().id().as_str(),
            GENERIC_APPLICATION_WORKFLOW_PACK_ID
        );
        assert!(
            manifest
                .application
                .opportunity_fields
                .iter()
                .chain(&manifest.application.application_fields)
                .all(|field| !field.required)
        );
        assert_eq!(
            manifest
                .application
                .opportunity_fields
                .iter()
                .map(|field| field.id.as_str())
                .collect::<Vec<_>>(),
            vec!["organization", "reference", "deadline", "source-url"]
        );

        let graph = WorkflowPackStageGraph::from_verified_bundle(&pack)
            .expect("compiled generic stage graph");
        assert_eq!(graph.terminal_stage().local_id_str(), "render");
        assert_eq!(graph.descriptors().len(), 9);
        assert!(
            graph
                .descriptors()
                .iter()
                .any(|stage| stage.local_id().as_str() == "compose")
        );

        let deliverables = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(&pack)
            .expect("compiled generic Deliverable catalog");
        let deliverable_descriptors = deliverables.descriptors();
        assert_eq!(
            deliverable_descriptors
                .iter()
                .map(|item| item.local_id().as_str())
                .collect::<Vec<_>>(),
            vec!["primary-document", "supporting-document"]
        );
        assert_eq!(
            deliverables
                .required_kinds()
                .iter()
                .map(|kind| kind.local_id_str())
                .collect::<Vec<_>>(),
            vec!["primary-document"]
        );
        let academic_only_ids = [
            "institution",
            "qualification",
            "teaching",
            "research",
            "employment",
            "cover-letter",
            "research-statement",
            "teaching-statement",
            "cv",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let declared_ids = manifest
            .application
            .opportunity_fields
            .iter()
            .chain(&manifest.application.application_fields)
            .map(|field| field.id.as_str())
            .chain(
                manifest
                    .requirements
                    .categories
                    .iter()
                    .map(|category| category.id.as_str()),
            )
            .chain(
                manifest
                    .evidence
                    .categories
                    .iter()
                    .map(|category| category.id.as_str()),
            )
            .chain(
                deliverable_descriptors
                    .iter()
                    .map(|item| item.local_id().as_str()),
            )
            .collect::<BTreeSet<_>>();
        assert!(declared_ids.is_disjoint(&academic_only_ids));
        let serialized_manifest = serde_json::to_string(manifest)
            .expect("generic Manifest JSON")
            .to_ascii_lowercase();
        for academic_only_label in [
            "academic",
            "institution",
            "cover letter",
            "research statement",
            "teaching statement",
            "academic cv",
        ] {
            assert!(!serialized_manifest.contains(academic_only_label));
        }

        let localization = WorkflowPackLocalizationRuntime::from_verified_bundle(&pack)
            .expect("generic localization");
        let chinese = localization.select_host_locale(WorkflowPackHostLocale::SimplifiedChinese);
        assert_eq!(chinese.selected_locale().as_str(), "zh-Hans");
        assert_eq!(
            localization
                .vocabulary(&chinese)
                .expect("Chinese vocabulary")
                .application_singular,
            "申请"
        );
    }

    #[test]
    fn template_upgrade_reopens_an_application_with_its_original_pack() {
        let root =
            std::env::temp_dir().join(format!("canisend-template-upgrade-{}", std::process::id()));
        Application::initialize_workspace_v4(&root).expect("new isolated workspace");
        let old = historical_academic_packs().expect("history").remove(0);
        let created = {
            let mut workspace = crate::application::open_workspace_v4(&root).expect("workspace");
            canisend_store::ApplicationFlowServiceV3::new(
                &mut workspace.database,
                &workspace.blobs,
                &root,
            )
            .create(
                &old,
                crate::ApplicationFlowCreateRequestV3 {
                    title: "Synthetic pre-upgrade application".to_owned(),
                    opportunity_metadata: BTreeMap::from([(
                        WorkflowPackItemId::try_new("institution").expect("field"),
                        ApplicationFieldValueV3::ShortText("Example University".to_owned()),
                    )]),
                    application_metadata: Default::default(),
                    source_text: "Supported teaching.".to_owned(),
                    requirements: vec![crate::ApplicationFlowRequirementDraftV3 {
                        category: old.manifest().requirements.categories[0].id.clone(),
                        statement: "Supported teaching.".to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: 19,
                    }],
                },
            )
            .expect("seed original binary's Pack binding")
            .stored
        };
        let id = created.snapshot.application.id.as_str();
        assert_eq!(
            Application::application_pack_manifest_v4(&root, id)
                .expect("new binary resolves historical Pack")
                .data,
            *old.manifest()
        );
        assert_eq!(
            Application::application_model_v4(&root, id)
                .expect("reopen original application")
                .data,
            created
        );
        std::fs::remove_dir_all(root).expect("remove isolated fixture");
    }

    #[test]
    fn built_in_registry_resolves_academic_and_generic_packs_exactly() {
        let academic = built_in_academic_job_pack().expect("academic Pack");
        let generic = built_in_generic_application_pack().expect("generic Pack");
        let registry = built_in_workflow_pack_registry().expect("built-in registry");

        assert_eq!(
            registry.len(),
            2 + historical_academic_packs().expect("history").len()
        );
        for pack in [&academic, &generic] {
            assert!(registry.contains_exact(
                pack.snapshot().id(),
                pack.snapshot().version(),
                pack.snapshot().content_digest(),
            ));
        }
        let history = historical_academic_packs().expect("verified historical Packs");
        let old = history
            .iter()
            .find(|pack| pack.snapshot().version().as_str() == "1.0.0")
            .expect("pre-upgrade academic Pack remains bundled");
        assert_eq!(
            old.snapshot().content_digest().as_str(),
            "3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07"
        );
        for pack in &history {
            let resolved = registry
                .resolve_exact(
                    pack.snapshot().id(),
                    pack.snapshot().version(),
                    pack.snapshot().content_digest(),
                )
                .expect("old binding still resolves after upgrade");
            assert_eq!(resolved.resources(), pack.resources());
            assert!(
                registry
                    .resolve_exact(
                        pack.snapshot().id(),
                        pack.snapshot().version(),
                        academic.snapshot().content_digest()
                    )
                    .is_err(),
                "never substitute latest bytes for old binding"
            );
        }
        assert_ne!(academic.snapshot().version(), old.snapshot().version());
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
