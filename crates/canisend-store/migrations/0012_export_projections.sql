ALTER TABLE projection_manifests RENAME TO projection_manifests_v1;

CREATE TABLE projection_manifests (
    artifact_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    projection_kind TEXT NOT NULL,
    generated_sha256 TEXT NOT NULL,
    observed_sha256 TEXT,
    status TEXT NOT NULL CHECK (status IN ('current', 'edited', 'missing', 'repair-required')),
    last_error TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (artifact_id, revision, relative_path),
    UNIQUE (relative_path),
    FOREIGN KEY (artifact_id, revision)
        REFERENCES artifact_revisions(artifact_id, revision) ON DELETE CASCADE
) STRICT;

INSERT INTO projection_manifests(
    artifact_id, revision, relative_path, sha256, projection_kind,
    generated_sha256, observed_sha256, status, last_error, updated_at
)
SELECT artifact_id, revision, relative_path, sha256, 'raw', sha256,
       CASE WHEN status = 'current' THEN sha256 ELSE NULL END,
       status, last_error, updated_at
FROM projection_manifests_v1;

DROP TABLE projection_manifests_v1;

CREATE TABLE export_heads (
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

PRAGMA user_version = 12;
