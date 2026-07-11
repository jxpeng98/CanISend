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
    (repo / ".codex-plugin").mkdir()

    shutil.copy2(SCRIPT, repo / "scripts" / "release.sh")
    (repo / "pyproject.toml").write_text('[project]\nname = "canisend"\nversion = "0.1.0"\n')
    (repo / "src" / "canisend" / "__init__.py").write_text('__version__ = "0.1.0"\n')
    (repo / "README.md").write_text(
        "![TestPyPI](https://img.shields.io/badge/TestPyPI-0.1.0-blue)\n"
        "Install `canisend==0.1.0` from TestPyPI.\n"
    )
    (repo / ".codex-plugin" / "plugin.json").write_text(
        '{\n  "name": "canisend",\n  "version": "0.1.0"\n}\n'
    )

    origin = tmp_path / "origin.git"
    subprocess.run(["git", "init", "--bare", str(origin)], check=True, text=True, capture_output=True)
    subprocess.run(["git", "init", "-b", "main"], cwd=repo, check=True, text=True, capture_output=True)
    run_git(repo, "config", "user.name", "Release Test")
    run_git(repo, "config", "user.email", "release-test@example.com")
    run_git(repo, "remote", "add", "origin", str(origin))
    run_git(
        repo,
        "add",
        "pyproject.toml",
        "src/canisend/__init__.py",
        "README.md",
        ".codex-plugin/plugin.json",
        "scripts/release.sh",
    )
    run_git(repo, "commit", "-m", "initial")
    return repo


def push_main(repo: Path) -> None:
    run_git(repo, "push", "-u", "origin", "main")


def invoke_repo_release(
    repo: Path,
    channel: str,
    version: str,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "bash",
            "scripts/release.sh",
            channel,
            "--version",
            version,
            "--skip-local-checks",
        ],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
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
    assert "TestPyPI-0.2.0b1-blue" in (repo / "README.md").read_text()
    assert "canisend==0.2.0b1" in (repo / "README.md").read_text()
    assert '"version": "0.2.0b1"' in (repo / ".codex-plugin" / "plugin.json").read_text()
    assert "chore: bump version to 0.2.0b1" in run_git(repo, "log", "-1", "--pretty=%s").stdout
    assert "v0.2.0b1" in run_git(repo, "tag", "--points-at", "HEAD").stdout
    assert 'version = "0.2.0b1"' in run_git(repo, "show", "v0.2.0b1:pyproject.toml").stdout
    assert "TestPyPI-0.2.0b1-blue" in run_git(repo, "show", "v0.2.0b1:README.md").stdout
    assert '"version": "0.2.0b1"' in run_git(repo, "show", "v0.2.0b1:.codex-plugin/plugin.json").stdout
    assert "v0.2.0b1" in run_git(repo, "ls-remote", "--tags", "origin", "v0.2.0b1").stdout


def test_release_bump_commit_includes_tracked_files_changed_by_automation(tmp_path):
    repo = create_minimal_release_repo(tmp_path)
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake_uv = bin_dir / "uv"
    fake_uv.write_text(
        "#!/usr/bin/env bash\n"
        "set -euo pipefail\n"
        "if [[ \"${1:-}\" == \"lock\" ]]; then\n"
        "  printf 'lock refreshed\\n' > uv.lock\n"
        "  printf 'automation refreshed\\n' > release-state.txt\n"
        "  exit 0\n"
        "fi\n"
        "exit 2\n"
    )
    fake_uv.chmod(0o755)
    (repo / "uv.lock").write_text("initial lock\n")
    (repo / "release-state.txt").write_text("initial state\n")
    run_git(repo, "add", "uv.lock", "release-state.txt")
    run_git(repo, "commit", "-m", "add lock refresh state")

    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}{os.pathsep}{env['PATH']}"
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
    assert "automation refreshed" in run_git(repo, "show", "HEAD:release-state.txt").stdout
    assert "automation refreshed" in run_git(repo, "show", "v0.2.0b1:release-state.txt").stdout
    assert run_git(repo, "status", "--porcelain").stdout == ""


