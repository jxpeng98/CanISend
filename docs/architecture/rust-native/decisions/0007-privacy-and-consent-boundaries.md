# ADR-RN-0007: Classify Data and Require Explicit Provider-Bound Consent

**Status:** Accepted

**Date:** 2026-07-17

## Context

CanISend processes private profile evidence, job material, strategy, drafts, and reviews. Host agents may already have
access to workspace files, while configured remote providers require a deliberate transmission boundary. Logs and
control responses must remain useful without copying private bodies.

## Decision

Data is classified as:

- `public`: versions, capability names, schema IDs, adapter IDs, and documentation.
- `private-local`: job adverts, evidence, strategies, drafts, reviews, and provider results.
- `provider-bound`: the exact private-local artifact revisions approved for one remote call.
- `secret`: provider tokens and credentials.

Rules:

- Secrets are read from environment variables or an approved OS credential integration, never workspace files.
- Normal logs, audit events, errors, and body-free agent context omit private bodies.
- A task descriptor declares private read scope.
- Configured-provider calls require explicit consent for the exact provider-bound manifest.
- Provider responses pass through the same candidate validator as host-agent responses.
- Telemetry and crash upload are off by default.
- Readiness is never represented as submission evidence.

## Consequences

- Application services must pass safe references rather than arbitrary document strings in normal responses.
- Provider tests inspect the exact serialized payload and redaction behavior.
- Workspace permission checks are best effort across platforms and do not replace user device security.
- Exported documents become user-managed private data.

## Rejected alternatives

- Treat all local agent access as provider consent: rejected because remote transmission is a distinct action.
- Persistent blanket provider consent in the first release: rejected because scope and revocation policy are not yet
  mature.
- Log full provider requests for debugging: rejected because logs would become a private-data replica.

## Revisit when

Revisit persistent consent only with an explicit, reviewable policy model and clear revocation semantics.
