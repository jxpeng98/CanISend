use canisend_contracts::{ArtifactReference, CompatibilityNotice, ConsentRequest, NextAction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionReceipt<T> {
    pub operation: String,
    pub status: String,
    pub summary: String,
    pub data: T,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_consents: Vec<ConsentRequest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_actions: Vec<NextAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<CompatibilityNotice>,
}

impl<T> ActionReceipt<T> {
    pub(crate) fn new(
        operation: &'static str,
        status: &'static str,
        summary: impl Into<String>,
        data: T,
    ) -> Self {
        Self {
            operation: operation.to_owned(),
            status: status.to_owned(),
            summary: summary.into(),
            data,
            artifacts: Vec::new(),
            required_consents: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
            compatibility: None,
        }
    }

    #[must_use]
    pub fn with_artifacts(
        mut self,
        artifacts: impl IntoIterator<Item = ArtifactReference>,
    ) -> Self {
        self.artifacts.extend(artifacts);
        self
    }

    #[must_use]
    pub fn with_required_consents(
        mut self,
        required_consents: impl IntoIterator<Item = ConsentRequest>,
    ) -> Self {
        self.required_consents.extend(required_consents);
        self
    }

    #[must_use]
    pub fn with_warnings(mut self, warnings: impl IntoIterator<Item = String>) -> Self {
        self.warnings.extend(warnings);
        self
    }

    #[must_use]
    pub fn with_next_actions(mut self, next_actions: impl IntoIterator<Item = NextAction>) -> Self {
        self.next_actions.extend(next_actions);
        self
    }

    #[must_use]
    pub fn with_compatibility(mut self, compatibility: CompatibilityNotice) -> Self {
        self.compatibility = Some(compatibility);
        self
    }
}

#[cfg(test)]
mod tests {
    use canisend_contracts::NextAction;
    use serde_json::json;

    use super::ActionReceipt;

    #[test]
    fn empty_adapter_metadata_preserves_the_original_receipt_shape() {
        let receipt = ActionReceipt::new("test.operation", "ok", "Completed", json!({"id": 1}));
        assert_eq!(
            serde_json::to_value(receipt).expect("serialize receipt"),
            json!({
                "operation": "test.operation",
                "status": "ok",
                "summary": "Completed",
                "data": {"id": 1}
            })
        );
    }

    #[test]
    fn adapter_metadata_is_deterministic_and_typed() {
        let receipt =
            ActionReceipt::new("test.operation", "ready", "Ready", ()).with_next_actions([
                NextAction {
                    action: "continue".to_owned(),
                    description: "Continue the bounded workflow".to_owned(),
                },
            ]);
        assert_eq!(receipt.next_actions.len(), 1);
        assert!(receipt.artifacts.is_empty());
        assert!(receipt.required_consents.is_empty());
        assert!(receipt.warnings.is_empty());
        assert!(receipt.compatibility.is_none());
    }
}
