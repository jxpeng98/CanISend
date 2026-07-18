CREATE TABLE render_heads (
    workflow_run_id TEXT PRIMARY KEY REFERENCES workflow_runs(id) ON DELETE CASCADE,
    package_artifact_id TEXT NOT NULL,
    package_artifact_revision INTEGER NOT NULL CHECK (package_artifact_revision > 0),
    artifact_id TEXT NOT NULL,
    artifact_revision INTEGER NOT NULL CHECK (artifact_revision > 0),
    updated_at TEXT NOT NULL,
    FOREIGN KEY (package_artifact_id, package_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (artifact_id, artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision)
) STRICT;

PRAGMA user_version = 13;
