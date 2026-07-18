CREATE TABLE review_heads (
    workflow_run_id TEXT PRIMARY KEY REFERENCES workflow_runs(id) ON DELETE CASCADE,
    document_set_artifact_id TEXT NOT NULL,
    document_set_artifact_revision INTEGER NOT NULL CHECK (document_set_artifact_revision > 0),
    artifact_id TEXT NOT NULL,
    artifact_revision INTEGER NOT NULL CHECK (artifact_revision > 0),
    deterministic_blocker_count INTEGER NOT NULL CHECK (deterministic_blocker_count >= 0),
    pending_human_count INTEGER NOT NULL CHECK (pending_human_count >= 0),
    updated_at TEXT NOT NULL,
    FOREIGN KEY (document_set_artifact_id, document_set_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (artifact_id, artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision)
) STRICT;

PRAGMA user_version = 10;
