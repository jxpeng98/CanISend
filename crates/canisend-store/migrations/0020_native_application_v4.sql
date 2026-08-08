CREATE TABLE application_v4_heads (
    application_id TEXT PRIMARY KEY,
    authority INTEGER NOT NULL DEFAULT 1 CHECK (authority = 1)
        REFERENCES workspace_metadata(singleton),
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

CREATE TRIGGER application_v4_heads_require_workspace_v4
BEFORE INSERT ON application_v4_heads
WHEN (SELECT workspace_format FROM workspace_metadata WHERE singleton = 1)
    <> 'canisend.workspace/v4'
BEGIN
    SELECT RAISE(ABORT, 'application_v4_heads requires Workspace v4 authority');
END;

CREATE INDEX application_v4_heads_opportunity
    ON application_v4_heads(opportunity_id, application_id);

CREATE INDEX application_v4_heads_pack
    ON application_v4_heads(pack_id, pack_version, pack_digest);

CREATE TABLE application_v4_revisions (
    application_id TEXT NOT NULL REFERENCES application_v4_heads(application_id),
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

CREATE TABLE application_v4_dependencies (
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
        REFERENCES application_v4_revisions(application_id, revision) ON DELETE CASCADE
) STRICT;

CREATE INDEX application_v4_dependencies_upstream
    ON application_v4_dependencies(
        upstream_kind,
        upstream_id,
        upstream_revision,
        application_id
    );

CREATE TABLE application_projection_v4_manifests (
    application_id TEXT NOT NULL,
    application_revision INTEGER NOT NULL CHECK (application_revision > 0),
    snapshot_sha256 TEXT NOT NULL CHECK (
        length(snapshot_sha256) = 64 AND
        snapshot_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    pack_id TEXT NOT NULL,
    pack_version TEXT NOT NULL,
    pack_digest TEXT NOT NULL CHECK (
        length(pack_digest) = 64 AND
        pack_digest NOT GLOB '*[^0-9a-f]*'
    ),
    relative_path TEXT NOT NULL UNIQUE,
    projection_kind TEXT NOT NULL CHECK (
        projection_kind IN (
            'application-model-json',
            'deliverable-metadata-json',
            'deliverable-content'
        )
    ),
    deliverable_id TEXT,
    deliverable_revision INTEGER CHECK (deliverable_revision > 0),
    source_sha256 TEXT NOT NULL CHECK (
        length(source_sha256) = 64 AND
        source_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    generated_sha256 TEXT NOT NULL CHECK (
        length(generated_sha256) = 64 AND
        generated_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    observed_sha256 TEXT CHECK (
        observed_sha256 IS NULL OR (
            length(observed_sha256) = 64 AND
            observed_sha256 NOT GLOB '*[^0-9a-f]*'
        )
    ),
    status TEXT NOT NULL CHECK (
        status IN ('current', 'edited', 'missing', 'repair-required')
    ),
    last_error TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (application_id, application_revision, relative_path),
    FOREIGN KEY (application_id, application_revision)
        REFERENCES application_v4_revisions(application_id, revision)
        ON DELETE CASCADE,
    CHECK (
        (projection_kind = 'application-model-json' AND
         deliverable_id IS NULL AND deliverable_revision IS NULL) OR
        (projection_kind != 'application-model-json' AND
         deliverable_id IS NOT NULL AND deliverable_revision IS NOT NULL)
    )
) STRICT;

CREATE INDEX application_projection_v4_application
    ON application_projection_v4_manifests(application_id, application_revision, relative_path);

CREATE INDEX application_projection_v4_deliverable
    ON application_projection_v4_manifests(deliverable_id, deliverable_revision);

CREATE TABLE application_pack_v4_migrations (
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
        REFERENCES application_v4_revisions(application_id, revision),
    FOREIGN KEY (application_id, to_application_revision)
        REFERENCES application_v4_revisions(application_id, revision)
) STRICT;

CREATE INDEX application_pack_v4_migrations_application
    ON application_pack_v4_migrations(application_id, to_application_revision);

CREATE TABLE application_source_v4_associations (
    application_id TEXT NOT NULL REFERENCES application_v4_heads(application_id)
        ON DELETE CASCADE,
    source_id TEXT NOT NULL,
    source_revision INTEGER NOT NULL CHECK (source_revision > 0),
    source_sha256 TEXT NOT NULL CHECK (
        length(source_sha256) = 64 AND
        source_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    consent_scope TEXT CHECK (
        consent_scope IS NULL OR consent_scope IN (
            'read-private-inputs', 'send-to-configured-provider',
            'fetch-user-supplied-url', 'export-private-artifacts', 'use-system-fonts'
        )
    ),
    associated_at TEXT NOT NULL,
    PRIMARY KEY (application_id, source_id),
    FOREIGN KEY (source_id, source_revision, source_sha256)
        REFERENCES workspace_source_v4_revisions(source_id, revision, normalized_sha256)
        ON DELETE RESTRICT
) STRICT;

CREATE INDEX application_source_v4_associations_source
    ON application_source_v4_associations(source_id, source_revision, application_id);

CREATE TABLE application_profile_v4_associations (
    application_id TEXT NOT NULL REFERENCES application_v4_heads(application_id)
        ON DELETE CASCADE,
    profile_source_id TEXT NOT NULL,
    profile_source_revision INTEGER NOT NULL CHECK (profile_source_revision > 0),
    profile_source_sha256 TEXT NOT NULL CHECK (
        length(profile_source_sha256) = 64 AND
        profile_source_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    consent_scope TEXT CHECK (
        consent_scope IS NULL OR consent_scope = 'read-private-inputs'
    ),
    associated_at TEXT NOT NULL,
    PRIMARY KEY (application_id, profile_source_id),
    FOREIGN KEY (profile_source_id, profile_source_revision, profile_source_sha256)
        REFERENCES profile_source_revisions(source_id, revision, sha256) ON DELETE RESTRICT
) STRICT;

CREATE INDEX application_profile_v4_associations_source
    ON application_profile_v4_associations(
        profile_source_id, profile_source_revision, application_id
    );

CREATE TABLE application_evidence_v4_associations (
    application_id TEXT NOT NULL REFERENCES application_v4_heads(application_id)
        ON DELETE CASCADE,
    evidence_id TEXT NOT NULL,
    evidence_revision INTEGER NOT NULL CHECK (evidence_revision > 0),
    evidence_sha256 TEXT NOT NULL CHECK (
        length(evidence_sha256) = 64 AND
        evidence_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    consent_scope TEXT CHECK (
        consent_scope IS NULL OR consent_scope = 'read-private-inputs'
    ),
    associated_at TEXT NOT NULL,
    PRIMARY KEY (application_id, evidence_id),
    FOREIGN KEY (evidence_id, evidence_revision, evidence_sha256)
        REFERENCES evidence_revisions(evidence_id, revision, sha256) ON DELETE RESTRICT
) STRICT;

CREATE INDEX application_evidence_v4_associations_evidence
    ON application_evidence_v4_associations(evidence_id, evidence_revision, application_id);

PRAGMA user_version = 20;
