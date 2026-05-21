from __future__ import annotations

import importlib.util
from pathlib import Path


def load_release_script():
    script_path = Path("scripts/release.py")
    spec = importlib.util.spec_from_file_location("release_script", script_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class FakeRunner:
    def __init__(self) -> None:
        self.commands: list[list[str]] = []

    def __call__(self, command: list[str], *, capture: bool = False) -> str:
        self.commands.append(command)
        if command[:3] == ["gh", "run", "list"]:
            if "--event" in command and command[command.index("--event") + 1] == "release":
                return '[{"databaseId": 654321, "status": "completed", "conclusion": "success"}]'
            return '[{"databaseId": 123456, "status": "completed", "conclusion": "success"}]'
        return ""


def test_beta_versions_must_be_pep440_prereleases():
    release = load_release_script()

    release.validate_version_for_channel("beta", "0.2.0b1")

    try:
        release.validate_version_for_channel("beta", "0.2.0")
    except SystemExit as exc:
        assert "beta releases require" in str(exc)
    else:
        raise AssertionError("stable-looking version accepted for beta")


def test_stable_versions_reject_prerelease_suffixes():
    release = load_release_script()

    release.validate_version_for_channel("stable", "0.2.0")

    try:
        release.validate_version_for_channel("stable", "0.2.0b1")
    except SystemExit as exc:
        assert "stable releases require" in str(exc)
    else:
        raise AssertionError("beta version accepted for stable")


def test_beta_release_uses_github_prerelease_flag_after_test_success():
    release = load_release_script()
    runner = FakeRunner()

    release.promote_release(channel="beta", version="0.2.0b1", ref="main", run=runner)

    assert ["gh", "workflow", "run", "release.yml", "-f", "publish_target=testpypi", "--ref", "main"] in runner.commands
    assert ["gh", "run", "watch", "123456", "--exit-status"] in runner.commands
    assert ["gh", "run", "watch", "654321", "--exit-status"] in runner.commands
    assert [
        "gh",
        "release",
        "create",
        "v0.2.0b1",
        "--target",
        "main",
        "--title",
        "CanISend 0.2.0b1",
        "--notes",
        "Beta release 0.2.0b1. See CHANGELOG.md and RELEASE.md for details.",
        "--prerelease",
    ] in runner.commands


def test_stable_release_creates_normal_github_release_after_test_success():
    release = load_release_script()
    runner = FakeRunner()

    release.promote_release(channel="stable", version="0.2.0", ref="main", run=runner)

    assert ["gh", "run", "watch", "123456", "--exit-status"] in runner.commands
    assert ["gh", "run", "watch", "654321", "--exit-status"] in runner.commands
    assert [
        "gh",
        "release",
        "create",
        "v0.2.0",
        "--target",
        "main",
        "--title",
        "CanISend 0.2.0",
        "--notes",
        "Stable release 0.2.0. See CHANGELOG.md and RELEASE.md for details.",
    ] in runner.commands
