from __future__ import annotations

import os
from pathlib import Path
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
