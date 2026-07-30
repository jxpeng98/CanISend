use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, EntityId, NextAction, PrivacyClassification,
    UtcTimestamp, WorkflowStage,
};
use canisend_store::{
    CatalogArtifactMetadata, CatalogSourceRole, CatalogSourceScope, ContentCatalogService,
    DEFAULT_MAX_BLOB_BYTES,
};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
};

const DEFAULT_SEARCH_LIMIT: usize = 50;
const MAX_SEARCH_LIMIT: usize = 100;
const MAX_QUERY_CHARS: usize = 200;
const MAX_PRIVATE_INDEX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SINGLE_INDEX_BODY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SNIPPET_CHARS: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentCategory {
    Source,
    Profile,
    JobAnalysis,
    Evidence,
    Planning,
    Materials,
    Review,
    Delivery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentCatalogStatus {
    Imported,
    Proposed,
    Confirmed,
    Generated,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentSourceScope {
    Job,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentSourceRole {
    Original,
    Normalized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSubjectJobReadModel {
    pub id: EntityId,
    pub title: String,
    pub institution: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentProvenanceReadModel {
    pub actor: ActorKind,
    pub reason: String,
    pub source_id: Option<EntityId>,
    pub source_scope: Option<ContentSourceScope>,
    pub source_role: Option<ContentSourceRole>,
    pub source_kind: Option<String>,
    pub content_type: Option<String>,
    pub locator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentCatalogEntryReadModel {
    pub artifact: ArtifactReference,
    pub title: String,
    pub category: ContentCategory,
    pub stage: WorkflowStage,
    pub status: ContentCatalogStatus,
    pub privacy: PrivacyClassification,
    pub size: u64,
    pub created_at: UtcTimestamp,
    pub provenance: ContentProvenanceReadModel,
    pub subject_jobs: Vec<ContentSubjectJobReadModel>,
    pub relationships: Vec<ArtifactReference>,
    pub private_body_searchable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentCatalogFilter {
    pub job_id: Option<String>,
    pub category: Option<ContentCategory>,
    pub stage: Option<WorkflowStage>,
    pub status: Option<ContentCatalogStatus>,
    pub privacy: Option<PrivacyClassification>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentCatalogReadModel {
    pub workspace: PathBuf,
    pub total_entries: u64,
    pub entries: Vec<ContentCatalogEntryReadModel>,
    pub filter: ContentCatalogFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSearchRequest {
    pub query: String,
    #[serde(default)]
    pub filter: ContentCatalogFilter,
    #[serde(default)]
    pub include_private_bodies: bool,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

impl ContentSearchRequest {
    #[must_use]
    pub fn metadata(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            filter: ContentCatalogFilter::default(),
            include_private_bodies: false,
            limit: DEFAULT_SEARCH_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentMatchField {
    Metadata,
    PrivateBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSearchResultReadModel {
    pub entry: ContentCatalogEntryReadModel,
    pub score: u64,
    pub matched_fields: Vec<ContentMatchField>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentIndexReadModel {
    pub strategy: String,
    pub metadata_entries: u64,
    pub private_body_entries: u64,
    pub private_body_bytes: u64,
    pub skipped_oversized_entries: u64,
    pub skipped_secret_entries: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSearchReadModel {
    pub workspace: PathBuf,
    pub query: String,
    pub include_private_bodies: bool,
    pub total_matches: u64,
    pub results: Vec<ContentSearchResultReadModel>,
    pub index: ContentIndexReadModel,
    pub filter: ContentCatalogFilter,
}

impl Application {
    pub fn content_catalog(
        root: &Path,
        filter: ContentCatalogFilter,
    ) -> Result<ActionReceipt<ContentCatalogReadModel>, ApplicationError> {
        let validated_filter = validate_filter(&filter)?;
        let workspace = open_workspace(root)?;
        let entries = project_catalog(ContentCatalogService::new(&workspace.database).list()?);
        let entries = entries
            .into_iter()
            .filter(|entry| filter_matches(entry, &filter, &validated_filter))
            .collect::<Vec<_>>();
        let total_entries = count(entries.len(), "content catalog entry count")?;
        Ok(ActionReceipt::new(
            "content.catalog.list",
            if entries.is_empty() {
                "empty"
            } else {
                "available"
            },
            format!("Loaded {total_entries} body-free content catalog item(s)"),
            ContentCatalogReadModel {
                workspace: workspace.paths.root,
                total_entries,
                entries,
                filter,
            },
        ))
    }

    pub fn search_content(
        root: &Path,
        mut request: ContentSearchRequest,
        private_read_consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<ContentSearchReadModel>, ApplicationError> {
        validate_search_request(&mut request)?;
        let validated_filter = validate_filter(&request.filter)?;
        if request.include_private_bodies && private_read_consent.is_none() {
            return Err(private_search_consent_required());
        }
        let workspace = open_workspace(root)?;
        let entries = project_catalog(ContentCatalogService::new(&workspace.database).list()?)
            .into_iter()
            .filter(|entry| filter_matches(entry, &request.filter, &validated_filter))
            .collect::<Vec<_>>();
        let mut index = InMemoryContentIndex::new(entries);
        let warnings = if request.include_private_bodies {
            index.index_private_bodies(&workspace.blobs)?
        } else {
            Vec::new()
        };
        let (total_matches, results) = index.search(&request.query, request.limit)?;
        let status = if results.is_empty() {
            "empty"
        } else {
            "available"
        };
        Ok(ActionReceipt::new(
            "content.search",
            status,
            format!("Found {total_matches} matching content item(s)"),
            ContentSearchReadModel {
                workspace: workspace.paths.root,
                query: request.query,
                include_private_bodies: request.include_private_bodies,
                total_matches,
                results,
                index: index.stats,
                filter: request.filter,
            },
        )
        .with_warnings(warnings))
    }
}

fn project_catalog(entries: Vec<CatalogArtifactMetadata>) -> Vec<ContentCatalogEntryReadModel> {
    entries.into_iter().map(project_entry).collect()
}

fn project_entry(entry: CatalogArtifactMetadata) -> ContentCatalogEntryReadModel {
    let source_scope = entry.source.as_ref().map(|source| match source.scope {
        CatalogSourceScope::Job => ContentSourceScope::Job,
        CatalogSourceScope::Profile => ContentSourceScope::Profile,
    });
    let source_role = entry.source.as_ref().map(|source| match source.role {
        CatalogSourceRole::Original => ContentSourceRole::Original,
        CatalogSourceRole::Normalized => ContentSourceRole::Normalized,
    });
    let category = content_category(entry.artifact.kind, source_scope);
    let status = content_status(&entry, category);
    ContentCatalogEntryReadModel {
        title: content_title(entry.artifact.kind, source_scope, source_role),
        category,
        stage: content_stage(entry.artifact.kind, source_scope),
        status,
        privacy: entry.privacy,
        size: entry.size,
        created_at: entry.created_at,
        provenance: ContentProvenanceReadModel {
            actor: entry.actor,
            reason: entry.reason,
            source_id: entry.source.as_ref().map(|source| source.source_id.clone()),
            source_scope,
            source_role,
            source_kind: entry
                .source
                .as_ref()
                .map(|source| source.source_kind.clone()),
            content_type: entry
                .source
                .as_ref()
                .map(|source| source.content_type.clone()),
            locator: entry.source.and_then(|source| source.locator),
        },
        subject_jobs: entry
            .subject_jobs
            .into_iter()
            .map(|job| ContentSubjectJobReadModel {
                id: job.id,
                title: job.title,
                institution: job.institution,
                archived: job.archived,
            })
            .collect(),
        relationships: entry.dependencies,
        private_body_searchable: body_is_searchable(entry.artifact.kind),
        artifact: entry.artifact,
    }
}

fn content_category(
    kind: ArtifactKind,
    source_scope: Option<ContentSourceScope>,
) -> ContentCategory {
    match (kind, source_scope) {
        (
            ArtifactKind::SourceOriginal | ArtifactKind::SourceNormalizedText,
            Some(ContentSourceScope::Profile),
        ) => ContentCategory::Profile,
        (ArtifactKind::SourceOriginal | ArtifactKind::SourceNormalizedText, _) => {
            ContentCategory::Source
        }
        (ArtifactKind::ParsedJob | ArtifactKind::Criteria, _) => ContentCategory::JobAnalysis,
        (ArtifactKind::EvidenceCatalog | ArtifactKind::EvidenceMatches, _) => {
            ContentCategory::Evidence
        }
        (ArtifactKind::ApplicationPlan, _) => ContentCategory::Planning,
        (
            ArtifactKind::CoverLetter
            | ArtifactKind::ResearchStatement
            | ArtifactKind::TeachingStatement
            | ArtifactKind::Cv
            | ArtifactKind::DocumentSet,
            _,
        ) => ContentCategory::Materials,
        (ArtifactKind::ReviewFindings, _) => ContentCategory::Review,
        (
            ArtifactKind::PackageManifest
            | ArtifactKind::ExportManifest
            | ArtifactKind::TypstSource
            | ArtifactKind::Pdf
            | ArtifactKind::RenderManifest,
            _,
        ) => ContentCategory::Delivery,
    }
}

fn content_stage(kind: ArtifactKind, source_scope: Option<ContentSourceScope>) -> WorkflowStage {
    match (kind, source_scope) {
        (
            ArtifactKind::SourceOriginal | ArtifactKind::SourceNormalizedText,
            Some(ContentSourceScope::Profile),
        ) => WorkflowStage::Evidence,
        (ArtifactKind::SourceOriginal | ArtifactKind::SourceNormalizedText, _) => {
            WorkflowStage::Intake
        }
        (ArtifactKind::ParsedJob, _) => WorkflowStage::Parse,
        (ArtifactKind::Criteria, _) => WorkflowStage::Criteria,
        (ArtifactKind::EvidenceCatalog, _) => WorkflowStage::Evidence,
        (ArtifactKind::EvidenceMatches, _) => WorkflowStage::Match,
        (ArtifactKind::ApplicationPlan, _) => WorkflowStage::Plan,
        (
            ArtifactKind::CoverLetter
            | ArtifactKind::ResearchStatement
            | ArtifactKind::TeachingStatement
            | ArtifactKind::Cv
            | ArtifactKind::DocumentSet,
            _,
        ) => WorkflowStage::Draft,
        (ArtifactKind::ReviewFindings, _) => WorkflowStage::Review,
        (ArtifactKind::PackageManifest | ArtifactKind::ExportManifest, _) => WorkflowStage::Package,
        (ArtifactKind::TypstSource | ArtifactKind::Pdf | ArtifactKind::RenderManifest, _) => {
            WorkflowStage::Render
        }
    }
}

fn content_status(
    entry: &CatalogArtifactMetadata,
    category: ContentCategory,
) -> ContentCatalogStatus {
    if entry.stale {
        return ContentCatalogStatus::Stale;
    }
    match category {
        ContentCategory::Source | ContentCategory::Profile => ContentCatalogStatus::Imported,
        ContentCategory::JobAnalysis if entry.artifact.kind == ArtifactKind::ParsedJob => {
            ContentCatalogStatus::Generated
        }
        ContentCategory::Evidence
            if entry.artifact.kind == ArtifactKind::EvidenceCatalog
                && !entry.current_stage_output =>
        {
            ContentCatalogStatus::Proposed
        }
        ContentCategory::JobAnalysis | ContentCategory::Evidence | ContentCategory::Planning => {
            ContentCatalogStatus::Confirmed
        }
        ContentCategory::Materials | ContentCategory::Review | ContentCategory::Delivery => {
            ContentCatalogStatus::Generated
        }
    }
}

fn content_title(
    kind: ArtifactKind,
    source_scope: Option<ContentSourceScope>,
    source_role: Option<ContentSourceRole>,
) -> String {
    match (kind, source_scope, source_role) {
        (ArtifactKind::SourceOriginal, Some(ContentSourceScope::Profile), _) => {
            "Original profile source"
        }
        (ArtifactKind::SourceNormalizedText, Some(ContentSourceScope::Profile), _) => {
            "Normalized profile source"
        }
        (ArtifactKind::SourceOriginal, _, Some(ContentSourceRole::Original) | None) => {
            "Original job advert"
        }
        (ArtifactKind::SourceNormalizedText, _, _) => "Normalized job advert",
        (ArtifactKind::ParsedJob, _, _) => "Parsed job",
        (ArtifactKind::Criteria, _, _) => "Selection criteria",
        (ArtifactKind::EvidenceCatalog, _, _) => "Evidence catalog",
        (ArtifactKind::EvidenceMatches, _, _) => "Evidence matches",
        (ArtifactKind::ApplicationPlan, _, _) => "Application plan",
        (ArtifactKind::CoverLetter, _, _) => "Cover letter",
        (ArtifactKind::ResearchStatement, _, _) => "Research statement",
        (ArtifactKind::TeachingStatement, _, _) => "Teaching statement",
        (ArtifactKind::Cv, _, _) => "Curriculum vitae",
        (ArtifactKind::DocumentSet, _, _) => "Application materials",
        (ArtifactKind::ReviewFindings, _, _) => "Review findings",
        (ArtifactKind::PackageManifest, _, _) => "Application package",
        (ArtifactKind::ExportManifest, _, _) => "Package export",
        (ArtifactKind::TypstSource, _, _) => "Typesetting source",
        (ArtifactKind::Pdf, _, _) => "Rendered PDF",
        (ArtifactKind::RenderManifest, _, _) => "Render manifest",
        (ArtifactKind::SourceOriginal, _, Some(ContentSourceRole::Normalized)) => {
            "Original job advert"
        }
    }
    .to_owned()
}

fn body_is_searchable(kind: ArtifactKind) -> bool {
    !matches!(kind, ArtifactKind::SourceOriginal | ArtifactKind::Pdf)
}

struct ValidatedContentFilter {
    job_id: Option<EntityId>,
    created_after: Option<UtcTimestamp>,
    created_before: Option<UtcTimestamp>,
}

fn validate_filter(
    filter: &ContentCatalogFilter,
) -> Result<ValidatedContentFilter, ApplicationError> {
    let job_id = filter.job_id.as_deref().map(parse_entity_id).transpose()?;
    let created_after = filter
        .created_after
        .as_deref()
        .map(|value| {
            UtcTimestamp::try_new(value).map_err(|error| {
                ApplicationError::InvalidInput(format!(
                    "invalid content created-after timestamp: {error}"
                ))
            })
        })
        .transpose()?;
    let created_before = filter
        .created_before
        .as_deref()
        .map(|value| {
            UtcTimestamp::try_new(value).map_err(|error| {
                ApplicationError::InvalidInput(format!(
                    "invalid content created-before timestamp: {error}"
                ))
            })
        })
        .transpose()?;
    if created_after
        .as_ref()
        .zip(created_before.as_ref())
        .is_some_and(|(after, before)| after > before)
    {
        return Err(ApplicationError::InvalidInput(
            "content created-after must not be later than created-before".to_owned(),
        ));
    }
    Ok(ValidatedContentFilter {
        job_id,
        created_after,
        created_before,
    })
}

fn filter_matches(
    entry: &ContentCatalogEntryReadModel,
    filter: &ContentCatalogFilter,
    validated: &ValidatedContentFilter,
) -> bool {
    validated
        .job_id
        .as_ref()
        .is_none_or(|job_id| entry.subject_jobs.iter().any(|job| &job.id == job_id))
        && filter
            .category
            .is_none_or(|category| entry.category == category)
        && filter.stage.is_none_or(|stage| entry.stage == stage)
        && filter.status.is_none_or(|status| entry.status == status)
        && filter
            .privacy
            .is_none_or(|privacy| entry.privacy == privacy)
        && validated
            .created_after
            .as_ref()
            .is_none_or(|after| &entry.created_at >= after)
        && validated
            .created_before
            .as_ref()
            .is_none_or(|before| &entry.created_at <= before)
}

fn validate_search_request(request: &mut ContentSearchRequest) -> Result<(), ApplicationError> {
    request.query = request.query.trim().to_owned();
    if request.query.chars().count() > MAX_QUERY_CHARS {
        return Err(ApplicationError::InvalidInput(format!(
            "content search query exceeds {MAX_QUERY_CHARS} characters"
        )));
    }
    if request.query.chars().any(char::is_control) {
        return Err(ApplicationError::InvalidInput(
            "content search query contains a control character".to_owned(),
        ));
    }
    if request.include_private_bodies && request.query.is_empty() {
        return Err(ApplicationError::InvalidInput(
            "private full-text content search requires a non-empty query".to_owned(),
        ));
    }
    if request.limit == 0 || request.limit > MAX_SEARCH_LIMIT {
        return Err(ApplicationError::InvalidInput(format!(
            "content search limit must be between 1 and {MAX_SEARCH_LIMIT}"
        )));
    }
    Ok(())
}

fn private_search_consent_required() -> ApplicationError {
    ApplicationError::ConsentRequired {
        message: "read-private-inputs consent is required for full-text content search".to_owned(),
        remediation: NextAction {
            action: "confirm private local search, then repeat with --allow-private-read"
                .to_owned(),
            description:
                "CanISend will build a bounded in-memory index and return only matching snippets"
                    .to_owned(),
        },
    }
}

fn default_search_limit() -> usize {
    DEFAULT_SEARCH_LIMIT
}

fn count(value: usize, label: &str) -> Result<u64, ApplicationError> {
    u64::try_from(value)
        .map_err(|_| ApplicationError::InvalidInput(format!("{label} does not fit u64")))
}

struct InMemoryContentIndex {
    entries: Vec<ContentCatalogEntryReadModel>,
    metadata: BTreeMap<String, BTreeSet<usize>>,
    bodies: BTreeMap<String, BTreeSet<usize>>,
    body_text: BTreeMap<usize, String>,
    stats: ContentIndexReadModel,
}

impl InMemoryContentIndex {
    fn new(entries: Vec<ContentCatalogEntryReadModel>) -> Self {
        let mut metadata = BTreeMap::<String, BTreeSet<usize>>::new();
        for (index, entry) in entries.iter().enumerate() {
            insert_index_tokens(&mut metadata, index, &metadata_text(entry));
        }
        Self {
            stats: ContentIndexReadModel {
                strategy: "deterministic-in-memory-rebuild".to_owned(),
                metadata_entries: u64::try_from(entries.len()).unwrap_or(u64::MAX),
                private_body_entries: 0,
                private_body_bytes: 0,
                skipped_oversized_entries: 0,
                skipped_secret_entries: 0,
                truncated: false,
            },
            entries,
            metadata,
            bodies: BTreeMap::new(),
            body_text: BTreeMap::new(),
        }
    }

    fn index_private_bodies(
        &mut self,
        blobs: &canisend_store::BlobStore,
    ) -> Result<Vec<String>, ApplicationError> {
        let mut warnings = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            if !entry.private_body_searchable {
                continue;
            }
            if entry.privacy == PrivacyClassification::Secret {
                self.stats.skipped_secret_entries += 1;
                continue;
            }
            if entry.size > MAX_SINGLE_INDEX_BODY_BYTES {
                self.stats.skipped_oversized_entries += 1;
                continue;
            }
            let next_total = self
                .stats
                .private_body_bytes
                .checked_add(entry.size)
                .ok_or_else(|| {
                    ApplicationError::InvalidInput(
                        "private content index byte count overflowed".to_owned(),
                    )
                })?;
            if next_total > MAX_PRIVATE_INDEX_BYTES {
                self.stats.truncated = true;
                break;
            }
            let bytes = blobs.read_verified(
                &entry.artifact.sha256,
                MAX_SINGLE_INDEX_BODY_BYTES.min(DEFAULT_MAX_BLOB_BYTES),
            )?;
            let Ok(text) = String::from_utf8(bytes) else {
                warnings.push(format!(
                    "Skipped non-UTF-8 artifact {} while building the private index",
                    entry.artifact.id
                ));
                continue;
            };
            insert_index_tokens(&mut self.bodies, index, &text);
            self.body_text.insert(index, text);
            self.stats.private_body_entries += 1;
            self.stats.private_body_bytes = next_total;
        }
        if self.stats.skipped_oversized_entries > 0 {
            warnings.push(format!(
                "Skipped {} artifact(s) above the per-item private search bound",
                self.stats.skipped_oversized_entries
            ));
        }
        if self.stats.skipped_secret_entries > 0 {
            warnings.push(format!(
                "Skipped {} secret-classified artifact(s)",
                self.stats.skipped_secret_entries
            ));
        }
        if self.stats.truncated {
            warnings.push(format!(
                "Private full-text indexing stopped at the {} MiB operation bound",
                MAX_PRIVATE_INDEX_BYTES / (1024 * 1024)
            ));
        }
        Ok(warnings)
    }

    fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<(u64, Vec<ContentSearchResultReadModel>), ApplicationError> {
        let terms = query_terms(query);
        let term_hits = terms
            .iter()
            .map(|term| SearchTermHits {
                metadata_exact: self.metadata.get(term).cloned().unwrap_or_default(),
                metadata_all: index_matches(&self.metadata, term),
                body_exact: self.bodies.get(term).cloned().unwrap_or_default(),
                body_all: index_matches(&self.bodies, term),
            })
            .collect::<Vec<_>>();
        let candidates = if terms.is_empty() {
            (0..self.entries.len()).collect::<BTreeSet<_>>()
        } else {
            let mut candidates: Option<BTreeSet<usize>> = None;
            for hits in &term_hits {
                let mut matching = hits.metadata_all.clone();
                matching.extend(&hits.body_all);
                candidates = Some(candidates.map_or(matching.clone(), |current| {
                    current.intersection(&matching).copied().collect()
                }));
            }
            candidates.unwrap_or_default()
        };
        let total_matches = count(candidates.len(), "content search match count")?;
        let mut results = candidates
            .into_iter()
            .map(|index| {
                let metadata_score = score_hits(&term_hits, index, true, 20, 12);
                let body_score = score_hits(&term_hits, index, false, 8, 4);
                let mut matched_fields = Vec::new();
                if metadata_score > 0 || terms.is_empty() {
                    matched_fields.push(ContentMatchField::Metadata);
                }
                if body_score > 0 {
                    matched_fields.push(ContentMatchField::PrivateBody);
                }
                ContentSearchResultReadModel {
                    entry: self.entries[index].clone(),
                    score: metadata_score + body_score,
                    matched_fields,
                    snippet: self
                        .body_text
                        .get(&index)
                        .and_then(|text| private_snippet(text, &terms)),
                }
            })
            .collect::<Vec<_>>();
        results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| {
                    right
                        .entry
                        .created_at
                        .as_str()
                        .cmp(left.entry.created_at.as_str())
                })
                .then_with(|| left.entry.artifact.id.cmp(&right.entry.artifact.id))
        });
        results.truncate(limit);
        Ok((total_matches, results))
    }
}

fn metadata_text(entry: &ContentCatalogEntryReadModel) -> String {
    let mut values = vec![
        entry.title.clone(),
        enum_name(entry.category),
        enum_name(entry.stage),
        enum_name(entry.status),
        enum_name(entry.privacy),
        entry.artifact.id.to_string(),
        entry.provenance.reason.clone(),
    ];
    for job in &entry.subject_jobs {
        values.push(job.title.clone());
        values.push(job.institution.clone());
    }
    if let Some(value) = &entry.provenance.source_kind {
        values.push(value.clone());
    }
    if let Some(value) = &entry.provenance.content_type {
        values.push(value.clone());
    }
    if let Some(value) = &entry.provenance.locator {
        values.push(value.clone());
    }
    values.join(" ")
}

fn enum_name<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}

fn insert_index_tokens(index: &mut BTreeMap<String, BTreeSet<usize>>, entry: usize, text: &str) {
    for token in searchable_tokens(text) {
        index.entry(token).or_default().insert(entry);
    }
}

fn searchable_tokens(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    for token in text
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        tokens.insert(token.to_owned());
        if !token.is_ascii() {
            let characters = token.chars().collect::<Vec<_>>();
            for window in characters.windows(2) {
                tokens.insert(window.iter().collect());
            }
        }
    }
    tokens
}

fn query_terms(query: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    for token in query
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        if token.is_ascii() || token.chars().count() <= 2 {
            terms.insert(token.to_owned());
        } else {
            let characters = token.chars().collect::<Vec<_>>();
            terms.extend(
                characters
                    .windows(2)
                    .map(|window| window.iter().collect::<String>()),
            );
        }
    }
    terms.into_iter().collect()
}

fn index_matches(index: &BTreeMap<String, BTreeSet<usize>>, term: &str) -> BTreeSet<usize> {
    let mut matches = index.get(term).cloned().unwrap_or_default();
    if term.is_ascii() && term.chars().count() >= 2 {
        for (key, entries) in index.range(term.to_owned()..) {
            if !key.starts_with(term) {
                break;
            }
            matches.extend(entries);
        }
    }
    matches
}

struct SearchTermHits {
    metadata_exact: BTreeSet<usize>,
    metadata_all: BTreeSet<usize>,
    body_exact: BTreeSet<usize>,
    body_all: BTreeSet<usize>,
}

fn score_hits(
    hits: &[SearchTermHits],
    entry: usize,
    metadata: bool,
    exact: u64,
    prefix: u64,
) -> u64 {
    hits.iter()
        .map(|hits| {
            let (exact_hits, all_hits) = if metadata {
                (&hits.metadata_exact, &hits.metadata_all)
            } else {
                (&hits.body_exact, &hits.body_all)
            };
            if exact_hits.contains(&entry) {
                exact
            } else if all_hits.contains(&entry) {
                prefix
            } else {
                0
            }
        })
        .sum()
}

fn private_snippet(text: &str, terms: &[String]) -> Option<String> {
    if terms.is_empty() {
        return None;
    }
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_lowercase();
    let byte_index = terms.iter().find_map(|term| lower.find(term))?;
    let character_index = lower[..byte_index].chars().count();
    let characters = normalized.chars().collect::<Vec<_>>();
    let start = character_index.saturating_sub(MAX_SNIPPET_CHARS / 3);
    let end = (start + MAX_SNIPPET_CHARS).min(characters.len());
    let mut snippet = characters[start..end].iter().collect::<String>();
    if start > 0 {
        snippet.insert(0, '…');
    }
    if end < characters.len() {
        snippet.push('…');
    }
    Some(snippet)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{ContentCatalogFilter, ContentCategory, ContentMatchField, ContentSearchRequest};
    use crate::{Application, ApplicationError, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-content-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn catalog_and_search_keep_private_bodies_behind_explicit_consent() {
        let root = temporary_root("workspace");
        let source = temporary_root("advert").with_extension("md");
        let sentinel = "PRIVATE-CONTENT-CATALOG-SENTINEL";
        fs::write(
            &source,
            format!("# Lecturer in Economics\n\nTeaching portfolio {sentinel}\n"),
        )
        .expect("advert");
        Application::initialize_workspace(&root).expect("workspace");
        let job = Application::create_job(&root, "Lecturer in Economics", "University X")
            .expect("job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("source");

        let catalog = Application::content_catalog(
            &root,
            ContentCatalogFilter {
                job_id: Some(job.id.to_string()),
                ..ContentCatalogFilter::default()
            },
        )
        .expect("catalog")
        .data;
        assert_eq!(catalog.entries.len(), 2);
        assert!(
            catalog
                .entries
                .iter()
                .all(|entry| entry.category == ContentCategory::Source)
        );
        assert!(
            !serde_json::to_string(&catalog)
                .expect("catalog JSON")
                .contains(sentinel)
        );

        let metadata =
            Application::search_content(&root, ContentSearchRequest::metadata("Economics"), None)
                .expect("metadata search")
                .data;
        assert_eq!(metadata.index.private_body_entries, 0);
        assert!(!metadata.results.is_empty());
        assert!(
            !serde_json::to_string(&metadata)
                .expect("metadata JSON")
                .contains(sentinel)
        );

        let private_request = ContentSearchRequest {
            query: sentinel.to_owned(),
            include_private_bodies: true,
            ..ContentSearchRequest::metadata("")
        };
        assert!(matches!(
            Application::search_content(&root, private_request.clone(), None),
            Err(ApplicationError::ConsentRequired { .. })
        ));
        let private = Application::search_content(
            &root,
            private_request,
            Some(PrivateReadConsent::granted_by_user()),
        )
        .expect("private search")
        .data;
        assert_eq!(private.total_matches, 1);
        assert!(
            private.results[0]
                .matched_fields
                .contains(&ContentMatchField::PrivateBody)
        );
        assert!(
            private.results[0]
                .snippet
                .as_deref()
                .is_some_and(|snippet| snippet.contains(sentinel))
        );

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn private_search_checks_consent_and_bounds_before_workspace_access() {
        let missing = temporary_root("missing");
        let too_long = "x".repeat(super::MAX_QUERY_CHARS + 1);
        let error =
            Application::search_content(&missing, ContentSearchRequest::metadata(too_long), None)
                .expect_err("query bound");
        assert!(matches!(error, ApplicationError::InvalidInput(_)));

        let error = Application::content_catalog(
            &missing,
            ContentCatalogFilter {
                created_after: Some("2026-08-01T00:00:00Z".to_owned()),
                created_before: Some("2026-07-01T00:00:00Z".to_owned()),
                ..ContentCatalogFilter::default()
            },
        )
        .expect_err("date bound");
        assert!(matches!(error, ApplicationError::InvalidInput(_)));

        let error = Application::search_content(
            &missing,
            ContentSearchRequest {
                query: "evidence".to_owned(),
                include_private_bodies: true,
                ..ContentSearchRequest::metadata("")
            },
            None,
        )
        .expect_err("consent");
        assert!(matches!(error, ApplicationError::ConsentRequired { .. }));
    }
}
