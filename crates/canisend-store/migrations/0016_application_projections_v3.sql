CREATE TABLE application_projection_v3_manifests (
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
        REFERENCES application_model_v3_revisions(application_id, revision)
        ON DELETE CASCADE,
    CHECK (
        (projection_kind = 'application-model-json' AND
         deliverable_id IS NULL AND deliverable_revision IS NULL) OR
        (projection_kind != 'application-model-json' AND
         deliverable_id IS NOT NULL AND deliverable_revision IS NOT NULL)
    )
) STRICT;

CREATE INDEX application_projection_v3_application
    ON application_projection_v3_manifests(application_id, application_revision, relative_path);

CREATE INDEX application_projection_v3_deliverable
    ON application_projection_v3_manifests(deliverable_id, deliverable_revision);

PRAGMA user_version = 16;
