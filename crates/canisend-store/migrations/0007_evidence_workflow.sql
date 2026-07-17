ALTER TABLE tasks ADD COLUMN profile_revision INTEGER CHECK (profile_revision > 0);
ALTER TABLE tasks ADD COLUMN candidate_sha256 TEXT;
ALTER TABLE stage_executions ADD COLUMN input_profile_revision INTEGER
    CHECK (input_profile_revision > 0);

ALTER TABLE evidence_items ADD COLUMN source_artifact_id TEXT REFERENCES artifacts(id);
ALTER TABLE evidence_revisions ADD COLUMN artifact_id TEXT REFERENCES artifacts(id);
ALTER TABLE evidence_revisions ADD COLUMN artifact_revision INTEGER CHECK (artifact_revision > 0);
ALTER TABLE evidence_revisions ADD COLUMN excluded INTEGER NOT NULL DEFAULT 0
    CHECK (excluded IN (0, 1));
ALTER TABLE evidence_revisions ADD COLUMN sensitivity TEXT;

CREATE INDEX evidence_source_artifact ON evidence_items(source_artifact_id);
CREATE INDEX evidence_catalog_revision ON evidence_revisions(artifact_id, artifact_revision);

PRAGMA user_version = 7;
