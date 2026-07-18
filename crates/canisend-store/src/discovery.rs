use std::collections::BTreeSet;

use canisend_contracts::{
    ActorKind, DiscoveryBatch, DiscoveryImportReport, DiscoveryLeadCandidate, DiscoveryLeadRecord,
    DiscoveryLeadSuggestion, DiscoveryRefreshPolicy, DiscoveryRefreshReceipt, DiscoverySourceKind,
    DiscoverySourceRecord, EntityId, JobRecord, Revision, UtcTimestamp,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{Database, StoreError, generate_id, job::load_job, now_utc};

const MAX_BATCH_LEADS: usize = 1_000;
const MAX_FUZZY_CANDIDATES: usize = 200;
const MAX_FUZZY_RESULTS: usize = 20;

pub struct DiscoveryService<'a> {
    database: &'a mut Database,
}

impl<'a> DiscoveryService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database) -> Self {
        Self { database }
    }

    pub fn import_report(
        &mut self,
        mut report: DiscoveryImportReport,
        actor: ActorKind,
    ) -> Result<DiscoveryImportReport, StoreError> {
        if !report.dry_run || report.receipt.is_some() {
            return Err(StoreError::InvalidInput(
                "discovery import must begin with an uncommitted dry-run report".to_owned(),
            ));
        }
        let batch = report.batch.as_ref().ok_or_else(|| {
            StoreError::InvalidInput("discovery import report has no normalized batch".to_owned())
        })?;
        validate_batch(batch)?;
        let accepted = u64::try_from(batch.leads.len())
            .map_err(|_| StoreError::Invariant("lead count does not fit u64".to_owned()))?;
        if report.accepted != accepted {
            return Err(StoreError::InvalidInput(
                "discovery report accepted count does not match its normalized batch".to_owned(),
            ));
        }
        let policy = default_policy(batch.source_kind);
        if batch.leads.len() > usize::try_from(policy.max_items).unwrap_or(usize::MAX) {
            return Err(StoreError::InvalidInput(format!(
                "discovery batch exceeds source policy limit of {} leads",
                policy.max_items
            )));
        }

        let source_id = generate_id()?;
        let receipt_id = generate_id()?;
        let event_id = generate_id()?;
        let started_at = now_utc()?;
        let completed_at = now_utc()?;
        let source_kind = enum_name(batch.source_kind)?;
        let actor = enum_name(actor)?;
        let policy_json = serde_json::to_string(&policy)?;
        let configuration_sha256 = digest_bytes(
            serde_json::to_vec(&(
                &batch.source_name,
                &batch.source_url,
                &batch.source_kind,
                &policy,
            ))?
            .as_slice(),
        );
        let transaction = self.database.immediate_transaction()?;
        let source_id = find_or_create_source(
            &transaction,
            &source_id,
            batch,
            &source_kind,
            &configuration_sha256,
            &policy_json,
            &started_at,
        )?;

        let mut seen_keys = BTreeSet::new();
        let mut inserted = 0_u64;
        let mut updated = 0_u64;
        let mut unchanged = 0_u64;
        for candidate in &batch.leads {
            let canonical_key = canonical_key(candidate);
            seen_keys.insert(canonical_key.clone());
            let source_sha256 = digest_bytes(&serde_json::to_vec(candidate)?);
            let existing: Option<(String, String, String)> = transaction
                .query_row(
                    "SELECT id, source_sha256, status FROM job_leads
                     WHERE discovery_source_id = ?1 AND canonical_key = ?2",
                    params![source_id.as_str(), canonical_key],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            match existing {
                None => {
                    insert_lead(
                        &transaction,
                        &source_id,
                        candidate,
                        &canonical_key,
                        &source_sha256,
                        &batch.observed_at,
                    )?;
                    inserted += 1;
                }
                Some((lead_id, previous_sha256, status)) => {
                    let promoted = status == "promoted";
                    if previous_sha256 == source_sha256 && status == "active" {
                        transaction.execute(
                            "UPDATE job_leads
                             SET last_seen_at = ?2, freshness = 'current'
                             WHERE id = ?1",
                            params![lead_id, batch.observed_at.as_str()],
                        )?;
                        unchanged += 1;
                    } else {
                        update_lead(
                            &transaction,
                            &lead_id,
                            candidate,
                            &source_sha256,
                            &batch.observed_at,
                            promoted,
                        )?;
                        updated += 1;
                    }
                }
            }
        }

        let removed = if policy.mark_missing_as_removed {
            mark_missing_removed(&transaction, &source_id, &seen_keys, &completed_at)?
        } else {
            0
        };
        expire_due_leads(&transaction, date_prefix(&completed_at)?, &completed_at)?;
        refresh_freshness(&transaction, &completed_at)?;
        transaction.execute(
            "UPDATE discovery_sources
             SET cursor = ?2, last_refreshed_at = ?3, configuration_sha256 = ?4,
                 policy_json = ?5, enabled = 1
             WHERE id = ?1",
            params![
                source_id.as_str(),
                batch.cursor,
                completed_at.as_str(),
                configuration_sha256,
                policy_json
            ],
        )?;
        let rejected = report.rejected;
        let observed = accepted.saturating_add(rejected);
        insert_receipt(
            &transaction,
            ReceiptInsert {
                id: &receipt_id,
                source_id: &source_id,
                observed,
                inserted,
                updated,
                unchanged,
                removed,
                rejected,
                cursor: batch.cursor.as_deref(),
                started_at: &started_at,
                completed_at: &completed_at,
            },
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'discovery.refresh', ?3, NULL, 'import normalized lead batch', ?4)",
            params![
                event_id.as_str(),
                actor,
                source_id.as_str(),
                completed_at.as_str()
            ],
        )?;
        transaction.commit()?;

        report.dry_run = false;
        report.receipt = Some(DiscoveryRefreshReceipt {
            id: receipt_id,
            source_id,
            observed,
            inserted,
            updated,
            unchanged,
            removed,
            rejected,
            cursor: batch.cursor.clone(),
            started_at,
            completed_at,
        });
        Ok(report)
    }

    pub fn list_sources(&self) -> Result<Vec<DiscoverySourceRecord>, StoreError> {
        let mut statement = self
            .database
            .connection()
            .prepare("SELECT id FROM discovery_sources ORDER BY created_at, id")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_source(self.database.connection(), &EntityId::try_new(id)?))
            .collect()
    }

    pub fn list_leads(
        &mut self,
        include_history: bool,
    ) -> Result<Vec<DiscoveryLeadRecord>, StoreError> {
        let now = now_utc()?;
        let date = date_prefix(&now)?.to_owned();
        let transaction = self.database.immediate_transaction()?;
        expire_due_leads(&transaction, &date, &now)?;
        refresh_freshness(&transaction, &now)?;
        transaction.commit()?;
        let mut statement = self.database.connection().prepare(if include_history {
            "SELECT id FROM job_leads ORDER BY last_seen_at DESC, id"
        } else {
            "SELECT id FROM job_leads WHERE status = 'active' ORDER BY last_seen_at DESC, id"
        })?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_lead(self.database.connection(), &EntityId::try_new(id)?))
            .collect()
    }

    pub fn get_lead(&self, lead_id: &EntityId) -> Result<DiscoveryLeadRecord, StoreError> {
        load_lead(self.database.connection(), lead_id)
    }

    pub fn suggestions(
        &self,
        lead_id: &EntityId,
        limit: usize,
    ) -> Result<Vec<DiscoveryLeadSuggestion>, StoreError> {
        let target = self.get_lead(lead_id)?;
        let limit = limit.clamp(1, MAX_FUZZY_RESULTS);
        let mut statement = self.database.connection().prepare(
            "SELECT id FROM job_leads
             WHERE id != ?1 AND status = 'active'
             ORDER BY last_seen_at DESC, id LIMIT ?2",
        )?;
        let candidate_limit = i64::try_from(MAX_FUZZY_CANDIDATES)
            .map_err(|_| StoreError::Invariant("fuzzy limit does not fit SQLite".to_owned()))?;
        let ids = statement
            .query_map(params![lead_id.as_str(), candidate_limit], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut suggestions = ids
            .into_iter()
            .map(|id| load_lead(self.database.connection(), &EntityId::try_new(id)?))
            .collect::<Result<Vec<_>, StoreError>>()?
            .into_iter()
            .filter_map(|lead| {
                let similarity_percent = similarity(&target, &lead);
                (similarity_percent >= 55).then_some(DiscoveryLeadSuggestion {
                    lead,
                    similarity_percent,
                })
            })
            .collect::<Vec<_>>();
        suggestions.sort_unstable_by(|left, right| {
            right
                .similarity_percent
                .cmp(&left.similarity_percent)
                .then_with(|| left.lead.id.cmp(&right.lead.id))
        });
        suggestions.truncate(limit);
        Ok(suggestions)
    }

    pub fn promote(
        &mut self,
        lead_id: &EntityId,
        actor: ActorKind,
    ) -> Result<JobRecord, StoreError> {
        let job_id = generate_id()?;
        let job_event_id = generate_id()?;
        let promote_event_id = generate_id()?;
        let created_at = now_utc()?;
        let actor = enum_name(actor)?;
        let transaction = self.database.immediate_transaction()?;
        let lead: Option<(String, String, String, Option<String>)> = transaction
            .query_row(
                "SELECT title, organization, status, promoted_job_id FROM job_leads WHERE id = ?1",
                params![lead_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let (title, organization, status, promoted_job_id) =
            lead.ok_or_else(|| StoreError::DiscoveryLeadNotFound(lead_id.to_string()))?;
        if status == "promoted" {
            let existing = promoted_job_id.ok_or_else(|| {
                StoreError::Invariant("promoted lead has no job reference".to_owned())
            })?;
            transaction.rollback()?;
            return load_job(self.database.connection(), &EntityId::try_new(existing)?);
        }
        transaction.execute(
            "INSERT INTO jobs(id, title, institution, archived, created_at, revision)
             VALUES (?1, ?2, ?3, 0, ?4, 1)",
            params![job_id.as_str(), title, organization, created_at.as_str()],
        )?;
        transaction.execute(
            "UPDATE job_leads
             SET status = 'promoted', promoted_job_id = ?2, status_changed_at = ?3,
                 revision = revision + 1
             WHERE id = ?1",
            params![lead_id.as_str(), job_id.as_str(), created_at.as_str()],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'job.create', ?3, 1, 'promote discovery lead', ?4)",
            params![
                job_event_id.as_str(),
                actor,
                job_id.as_str(),
                created_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'discovery.promote', ?3,
                       (SELECT revision FROM job_leads WHERE id = ?3),
                       'promote lead into direct intake', ?4)",
            params![
                promote_event_id.as_str(),
                actor,
                lead_id.as_str(),
                created_at.as_str()
            ],
        )?;
        transaction.commit()?;
        load_job(self.database.connection(), &job_id)
    }
}

fn find_or_create_source(
    transaction: &Transaction<'_>,
    proposed_id: &EntityId,
    batch: &DiscoveryBatch,
    source_kind: &str,
    configuration_sha256: &str,
    policy_json: &str,
    created_at: &UtcTimestamp,
) -> Result<EntityId, StoreError> {
    let existing: Option<String> = transaction
        .query_row(
            "SELECT id FROM discovery_sources
             WHERE kind = ?1 AND name = ?2
               AND ((endpoint IS NULL AND ?3 IS NULL) OR endpoint = ?3)",
            params![source_kind, batch.source_name.trim(), batch.source_url],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(existing) = existing {
        return EntityId::try_new(existing).map_err(StoreError::from);
    }
    transaction.execute(
        "INSERT INTO discovery_sources(
            id, kind, configuration_sha256, created_at, name, endpoint, enabled,
            policy_json, cursor, last_refreshed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, NULL, NULL)",
        params![
            proposed_id.as_str(),
            source_kind,
            configuration_sha256,
            created_at.as_str(),
            batch.source_name.trim(),
            batch.source_url,
            policy_json
        ],
    )?;
    Ok(proposed_id.clone())
}

fn insert_lead(
    transaction: &Transaction<'_>,
    source_id: &EntityId,
    candidate: &DiscoveryLeadCandidate,
    canonical_key: &str,
    source_sha256: &str,
    observed_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    let lead_id = generate_id()?;
    transaction.execute(
        "INSERT INTO job_leads(
            id, discovery_source_id, canonical_key, source_sha256, created_at,
            external_id, title, organization, location, deadline, url, summary,
            metadata_json, status, freshness, first_seen_at, last_seen_at, revision,
            promoted_job_id, status_changed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                   ?13, 'active', 'current', ?5, ?5, 1, NULL, ?5)",
        params![
            lead_id.as_str(),
            source_id.as_str(),
            canonical_key,
            source_sha256,
            observed_at.as_str(),
            candidate.external_id,
            candidate.title,
            candidate.organization,
            candidate.location,
            candidate.deadline,
            candidate.url,
            candidate.summary,
            serde_json::to_string(&candidate.metadata)?
        ],
    )?;
    Ok(())
}

