# Release Playbook

Use this playbook for package release preparation and TestPyPI dry runs.

## Local Release Checks

Run these before triggering any remote publish workflow:

```bash
uv run pytest
uv build
uvx twine check dist/*
uv run python -m academic_prep.package_check dist/*.whl
```

Smoke test the built wheel in a clean environment:

```bash
python -m venv /tmp/aap-smoke
/tmp/aap-smoke/bin/pip install dist/*.whl
/tmp/aap-smoke/bin/academic-prep --help
/tmp/aap-smoke/bin/academic-prep init-workspace --workspace /tmp/aap-workspace
/tmp/aap-smoke/bin/academic-prep doctor --workspace /tmp/aap-workspace
```

## Trusted Publishing Setup

Configure Trusted Publishing before the first remote publish.

TestPyPI:

- Project: `academic-application-prep`
- Owner: repository owner on GitHub
- Repository: `auto-academic-jobs`
- Workflow: `.github/workflows/release.yml`
- Environment: `testpypi`
- Upload endpoint used by the workflow: `https://test.pypi.org/legacy/`

PyPI:

- Project: `academic-application-prep`
- Owner: repository owner on GitHub
- Repository: `auto-academic-jobs`
- Workflow: `.github/workflows/release.yml`
- Environment: `pypi`

PyPI's Trusted Publishing flow uses GitHub Actions OIDC with `id-token: write`; no PyPI API token should be stored in this repository.

## TestPyPI Dry Run

After pushing the release workflow to GitHub and configuring TestPyPI Trusted Publishing, trigger the manual workflow:

```bash
gh workflow run release.yml -f publish_target=testpypi
```

Watch the run:

```bash
gh run list --workflow release.yml --limit 5
gh run watch
```

Install from TestPyPI with PyPI as the dependency fallback index:

```bash
python -m venv /tmp/aap-testpypi
/tmp/aap-testpypi/bin/pip install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  academic-application-prep==0.1.0
/tmp/aap-testpypi/bin/academic-prep --help
/tmp/aap-testpypi/bin/academic-prep init-workspace --workspace /tmp/aap-testpypi-workspace
/tmp/aap-testpypi/bin/academic-prep doctor --workspace /tmp/aap-testpypi-workspace
```

Check that the TestPyPI project page renders the README and metadata correctly before publishing to PyPI.

## PyPI Release

Only publish to PyPI after the TestPyPI dry run passes.

1. Confirm `pyproject.toml` and `src/academic_prep/__init__.py` have the intended version.
2. Confirm `CHANGELOG.md` has release notes for that version.
3. Confirm local release checks pass.
4. Create a GitHub Release for the version tag.
5. The published GitHub Release triggers `.github/workflows/release.yml` and publishes to PyPI through the `pypi` environment.

Post-release smoke test:

```bash
uv tool install academic-application-prep
academic-prep --help
academic-prep init-workspace --workspace /tmp/aap-pypi-workspace
academic-prep doctor --workspace /tmp/aap-pypi-workspace
```
