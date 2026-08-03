use canisend_contracts::{
    ApplicationPackBindingV3, DeliverableKindId, ExecutionMode, StageId, WorkflowPackFieldType,
    WorkflowPackItemId, WorkflowPackLocaleId, WorkflowPackStageOutput, WorkflowPackVocabulary,
};
use canisend_core::{
    ResolvedWorkflowPackText, VerifiedWorkflowPackBundle, WorkflowPackDeliverableCatalogRuntime,
    WorkflowPackHostLocale, WorkflowPackLocaleMatch, WorkflowPackLocalizationRuntime,
    WorkflowPackStageGraph,
};
use canisend_resources::{ACADEMIC_JOB_WORKFLOW_PACK_ID, GENERIC_APPLICATION_WORKFLOW_PACK_ID};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, built_in_academic_job_pack,
    built_in_generic_application_pack,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowPackPresentationLocale {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackPresentationLocaleMatch {
    Exact,
    Compatible,
    PackDefault,
}

impl From<WorkflowPackLocaleMatch> for WorkflowPackPresentationLocaleMatch {
    fn from(value: WorkflowPackLocaleMatch) -> Self {
        match value {
            WorkflowPackLocaleMatch::Exact => Self::Exact,
            WorkflowPackLocaleMatch::Compatible => Self::Compatible,
            WorkflowPackLocaleMatch::PackDefault => Self::PackDefault,
        }
    }
}

impl WorkflowPackPresentationLocale {
    const fn host(self) -> WorkflowPackHostLocale {
        match self {
            Self::English => WorkflowPackHostLocale::English,
            Self::SimplifiedChinese => WorkflowPackHostLocale::SimplifiedChinese,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPresentationLabel {
    pub value: String,
    pub locale: WorkflowPackLocaleId,
    pub used_default_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPresentationFieldOption {
    pub id: WorkflowPackItemId,
    pub label: WorkflowPackPresentationLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPresentationField {
    pub id: WorkflowPackItemId,
    pub label: WorkflowPackPresentationLabel,
    pub field_type: WorkflowPackFieldType,
    pub required: bool,
    pub options: Vec<WorkflowPackPresentationFieldOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPresentationCategory {
    pub id: WorkflowPackItemId,
    pub label: WorkflowPackPresentationLabel,
    pub fields: Vec<WorkflowPackPresentationField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPresentationStage {
    pub id: WorkflowPackItemId,
    pub qualified_id: StageId,
    pub label: WorkflowPackPresentationLabel,
    pub depends_on: Vec<StageId>,
    pub output: WorkflowPackStageOutput,
    pub execution_modes: Vec<ExecutionMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPresentationDeliverable {
    pub id: WorkflowPackItemId,
    pub qualified_id: DeliverableKindId,
    pub label: WorkflowPackPresentationLabel,
    pub minimum: u16,
    pub maximum: u16,
    pub legacy_task_operation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPresentationReadModel {
    pub pack: ApplicationPackBindingV3,
    pub requested_locale: WorkflowPackLocaleId,
    pub selected_locale: WorkflowPackLocaleId,
    pub locale_match: WorkflowPackPresentationLocaleMatch,
    pub vocabulary: WorkflowPackVocabulary,
    pub opportunity_fields: Vec<WorkflowPackPresentationField>,
    pub application_fields: Vec<WorkflowPackPresentationField>,
    pub requirement_categories: Vec<WorkflowPackPresentationCategory>,
    pub evidence_categories: Vec<WorkflowPackPresentationCategory>,
    pub stages: Vec<WorkflowPackPresentationStage>,
    pub deliverables: Vec<WorkflowPackPresentationDeliverable>,
}

impl Application {
    pub fn built_in_pack_presentation(
        pack_id: &str,
        locale: WorkflowPackPresentationLocale,
    ) -> Result<ActionReceipt<WorkflowPackPresentationReadModel>, ApplicationError> {
        let pack = match pack_id {
            ACADEMIC_JOB_WORKFLOW_PACK_ID => built_in_academic_job_pack()?,
            GENERIC_APPLICATION_WORKFLOW_PACK_ID => built_in_generic_application_pack()?,
            _ => {
                return Err(ApplicationError::InvalidInput(format!(
                    "unknown built-in workflow Pack: {pack_id}"
                )));
            }
        };
        pack_presentation(&pack, locale)
    }

    pub fn built_in_academic_pack_presentation(
        locale: WorkflowPackPresentationLocale,
    ) -> Result<ActionReceipt<WorkflowPackPresentationReadModel>, ApplicationError> {
        Self::built_in_pack_presentation(ACADEMIC_JOB_WORKFLOW_PACK_ID, locale)
    }

    pub fn built_in_generic_pack_presentation(
        locale: WorkflowPackPresentationLocale,
    ) -> Result<ActionReceipt<WorkflowPackPresentationReadModel>, ApplicationError> {
        Self::built_in_pack_presentation(GENERIC_APPLICATION_WORKFLOW_PACK_ID, locale)
    }
}

fn pack_presentation(
    pack: &VerifiedWorkflowPackBundle,
    locale: WorkflowPackPresentationLocale,
) -> Result<ActionReceipt<WorkflowPackPresentationReadModel>, ApplicationError> {
    let localization = WorkflowPackLocalizationRuntime::from_verified_bundle(pack)
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    let selection = localization.select_host_locale(locale.host());
    let vocabulary = localization
        .vocabulary(&selection)
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?
        .clone();
    let stage_graph = WorkflowPackStageGraph::from_verified_bundle(pack)
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    let deliverable_catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(pack)
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    let manifest = pack.manifest();
    let fields = |definitions: &[canisend_contracts::WorkflowPackFieldDefinition]| {
        definitions
            .iter()
            .map(|definition| presentation_field(&localization, &selection, definition))
            .collect::<Result<Vec<_>, ApplicationError>>()
    };
    let categories = |definitions: &[canisend_contracts::WorkflowPackCategoryDefinition]| {
        definitions
            .iter()
            .map(|definition| {
                Ok(WorkflowPackPresentationCategory {
                    id: definition.id.clone(),
                    label: presentation_label(&localization, &selection, &definition.labels)?,
                    fields: fields(&definition.fields)?,
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()
    };
    let stages = stage_graph
        .descriptors()
        .into_iter()
        .map(|descriptor| {
            Ok(WorkflowPackPresentationStage {
                id: descriptor.local_id().clone(),
                qualified_id: descriptor.stage().clone(),
                label: presentation_label(&localization, &selection, descriptor.labels())?,
                depends_on: descriptor.depends_on().to_vec(),
                output: descriptor.output(),
                execution_modes: descriptor.execution_modes().to_vec(),
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    let deliverables = deliverable_catalog
        .descriptors()
        .into_iter()
        .map(|descriptor| {
            Ok(WorkflowPackPresentationDeliverable {
                id: descriptor.local_id().clone(),
                qualified_id: descriptor.kind().clone(),
                label: presentation_label(&localization, &selection, descriptor.labels())?,
                minimum: descriptor.minimum(),
                maximum: descriptor.maximum(),
                legacy_task_operation: academic_legacy_task_operation(
                    descriptor.local_id().as_str(),
                )
                .map(ToOwned::to_owned),
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    let data = WorkflowPackPresentationReadModel {
        pack: ApplicationPackBindingV3 {
            id: pack.snapshot().id().clone(),
            version: pack.snapshot().version().clone(),
            content_digest: pack.snapshot().content_digest().clone(),
        },
        requested_locale: selection.requested_locale().clone(),
        selected_locale: selection.selected_locale().clone(),
        locale_match: selection.match_kind().into(),
        vocabulary,
        opportunity_fields: fields(&manifest.application.opportunity_fields)?,
        application_fields: fields(&manifest.application.application_fields)?,
        requirement_categories: categories(&manifest.requirements.categories)?,
        evidence_categories: categories(&manifest.evidence.categories)?,
        stages,
        deliverables,
    };
    Ok(ActionReceipt::new(
        "workflow-pack.presentation",
        "available",
        format!(
            "Resolved {} presentation labels for {}",
            data.selected_locale, data.pack.id
        ),
        data,
    ))
}

fn presentation_field(
    localization: &WorkflowPackLocalizationRuntime,
    selection: &canisend_core::WorkflowPackLocaleSelection,
    definition: &canisend_contracts::WorkflowPackFieldDefinition,
) -> Result<WorkflowPackPresentationField, ApplicationError> {
    Ok(WorkflowPackPresentationField {
        id: definition.id.clone(),
        label: presentation_label(localization, selection, &definition.labels)?,
        field_type: definition.field_type,
        required: definition.required,
        options: definition
            .options
            .iter()
            .map(|option| {
                Ok(WorkflowPackPresentationFieldOption {
                    id: option.id.clone(),
                    label: presentation_label(localization, selection, &option.labels)?,
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?,
    })
}

fn presentation_label(
    localization: &WorkflowPackLocalizationRuntime,
    selection: &canisend_core::WorkflowPackLocaleSelection,
    text: &canisend_contracts::WorkflowPackLocalizedText,
) -> Result<WorkflowPackPresentationLabel, ApplicationError> {
    let resolved = localization
        .resolve_text(selection, text)
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    Ok(label_from_resolved(resolved, selection.selected_locale()))
}

fn label_from_resolved(
    resolved: ResolvedWorkflowPackText<'_>,
    selected_locale: &WorkflowPackLocaleId,
) -> WorkflowPackPresentationLabel {
    WorkflowPackPresentationLabel {
        value: resolved.value().to_owned(),
        locale: resolved.locale().clone(),
        used_default_fallback: resolved.locale() != selected_locale,
    }
}

fn academic_legacy_task_operation(deliverable_id: &str) -> Option<&'static str> {
    match deliverable_id {
        "cover-letter" => Some("cover-letter-draft"),
        "research-statement" => Some("research-statement-draft"),
        "teaching-statement" => Some("teaching-statement-draft"),
        "cv" => Some("cv-draft"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn academic_presentation_is_pack_bound_ordered_and_bilingual() {
        let english = Application::built_in_academic_pack_presentation(
            WorkflowPackPresentationLocale::English,
        )
        .expect("English presentation")
        .data;
        let chinese = Application::built_in_academic_pack_presentation(
            WorkflowPackPresentationLocale::SimplifiedChinese,
        )
        .expect("Chinese presentation")
        .data;

        assert_eq!(english.pack.id.as_str(), "org.canisend.academic-job");
        assert_eq!(english.selected_locale.as_str(), "en");
        assert_eq!(
            english.locale_match,
            WorkflowPackPresentationLocaleMatch::Exact
        );
        assert_eq!(chinese.requested_locale.as_str(), "zh-CN");
        assert_eq!(chinese.selected_locale.as_str(), "zh-Hans");
        assert_eq!(
            chinese.locale_match,
            WorkflowPackPresentationLocaleMatch::Compatible
        );
        assert_eq!(english.stages.len(), 10);
        let english_intake = english
            .stages
            .iter()
            .find(|stage| stage.id.as_str() == "intake")
            .expect("English intake stage");
        let chinese_intake = chinese
            .stages
            .iter()
            .find(|stage| stage.id.as_str() == "intake")
            .expect("Chinese intake stage");
        assert_eq!(english_intake.label.value, "Intake");
        assert_eq!(chinese_intake.label.value, "导入");
        assert_eq!(english.deliverables.len(), 4);
        assert_eq!(english.deliverables[3].id.as_str(), "cv");
        assert_eq!(english.deliverables[3].label.value, "Academic CV");
        assert_eq!(chinese.deliverables[3].label.value, "学术简历");
        assert!(
            english
                .deliverables
                .iter()
                .all(|item| item.qualified_id.pack_id_str() == english.pack.id.as_str())
        );
        assert!(
            english
                .stages
                .iter()
                .all(|item| item.qualified_id.pack_id_str() == english.pack.id.as_str())
        );
        let chinese_json = serde_json::to_value(&chinese).expect("presentation JSON");
        assert_eq!(chinese_json["requested_locale"], "zh-CN");
        assert_eq!(chinese_json["selected_locale"], "zh-Hans");
        assert_eq!(chinese_json["locale_match"], "compatible");
    }

    #[test]
    fn presentation_exposes_pack_owned_form_fields_and_vocabularies() {
        let presentation = Application::built_in_academic_pack_presentation(
            WorkflowPackPresentationLocale::SimplifiedChinese,
        )
        .expect("presentation")
        .data;
        assert_eq!(presentation.vocabulary.application_singular, "学术职位申请");
        assert_eq!(presentation.opportunity_fields.len(), 1);
        assert_eq!(
            presentation.opportunity_fields[0].id.as_str(),
            "institution"
        );
        assert_eq!(presentation.opportunity_fields[0].label.value, "机构");
        assert!(presentation.opportunity_fields[0].required);
        assert_eq!(presentation.requirement_categories.len(), 8);
        assert_eq!(presentation.evidence_categories.len(), 8);
    }

    #[test]
    fn generic_presentation_uses_the_same_verified_pack_driven_model() {
        let presentation = Application::built_in_generic_pack_presentation(
            WorkflowPackPresentationLocale::SimplifiedChinese,
        )
        .expect("generic presentation")
        .data;
        assert_eq!(
            presentation.pack.id.as_str(),
            GENERIC_APPLICATION_WORKFLOW_PACK_ID
        );
        assert_eq!(presentation.stages.len(), 9);
        assert_eq!(presentation.deliverables.len(), 2);
        assert_eq!(presentation.opportunity_fields.len(), 4);
        assert_eq!(presentation.application_fields.len(), 3);
        assert!(
            presentation
                .stages
                .iter()
                .all(|stage| stage.qualified_id.pack_id_str()
                    == GENERIC_APPLICATION_WORKFLOW_PACK_ID)
        );
    }
}
