# M1-ARCH-003 — CLI application-facade hygiene

Date: 2026-08-03

## Outcome

The production CLI no longer depends directly on `canisend-io`, `canisend-resources`, or
`canisend-store`. Its remaining internal dependencies are the shared `canisend-app` facade,
versioned `canisend-contracts`, and the ADR-approved `canisend-mcp` stdio-host boundary.

The workspace policy now records 26 actual edges against the 25-edge target. The only remaining
delta is the reviewed, expiring Store→IO render/projection exception owned by M1-ARCH-001/002.

## Facade changes

- Bounded JSON candidate reads, task-completion file/stdin parsing, and create-new private JSON
  projection are exposed through `Application` and retain the IO adapter's size, regular-file,
  extension, symlink, `.canisend`, permission, and create-new checks.
- Agent skill install/status/uninstall states are re-exported by `canisend-app`, so CLI
  presentation does not name the resource implementation crate.
- The ignored release performance contract constructs generic benchmark applications and imports
  its HTML fixture through public application operations rather than opening Store or normalizing
  input through IO directly.
- The stale normal Store dependency was removed rather than reclassified as a dev dependency.

## Verification contract

The candidate IO regression proves create-new behavior and rejects writes into private workspace
state. CLI tests and Clippy compile the adapter without concrete Store/IO/Resources dependencies.
`xtask architecture graph-check` derives locked Cargo metadata and requires exact agreement with
the 26-edge graph; reintroducing any removed edge fails `release check`.

## Remaining boundary

M1-ARCH-003 is implemented in source. M1B remains open until the Store→IO ownership decision and
renderer/projector failure, stale revision, CAS cleanup, and repair-convergence evidence are
complete. No version transition, release candidate, push, tag, or publication occurred.
