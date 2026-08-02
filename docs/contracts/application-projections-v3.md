# CanISend Application projection v3 contract

**Projection format:** `canisend.application-projections/v3`

**Authority:** immutable `canisend.application-model/v3` revisions and referenced SHA-256 Blobs

**Managed root:** `applications/APPLICATION_ID/`

## Projection set

For the current Application revision, the Store deterministically publishes:

- `application.json`, containing the complete neutral Application-model snapshot;
- `deliverables/DELIVERABLE_ID/deliverable.json`, containing one Deliverable record; and
- `deliverables/DELIVERABLE_ID/content.EXT`, when the Deliverable binds materialized content.

The bounded extension is selected from the declared media type. Unknown media types use `.bin`.
Every manifest row binds the Application revision and snapshot digest, exact Pack ID/version/digest,
optional Deliverable revision, source digest, generated digest, observed digest, and edit status.
Projection files never become authoritative and projection operations never submit an Application.

## Publication and ownership

Before claiming any path, projection validates the complete Application and every referenced content
Blob, generates all bytes, and preflights every destination and parent. Existing unmanaged files,
non-directory parents, symbolic links, and edited managed files fail closed before the manifest
transaction. The transaction rechecks the current Application revision and records every path as
managed and missing. The authoritative Application revision commit has already registered each
materialized content identity in the Blob-reference ledger.

Files are then published with the existing atomic projection writer. A later write failure leaves
the recorded path missing or repair-required, so recovery can converge without changing the
Application. A concurrent Application head change is reported as stale.

## Observation and reconciliation

Inspection recalculates filesystem digests and records exactly one of `current`, `edited`,
`missing`, or `repair-required`.

- `repair_all` rebuilds only missing or repair-required managed paths from the exact recorded
  immutable Application revision and verified Blob. It preserves current and edited files.
- `replace` explicitly discards one managed edit and regenerates the canonical file.
- `copy_as_new` requires an edited managed file, creates a new user-owned path below the same
  Application tree, and then regenerates the managed path.

Neither reconciliation action changes authoritative revisions. A changed generation recipe or a
missing/corrupt authoritative Blob fails closed instead of silently producing different bytes.

## Legacy recognition

A migrated academic Application recognizes old `jobs/JOB_ID/` projections only through the
one-to-one migration link and pre-existing v2 projection manifests. The Store does not scan either
`jobs/` or `applications/`, infer ownership from a directory name, re-own a legacy path, or alter
legacy bytes during recognition. Unmanaged legacy files are not included in the catalog.

## Backup and restore

Projection files remain excluded from backups. Materialized Deliverable content is included through
the Blob-reference ledger. Restore creates the neutral projection root and regenerates both v2 and
v3 managed projections inside staging before atomically publishing the recovered Workspace.
