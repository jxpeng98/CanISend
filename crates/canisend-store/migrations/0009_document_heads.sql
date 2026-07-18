CREATE TABLE document_heads (
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    plan_artifact_id TEXT NOT NULL,
    plan_artifact_revision INTEGER NOT NULL CHECK (plan_artifact_revision > 0),
    planned_document_id TEXT NOT NULL,
    planned_document_revision INTEGER NOT NULL CHECK (planned_document_revision > 0),
    kind TEXT NOT NULL CHECK (
        kind IN ('cover-letter', 'research-statement', 'teaching-statement', 'cv')
    ),
    artifact_id TEXT NOT NULL,
    artifact_revision INTEGER NOT NULL CHECK (artifact_revision > 0),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (workflow_run_id, planned_document_id),
    UNIQUE (workflow_run_id, kind),
    FOREIGN KEY (plan_artifact_id, plan_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (artifact_id, artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision)
) STRICT;

PRAGMA user_version = 9;