fn update_lead(
    transaction: &Transaction<'_>,
    lead_id: &str,
    candidate: &DiscoveryLeadCandidate,
    source_sha256: &str,
    observed_at: &UtcTimestamp,
    promoted: bool,
) -> Result<(), StoreError> {
    transaction.execute(
        "UPDATE job_leads SET
            source_sha256 = ?2, external_id = ?3, title = ?4, organization = ?5,
            location = ?6, deadline = ?7, url = ?8, summary = ?9,
            metadata_json = ?10, status = CASE WHEN ?11 THEN 'promoted' ELSE 'active' END,
            freshness = 'current', last_seen_at = ?12, status_changed_at = ?12,
            revision = revision + 1
         WHERE id = ?1",
        params![
            lead_id,
            source_sha256,
            candidate.external_id,
            candidate.title,
            candidate.organization,
            candidate.location,
            candidate.deadline,
            candidate.url,
            candidate.summary,
            serde_json::to_string(&candidate.metadata)?,
            promoted,
            observed_at.as_str()
        ],
    )?;
    Ok(())
}

fn mark_missing_removed(
    transaction: &Transaction<'_>,
    source_id: &EntityId,
    seen_keys: &BTreeSet<String>,
    changed_at: &UtcTimestamp,
) -> Result<u64, StoreError> {
    let mut statement = transaction.prepare(
        "SELECT id, canonical_key FROM job_leads
         WHERE discovery_source_id = ?1 AND status = 'active'",
    )?;
    let rows = statement
        .query_map(params![source_id.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    let mut removed = 0_u64;
    for (lead_id, key) in rows {
        if !seen_keys.contains(&key) {
            transaction.execute(
                "UPDATE job_leads
                 SET status = 'removed', freshness = 'stale', status_changed_at = ?2,
                     revision = revision + 1
                 WHERE id = ?1",
                params![lead_id, changed_at.as_str()],
            )?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn expire_due_leads(
    transaction: &Transaction<'_>,
    date: &str,
    changed_at: &UtcTimestamp,
) -> Result<u64, StoreError> {
    let changed = transaction.execute(
        "UPDATE job_leads
         SET status = 'expired', freshness = 'stale', revision = revision + 1,
             status_changed_at = ?2
         WHERE status = 'active' AND deadline IS NOT NULL AND deadline < ?1",
        params![date, changed_at.as_str()],
    )?;
    u64::try_from(changed)
        .map_err(|_| StoreError::Invariant("expired lead count does not fit u64".to_owned()))
}

fn refresh_freshness(transaction: &Transaction<'_>, now: &UtcTimestamp) -> Result<(), StoreError> {
    let now = OffsetDateTime::parse(now.as_str(), &Rfc3339)
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    let mut statement = transaction.prepare(
        "SELECT leads.id, leads.last_seen_at, leads.freshness, sources.policy_json
         FROM job_leads AS leads
         JOIN discovery_sources AS sources ON sources.id = leads.discovery_source_id
         WHERE leads.status = 'active'",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for (lead_id, last_seen_at, freshness, policy_json) in rows {
        let last_seen_at = OffsetDateTime::parse(&last_seen_at, &Rfc3339)
            .map_err(|error| StoreError::Invariant(error.to_string()))?;
        let policy: DiscoveryRefreshPolicy = serde_json::from_str(&policy_json)?;
        let stale = (now - last_seen_at).whole_seconds()
            > i64::try_from(policy.stale_after_seconds).unwrap_or(i64::MAX);
        let expected = if stale { "stale" } else { "current" };
        if freshness != expected {
            transaction.execute(
                "UPDATE job_leads SET freshness = ?2, revision = revision + 1 WHERE id = ?1",
                params![lead_id, expected],
            )?;
        }
    }
    Ok(())
}

struct ReceiptInsert<'a> {
    id: &'a EntityId,
    source_id: &'a EntityId,
    observed: u64,
    inserted: u64,
    updated: u64,
    unchanged: u64,
    removed: u64,
    rejected: u64,
    cursor: Option<&'a str>,
    started_at: &'a UtcTimestamp,
    completed_at: &'a UtcTimestamp,
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    receipt: ReceiptInsert<'_>,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO discovery_refresh_receipts(
            id, discovery_source_id, observed, inserted, updated, unchanged,
            removed, rejected, cursor, started_at, completed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            receipt.id.as_str(),
            receipt.source_id.as_str(),
            to_i64(receipt.observed)?,
            to_i64(receipt.inserted)?,
            to_i64(receipt.updated)?,
            to_i64(receipt.unchanged)?,
            to_i64(receipt.removed)?,
            to_i64(receipt.rejected)?,
            receipt.cursor,
            receipt.started_at.as_str(),
            receipt.completed_at.as_str()
        ],
    )?;
    Ok(())
}

fn load_source(
    connection: &Connection,
    source_id: &EntityId,
) -> Result<DiscoverySourceRecord, StoreError> {
    type Row = (
        String,
        String,
        Option<String>,
        i64,
        String,
        Option<String>,
        Option<String>,
        String,
    );
    let row: Option<Row> = connection
        .query_row(
            "SELECT kind, name, endpoint, enabled, policy_json, cursor, last_refreshed_at,
                    created_at
             FROM discovery_sources WHERE id = ?1",
            params![source_id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()?;
    let (kind, name, endpoint, enabled, policy_json, cursor, last_refreshed_at, created_at) =
        row.ok_or_else(|| StoreError::DiscoverySourceNotFound(source_id.to_string()))?;
    Ok(DiscoverySourceRecord {
        id: source_id.clone(),
        kind: enum_value(&kind)?,
        name,
        endpoint,
        enabled: enabled != 0,
        policy: serde_json::from_str(&policy_json)?,
        cursor,
        last_refreshed_at: last_refreshed_at.map(UtcTimestamp::try_new).transpose()?,
        created_at: UtcTimestamp::try_new(created_at)?,
    })
}

fn load_lead(
    connection: &Connection,
    lead_id: &EntityId,
) -> Result<DiscoveryLeadRecord, StoreError> {
    type Row = (
        String,
        Option<String>,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        i64,
        Option<String>,
    );
    let row: Option<Row> = connection
        .query_row(
            "SELECT discovery_source_id, external_id, canonical_key, title, organization,
                    location, deadline, url, summary, metadata_json, status, freshness,
                    first_seen_at, revision, promoted_job_id
             FROM job_leads WHERE id = ?1",
            params![lead_id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                ))
            },
        )
        .optional()?;
    let (
        source_id,
        external_id,
        canonical_key,
        title,
        organization,
        location,
        deadline,
        url,
        summary,
        metadata_json,
        status,
        freshness,
        first_seen_at,
        revision,
        promoted_job_id,
    ) = row.ok_or_else(|| StoreError::DiscoveryLeadNotFound(lead_id.to_string()))?;
    let last_seen_at: String = connection.query_row(
        "SELECT last_seen_at FROM job_leads WHERE id = ?1",
        params![lead_id.as_str()],
        |row| row.get(0),
    )?;
    Ok(DiscoveryLeadRecord {
        id: lead_id.clone(),
        source_id: EntityId::try_new(source_id)?,
        external_id,
        canonical_key,
        title,
        organization,
        location,
        deadline,
        url,
        summary,
        metadata: serde_json::from_str(&metadata_json)?,
        status: enum_value(&status)?,
        freshness: enum_value(&freshness)?,
        first_seen_at: UtcTimestamp::try_new(first_seen_at)?,
        last_seen_at: UtcTimestamp::try_new(last_seen_at)?,
        revision: Revision::try_new(to_u64(revision)?)?,
        promoted_job_id: promoted_job_id.map(EntityId::try_new).transpose()?,
    })
}

fn validate_batch(batch: &DiscoveryBatch) -> Result<(), StoreError> {
    if batch.source_name.trim().is_empty() || batch.source_name.len() > 300 {
        return Err(StoreError::InvalidInput(
            "discovery source name must contain between 1 and 300 bytes".to_owned(),
        ));
    }
    if batch.leads.len() > MAX_BATCH_LEADS {
        return Err(StoreError::InvalidInput(format!(
            "discovery batch exceeds {MAX_BATCH_LEADS} leads"
        )));
    }
    if batch.leads.iter().any(|lead| {
        lead.title.trim().is_empty()
            || lead.organization.trim().is_empty()
            || lead.url.trim().is_empty()
    }) {
        return Err(StoreError::InvalidInput(
            "normalized discovery leads require title, organization, and URL".to_owned(),
        ));
    }
    Ok(())
}

fn default_policy(kind: DiscoverySourceKind) -> DiscoveryRefreshPolicy {
    DiscoveryRefreshPolicy {
        max_items: u32::try_from(MAX_BATCH_LEADS).expect("batch limit fits u32"),
        stale_after_seconds: 7 * 24 * 60 * 60,
        mark_missing_as_removed: !matches!(
            kind,
            DiscoverySourceKind::Csv | DiscoverySourceKind::Json
        ),
    }
}

fn canonical_key(candidate: &DiscoveryLeadCandidate) -> String {
    let identity = if let Some(external_id) = &candidate.external_id {
        format!("external:{}", normalize_identity(external_id))
    } else {
        [
            normalize_identity(&candidate.organization),
            normalize_identity(&candidate.title),
            normalize_identity(candidate.location.as_deref().unwrap_or_default()),
            normalize_identity(&candidate.url),
        ]
        .join("\u{1f}")
    };
    digest_bytes(identity.as_bytes())
}

fn normalize_identity(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn similarity(left: &DiscoveryLeadRecord, right: &DiscoveryLeadRecord) -> u8 {
    let title = jaccard(&left.title, &right.title);
    let organization = jaccard(&left.organization, &right.organization);
    let location = jaccard(
        left.location.as_deref().unwrap_or_default(),
        right.location.as_deref().unwrap_or_default(),
    );
    let weighted = title * 60 + organization * 30 + location * 10;
    u8::try_from(weighted / 100).unwrap_or(100)
}

fn jaccard(left: &str, right: &str) -> u16 {
    let left = tokens(left);
    let right = tokens(right);
    if left.is_empty() && right.is_empty() {
        return 100;
    }
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count();
    u16::try_from(intersection * 100 / union).unwrap_or(100)
}

fn tokens(value: &str) -> BTreeSet<String> {
    normalize_identity(value)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn date_prefix(timestamp: &UtcTimestamp) -> Result<&str, StoreError> {
    timestamp
        .as_str()
        .get(..10)
        .ok_or_else(|| StoreError::Invariant("UTC timestamp has no date prefix".to_owned()))
}

fn digest_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn enum_name<T: Serialize>(value: T) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| StoreError::Invariant("enum did not serialize as a string".to_owned()))
}

fn enum_value<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(StoreError::from)
}

fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value)
        .map_err(|_| StoreError::Invariant("unsigned value does not fit SQLite".to_owned()))
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite value".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use canisend_contracts::{
        ActorKind, DiscoveryBatch, DiscoveryImportReport, DiscoveryLeadCandidate,
        DiscoveryLeadStatus, DiscoverySourceKind, UtcTimestamp,
    };

    use super::DiscoveryService;
    use crate::Workspace;

    fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::try_new(value).expect("timestamp")
    }

    fn candidate(id: &str, title: &str, url: &str) -> DiscoveryLeadCandidate {
        DiscoveryLeadCandidate {
            external_id: Some(id.to_owned()),
            title: title.to_owned(),
            organization: "University X".to_owned(),
            location: Some("London".to_owned()),
            deadline: Some("2099-09-01".to_owned()),
            url: url.to_owned(),
            summary: None,
            metadata: BTreeMap::new(),
        }
    }

    fn report_for_source(
        leads: Vec<DiscoveryLeadCandidate>,
        cursor: &str,
        source_name: &str,
    ) -> DiscoveryImportReport {
        DiscoveryImportReport {
            dry_run: true,
            accepted: u64::try_from(leads.len()).expect("lead count"),
            rejected: 0,
            diagnostics: Vec::new(),
            batch: Some(DiscoveryBatch {
                source_kind: DiscoverySourceKind::HostAgent,
                source_name: source_name.to_owned(),
                source_url: None,
                cursor: Some(cursor.to_owned()),
                observed_at: timestamp("2026-07-17T10:00:00Z"),
                leads,
            }),
            receipt: None,
        }
    }

    fn report(leads: Vec<DiscoveryLeadCandidate>, cursor: &str) -> DiscoveryImportReport {
        report_for_source(leads, cursor, "Agent search")
    }

    #[test]
    fn refresh_is_deterministic_preserves_history_and_promotes_idempotently() {
        let root =
            std::env::temp_dir().join(format!("canisend-discovery-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut workspace = Workspace::init(&root).expect("workspace");
        let mut discovery = DiscoveryService::new(&mut workspace.database);
        let first = discovery
            .import_report(
                report(
                    vec![
                        candidate("1", "Lecturer in Economics", "https://example.edu/1"),
                        candidate("2", "Research Fellow", "https://example.edu/2"),
                    ],
                    "one",
                ),
                ActorKind::HostAgent,
            )
            .expect("first refresh");
        let first_receipt = first.receipt.expect("receipt");
        assert_eq!(first_receipt.inserted, 2);

        let second = discovery
            .import_report(
                report(
                    vec![candidate(
                        "1",
                        "Lecturer in Applied Economics",
                        "https://example.edu/1",
                    )],
                    "two",
                ),
                ActorKind::HostAgent,
            )
            .expect("second refresh");
        let second_receipt = second.receipt.expect("receipt");
        assert_eq!(second_receipt.updated, 1);
        assert_eq!(second_receipt.removed, 1);
        let leads = discovery.list_leads(true).expect("lead history");
        assert_eq!(leads.len(), 2);
        assert!(
            leads
                .iter()
                .any(|lead| lead.status == DiscoveryLeadStatus::Removed)
        );
        let active_id = leads
            .iter()
            .find(|lead| lead.status == DiscoveryLeadStatus::Active)
            .expect("active lead")
            .id
            .clone();
        discovery
            .import_report(
                report_for_source(
                    vec![candidate(
                        "99",
                        "Lecturer in Applied Economics",
                        "https://other.example/99",
                    )],
                    "other-one",
                    "Second agent search",
                ),
                ActorKind::HostAgent,
            )
            .expect("second source");
        let suggestions = discovery
            .suggestions(&active_id, 5)
            .expect("bounded suggestions");
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].similarity_percent, 100);
        let promoted = discovery
            .promote(&active_id, ActorKind::User)
            .expect("promote");
        let repeated = discovery
            .promote(&active_id, ActorKind::User)
            .expect("idempotent promotion");
        assert_eq!(promoted.id, repeated.id);
        assert_eq!(promoted.title, "Lecturer in Applied Economics");
        assert_eq!(discovery.list_sources().expect("sources").len(), 2);
        drop(workspace);
        std::fs::remove_dir_all(root).expect("remove workspace");
    }
}
