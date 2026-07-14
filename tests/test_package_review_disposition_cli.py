from __future__ import annotations

import json
from pathlib import Path

import yaml
from typer.testing import CliRunner

from canisend.cli import app
from canisend.stage_runtime import run_deterministic_stage
from tests.test_package_review_stage import _reviewed_dual_documents
from tests.test_user_mutation_cli import _artifact, _invoke_json, _job_args


def test_package_review_cli_is_consent_guarded_body_free_and_resumable(
    tmp_path: Path,
) -> None:
    workspace, job, _cover, _research = _reviewed_dual_documents(
        tmp_path,
        include_cv=False,
    )
    run_deterministic_stage(workspace, job, stage="package_review")
    review = json.loads(
        (job / "package_review_findings.json").read_text(encoding="utf-8")
    )
    runner = CliRunner()

    missing = _invoke_json(
        runner,
        ["package-review", "status", *_job_args(workspace, job)],
    )
    assert missing["workflow"]["readiness"] == "action_required"
    assert missing["missing_fields"] == ["package_review_dispositions.yaml"]
    assert missing["next_actions"][0]["id"] == (
        "package_review.dispositions_initialize"
    )

    denied = _invoke_json(
        runner,
        ["package-review", "init", *_job_args(workspace, job)],
        expected_exit=1,
    )
    assert denied["error"]["code"] == "user_input.consent_required"
    assert not (job / "package_review_dispositions.yaml").exists()

    initialized = _invoke_json(
        runner,
        [
            "package-review",
            "init",
            *_job_args(workspace, job),
            "--confirm-user-owned-write",
        ],
    )
    snapshot = _artifact(initialized, "package_review_dispositions")
    assert snapshot["privacy_tier"] == 2
    assert initialized["extensions"][
        "canisend.application_package_readiness"
    ] == "review_required"

    revision = 0
    for index, finding in enumerate(review["findings"]):
        patch = tmp_path / f"package-review-disposition-{index}.yaml"
        patch.write_text(
            yaml.safe_dump(
                {
                    "operation": "set_package_finding_disposition",
                    "finding_id": finding["finding_id"],
                    "disposition": "accepted",
                },
                sort_keys=False,
            ),
            encoding="utf-8",
        )
        updated = _invoke_json(
            runner,
            [
                "package-review",
                "update",
                *_job_args(workspace, job),
                "--patch-file",
                str(patch),
                "--expected-revision",
                str(revision),
                "--expected-sha256",
                snapshot["sha256"],
                "--confirm-user-owned-write",
            ],
        )
        assert finding["message"] not in json.dumps(updated)
        snapshot = _artifact(updated, "package_review_dispositions")
        revision += 1

    assert updated["workflow"]["readiness"] == "ready_for_next_stage"
    assert updated["extensions"][
        "canisend.application_package_readiness"
    ] == "reviewed"
    assert updated["next_actions"][0]["id"] == "package.check"

    resumed = _invoke_json(
        runner,
        ["package-review", "status", *_job_args(workspace, job)],
    )
    assert resumed["workflow"]["readiness"] == "ready_for_next_stage"


def test_agent_capabilities_advertise_package_review_operations() -> None:
    response = CliRunner().invoke(app, ["agent", "capabilities", "--format", "json"])

    assert response.exit_code == 0, response.output
    operations = json.loads(response.output)["capabilities"]["operations"]
    assert "package_review.dispositions_status" in operations
    assert "package_review.dispositions_initialize" in operations
    assert "package_review.dispositions_update" in operations
