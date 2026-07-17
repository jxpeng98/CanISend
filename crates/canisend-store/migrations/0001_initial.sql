CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
) STRICT;

CREATE TABLE workspace_metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    workspace_id TEXT NOT NULL UNIQUE,
    workspace_format TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    institution TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE sources (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE source_revisions (
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision > 0),
    sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (source_id, revision)
) STRICT;

CREATE TABLE evidence_items (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE evidence_revisions (
    evidence_id TEXT NOT NULL REFERENCES evidence_items(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision > 0),
    sha256 TEXT NOT NULL,
    confirmed INTEGER NOT NULL CHECK (confirmed IN (0, 1)),
    created_at TEXT NOT NULL,
    PRIMARY KEY (evidence_id, revision)
) STRICT;

CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    head_revision INTEGER NOT NULL DEFAULT 0 CHECK (head_revision >= 0),
    stale INTEGER NOT NULL DEFAULT 0 CHECK (stale IN (0, 1)),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE artifact_revisions (
    artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision > 0),
    sha256 TEXT NOT NULL,
    size INTEGER NOT NULL CHECK (size >= 0),
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (artifact_id, revision)
) STRICT;

CREATE TABLE artifact_dependencies (
    artifact_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    depends_on_artifact_id TEXT NOT NULL,
    depends_on_revision INTEGER NOT NULL CHECK (depends_on_revision > 0),
    depends_on_sha256 TEXT NOT NULL,
    PRIMARY KEY (artifact_id, revision, depends_on_artifact_id),
    FOREIGN KEY (artifact_id, revision) REFERENCES artifact_revisions(artifact_id, revision) ON DELETE CASCADE,
    FOREIGN KEY (depends_on_artifact_id, depends_on_revision)
        REFERENCES artifact_revisions(artifact_id, revision)
) STRICT;

CREATE INDEX artifact_dependencies_upstream
    ON artifact_dependencies(depends_on_artifact_id, depends_on_revision);

CREATE TABLE blob_references (
    sha256 TEXT NOT NULL,
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    owner_revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (sha256, owner_type, owner_id, owner_revision)
) STRICT;

CREATE TABLE workflow_runs (
    id TEXT PRIMARY KEY,
    job_id TEXT REFERENCES jobs(id),
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE stage_executions (
    id TEXT PRIMARY KEY,
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    stage TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    stage_execution_id TEXT REFERENCES stage_executions(id),
    status TEXT NOT NULL,
    lease_expires_at TEXT,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE task_inputs (
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    PRIMARY KEY (task_id, artifact_id)
) STRICT;

CREATE TABLE task_results (
    task_id TEXT PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    committed_at TEXT NOT NULL
) STRICT;

CREATE TABLE consents (
    id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    actor TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    granted_at TEXT NOT NULL
) STRICT;

CREATE TABLE audit_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    subject_revision INTEGER,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE projection_manifests (
    artifact_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('current', 'repair-required')),
    last_error TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (artifact_id, revision, relative_path),
    FOREIGN KEY (artifact_id, revision) REFERENCES artifact_revisions(artifact_id, revision) ON DELETE CASCADE
) STRICT;

CREATE TABLE discovery_sources (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    configuration_sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE job_leads (
    id TEXT PRIMARY KEY,
    discovery_source_id TEXT REFERENCES discovery_sources(id),
    canonical_key TEXT NOT NULL,
    source_sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE provider_invocations (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    request_manifest_sha256 TEXT NOT NULL,
    response_sha256 TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

PRAGMA user_version = 1;
