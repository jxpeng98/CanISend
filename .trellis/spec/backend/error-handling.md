# Error Handling

> How failures remain typed and consistent across product surfaces.

---

## Overview

Errors remain typed at their owning layer and are converted once at the shared application
boundary. Fail closed before mutation at version, path, consent, integrity, and revision
boundaries.

## Error Types

- Use `thiserror` enums such as `StoreError`, `IoAdapterError`, and `ApplicationError`.
- Preserve sources with `#[from]` or `#[source]`; attach paths and bounded context where useful.
- Public adapters consume `ApplicationFailure` with stable `ErrorCode`, retryability, details, and
  remediation instead of inventing host-specific classifications.

## Error Handling Patterns

- Propagate with `?` until the layer that owns classification or remediation.
- Validate all inputs before authoritative writes; transaction failures must roll back atomically.
- Treat stale revisions, replayed approval tokens, wrong Application/Pack context, and denied
  consent as expected typed failures, not generic internal errors.

## Adapter Responses

CLI, MCP, Tauri, and GUI must preserve the `ApplicationFailure` meaning from
`crates/canisend-app/src/error.rs`. Transport envelopes may differ, but operation code,
retryability, no-mutation behavior, and remediation must agree.

## Common Mistakes

- Do not map every SQLite or filesystem failure to retryable; classify at `canisend-app`.
- Do not expose private source bodies, credentials, or internal paths in routine diagnostics.
- Do not catch an integrity or unsupported-version error and continue with partial behavior.

Examples: `crates/canisend-store/src/lib.rs`, `crates/canisend-app/src/error.rs`, and the approval
brokers under `crates/canisend-app/src/`.
