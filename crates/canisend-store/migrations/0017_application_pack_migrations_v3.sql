CREATE TABLE application_pack_v3_migrations (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL,
    from_application_revision INTEGER NOT NULL CHECK (from_application_revision > 0),
    to_application_revision INTEGER NOT NULL CHECK (
        to_application_revision = from_application_revision + 1
    ),
    pack_id TEXT NOT NULL,
    from_pack_version TEXT NOT NULL,
    from_pack_digest TEXT NOT NULL CHECK (
        length(from_pack_digest) = 64 AND
        from_pack_digest NOT GLOB '*[^0-9a-f]*'
    ),
    to_pack_version TEXT NOT NULL CHECK (to_pack_version <> from_pack_version),
    to_pack_digest TEXT NOT NULL CHECK (
        length(to_pack_digest) = 64 AND
        to_pack_digest NOT GLOB '*[^0-9a-f]*'
    ),
    source_manifest_sha256 TEXT NOT NULL CHECK (
        length(source_manifest_sha256) = 64 AND
        source_manifest_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    target_manifest_sha256 TEXT NOT NULL CHECK (
        length(target_manifest_sha256) = 64 AND
        target_manifest_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    preview_sha256 TEXT NOT NULL CHECK (
        length(preview_sha256) = 64 AND
        preview_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    plan_invalidated INTEGER NOT NULL CHECK (plan_invalidated IN (0, 1)),
    stale_deliverable_count INTEGER NOT NULL CHECK (stale_deliverable_count >= 0),
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CHECK (to_pack_digest <> from_pack_digest),
    CHECK (target_manifest_sha256 <> source_manifest_sha256),
    UNIQUE (application_id, to_application_revision),
    FOREIGN KEY (application_id, from_application_revision)
        REFERENCES application_model_v3_revisions(application_id, revision),
    FOREIGN KEY (application_id, to_application_revision)
        REFERENCES application_model_v3_revisions(application_id, revision)
) STRICT;

CREATE INDEX application_pack_v3_migrations_application
    ON application_pack_v3_migrations(application_id, to_application_revision);

PRAGMA user_version = 17;
