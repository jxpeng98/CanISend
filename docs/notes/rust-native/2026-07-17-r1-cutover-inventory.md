# R1 Repository Cutover Inventory

**Date:** 2026-07-17

**Status:** Prepared while R0 native CI is running; no deletion is authorized by this note alone

## Baseline

The active branch contains 85 tracked Python source files, 95 tracked Python test/support files, six Python/release
scripts, 40 Python-era schemas, and multiple mirrored skill/resource trees. The complete state is recoverable from
`archive/python-v0.6.0b1-final`.

The branch was clean before Rust-native roadmap work started. Local ignored caches such as `.pytest_cache/`,
`__pycache__/`, `.venv/`, and `.claude/settings.local.json` are not part of the cutover and must not be deleted as if
they were tracked product files.

## Retain as active Rust-native sources

- `LICENSE`.
- `assets/canisend-logo.svg`, subject to a later packaging review.
- `docs/history/python-era.md`.
- `docs/architecture/rust-native/`.
- `docs/notes/rust-native/`.
- `docs/superpowers/plans/2026-07-17-rust-native-greenfield-roadmap.md`.
- `fixtures/v2-spec/`.
- `spikes/r0-dependencies/` until the production dependency choices and CI checks supersede it.
- `.github/workflows/rust-r0-spikes.yml` until R1 CI incorporates the same evidence.

## Replace during R1

- Root `README.md` with a Rust-native alpha README.
- Root `.gitignore` with Cargo, workspace-private-state, editor, and release-output rules.
- `.github/workflows/ci.yml` with Rust formatting, Clippy, test, schema/resource, and binary smoke jobs.
- `.github/workflows/release.yml` with native archive scaffolding later in R1/R11; the Python publication workflow is
  removed immediately.
- `CHANGELOG.md` with a Rust-native development entry while preserving a link to the archived history.
- `RELEASE.md` with the new native release policy.

## Remove from the active branch

### Python implementation and build

- `src/canisend/`.
- `tests/`.
- `scripts/`.
- `pyproject.toml`.
- `uv.lock`.
- Python wheel/sdist rules and PyPI/TestPyPI automation.

### Python-era contracts and compatibility assets

- Root `schemas/`.
- Root `prompts/`.
- Root `templates/`.
- Root `skills/`.
- Root `agent-skills/`.
- Root `platform-bridges/`.
- Root `.codex-plugin/`.
- Root `.env.example`.

These resources are not copied wholesale. R2 will add reviewed v2 resources under
`crates/canisend-resources/resources/` and generate v2 schemas from Rust types.

### Python-era examples and generated workspace content

- Root `examples/`.
- Tracked `jobs/` examples.
- Tracked `profile/` placeholders.
- Tracked `dist/` placeholder state.
- `canisend_v1_proposal.md`.

The synthetic Rust v2 fixture is the new executable example foundation.

### Python-era documentation

- `docs/architecture/decisions/`.
- Old stage migration guides.
- Old discovery adapter guide.
- Old Python-era plans under `docs/superpowers/plans/`, except the Rust-native roadmap.

The archive tag is the authoritative source for those documents. Keeping all of them in the active Rust tree would
make search results and agent context ambiguous.

## Cutover sequence

1. Require the R0 Ubuntu/macOS/Windows dependency matrix to pass.
2. Record the R0 CI run in the dependency note and roadmap.
3. Commit R0 completion.
4. Remove tracked Python-era implementation, tests, assets, examples, and documentation with Git-aware deletion.
5. Add the Cargo workspace in the same branch before committing a permanently non-buildable intermediate state.
6. Add minimal v2 CLI/contracts/resources tests and Rust CI.
7. Verify there is no tracked `.py`, `pyproject.toml`, `uv.lock`, Pytest command, PyPI workflow, or Python setup action.
8. Commit the R1 cutover only after Cargo format, Clippy, tests, and release build pass.

## Safety checks

Before deletion:

```text
git status --short
git rev-parse archive/python-v0.6.0b1-final^{commit}
git diff archive/python-v0.6.0b1-final..HEAD -- docs/architecture/rust-native docs/notes/rust-native fixtures/v2-spec
```

After cutover:

```text
git ls-files '*.py' pyproject.toml uv.lock
rg -n 'pytest|setup-python|TestPyPI|PyPI' .github Cargo.toml crates docs/notes/rust-native
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release --locked
```

The first command after cutover must return no tracked Python product files. Historical notes may mention Python and
Pytest; that prose is not an active runtime or test dependency.
