#!/usr/bin/env python3
"""Run the packaged decision-spine smoke test without echoing private bodies."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
from typing import Any, Sequence


EXAMPLE_JOB = "jobs/2026-06-15_example-university_lecturer-in-applied-economics"
EXPECTED_STAGES = {"evidence", "parse", "confirm", "match", "brief"}
EXPECTED_USER_MUTATION_RECEIPTS = 4


class SmokeFailure(RuntimeError):
    """A body-free failure raised by the release smoke test."""


def _run(
    canisend: str,
    arguments: Sequence[str],
    *,
    expect_json: bool,
) -> dict[str, Any] | None:
    completed = subprocess.run(
        [canisend, *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="strict",
    )
    operation = " ".join(arguments[:2])
    if completed.returncode != 0:
        raise SmokeFailure(
            f"CanISend smoke operation {operation!r} failed "
            f"with exit code {completed.returncode}."
        )
    if not expect_json:
        return None
    try:
        payload = json.loads(completed.stdout)
    except (json.JSONDecodeError, TypeError) as exc:
        raise SmokeFailure(
            f"CanISend smoke operation {operation!r} did not return one JSON value."
        ) from exc
    if not isinstance(payload, dict) or payload.get("protocol") != "canisend.agent/v1":
        raise SmokeFailure(
            f"CanISend smoke operation {operation!r} returned an unexpected contract."
        )
    return payload


def _job_arguments(workspace: Path) -> list[str]:
    return [
        "--workspace",
        str(workspace),
        "--job",
        EXAMPLE_JOB,
        "--format",
        "json",
    ]


def _artifact(payload: dict[str, Any], kind: str) -> dict[str, Any]:
    artifacts = payload.get("artifacts")
    if not isinstance(artifacts, list):
        raise SmokeFailure("A user-mutation response omitted its artifact references.")
    matching = [
        item
        for item in artifacts
        if isinstance(item, dict) and item.get("kind") == kind
    ]
    if len(matching) != 1:
        raise SmokeFailure(f"Expected exactly one {kind!r} artifact reference.")
    return matching[0]


def _assert_workspace_contract(workspace: Path) -> None:
    job = workspace / EXAMPLE_JOB
    expected_artifacts = {
        "evidence_catalog.json",
        "criteria.json",
        "criterion_matches.json",
        "confirmed_corrections.yaml",
        "application_decision.yaml",
        "application_brief.yaml",
        "required_document_plan.json",
    }
    missing = sorted(name for name in expected_artifacts if not (job / name).is_file())
    if missing:
        raise SmokeFailure("The decision-spine smoke test did not create every expected artifact.")

    receipt_paths = sorted(
        (job / "workflow" / "user-mutations" / "events").glob("*/receipt.json")
    )
    if len(receipt_paths) != EXPECTED_USER_MUTATION_RECEIPTS:
        raise SmokeFailure(
            "The decision-spine smoke test did not create exactly four mutation receipts."
        )

    manifest_paths = sorted((job / "workflow" / "runs").glob("*/manifest.json"))
    if len(manifest_paths) != len(EXPECTED_STAGES):
        raise SmokeFailure("The decision-spine smoke test created an unexpected run count.")
    stages: set[str] = set()
    for manifest_path in manifest_paths:
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as exc:
            raise SmokeFailure("A decision-spine run manifest could not be validated.") from exc
        stage = manifest.get("stage") if isinstance(manifest, dict) else None
        if not isinstance(stage, str) or manifest.get("status") != "succeeded":
            raise SmokeFailure("A decision-spine stage did not finish successfully.")
        stages.add(stage)
        if not (manifest_path.parent / "preparation.json").is_file() or not (
            manifest_path.parent / "submission.json"
        ).is_file():
            raise SmokeFailure("A decision-spine run omitted its preparation or submission record.")
    if stages != EXPECTED_STAGES:
        raise SmokeFailure("The decision-spine smoke test ran an unexpected stage set.")

    try:
        plan = json.loads((job / "required_document_plan.json").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise SmokeFailure("The required-document plan is not valid JSON.") from exc
    if not isinstance(plan, dict) or plan.get("job_id") != job.name:
        raise SmokeFailure("The required-document plan does not belong to the example job.")
    requirements = plan.get("requirements")
    if (
        not isinstance(requirements, list)
        or not requirements
        or any(
            not isinstance(item, dict) or item.get("source_state") != "known"
            for item in requirements
        )
    ):
        raise SmokeFailure(
            "The required-document plan did not retain resolvable advert source receipts."
        )
    if (
        plan.get("requirements_state") != "unconfirmed"
        or "documents.requirements_unconfirmed" not in plan.get("blockers", [])
    ):
        raise SmokeFailure(
            "The smoke plan inferred requirement confirmation without a user decision."
        )


def run_smoke(canisend: str, workspace: Path) -> None:
    workspace = workspace.resolve()
    if workspace.exists():
        raise SmokeFailure("The smoke workspace must not already exist.")
    workspace.parent.mkdir(parents=True, exist_ok=True)

    _run(canisend, ["--help"], expect_json=False)
    _run(
        canisend,
        ["run-example", "--workspace", str(workspace), "--overwrite"],
        expect_json=False,
    )
    _run(
        canisend,
        ["doctor", "--workspace", str(workspace), "--format", "json"],
        expect_json=False,
    )
    _run(canisend, ["agent", "capabilities", "--format", "json"], expect_json=True)

    job_args = _job_arguments(workspace)
    for stage in ("evidence", "parse", "confirm"):
        _run(
            canisend,
            ["stage", "run", *job_args, "--stage", stage],
            expect_json=True,
        )

    _run(canisend, ["corrections", "status", *job_args], expect_json=True)
    _run(
        canisend,
        ["corrections", "init", *job_args, "--confirm-user-owned-write"],
        expect_json=True,
    )
    _run(
        canisend,
        ["stage", "run", *job_args, "--stage", "match"],
        expect_json=True,
    )

    _run(canisend, ["decision", "status", *job_args], expect_json=True)
    initialized = _run(
        canisend,
        ["decision", "init", *job_args, "--confirm-user-owned-write"],
        expect_json=True,
    )
    if initialized is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Decision initialization returned no response.")
    decision = _artifact(initialized, "application_decision")
    extensions = initialized.get("extensions")
    if not isinstance(extensions, dict):
        raise SmokeFailure("Decision initialization omitted its control metadata.")
    revision = extensions.get("canisend.user_artifact_revision")
    sha256 = decision.get("sha256")
    if not isinstance(revision, int) or not isinstance(sha256, str):
        raise SmokeFailure("Decision initialization omitted its compare-and-swap baseline.")

    patch_path = workspace / ".canisend-smoke-decision-patch.json"
    patch_path.write_text(
        json.dumps({"operation": "set_decision", "decision": "apply"}) + "\n",
        encoding="utf-8",
    )
    try:
        _run(
            canisend,
            [
                "decision",
                "update",
                *job_args,
                "--patch-file",
                str(patch_path),
                "--expected-revision",
                str(revision),
                "--expected-sha256",
                sha256,
                "--confirm-user-owned-write",
            ],
            expect_json=True,
        )
    finally:
        patch_path.unlink(missing_ok=True)

    _run(canisend, ["brief", "status", *job_args], expect_json=True)
    _run(
        canisend,
        ["brief", "init", *job_args, "--confirm-user-owned-write"],
        expect_json=True,
    )
    _run(
        canisend,
        ["stage", "run", *job_args, "--stage", "brief"],
        expect_json=True,
    )
    _assert_workspace_contract(workspace)


def _parse_args(arguments: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--canisend",
        required=True,
        help="CanISend console executable to test.",
    )
    parser.add_argument(
        "--workspace",
        required=True,
        type=Path,
        help="Fresh workspace path used for the smoke run.",
    )
    return parser.parse_args(arguments)


def main(arguments: Sequence[str] | None = None) -> int:
    args = _parse_args(arguments)
    try:
        run_smoke(args.canisend, args.workspace)
    except (OSError, UnicodeError, SmokeFailure) as exc:
        print(f"decision-spine smoke failed: {exc}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "status": "ok",
                "successful_stage_count": len(EXPECTED_STAGES),
                "mutation_receipt_count": EXPECTED_USER_MUTATION_RECEIPTS,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
