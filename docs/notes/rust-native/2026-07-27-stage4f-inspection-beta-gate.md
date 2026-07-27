# Stage 4F inspection parity and Beta gate

**Date:** 2026-07-27

**Decision:** Stage 4F implementation is complete at `35/35`; the source remains
`1.0.0-alpha.2` until a separately authorized Beta transition.

## Delivered surface

- `canisend-resources` verifies the compiled catalog and exports a versioned manifest plus exact
  public files using create-new and rollback-safe writes.
- `canisend-app` owns deterministic schema/resource list, detail, combined inspection, and export
  actions that do not require a workspace.
- Existing CLI schema/resource output now routes through the shared application facade.
- Diagnostics provides explicit background loading, schema/resource filters, copyable bounded
  metadata, a new-or-empty destination preview, and an exact-file export receipt in English and
  Simplified Chinese.
- The GUI never renders unbounded resource bodies, exports workspace bodies, launches an Agent
  host, or exposes a shell.

## Qualification

The Stage 4F checkout passed:

- `cargo fmt --all -- --check`;
- `cargo test --workspace --locked`: 243 passed, 4 intentionally ignored, 0 failed;
- `cargo clippy --workspace --all-targets --locked -- -D warnings`;
- `cargo run -p xtask --locked -- release check`, including 40 schemas, verified resources, and
  `35 implemented` with no deferred Beta operation;
- the dedicated no-workspace catalog regression, including manifest/digest round trip and repeated
  export refusal; and
- an optimized, ad-hoc-signed macOS app smoke covering final-byte bundle verification, English and
  Simplified Chinese semantics, keyboard Tab order, 200% text/focus visibility, and reduced
  motion.

The debug GUI test link still emits the known macOS `__eh_frame section too large` warning. It
does not fail linking or any test; release linking and the staged application both completed.

## Public Alpha audit

The audit read only public GitHub issue number/state metadata. It did not read issue titles,
bodies, comments, attachments, telemetry, or workspace data. The snapshot contained zero public
issues and therefore zero open release blockers.

[`release/beta-readiness.json`](../../../release/beta-readiness.json) records the public
[`v1.0.0-alpha.3`](https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.3) identity,
source commit `cebf59629d615d4b950b5823524922b56bde68ce`, successful native candidate run
`30305363882`, build-once promotion run `30306912005`, four clear blocker classes, and disabled
default telemetry.

[`release/beta-contract-freeze.json`](../../../release/beta-contract-freeze.json) binds the
Agent v2 snapshots, all 40 public schemas, workspace v2, and migrations through schema 13 to that
Alpha.3 baseline.

## Exact remaining Beta gate

Stage 4F does not authorize a version change, tag, publication, package-manager update, or native
release matrix. Before an Alpha-to-Beta write:

1. refresh Beta readiness so its public issue snapshot is less than 24 hours old;
2. run the source gate and the name-only signing configuration audit;
3. explicitly authorize and review the dry-run-first `prepare-stage v1.0.0-beta.1` transition;
4. qualify the exact Beta native artifacts and record that evidence; and
5. activate whole-product feature freeze against the exact qualified Beta commit.

Until those actions are authorized and pass, the qualification ledger remains `pre-beta`, feature
freeze remains `planned`, and Stable authorization remains false.
