# R9.3 PDF Output Checkpoint

**Date:** 2026-07-18

**Branch:** `rewrite/rust-native`

**Commits:** `687dc5b` (render contracts), `359b311` (render implementation)

## Outcome

R9.3 completes the production Render stage. A current package can now be compiled entirely by the standalone Rust
binary, stored as immutable Typst/PDF artifacts, inspected through a revision-bound manifest, and exported only after
explicit private-artifact consent. No Python, Pytest, Typst executable, runtime package download, network request, or
system-font scan participates in this path.

## Trust boundary

`package export` deliberately creates editable `.typ` projections. They are useful workspace files but are not
authoritative inputs. `render build` loads the exact structured document artifacts frozen by the current package,
regenerates escaped Typst in memory, and sends only that regenerated source to the restricted compiler. The
end-to-end test replaces a managed `.typ` file with a filesystem-read expression and proves that rendering still
succeeds from structured authority rather than executing the edit.

The compiler continues to expose only an in-memory main file, embedded fonts, and embedded templates. Error strings
contain bounded categories and counts rather than private source bodies, file names, or diagnostic excerpts.

## Durable artifact graph

SQLite migration 13 adds `render_heads`, binding one workflow run and exact package artifact to its current
`render-manifest` artifact. Each manifest entry freezes:

- structured document kind and artifact revision/hash;
- regenerated `typst-source` artifact revision/hash;
- validated `pdf` artifact revision/hash;
- page count, byte count, warning count, and elapsed milliseconds.

All documents compile and all output blobs are verified before the database transaction begins. One immediate
transaction re-verifies the package/documents, inserts Typst and PDF artifact metadata/dependencies, inserts the
manifest, completes the Render stage, updates the head, writes the audit event, and marks the workflow complete.
Process interruption before commit may leave an unreferenced content-addressed blob, but cannot expose a partial
render as current.

Profile, evidence, plan, review, package, job-revision, and scoped workflow rerun paths now invalidate the render head
and stale the prior manifest. Repeating `render build` for an unchanged package returns the same artifact and
manifest.

## PDF validation and export

The embedded compiler output is parsed again with `lopdf`. CanISend rejects malformed, encrypted, zero-page,
over-page-limit, and over-size outputs before commit. Export reads each blob through SHA-256 verification, repeats
PDF validation, and compares page/byte metadata with the manifest.

`render export` checks `--allow-private-export` before opening the workspace. The destination must be a safe
`jobs/JOB_ID/` path whose final directory is new or empty; every existing path component must be a real directory,
not a symlink. Every PDF and `render-manifest.json` uses create-new semantics, so existing files are never replaced.
The exported manifest structurally records `submission_performed: false`.

## Agent and contract surface

The Render graph output is now `ArtifactKind::RenderManifest`, not a single PDF. Rust types generate
`canisend.rendered-document/v2` and `canisend.render-manifest/v2`, increasing the public catalog to 40 schemas and the
embedded catalog to 51 resources. Codex, Claude, and generic packs contain both render schemas and explicit trusted
input/consent guidance; each pack now contains 31 integrity-manifested files.

Public CLI operations are:

```text
canisend --workspace WORKSPACE render build --job JOB_ID --json
canisend --workspace WORKSPACE render show --job JOB_ID --json
canisend --workspace WORKSPACE render export --job JOB_ID \
  --destination jobs/JOB_ID/rendered --allow-private-export --json
```

## Verification

- 75 Rust tests passed, including two explicit consent tests and the full four-document render/export/invalidation
  path.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo run -p xtask -- release check` passed with 40 schemas and 51 embedded resources.
- `cargo build --release --locked -p canisend-cli` passed.
- Release `doctor` verified the embedded renderer and reported no Python/runtime-package/system-font dependency.
- Release host-agent smoke passed using the rebuilt binary.
- macOS arm64 release binary: 48,874,480 bytes.
- R9.2 remote clean-checkout gate: GitHub Actions run `29627425855`, success.

## Remaining R9.4 work

R9.3 proves the production path locally on macOS arm64. R9.4 must still run full-package render fixtures on native
Ubuntu, macOS, and Windows targets; expand Unicode, mathematical text, URLs, list, page-break, and missing-user-font
coverage; record per-target render time and binary size; and ship the complete embedded font/template license notice.
