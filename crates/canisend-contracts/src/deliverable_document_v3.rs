use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ContentRevisionReferenceV3, RequirementRevisionReferenceV3, WorkflowPackItemId};

pub const DELIVERABLE_DOCUMENT_MEDIA_TYPE_V3: &str = "application/vnd.canisend.deliverable+json";
pub const MAX_DELIVERABLE_DOCUMENT_BYTES_V3: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum DeliverableDocumentFormatV3 {
    #[serde(rename = "canisend.deliverable-document/v3")]
    V3,
}

/// The reviewer checks this classification; the kernel verifies its declared citations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DeliverableTextRoleV3 {
    EvidenceBound,
    RequirementBound,
    Intent,
    NonFactual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableTextV3 {
    pub text: String,
    pub role: DeliverableTextRoleV3,
    pub evidence: Vec<ContentRevisionReferenceV3>,
    pub requirements: Vec<RequirementRevisionReferenceV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableFieldV3 {
    pub key: WorkflowPackItemId,
    pub value: DeliverableTextV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableSectionV3 {
    pub id: WorkflowPackItemId,
    pub heading: Option<String>,
    pub body: DeliverableTextV3,
}

/// Immutable data, never Typst or Markdown code. Each field and section body has an explicit
/// provenance classification. Missing facts can be drafted but cannot pass readiness validators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableDocumentV3 {
    pub format: DeliverableDocumentFormatV3,
    pub fields: Vec<DeliverableFieldV3>,
    pub sections: Vec<DeliverableSectionV3>,
    pub unresolved_fields: Vec<WorkflowPackItemId>,
}

impl DeliverableDocumentV3 {
    /// Bound input before allocation and reject invalid structure without exposing private text.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.is_empty() || bytes.len() > MAX_DELIVERABLE_DOCUMENT_BYTES_V3 {
            return Err("structured Deliverable exceeds its byte limit");
        }
        let document: Self = serde_json::from_slice(bytes)
            .map_err(|_| "structured Deliverable does not match its JSON contract")?;
        document.validate()?;
        Ok(document)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.fields.len() > 64
            || self.sections.is_empty()
            || self.sections.len() > 128
            || self.unresolved_fields.len() > 64
        {
            return Err("structured Deliverable has an invalid field or section count");
        }
        let mut keys = BTreeSet::new();
        for field in &self.fields {
            if !keys.insert(&field.key) {
                return Err("structured Deliverable has duplicate field keys");
            }
        }
        let mut ids = BTreeSet::new();
        for section in &self.sections {
            if !ids.insert(&section.id)
                || section
                    .heading
                    .as_ref()
                    .is_some_and(|heading| !valid_text(heading, 512))
            {
                return Err("structured Deliverable has invalid section metadata");
            }
        }
        let mut total = 0;
        for value in self.texts() {
            if !valid_text(&value.text, 64 * 1024)
                || value.evidence.len() > 32
                || value.requirements.len() > 32
            {
                return Err("structured Deliverable has invalid text or citation bounds");
            }
            total += value.text.len();
        }
        if total > 256 * 1024 {
            return Err("structured Deliverable exceeds its total text limit");
        }
        Ok(())
    }

    pub fn texts(&self) -> impl Iterator<Item = &DeliverableTextV3> {
        self.fields
            .iter()
            .map(|field| &field.value)
            .chain(self.sections.iter().map(|section| &section.body))
    }
}

fn valid_text(text: &str, maximum: usize) -> bool {
    !text.trim().is_empty()
        && text.len() <= maximum
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_structure_duplicate_keys_and_unbounded_text_without_echoing_content() {
        let valid = br#"{"format":"canisend.deliverable-document/v3","fields":[],"sections":[{"id":"body","heading":null,"body":{"text":"Private sentinel","role":"non-factual","evidence":[],"requirements":[]}}],"unresolved_fields":[]}"#;
        let mut document = DeliverableDocumentV3::decode(valid).expect("bounded document");
        document.sections.push(document.sections[0].clone());
        assert!(document.validate().is_err());
        document.sections.pop();
        document.sections[0].body.text = "x".repeat(64 * 1024 + 1);
        assert!(document.validate().is_err());
        assert!(
            !DeliverableDocumentV3::decode(br#"{"private sentinel":true}"#)
                .expect_err("unknown shape")
                .contains("sentinel")
        );
    }
}
