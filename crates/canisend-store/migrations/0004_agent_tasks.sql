ALTER TABLE tasks ADD COLUMN job_id TEXT REFERENCES jobs(id);
ALTER TABLE tasks ADD COLUMN job_revision INTEGER CHECK (job_revision > 0);
ALTER TABLE tasks ADD COLUMN operation TEXT;
ALTER TABLE tasks ADD COLUMN actor TEXT;
ALTER TABLE tasks ADD COLUMN execution_mode TEXT;
ALTER TABLE tasks ADD COLUMN allowed_output_kind TEXT;
ALTER TABLE tasks ADD COLUMN candidate_schema_id TEXT;
ALTER TABLE tasks ADD COLUMN candidate_schema_version TEXT;
ALTER TABLE tasks ADD COLUMN lease_id TEXT;
ALTER TABLE tasks ADD COLUMN descriptor_json TEXT;
ALTER TABLE tasks ADD COLUMN cancelled_at TEXT;
ALTER TABLE tasks ADD COLUMN completed_at TEXT;

CREATE INDEX tasks_job_status ON tasks(job_id, status, created_at);
CREATE UNIQUE INDEX tasks_lease_id ON tasks(lease_id) WHERE lease_id IS NOT NULL;

PRAGMA user_version = 4;
