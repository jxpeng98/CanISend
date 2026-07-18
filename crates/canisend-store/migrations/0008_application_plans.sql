CREATE TABLE application_plan_heads (
    workflow_run_id TEXT PRIMARY KEY REFERENCES workflow_runs(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL REFERENCES artifacts(id),
    artifact_revision INTEGER NOT NULL CHECK (artifact_revision > 0),
    decision TEXT NOT NULL CHECK (decision IN ('apply', 'hold', 'skip')),
    blocking_count INTEGER NOT NULL CHECK (blocking_count >= 0),
    updated_at TEXT NOT NULL,
    FOREIGN KEY (artifact_id, artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision)
) STRICT;

PRAGMA user_version = 8;
