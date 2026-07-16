# Release Playbook

Use this playbook for package release preparation and TestPyPI dry runs.

## Release Channels

Use `scripts/release.sh` as the main release orchestrator:

```bash
scripts/release.sh test --version 0.3.0.dev2
scripts/release.sh beta --version 0.6.0b1
scripts/release.sh stable --version 0.6.0
```

Channel behavior:

- `test`: creates and pushes `test/v0.3.0.dev2`; `release.yml` publishes to TestPyPI and smoke-tests installation from TestPyPI. Use a disposable version because TestPyPI versions cannot be overwritten.
- `beta`: requires a PEP 440 prerelease version such as `0.6.0b1`; creates and pushes `v0.6.0b1`; `release.yml` publishes to TestPyPI first, then PyPI as a prerelease, then creates a GitHub prerelease.
- `stable`: requires a final version such as `0.6.0`; creates and pushes `v0.6.0`; `release.yml` publishes to TestPyPI first, then PyPI as a stable release, then creates a GitHub Release.

The script does not call `gh workflow run` or create GitHub releases locally. It updates both version files, commits
the version bump, pushes the current branch, verifies that the candidate commit is the remote branch head, and only
then pushes the release tag. The workflow intentionally publishes to PyPI only after TestPyPI publish and smoke testing succeed.

`test/v0.3.0.dev1` is a retained failed candidate: remote CI exposed ANSI-sensitive assertions before any TestPyPI
upload. The tag must not be moved or reused. `0.3.0.dev2` is the current TestPyPI development checkpoint.

Stable releases must start from `main`; the final candidate must be reachable from `origin/main` both locally and in
the tag-triggered workflow. A reviewed prerelease may originate from a non-main branch, but that branch is pushed and
the candidate commit is verified before the tag is created. Do not reuse `v0.2.0` or any published tag/version; the
accepted Stage 3 candidate is `0.3.0b1`; the Stage 4 beta candidate is `0.6.0b1`.

Before validating an upgraded private workspace, follow `docs/stage4-migration.md`; release automation refreshes
packaged resources but never migrates or rewrites user-owned workspace data.

## Local Release Checks

Run these before triggering any remote publish workflow:

```bash
uv run python scripts/sync_workspace_skill_mirror.py --check
uv run python -m pytest
uv build
uvx twine check dist/*
uv run python -m canisend.package_check dist/*.whl
```

Smoke test the built wheel in a clean environment:

```bash
python -m venv /tmp/canisend-smoke
/tmp/canisend-smoke/bin/pip install dist/*.whl
/tmp/canisend-smoke/bin/canisend --help
/tmp/canisend-smoke/bin/canisend init-workspace --workspace /tmp/canisend-workspace
/tmp/canisend-smoke/bin/canisend doctor --workspace /tmp/canisend-workspace --format json
/tmp/canisend-smoke/bin/canisend agent capabilities --format json
```

The automated clean-wheel smoke runs the fake-data Decision Spine through Evidence, Parse, Confirm,
corrections status/init, Match, a scoped current-basis `decision=apply` update, Brief status/init, and deterministic
Brief planning, then runs the compatible deterministic package pipeline. The shared smoke helper parses only
body-free revision/hash metadata, writes a synthetic private patch to temporary workspace scratch, and removes it
after use. It asserts three user-owned YAML files, the core-owned `required_document_plan.json`, four immutable
body-free mutation receipts, successful manifests for Evidence, Parse, Confirm, Match, and Brief, byte preservation
across `run`, and structured Match parity across Markdown and Typst content. It also runs the offline Stage 4
discovery smoke across packaged schemas/examples/migration resources, Greenhouse and Lever fixtures, CSV import,
normalized host search, catalog deduplication, stable-ID selection, private-safe persistence, and the full-advert
boundary.

Stage 2 was locally accepted after this smoke was combined with focused privacy/CAS/recovery, provenance, view-
fallback, adversarial source-receipt and Markdown tests, 942 passing tests on supported Python 3.11-3.13 plus the
additional Python 3.14 development interpreter, package/resource validation, Twine checks, and a clean
installed-wheel run. This is not a remote CI or publication result and does not claim Draft, package, or submission
readiness.

## Trusted Publishing Setup

Configure Trusted Publishing before the first remote publish.

TestPyPI:

- Project: `canisend`
- Owner: `jxpeng98`
- Repository: `CanISend`
- Workflow: `.github/workflows/release.yml`
- Environment: `testpypi`
- Upload endpoint used by the workflow: `https://test.pypi.org/legacy/`

PyPI:

- Project: `canisend`
- Owner: `jxpeng98`
- Repository: `CanISend`
- Workflow: `.github/workflows/release.yml`
- Environment: `pypi`

PyPI's Trusted Publishing flow uses GitHub Actions OIDC with `id-token: write`; no PyPI API token should be stored in this repository.

The TestPyPI Trusted Publisher should accept claims from tag-triggered workflow runs matching:

- `repository`: `jxpeng98/CanISend`
- `workflow_ref`: `jxpeng98/CanISend/.github/workflows/release.yml@refs/tags/test/v0.3.0.dev2` for the retained TestPyPI-only checkpoint, or `refs/tags/v0.6.0b1` / `refs/tags/v0.6.0` for the current prerelease and stable tags.
- `environment`: `testpypi`

## TestPyPI Dry Run

After pushing the release workflow to GitHub and configuring TestPyPI Trusted Publishing, push a TestPyPI-only tag:

```bash
scripts/release.sh test --version 0.3.0.dev2
```

Watch the run:

```bash
gh run list --workflow release.yml --limit 5
gh run watch
```

Install from TestPyPI with PyPI as the dependency fallback index:

```bash
python -m venv /tmp/canisend-testpypi
/tmp/canisend-testpypi/bin/pip install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  canisend==0.3.0.dev2
/tmp/canisend-testpypi/bin/canisend --help
/tmp/canisend-testpypi/bin/canisend init-workspace --workspace /tmp/canisend-testpypi-workspace
/tmp/canisend-testpypi/bin/canisend doctor --workspace /tmp/canisend-testpypi-workspace
```

Check that the TestPyPI project page renders the README and metadata correctly before publishing to PyPI.

## PyPI Release

Only publish to PyPI from `v*` tags. The workflow itself gates PyPI on TestPyPI publish and smoke-test success.

1. Confirm `CHANGELOG.md` has release notes for the intended version.
2. Confirm the git working tree is clean.
3. Run `scripts/release.sh beta --version 0.6.0b1` for prerelease publishing, or `scripts/release.sh stable --version 0.6.0` for stable publishing.
4. The script bumps `pyproject.toml` and `src/canisend/__init__.py`, runs local release checks, commits the version bump, pushes the current branch, and pushes the release tag.
5. The pushed tag triggers `.github/workflows/release.yml`, publishes to TestPyPI, smoke-tests TestPyPI, publishes to PyPI through the `pypi` environment, and creates the GitHub Release.

Post-release smoke test:

```bash
uv tool install --prerelease allow canisend==0.6.0b1
canisend --help
canisend init-workspace --workspace /tmp/canisend-pypi-workspace
canisend doctor --workspace /tmp/canisend-pypi-workspace
```
