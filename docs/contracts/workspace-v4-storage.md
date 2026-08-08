# CanISend Workspace v4 storage contract

**Authority format:** `canisend.workspace/v4`

**Native database schema:** 20

**Pack authority:** exact Application-level Pack ID, semantic version, and content digest

## Boundary

Workspace v4 is one neutral container for any number of Applications. It has no academic,
generic, or other domain mode. Every Application owns its exact Pack binding and pack-defined
metadata; Workspace-scoped Sources, Profile Sources, and Evidence become visible to an
Application only through an explicit revision-and-digest-bound association.

Schema migration 20 adds the native v4 tables without copying or reinterpreting v2/v3 data. A
clean Alpha.7 Workspace is created directly at schema 20 and does not create a
`workspace_v3_authority` row. A pre-Alpha.7 Workspace v4 carrying the retired v3 storage bridge is
unsupported: the v4 opener reads the SQLite header, returns a compatibility boundary, and does
not open, migrate, or mutate the database. The supported recovery path is a new clean Workspace
and explicit import of reviewed Sources.

## Native authority tables

- `application_v4_heads` stores Application and Opportunity identities, exact Pack ID/version/
  digest, the positive current revision, and timestamps. Its foreign key and trigger require the
  singleton Workspace v4 authority.
- `application_v4_revisions` stores immutable snapshot JSON, its recalculated SHA-256, actor,
  bounded reason code, and commit time.
- `application_v4_dependencies` stores revision-specific Plan-to-Requirement and
  Deliverable-to-Plan/Evidence edges.
- `application_projection_v4_manifests` binds each managed projection to one Application
  revision, snapshot digest, exact Pack binding, generated/observed digests, and edit state.
- `application_pack_v4_migrations` binds reviewed source and target Pack versions/digests,
  manifest digests, preview digest, invalidation result, and the two immutable Application
  revisions.
- `application_source_v4_associations`, `application_profile_v4_associations`, and
  `application_evidence_v4_associations` bind one selected Application to one exact resource
  revision and digest with the applicable consent scope.

The historical v3 tables remain append-only migration history for unsupported v2/v3 surfaces.
Native v4 repository, association, Pack-migration, projection, status, repair, backup, and restore
paths select only the native tables. Tests assert that mixed academic/generic creation, revision,
association, projection, and Pack migration leave every v3 Application table empty.

## Pack and isolation invariants

Pack columns are `NOT NULL` and digest-constrained in SQLite. Rust structural and semantic
validation requires the same binding on the Application, Opportunity, Requirements, Plan, and
Deliverables. Reads recalculate the snapshot digest and compare head identity, revision,
Opportunity, and Pack fields before returning data. Ordinary commits reject Pack substitution;
only the reviewed Pack-migration boundary may advance the version and digest.

All writes use immediate transactions with exact expected revisions. A missing or mismatched Pack,
stale revision, wrong association digest, consent denial, projection conflict, or database error
rolls back the complete write. Queries start from one selected Application ID, so a failure cannot
fall through to another Application or expose another Application's associations.

## Application removal

Workspace v4 uses an audited logical deletion instead of physically deleting Application history.
The dedicated `application.archive` boundary requires the exact current revision and atomically
commits a new immutable snapshot in which both the Application lifecycle and its Opportunity are
archived. Ordinary model commits cannot enter the archived state or revive it; repeating the same
archive at the current revision is idempotent and does not add another revision or audit event.

Archival preserves immutable revisions, Pack identity, Blob references, projections, and explicit
Source/Profile/Evidence associations for recovery and audit. It never deletes Workspace-scoped or
shared data. Mixed-Pack tests archive each Application in turn and compare the other Application's
complete stored snapshot, digest, revision, lifecycle, and Pack binding before and after.

## Recovery and compatibility

Workspace check, backup, restore-to-new-path, and projection repair operate over the table family
selected by the authoritative Workspace format. Immutable bodies remain content-addressed Blobs
registered in the shared Blob-reference ledger. Unsupported Workspace formats and retired bridge
storage return stable, body-free remediation before the v4 database opener runs.
