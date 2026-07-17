ALTER TABLE workspace_metadata ADD COLUMN profile_revision INTEGER NOT NULL DEFAULT 0
    CHECK (profile_revision >= 0);

CREATE TABLE profile_sources (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE profile_source_revisions (
    source_id TEXT NOT NULL REFERENCES profile_sources(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision > 0),
    sha256 TEXT NOT NULL,
    original_artifact_id TEXT NOT NULL,
    original_artifact_revision INTEGER NOT NULL CHECK (original_artifact_revision > 0),
    normalized_artifact_id TEXT NOT NULL,
    normalized_artifact_revision INTEGER NOT NULL CHECK (normalized_artifact_revision > 0),
    content_type TEXT NOT NULL,
    sensitivity TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (source_id, revision),
    FOREIGN KEY (original_artifact_id, original_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision),
    FOREIGN KEY (normalized_artifact_id, normalized_artifact_revision)
        REFERENCES artifact_revisions(artifact_id, revision)
) STRICT;

CREATE INDEX profile_sources_created ON profile_sources(created_at, id);

PRAGMA user_version = 6;
