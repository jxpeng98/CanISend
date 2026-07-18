CREATE TABLE package_heads (
    workflow_run_id TEXT PRIMARY KEY REFERENCES workflow_runs(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL,
    artifact_revision INTEGER NOT NULL CHECK (artifact_revision > 0),
    plan_artifact_id TEXT NOT NULL,
    plan_artifact_revision INTEGER NOT NULL CHECK (plan_artifact_revision > 0),
    evidence_artifact_id TEXT NOT NULL,
    evidence_artifact_revision INTEGER NOT NULL CHECK (evidence_artifact_revision > 0),
    profile_revision INTEGER NOT NULL CHECK (profile_revision > 0),
    document_set_artifact_id TEXT NOT NULL,
    document_set_artifact_revision INTEGER NOT NULL CHECK (document_set_artifact_revision > 0),
    review_artifact_id TEXT NOT NULL,
    review_artifact_revision INTEGER NOT NULL CHECK (review_artifact_revision > 0),
    readiness_state TEXT NOT NULL,
    checked_at TEXT NOT NULL,
    FOREIGN KEY (artifact_id, artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (plan_artifact_id, plan_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (evidence_artifact_id, evidence_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (document_set_artifact_id, document_set_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (review_artifact_id, review_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision)
) STRICT;

PRAGMA user_version = 11;
