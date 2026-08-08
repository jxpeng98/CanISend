# Application associations v4

Workspace v4 stores reusable Sources, Profile Sources, and confirmed Evidence at Workspace scope.
None of those records is implicitly visible to an Application. Their use requires a typed,
revision-bound association created through the Rust application boundary.

## Authority

SQLite owns Source revision metadata and the three association families. Immutable,
content-addressed Blobs own original and normalized Source bodies. An association records the
selected Application ID, resource ID, exact revision, exact digest, applicable consent scope, and
association time. The database foreign keys bind all three identity fields to the selected
resource revision.

The three association families are deliberately separate:

- Application to Source;
- Application to Profile Source; and
- Application to confirmed, non-excluded Evidence.

A polymorphic resource name is not an authority and cannot substitute one family for another.

## Consent and isolation

Pasted text is already an explicit user input and can be attached as part of the Application
creation transaction. Reading a private local file, Profile Source, or Evidence revision requires
the exact private-input consent before its association is inserted. A denied or mismatched consent
leaves the association and audit tables unchanged.

Queries always begin with one selected Application ID. A Workspace-level record that has no link
to that Application is not returned, even when another Application links the same record.

## Revision and deletion rules

Associations never float to a newer resource revision. When a Source or linked record advances,
the existing link is reported as stale and must be reviewed, unlinked, and explicitly rebound.
Unrelated Applications and links remain current.

A linked Source cannot be deleted. After every Application link is explicitly removed, its Source
metadata and Blob references may be deleted; immutable Blob garbage collection remains a separate
bounded maintenance operation. Every associate, unlink, revision, and delete operation writes a
body-free audit event.

## Application creation

The current neutral Application flow stores its pasted Source, exact Requirement spans,
Application snapshot, Blob references, typed Source association, and audit event in one database
transaction after the prepared Blob digest is verified. Validation failures occur before Blob or
database mutation, and database failures cannot leave a partial Application association.
