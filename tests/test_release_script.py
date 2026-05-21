from __future__ import annotations

import os
from pathlib import Path
import subprocess


SCRIPT = Path("scripts/release.sh")


def run_release(*args: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["CANISEND_RELEASE_DRY_RUN"] = "1"
    env["CANISEND_RELEASE_FAKE_TEST_RUN_ID"] = "123456"
    env["CANISEND_RELEASE_FAKE_PYPI_RUN_ID"] = "654321"
    return subprocess.run(
        ["bash", str(SCRIPT), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )


def test_release_shell_script_has_valid_syntax():
    result = subprocess.run(["bash", "-n", str(SCRIPT)], check=False, text=True, capture_output=True)

    assert result.returncode == 0, result.stderr


def test_beta_versions_must_be_pep440_prereleases():
    valid = run_release("beta", "--version", "0.2.0b1", "--skip-local-checks", "--skip-testpypi-smoke")
    invalid = run_release("beta", "--version", "0.2.0", "--skip-local-checks", "--skip-testpypi-smoke")

    assert valid.returncode == 0, valid.stderr
    assert invalid.returncode != 0
    assert "beta releases require" in invalid.stderr


def test_stable_versions_reject_prerelease_suffixes():
    valid = run_release("stable", "--version", "0.2.0", "--skip-local-checks", "--skip-testpypi-smoke")
    invalid = run_release("stable", "--version", "0.2.0b1", "--skip-local-checks", "--skip-testpypi-smoke")

    assert valid.returncode == 0, valid.stderr
    assert invalid.returncode != 0
    assert "stable releases require" in invalid.stderr


def test_beta_release_uses_github_prerelease_flag_after_test_success():
    result = run_release("beta", "--version", "0.2.0b1", "--skip-local-checks", "--skip-testpypi-smoke")

    assert result.returncode == 0, result.stderr
    assert "gh workflow run release.yml -f publish_target=testpypi --ref main" in result.stdout
    assert "gh run watch 123456 --exit-status" in result.stdout
    assert "gh release create v0.2.0b1" in result.stdout
    assert "--prerelease" in result.stdout
    assert "gh run watch 654321 --exit-status" in result.stdout


def test_stable_release_creates_normal_github_release_after_test_success():
    result = run_release("stable", "--version", "0.2.0", "--skip-local-checks", "--skip-testpypi-smoke")

    assert result.returncode == 0, result.stderr
    assert "gh run watch 123456 --exit-status" in result.stdout
    assert "gh release create v0.2.0" in result.stdout
    assert "--prerelease" not in result.stdout
    assert "gh run watch 654321 --exit-status" in result.stdout
