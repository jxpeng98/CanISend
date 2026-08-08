CREATE TABLE workspace_source_v4_heads (
    source_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (
        kind IN ('pasted-text', 'local-file', 'text-pdf', 'url')
    ),
    head_revision INTEGER NOT NULL CHECK (head_revision > 0),
    created_at TEXT NOT NULL,
    UNIQUE (source_id, head_revision)
) STRICT;

CREATE TABLE workspace_source_v4_revisions (
    source_id TEXT NOT NULL REFERENCES workspace_source_v4_heads(source_id) ON DELETE RESTRICT,
    revision INTEGER NOT NULL CHECK (revision > 0),
    locator TEXT NOT NULL,
    content_type TEXT NOT NULL,
    original_sha256 TEXT NOT NULL CHECK (
        length(original_sha256) = 64 AND
        original_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    normalized_sha256 TEXT NOT NULL CHECK (
        length(normalized_sha256) = 64 AND
        normalized_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    original_bytes INTEGER NOT NULL CHECK (original_bytes >= 0),
    normalized_text_bytes INTEGER NOT NULL CHECK (normalized_text_bytes >= 0),
    privacy TEXT NOT NULL CHECK (
        privacy IN ('public', 'private-local', 'provider-bound', 'secret')
    ),
    created_at TEXT NOT NULL,
    PRIMARY KEY (source_id, revision),
    UNIQUE (source_id, revision, normalized_sha256)
) STRICT;

CREATE INDEX workspace_source_v4_revisions_digest
    ON workspace_source_v4_revisions(normalized_sha256, source_id, revision);

CREATE TABLE application_source_associations_v4 (
    application_id TEXT NOT NULL REFERENCES application_model_v3_heads(application_id)
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

CREATE INDEX application_source_associations_v4_source
    ON application_source_associations_v4(source_id, source_revision, application_id);

CREATE UNIQUE INDEX profile_source_revisions_v4_exact
    ON profile_source_revisions(source_id, revision, sha256);

CREATE TABLE application_profile_associations_v4 (
    application_id TEXT NOT NULL REFERENCES application_model_v3_heads(application_id)
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

CREATE INDEX application_profile_associations_v4_source
    ON application_profile_associations_v4(
        profile_source_id, profile_source_revision, application_id
    );

CREATE UNIQUE INDEX evidence_revisions_v4_exact
    ON evidence_revisions(evidence_id, revision, sha256);

CREATE TABLE application_evidence_associations_v4 (
    application_id TEXT NOT NULL REFERENCES application_model_v3_heads(application_id)
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

CREATE INDEX application_evidence_associations_v4_evidence
    ON application_evidence_associations_v4(evidence_id, evidence_revision, application_id);

PRAGMA user_version = 18;