def test_release_checks_only_current_version_distributions(tmp_path):
    repo = create_minimal_release_repo(tmp_path)
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake_uv = bin_dir / "uv"
    fake_uv.write_text(
        "#!/usr/bin/env bash\n"
        "set -euo pipefail\n"
        "if [[ \"${1:-}\" == \"lock\" ]]; then exit 0; fi\n"
        "if [[ \"${1:-}\" == \"run\" && \"${2:-}\" == \"python\" && \"${3:-}\" == \"-m\" && \"${4:-}\" == \"pytest\" ]]; then exit 0; fi\n"
        "if [[ \"${1:-}\" == \"build\" ]]; then\n"
        "  mkdir -p dist\n"
        "  : > dist/canisend-0.1.0-py3-none-any.whl\n"
        "  : > dist/canisend-0.1.0.tar.gz\n"
        "  : > dist/canisend-0.2.0b1-py3-none-any.whl\n"
        "  : > dist/canisend-0.2.0b1.tar.gz\n"
        "  exit 0\n"
        "fi\n"
        "if [[ \"${1:-}\" == \"run\" && \"${2:-}\" == \"python\" ]]; then\n"
        "  printf '%s\\n' \"$*\" > package-check-args.txt\n"
        "  exit 0\n"
        "fi\n"
        "if [[ \"${1:-}\" == \"venv\" ]]; then\n"
        "  mkdir -p \"$3/bin\"\n"
        "  printf '#!/usr/bin/env bash\\nexit 0\\n' > \"$3/bin/canisend\"\n"
        "  printf '#!/usr/bin/env bash\\nexit 0\\n' > \"$3/bin/python\"\n"
        "  chmod +x \"$3/bin/canisend\" \"$3/bin/python\"\n"
        "  exit 0\n"
        "fi\n"
        "if [[ \"${1:-}\" == \"pip\" && \"${2:-}\" == \"install\" ]]; then exit 0; fi\n"
        "exit 2\n"
    )
    fake_uv.chmod(0o755)
    fake_uvx = bin_dir / "uvx"
    fake_uvx.write_text(
        "#!/usr/bin/env bash\n"
        "set -euo pipefail\n"
        "printf '%s\\n' \"$*\" > twine-args.txt\n"
    )
    fake_uvx.chmod(0o755)

    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}{os.pathsep}{env['PATH']}"
    result = subprocess.run(
        ["bash", "scripts/release.sh", "beta", "--version", "0.2.0b1"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    assert result.returncode == 0, result.stderr
    assert "dist/canisend-0.2.0b1-py3-none-any.whl" in (repo / "twine-args.txt").read_text()
    assert "dist/canisend-0.2.0b1.tar.gz" in (repo / "twine-args.txt").read_text()
    assert "dist/canisend-0.1.0" not in (repo / "twine-args.txt").read_text()
    assert "dist/canisend-0.2.0b1-py3-none-any.whl" in (repo / "package-check-args.txt").read_text()
    assert "dist/canisend-0.1.0" not in (repo / "package-check-args.txt").read_text()


def test_stable_release_requires_main_and_rejects_unreachable_feature_commit(tmp_path):
    repo = create_minimal_release_repo(tmp_path)
    push_main(repo)
    run_git(repo, "switch", "-c", "feature/release")
    (repo / "feature.txt").write_text("feature-only\n", encoding="utf-8")
    run_git(repo, "add", "feature.txt")
    run_git(repo, "commit", "-m", "feature commit")

    result = invoke_repo_release(repo, "stable", "0.2.0")

    assert result.returncode != 0
    assert "stable releases must start from main" in result.stderr
    assert 'version = "0.1.0"' in (repo / "pyproject.toml").read_text()
    assert "v0.2.0" not in run_git(repo, "tag").stdout


def test_stable_release_pushes_and_verifies_candidate_on_origin_main(tmp_path):
    repo = create_minimal_release_repo(tmp_path)
    push_main(repo)

    result = invoke_repo_release(repo, "stable", "0.2.0")

    assert result.returncode == 0, result.stderr
    local_head = run_git(repo, "rev-parse", "HEAD").stdout.strip()
    remote_head = run_git(repo, "ls-remote", "--heads", "origin", "main").stdout.split()[0]
    assert remote_head == local_head
    assert "Verified candidate commit on origin/main" in result.stdout
    assert "v0.2.0" in run_git(repo, "ls-remote", "--tags", "origin", "v0.2.0").stdout


def test_beta_release_allows_reviewed_non_main_branch_and_pushes_candidate(tmp_path):
    repo = create_minimal_release_repo(tmp_path)
    push_main(repo)
    run_git(repo, "switch", "-c", "feature/beta")
    (repo / "feature.txt").write_text("beta feature\n", encoding="utf-8")
    run_git(repo, "add", "feature.txt")
    run_git(repo, "commit", "-m", "beta feature")

    result = invoke_repo_release(repo, "beta", "0.2.0b1")

    assert result.returncode == 0, result.stderr
    local_head = run_git(repo, "rev-parse", "HEAD").stdout.strip()
    remote_head = run_git(repo, "ls-remote", "--heads", "origin", "feature/beta").stdout.split()[0]
    assert remote_head == local_head
    assert "Prerelease channel beta permits a non-main source branch" in result.stdout


def test_release_refuses_existing_tag_before_changing_version_files(tmp_path):
    repo = create_minimal_release_repo(tmp_path)
    push_main(repo)
    run_git(repo, "tag", "-a", "v0.2.0", "-m", "existing release")
    run_git(repo, "push", "origin", "v0.2.0")

    result = invoke_repo_release(repo, "stable", "0.2.0")

    assert result.returncode != 0
    assert "Tag already exists" in result.stderr
    assert 'version = "0.1.0"' in (repo / "pyproject.toml").read_text()
    assert run_git(repo, "log", "-1", "--pretty=%s").stdout.strip() == "initial"


def test_release_script_contains_remote_candidate_verification():
    script = SCRIPT.read_text()

    assert "git ls-remote --exit-code --heads origin" in script
    assert "Verified candidate commit on origin/" in script
    assert "git merge-base --is-ancestor HEAD" in script
