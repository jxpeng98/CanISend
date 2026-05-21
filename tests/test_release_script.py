from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess


SCRIPT = Path("scripts/release.sh")


def run_release(*args: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["CANISEND_RELEASE_DRY_RUN"] = "1"
    return subprocess.run(
        ["bash", str(SCRIPT), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )


def run_git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def create_minimal_release_repo(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "scripts").mkdir()
    (repo / "src" / "canisend").mkdir(parents=True)

    shutil.copy2(SCRIPT, repo / "scripts" / "release.sh")
    (repo / "pyproject.toml").write_text('[project]\nname = "canisend"\nversion = "0.1.0"\n')
    (repo / "src" / "canisend" / "__init__.py").write_text('__version__ = "0.1.0"\n')

    origin = tmp_path / "origin.git"
    subprocess.run(["git", "init", "--bare", str(origin)], check=True, text=True, capture_output=True)
    subprocess.run(["git", "init", "-b", "main"], cwd=repo, check=True, text=True, capture_output=True)
    run_git(repo, "config", "user.name", "Release Test")
    run_git(repo, "config", "user.email", "release-test@example.com")
    run_git(repo, "remote", "add", "origin", str(origin))
    run_git(repo, "add", "pyproject.toml", "src/canisend/__init__.py", "scripts/release.sh")
    run_git(repo, "commit", "-m", "initial")
    return repo


def test_release_shell_script_has_valid_syntax():
    result = subprocess.run(["bash", "-n", str(SCRIPT)], check=False, text=True, capture_output=True)

    assert result.returncode == 0, result.stderr


def test_beta_versions_must_be_pep440_prereleases():
    valid = run_release("beta", "--version", "0.2.0b1", "--skip-local-checks")
    invalid = run_release("beta", "--version", "0.2.0", "--skip-local-checks")

    assert valid.returncode == 0, valid.stderr
    assert invalid.returncode != 0
    assert "beta releases require" in invalid.stderr


def test_stable_versions_reject_prerelease_suffixes():
    valid = run_release("stable", "--version", "0.2.0", "--skip-local-checks")
    invalid = run_release("stable", "--version", "0.2.0b1", "--skip-local-checks")

    assert valid.returncode == 0, valid.stderr
    assert invalid.returncode != 0
    assert "stable releases require" in invalid.stderr


def test_test_release_pushes_testpypi_only_tag():
    result = run_release("test", "--version", "0.2.0.dev1", "--skip-local-checks")

    assert result.returncode == 0, result.stderr
    assert "git tag -a test/v0.2.0.dev1 HEAD -m CanISend 0.2.0.dev1 TestPyPI" in result.stdout
    assert "git push origin test/v0.2.0.dev1" in result.stdout
    assert "gh workflow run" not in result.stdout
    assert "gh release create" not in result.stdout


def test_beta_release_pushes_prerelease_tag_for_tag_driven_workflow():
    result = run_release("beta", "--version", "0.2.0b1", "--skip-local-checks")

    assert result.returncode == 0, result.stderr
    assert "git tag -a v0.2.0b1 HEAD -m CanISend 0.2.0b1 beta" in result.stdout
    assert "git push origin v0.2.0b1" in result.stdout
    assert "gh workflow run" not in result.stdout
    assert "gh release create" not in result.stdout
    assert "gh run watch" not in result.stdout


def test_stable_release_pushes_final_tag_for_tag_driven_workflow():
    result = run_release("stable", "--version", "0.2.0", "--skip-local-checks")

    assert result.returncode == 0, result.stderr
    assert "git tag -a v0.2.0 HEAD -m CanISend 0.2.0 stable" in result.stdout
    assert "git push origin v0.2.0" in result.stdout
    assert "gh workflow run" not in result.stdout
    assert "gh release create" not in result.stdout
    assert "gh run watch" not in result.stdout


def test_dry_run_release_does_not_require_attached_branch(tmp_path):
    repo = create_minimal_release_repo(tmp_path)
    run_git(repo, "checkout", "--detach")
    env = os.environ.copy()
    env["CANISEND_RELEASE_DRY_RUN"] = "1"

    result = subprocess.run(
        ["bash", "scripts/release.sh", "beta", "--version", "0.2.0b1", "--skip-local-checks"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    assert result.returncode == 0, result.stderr
    assert "git push origin main" in result.stdout
    assert "git push origin v0.2.0b1" in result.stdout


def test_beta_release_bumps_version_files_before_tagging(tmp_path):
    repo = create_minimal_release_repo(tmp_path)

    result = subprocess.run(
        ["bash", "scripts/release.sh", "beta", "--version", "0.2.0b1", "--skip-local-checks"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    assert result.returncode == 0, result.stderr
    assert 'version = "0.2.0b1"' in (repo / "pyproject.toml").read_text()
    assert '__version__ = "0.2.0b1"' in (repo / "src" / "canisend" / "__init__.py").read_text()
    assert "chore: bump version to 0.2.0b1" in run_git(repo, "log", "-1", "--pretty=%s").stdout
    assert "v0.2.0b1" in run_git(repo, "tag", "--points-at", "HEAD").stdout
    assert 'version = "0.2.0b1"' in run_git(repo, "show", "v0.2.0b1:pyproject.toml").stdout
    assert "v0.2.0b1" in run_git(repo, "ls-remote", "--tags", "origin", "v0.2.0b1").stdout
