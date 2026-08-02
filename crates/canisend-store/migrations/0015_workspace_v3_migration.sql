CREATE TABLE workspace_v3_migrations (
    id TEXT PRIMARY KEY,
    source_workspace_format TEXT NOT NULL CHECK (
        source_workspace_format = 'canisend.workspace/v2'
    ),
    target_workspace_format TEXT NOT NULL CHECK (
        target_workspace_format = 'canisend.workspace/v3'
    ),
    source_schema_version INTEGER NOT NULL CHECK (source_schema_version > 0),
    target_schema_version INTEGER NOT NULL CHECK (target_schema_version > 0),
    pack_id TEXT NOT NULL,
    pack_version TEXT NOT NULL,
    pack_digest TEXT NOT NULL CHECK (
        length(pack_digest) = 64 AND
        pack_digest NOT GLOB '*[^0-9a-f]*'
    ),
    preview_sha256 TEXT NOT NULL CHECK (
        length(preview_sha256) = 64 AND
        preview_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    source_inventory_sha256 TEXT NOT NULL CHECK (
        length(source_inventory_sha256) = 64 AND
        source_inventory_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    source_inventory_count INTEGER NOT NULL CHECK (source_inventory_count >= 0),
    referenced_blob_count INTEGER NOT NULL CHECK (referenced_blob_count >= 0),
    referenced_blob_bytes INTEGER NOT NULL CHECK (referenced_blob_bytes >= 0),
    backup_manifest_sha256 TEXT NOT NULL CHECK (
        length(backup_manifest_sha256) = 64 AND
        backup_manifest_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    started_at TEXT NOT NULL,
    completed_at TEXT NOT NULL
) STRICT;

CREATE TABLE workspace_v3_application_links (
    migration_id TEXT NOT NULL REFERENCES workspace_v3_migrations(id),
    legacy_job_id TEXT NOT NULL REFERENCES jobs(id),
    opportunity_id TEXT NOT NULL,
    application_id TEXT NOT NULL REFERENCES application_model_v3_heads(application_id),
    PRIMARY KEY (migration_id, legacy_job_id),
    UNIQUE (migration_id, opportunity_id),
    UNIQUE (migration_id, application_id)
) STRICT;

CREATE TABLE workspace_v3_legacy_bindings (
    migration_id TEXT NOT NULL REFERENCES workspace_v3_migrations(id),
    source_table TEXT NOT NULL,
    source_key_sha256 TEXT NOT NULL CHECK (
        length(source_key_sha256) = 64 AND
        source_key_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    row_sha256 TEXT NOT NULL CHECK (
        length(row_sha256) = 64 AND
        row_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    application_id TEXT REFERENCES application_model_v3_heads(application_id),
    pack_id TEXT NOT NULL,
    pack_version TEXT NOT NULL,
    pack_digest TEXT NOT NULL CHECK (
        length(pack_digest) = 64 AND
        pack_digest NOT GLOB '*[^0-9a-f]*'
    ),
    PRIMARY KEY (migration_id, source_table, source_key_sha256)
) STRICT;

CREATE INDEX workspace_v3_legacy_bindings_application
    ON workspace_v3_legacy_bindings(application_id, source_table);

PRAGMA user_version = 15;
