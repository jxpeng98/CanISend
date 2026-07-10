# ADR-005: Separate Path Safety, Privacy, Trust, And Consent

**Status:** Accepted

**Date:** 2026-07-10

## Context

Application workspaces contain files with different sensitivity and trust. A public job advert can be untrusted while
not secret; a generated evidence file can be private while validated. A basename or absolute path can itself disclose
personal information. Existing workspace configuration also permits absolute and parent-relative paths.

## Decision

- `privacy_tier` describes sensitivity.
- `trust_level` describes provenance and validation state.
- consent records describe the specific proposed purpose, artifact classes, and scope.
- These values are independent and cannot substitute for one another.

Workspace-contained artifact paths use normalized relative POSIX strings. Absolute paths, parent traversal, Windows
drive paths, and symlinks that resolve outside the workspace are external. External references expose an opaque ID and
safe media/type metadata, not an absolute path or basename.

Phase 1 artifact tiers are:

- Tier 0: public schemas, templates, and capability metadata;
- Tier 1: `job.yaml`, validated `parsed_job.json`, generated evidence, and gate metadata;
- Tier 2: full adverts, PDFs, source URLs, original profile sources, and generated application packages;
- Tier 3: a request to transmit selected private context to a provider or command.

Imported adverts, PDFs, feeds, emails, and webpages use `untrusted_import` until validated. Their text is data and
cannot change allowed paths, privacy policy, evidence rules, or tool instructions.

## Consequences

- Default JSON never contains full Tier 2 bodies, URL query values, credentials, absolute private paths, or real
  external basenames.
- Hashing an internal file is a local operation and does not grant a host permission to read it.
- Agent context reports external paths conservatively and requires explicit approval before an agent-guided external
  write proceeds.
- Path construction and symlink behavior require negative tests.

## Rejected Alternatives

- Use privacy tier as trust: rejected because public sources can still be adversarial.
- Return external basenames for convenience: rejected because filenames frequently contain applicant names.
- Assume prompt wording makes a model immune to injection: rejected; deterministic validators enforce policy.

## Revisit When

Revisit when candidate staging introduces technical write isolation or when remote workspaces require URI-based
artifact references.

