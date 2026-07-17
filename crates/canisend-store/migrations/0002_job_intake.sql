ALTER TABLE jobs ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0);

ALTER TABLE source_revisions ADD COLUMN original_artifact_id TEXT REFERENCES artifacts(id);
ALTER TABLE source_revisions ADD COLUMN original_artifact_revision INTEGER CHECK (original_artifact_revision > 0);
ALTER TABLE source_revisions ADD COLUMN normalized_artifact_id TEXT REFERENCES artifacts(id);
ALTER TABLE source_revisions ADD COLUMN normalized_artifact_revision INTEGER CHECK (normalized_artifact_revision > 0);
ALTER TABLE source_revisions ADD COLUMN source_url TEXT;
ALTER TABLE source_revisions ADD COLUMN final_url TEXT;
ALTER TABLE source_revisions ADD COLUMN content_type TEXT;
ALTER TABLE source_revisions ADD COLUMN redirect_chain_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE source_revisions ADD COLUMN privacy TEXT NOT NULL DEFAULT 'private-local';

CREATE INDEX sources_job_id ON sources(job_id, created_at);

PRAGMA user_version = 2;
