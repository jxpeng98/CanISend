CREATE TABLE workspace_v3_authority (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    workspace_format TEXT NOT NULL CHECK (workspace_format = 'canisend.workspace/v3'),
    activated_at TEXT NOT NULL,
    reason TEXT NOT NULL
) STRICT;

CREATE TABLE application_model_v3_heads (
    application_id TEXT PRIMARY KEY,
    authority INTEGER NOT NULL DEFAULT 1 REFERENCES workspace_v3_authority(singleton),
    opportunity_id TEXT NOT NULL,
    pack_id TEXT NOT NULL,
    pack_version TEXT NOT NULL,
    pack_digest TEXT NOT NULL CHECK (
        length(pack_digest) = 64 AND
        pack_digest NOT GLOB '*[^0-9a-f]*'
    ),
    head_revision INTEGER NOT NULL CHECK (head_revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX application_model_v3_heads_opportunity
    ON application_model_v3_heads(opportunity_id, application_id);

CREATE INDEX application_model_v3_heads_pack
    ON application_model_v3_heads(pack_id, pack_version, pack_digest);

CREATE TABLE application_model_v3_revisions (
    application_id TEXT NOT NULL REFERENCES application_model_v3_heads(application_id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    snapshot_json TEXT NOT NULL CHECK (json_valid(snapshot_json)),
    snapshot_sha256 TEXT NOT NULL CHECK (
        length(snapshot_sha256) = 64 AND
        snapshot_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (application_id, revision)
) STRICT;

CREATE TABLE application_model_v3_dependencies (
    application_id TEXT NOT NULL,
    application_revision INTEGER NOT NULL CHECK (application_revision > 0),
    dependent_kind TEXT NOT NULL CHECK (dependent_kind IN ('plan', 'deliverable')),
    dependent_id TEXT NOT NULL,
    dependent_revision INTEGER NOT NULL CHECK (dependent_revision > 0),
    upstream_kind TEXT NOT NULL CHECK (upstream_kind IN ('requirement', 'plan', 'evidence')),
    upstream_id TEXT NOT NULL,
    upstream_revision INTEGER NOT NULL CHECK (upstream_revision > 0),
    PRIMARY KEY (
        application_id,
        application_revision,
        dependent_kind,
        dependent_id,
        upstream_kind,
        upstream_id
    ),
    FOREIGN KEY (application_id, application_revision)
        REFERENCES application_model_v3_revisions(application_id, revision) ON DELETE CASCADE
) STRICT;

CREATE INDEX application_model_v3_dependencies_upstream
    ON application_model_v3_dependencies(
        upstream_kind,
        upstream_id,
        upstream_revision,
        application_id
    );

PRAGMA user_version = 14;
