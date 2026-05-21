#!/usr/bin/env python3
from __future__ import annotations

import argparse
import glob
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from typing import Callable


RunCommand = Callable[[list[str]], str]
PRERELEASE_RE = re.compile(r"(a|b|rc)\d+$")


def run_command(command: list[str], *, capture: bool = False) -> str:
    print("+ " + " ".join(command))
    completed = subprocess.run(
        command,
        check=True,
        text=True,
        stdout=subprocess.PIPE if capture else None,
    )
    return completed.stdout if capture else ""


def read_project_version() -> str:
    metadata = tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8"))
    project_version = metadata["project"]["version"]
    init_text = Path("src/canisend/__init__.py").read_text(encoding="utf-8")
    match = re.search(r'__version__ = "([^"]+)"', init_text)
    if not match:
        raise SystemExit("src/canisend/__init__.py does not define __version__")
    package_version = match.group(1)
    if project_version != package_version:
        raise SystemExit(f"Version mismatch: pyproject.toml={project_version}, __init__.py={package_version}")
    return project_version


def is_prerelease(version: str) -> bool:
    return bool(PRERELEASE_RE.search(version))


def validate_version_for_channel(channel: str, version: str) -> None:
    if channel == "beta" and not is_prerelease(version):
        raise SystemExit("beta releases require a PEP 440 prerelease version such as 0.2.0b1 or 0.2.0rc1")
    if channel == "stable" and is_prerelease(version):
        raise SystemExit("stable releases require a final version such as 0.2.0, not 0.2.0b1 or 0.2.0rc1")


def local_release_checks(run: Callable[..., str] = run_command) -> None:
    run(["uv", "run", "pytest", "-v"])
    run(["uv", "build"])
    distributions = sorted(glob.glob("dist/*"))
    wheels = sorted(glob.glob("dist/*.whl"))
    if not distributions:
        raise SystemExit("uv build did not create any distributions under dist/")
    if not wheels:
        raise SystemExit("uv build did not create a wheel under dist/")
    run(["uvx", "twine", "check", *distributions])
    run(["uv", "run", "python", "-m", "canisend.package_check", *wheels])


def latest_run_id(ref: str, run: Callable[..., str] = run_command) -> int:
    output = run(
        [
            "gh",
            "run",
            "list",
            "--workflow",
            "release.yml",
            "--event",
            "workflow_dispatch",
            "--branch",
            ref,
            "--json",
            "databaseId,status,conclusion,createdAt",
            "--limit",
            "1",
        ],
        capture=True,
    )
    runs = json.loads(output)
    if not runs:
        raise SystemExit("No workflow_dispatch release run found after triggering TestPyPI")
    return int(runs[0]["databaseId"])


def latest_pypi_release_run_id(run: Callable[..., str] = run_command) -> int:
    output = run(
        [
            "gh",
            "run",
            "list",
            "--workflow",
            "release.yml",
            "--event",
            "release",
            "--json",
            "databaseId,status,conclusion,createdAt",
            "--limit",
            "1",
        ],
        capture=True,
    )
    runs = json.loads(output)
    if not runs:
        raise SystemExit("No release-event workflow run found after creating the GitHub Release")
    return int(runs[0]["databaseId"])


def trigger_testpypi(ref: str, run: Callable[..., str] = run_command) -> int:
    run(["gh", "workflow", "run", "release.yml", "-f", "publish_target=testpypi", "--ref", ref])
    run_id = latest_run_id(ref, run=run)
    run(["gh", "run", "watch", str(run_id), "--exit-status"])
    return run_id


def smoke_test_testpypi(version: str, run: Callable[..., str] = run_command) -> None:
    venv = f"/tmp/canisend-testpypi-{version}"
    pip = f"{venv}/bin/pip"
    cli = f"{venv}/bin/canisend"
    workspace = f"/tmp/canisend-testpypi-workspace-{version}"
    run(["python", "-m", "venv", venv])
    run(
        [
            pip,
            "install",
            "--index-url",
            "https://test.pypi.org/simple/",
            "--extra-index-url",
            "https://pypi.org/simple/",
            f"canisend=={version}",
        ]
    )
    run([cli, "--help"])
    run([cli, "init-workspace", "--workspace", workspace])
    run([cli, "doctor", "--workspace", workspace])


def github_release_command(channel: str, version: str, ref: str) -> list[str]:
    label = "Beta" if channel == "beta" else "Stable"
    command = [
        "gh",
        "release",
        "create",
        f"v{version}",
        "--target",
        ref,
        "--title",
        f"CanISend {version}",
        "--notes",
        f"{label} release {version}. See CHANGELOG.md and RELEASE.md for details.",
    ]
    if channel == "beta":
        command.append("--prerelease")
    return command


def create_github_release(channel: str, version: str, ref: str, run: Callable[..., str] = run_command) -> None:
    run(github_release_command(channel, version, ref))


def wait_for_pypi_publish(run: Callable[..., str] = run_command) -> int:
    run_id = latest_pypi_release_run_id(run=run)
    run(["gh", "run", "watch", str(run_id), "--exit-status"])
    return run_id


def promote_release(channel: str, version: str, ref: str, run: Callable[..., str] = run_command) -> None:
    validate_version_for_channel(channel, version)
    trigger_testpypi(ref=ref, run=run)
    create_github_release(channel=channel, version=version, ref=ref, run=run)
    wait_for_pypi_publish(run=run)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Trigger CanISend TestPyPI, beta, or stable release flows.")
    parser.add_argument("channel", choices=["test", "beta", "stable"], help="Release channel to run.")
    parser.add_argument("--version", default=None, help="Version to release. Defaults to pyproject.toml version.")
    parser.add_argument("--ref", default="main", help="Git ref used by GitHub Actions. Defaults to main.")
    parser.add_argument("--skip-local-checks", action="store_true", help="Skip local pytest/build/package checks.")
    parser.add_argument("--skip-testpypi-smoke", action="store_true", help="Skip install smoke test from TestPyPI.")
    parser.add_argument("--no-wait-pypi", action="store_true", help="Do not wait for the PyPI publish workflow after creating the GitHub Release.")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    version = args.version or read_project_version()

    if not args.skip_local_checks:
        local_release_checks()

    if args.channel == "test":
        validate_version_for_channel("test", version)
        trigger_testpypi(ref=args.ref)
        if not args.skip_testpypi_smoke:
            smoke_test_testpypi(version)
        return 0

    validate_version_for_channel(args.channel, version)
    if version != read_project_version():
        raise SystemExit(f"--version {version} does not match project version {read_project_version()}")

    trigger_testpypi(ref=args.ref)
    if not args.skip_testpypi_smoke:
        smoke_test_testpypi(version)
    create_github_release(channel=args.channel, version=version, ref=args.ref)
    if not args.no_wait_pypi:
        wait_for_pypi_publish()
    print(f"{args.channel} release v{version} created and PyPI publish workflow was triggered.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
