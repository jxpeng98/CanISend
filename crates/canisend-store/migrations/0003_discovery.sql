ALTER TABLE discovery_sources ADD COLUMN name TEXT NOT NULL DEFAULT '';
ALTER TABLE discovery_sources ADD COLUMN endpoint TEXT;
ALTER TABLE discovery_sources ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1));
ALTER TABLE discovery_sources ADD COLUMN policy_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE discovery_sources ADD COLUMN cursor TEXT;
ALTER TABLE discovery_sources ADD COLUMN last_refreshed_at TEXT;

CREATE UNIQUE INDEX discovery_sources_identity
    ON discovery_sources(kind, name, IFNULL(endpoint, ''));

ALTER TABLE job_leads ADD COLUMN external_id TEXT;
ALTER TABLE job_leads ADD COLUMN title TEXT NOT NULL DEFAULT '';
ALTER TABLE job_leads ADD COLUMN organization TEXT NOT NULL DEFAULT '';
ALTER TABLE job_leads ADD COLUMN location TEXT;
ALTER TABLE job_leads ADD COLUMN deadline TEXT;
ALTER TABLE job_leads ADD COLUMN url TEXT NOT NULL DEFAULT '';
ALTER TABLE job_leads ADD COLUMN summary TEXT;
ALTER TABLE job_leads ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE job_leads ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE job_leads ADD COLUMN freshness TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE job_leads ADD COLUMN first_seen_at TEXT NOT NULL DEFAULT '';
ALTER TABLE job_leads ADD COLUMN last_seen_at TEXT NOT NULL DEFAULT '';
ALTER TABLE job_leads ADD COLUMN status_changed_at TEXT NOT NULL DEFAULT '';
ALTER TABLE job_leads ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0);
ALTER TABLE job_leads ADD COLUMN promoted_job_id TEXT REFERENCES jobs(id);

CREATE UNIQUE INDEX job_leads_source_exact_key
    ON job_leads(discovery_source_id, canonical_key);
CREATE INDEX job_leads_status_seen
    ON job_leads(status, last_seen_at);

CREATE TABLE discovery_refresh_receipts (
    id TEXT PRIMARY KEY,
    discovery_source_id TEXT NOT NULL REFERENCES discovery_sources(id),
    observed INTEGER NOT NULL CHECK (observed >= 0),
    inserted INTEGER NOT NULL CHECK (inserted >= 0),
    updated INTEGER NOT NULL CHECK (updated >= 0),
    unchanged INTEGER NOT NULL CHECK (unchanged >= 0),
    removed INTEGER NOT NULL CHECK (removed >= 0),
    rejected INTEGER NOT NULL CHECK (rejected >= 0),
    cursor TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT NOT NULL
) STRICT;

CREATE INDEX discovery_receipts_source_time
    ON discovery_refresh_receipts(discovery_source_id, completed_at);

PRAGMA user_version = 3;
