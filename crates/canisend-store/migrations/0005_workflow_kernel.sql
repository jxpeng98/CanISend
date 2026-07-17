ALTER TABLE workflow_runs ADD COLUMN job_revision INTEGER CHECK (job_revision > 0);

ALTER TABLE stage_executions ADD COLUMN execution_mode TEXT;
ALTER TABLE stage_executions ADD COLUMN output_artifact_id TEXT REFERENCES artifacts(id);
ALTER TABLE stage_executions ADD COLUMN output_artifact_revision INTEGER CHECK (output_artifact_revision > 0);
ALTER TABLE stage_executions ADD COLUMN started_at TEXT;
ALTER TABLE stage_executions ADD COLUMN completed_at TEXT;
ALTER TABLE stage_executions ADD COLUMN updated_at TEXT;

CREATE UNIQUE INDEX workflow_stage_unique ON stage_executions(workflow_run_id, stage);
CREATE INDEX workflow_runs_job_status ON workflow_runs(job_id, status, created_at);

PRAGMA user_version = 5;
