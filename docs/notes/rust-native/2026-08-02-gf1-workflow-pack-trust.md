# GF1 workflow-pack byte trust-boundary implementation record

**Date:** 2026-08-02

**Roadmap task:** GF1-TRUST-001 core byte-boundary foundation; partial foundation for
M1F-PACK-001 and GF5-SDK-001

**State:** Implemented and source-gate verified in this change. Symlink-safe Pack-directory reads,
explicit installation/update approval, immutable storage, renderer virtual-filesystem isolation,
publisher policy, and linked work-item inspection remain required before the full roadmap task
becomes Verified.

## Implemented boundary

- Added a pure `WorkflowPackByteLoader` accepting only already-supplied Manifest and resource
  bytes; it has no filesystem, network, process, database, credential, or installation authority.
- Enforced nonempty/maximum Manifest bytes, resource count, per-resource bytes, and aggregate
  bytes before JSON decoding or trust in Manifest declarations.
- Restricted v1 resource bodies to UTF-8 text data and rejected disallowed control bytes,
  executable shebangs, and common ELF, PE, Mach-O, WebAssembly, and ZIP signatures.
- Reused the existing typed Manifest validation, runtime compatibility, safe paths, exact resource
  set/size/SHA-256, kernel capability registry, and canonical bundle-digest gates.
- Returned a `VerifiedWorkflowPackCandidate` containing the existing immutable verified bundle and
  a deterministic body-free `canisend.workflow-pack-trust-report/v1`.
- Reported origin, exact identity/version/digest, publisher declaration, Manifest/resource totals,
  sorted capability references, and each passed validation class.
- Explicitly reported publisher authentication as declaration-only, v1 signature status as not
  specified, installation as disabled, and Pack execution authority as absent.

## Defensive invariant

Digest validity is not publisher trust. A successful result means the supplied bytes are bounded,
internally consistent, runtime-compatible, data-only, and limited to kernel-registered capability
references. It does not authenticate a publisher, approve installation, grant a capability, or
judge Pack quality.

Resource content is never executed by this boundary. Static byte checks reduce accidental or
mislabelled binary/script inputs; the decisive control remains that a Pack cannot define code and
future Renderers/adapters must operate behind kernel-owned bounded ports.

## Test coverage

- successful external candidate with exact bundle identity and sorted capability report;
- report status for data-only execution, declaration-only publisher identity, absent v1 signature,
  disabled installation, and no embedded resource bodies;
- pre-parse Manifest, resource-count, and individual-resource byte limits;
- malformed JSON;
- valid multilingual UTF-8/template text;
- ELF, PE, ZIP, invalid UTF-8, control-byte, and shebang rejection;
- aggregate digest mismatch and unavailable capability propagation through the byte loader; and
- all existing unsafe-path, extension, resource-set, size/hash, runtime-compatibility, substitution,
  graph, and Deliverable catalog regressions.

## Verification

```console
cargo test -p canisend-contracts -p canisend-core --locked
cargo clippy -p canisend-contracts -p canisend-core \
  --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

The future external Pack adapter must read a user-selected directory without following symlinks,
enforce metadata and streaming limits before allocation, bind the exact reviewed candidate to an
explicit one-shot install/update approval, and store it atomically under immutable ID/version/
digest identity. Reopen must reverify the snapshot. Renderer access must be limited to the exact
verified resource map or an equivalent virtual filesystem—never arbitrary host paths or external
packages.

Publisher authentication/signatures remain a separate future policy. Adding a signature must not
grant capabilities or bypass any byte, path, consent, compatibility, digest, or renderer check.

## Rollback

Revert the byte verifier, Trust Report types/tests, shared limit constant, documentation, and
Roadmap evidence row together. Existing Workspace and Agent v2 behavior require no rollback
because no current runtime path reads or installs external Packs.
