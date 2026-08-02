use std::collections::BTreeMap;

use canisend_contracts::{
    SemanticVersion, Sha256Digest, WORKFLOW_PACK_MAX_LOCALES, WorkflowPackId, WorkflowPackLocaleId,
    WorkflowPackLocalizedText, WorkflowPackVocabulary,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::VerifiedWorkflowPackBundle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowPackHostLocale {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

impl WorkflowPackHostLocale {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh-CN",
        }
    }

    const fn compatible_pack_locales(self) -> &'static [&'static str] {
        match self {
            Self::English => &["en"],
            Self::SimplifiedChinese => &["zh-CN", "zh-Hans", "zh"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackLocaleMatch {
    Exact,
    Compatible,
    PackDefault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackLocaleSelection {
    pack_id: WorkflowPackId,
    pack_version: SemanticVersion,
    content_digest: Sha256Digest,
    requested_locale: WorkflowPackLocaleId,
    selected_locale: WorkflowPackLocaleId,
    match_kind: WorkflowPackLocaleMatch,
}

impl WorkflowPackLocaleSelection {
    #[must_use]
    pub const fn pack_id(&self) -> &WorkflowPackId {
        &self.pack_id
    }

    #[must_use]
    pub const fn pack_version(&self) -> &SemanticVersion {
        &self.pack_version
    }

    #[must_use]
    pub const fn content_digest(&self) -> &Sha256Digest {
        &self.content_digest
    }

    #[must_use]
    pub const fn requested_locale(&self) -> &WorkflowPackLocaleId {
        &self.requested_locale
    }

    #[must_use]
    pub const fn selected_locale(&self) -> &WorkflowPackLocaleId {
        &self.selected_locale
    }

    #[must_use]
    pub const fn match_kind(&self) -> WorkflowPackLocaleMatch {
        self.match_kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowPackTextMatch {
    SelectedLocale,
    PackDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedWorkflowPackText<'a> {
    locale: &'a WorkflowPackLocaleId,
    value: &'a str,
    match_kind: WorkflowPackTextMatch,
}

impl<'a> ResolvedWorkflowPackText<'a> {
    #[must_use]
    pub const fn locale(&self) -> &WorkflowPackLocaleId {
        self.locale
    }

    #[must_use]
    pub const fn value(&self) -> &str {
        self.value
    }

    #[must_use]
    pub const fn match_kind(&self) -> WorkflowPackTextMatch {
        self.match_kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPackLocalizationRuntime {
    pack_id: WorkflowPackId,
    pack_version: SemanticVersion,
    content_digest: Sha256Digest,
    default_locale: WorkflowPackLocaleId,
    vocabularies: BTreeMap<WorkflowPackLocaleId, WorkflowPackVocabulary>,
}

impl WorkflowPackLocalizationRuntime {
    pub(crate) fn try_new(
        pack_id: WorkflowPackId,
        pack_version: SemanticVersion,
        content_digest: Sha256Digest,
        default_locale: WorkflowPackLocaleId,
        vocabularies: BTreeMap<WorkflowPackLocaleId, WorkflowPackVocabulary>,
    ) -> Result<Self, WorkflowPackLocalizationError> {
        if vocabularies.is_empty() || vocabularies.len() > WORKFLOW_PACK_MAX_LOCALES {
            return Err(WorkflowPackLocalizationError::LocaleCountInvalid {
                minimum: 1,
                maximum: WORKFLOW_PACK_MAX_LOCALES,
                actual: vocabularies.len(),
            });
        }
        if !vocabularies.contains_key(&default_locale) {
            return Err(WorkflowPackLocalizationError::DefaultLocaleMissing {
                pack_id: pack_id.clone(),
                locale: default_locale,
            });
        }
        Ok(Self {
            pack_id,
            pack_version,
            content_digest,
            default_locale,
            vocabularies,
        })
    }

    pub fn from_verified_bundle(
        bundle: &VerifiedWorkflowPackBundle,
    ) -> Result<Self, WorkflowPackLocalizationError> {
        let manifest = bundle.manifest();
        Self::try_new(
            manifest.id.clone(),
            manifest.version.clone(),
            manifest.content_digest.clone(),
            manifest.default_locale.clone(),
            manifest.locales.clone(),
        )
    }

    #[must_use]
    pub const fn pack_id(&self) -> &WorkflowPackId {
        &self.pack_id
    }

    #[must_use]
    pub const fn pack_version(&self) -> &SemanticVersion {
        &self.pack_version
    }

    #[must_use]
    pub const fn content_digest(&self) -> &Sha256Digest {
        &self.content_digest
    }

    #[must_use]
    pub const fn default_locale(&self) -> &WorkflowPackLocaleId {
        &self.default_locale
    }

    pub fn supported_locales(&self) -> impl Iterator<Item = &WorkflowPackLocaleId> {
        self.vocabularies.keys()
    }

    #[must_use]
    pub fn select_host_locale(
        &self,
        requested: WorkflowPackHostLocale,
    ) -> WorkflowPackLocaleSelection {
        let requested_locale = WorkflowPackLocaleId::try_new(requested.code())
            .expect("built-in host locale codes satisfy the workflow-pack locale contract");
        for (index, candidate) in requested.compatible_pack_locales().iter().enumerate() {
            if let Some(selected_locale) = self.locale_by_code(candidate) {
                return WorkflowPackLocaleSelection {
                    pack_id: self.pack_id.clone(),
                    pack_version: self.pack_version.clone(),
                    content_digest: self.content_digest.clone(),
                    requested_locale,
                    selected_locale: selected_locale.clone(),
                    match_kind: if index == 0 {
                        WorkflowPackLocaleMatch::Exact
                    } else {
                        WorkflowPackLocaleMatch::Compatible
                    },
                };
            }
        }
        self.default_selection(requested_locale)
    }

    #[must_use]
    pub fn select_locale(&self, requested: &WorkflowPackLocaleId) -> WorkflowPackLocaleSelection {
        if let Some(selected_locale) = self.vocabularies.get_key_value(requested).map(|(id, _)| id)
        {
            return WorkflowPackLocaleSelection {
                pack_id: self.pack_id.clone(),
                pack_version: self.pack_version.clone(),
                content_digest: self.content_digest.clone(),
                requested_locale: requested.clone(),
                selected_locale: selected_locale.clone(),
                match_kind: WorkflowPackLocaleMatch::Exact,
            };
        }
        if let Some((language, _)) = requested.as_str().split_once('-')
            && let Some(selected_locale) = self.locale_by_code(language)
        {
            return WorkflowPackLocaleSelection {
                pack_id: self.pack_id.clone(),
                pack_version: self.pack_version.clone(),
                content_digest: self.content_digest.clone(),
                requested_locale: requested.clone(),
                selected_locale: selected_locale.clone(),
                match_kind: WorkflowPackLocaleMatch::Compatible,
            };
        }
        self.default_selection(requested.clone())
    }

    pub fn vocabulary(
        &self,
        selection: &WorkflowPackLocaleSelection,
    ) -> Result<&WorkflowPackVocabulary, WorkflowPackLocalizationError> {
        self.validate_selection(selection)?;
        self.vocabularies
            .get(&selection.selected_locale)
            .ok_or_else(|| WorkflowPackLocalizationError::SelectedLocaleMissing {
                pack_id: self.pack_id.clone(),
                locale: selection.selected_locale.clone(),
            })
    }

    pub fn resolve_text<'a>(
        &'a self,
        selection: &WorkflowPackLocaleSelection,
        text: &'a WorkflowPackLocalizedText,
    ) -> Result<ResolvedWorkflowPackText<'a>, WorkflowPackLocalizationError> {
        self.validate_selection(selection)?;
        if let Some((locale, value)) = text.0.get_key_value(&selection.selected_locale) {
            return Ok(ResolvedWorkflowPackText {
                locale,
                value,
                match_kind: WorkflowPackTextMatch::SelectedLocale,
            });
        }
        let (locale, value) = text.0.get_key_value(&self.default_locale).ok_or_else(|| {
            WorkflowPackLocalizationError::LocalizedTextDefaultMissing {
                pack_id: self.pack_id.clone(),
                locale: self.default_locale.clone(),
            }
        })?;
        Ok(ResolvedWorkflowPackText {
            locale,
            value,
            match_kind: WorkflowPackTextMatch::PackDefault,
        })
    }

    fn default_selection(
        &self,
        requested_locale: WorkflowPackLocaleId,
    ) -> WorkflowPackLocaleSelection {
        WorkflowPackLocaleSelection {
            pack_id: self.pack_id.clone(),
            pack_version: self.pack_version.clone(),
            content_digest: self.content_digest.clone(),
            requested_locale,
            selected_locale: self.default_locale.clone(),
            match_kind: WorkflowPackLocaleMatch::PackDefault,
        }
    }

    fn locale_by_code(&self, code: &str) -> Option<&WorkflowPackLocaleId> {
        self.vocabularies
            .keys()
            .find(|locale| locale.as_str() == code)
    }

    fn validate_selection(
        &self,
        selection: &WorkflowPackLocaleSelection,
    ) -> Result<(), WorkflowPackLocalizationError> {
        let mismatch = if selection.pack_id != self.pack_id {
            Some(WorkflowPackSelectionBindingMismatch::PackId)
        } else if selection.pack_version != self.pack_version {
            Some(WorkflowPackSelectionBindingMismatch::PackVersion)
        } else if selection.content_digest != self.content_digest {
            Some(WorkflowPackSelectionBindingMismatch::ContentDigest)
        } else {
            None
        };
        if let Some(mismatch) = mismatch {
            return Err(WorkflowPackLocalizationError::SelectionBindingMismatch {
                expected_pack_id: self.pack_id.clone(),
                actual_pack_id: selection.pack_id.clone(),
                mismatch,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WorkflowPackSelectionBindingMismatch {
    #[error("pack ID")]
    PackId,
    #[error("pack version")]
    PackVersion,
    #[error("content digest")]
    ContentDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowPackLocalizationError {
    #[error("workflow pack locale count must be between {minimum} and {maximum}; found {actual}")]
    LocaleCountInvalid {
        minimum: usize,
        maximum: usize,
        actual: usize,
    },
    #[error("workflow pack {pack_id} default locale {locale} is missing")]
    DefaultLocaleMissing {
        pack_id: WorkflowPackId,
        locale: WorkflowPackLocaleId,
    },
    #[error(
        "workflow pack locale selection for {actual_pack_id} does not match {expected_pack_id}: {mismatch} differs"
    )]
    SelectionBindingMismatch {
        expected_pack_id: WorkflowPackId,
        actual_pack_id: WorkflowPackId,
        mismatch: WorkflowPackSelectionBindingMismatch,
    },
    #[error("workflow pack {pack_id} selected locale {locale} is unavailable")]
    SelectedLocaleMissing {
        pack_id: WorkflowPackId,
        locale: WorkflowPackLocaleId,
    },
    #[error("workflow pack {pack_id} localized text is missing default locale {locale}")]
    LocalizedTextDefaultMissing {
        pack_id: WorkflowPackId,
        locale: WorkflowPackLocaleId,
    },
}
