from __future__ import annotations

import json
from pathlib import Path

import yaml
from typer.testing import CliRunner

from canisend.cli import app
from tests.test_draft_views import _reviewed_draft
from tests.test_user_mutation_cli import _artifact, _invoke_json, _job_args


def test_review_disposition_cli_is_consent_guarded_body_free_and_resumable(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    runner = CliRunner()

    missing = _invoke_json(
        runner,
        ["review-dispositions", "status", *_job_args(workspace, job)],
    )
    assert missing["workflow"]["readiness"] == "action_required"
    assert missing["missing_fields"] == ["review_dispositions.yaml"]
    assert missing["next_actions"][0]["id"] == "review.dispositions_initialize"

    denied = _invoke_json(
        runner,
        ["review-dispositions", "init", *_job_args(workspace, job)],
        expected_exit=1,
    )
    assert denied["error"]["code"] == "user_input.consent_required"
    assert not (job / "review_dispositions.yaml").exists()

    initialized = _invoke_json(
        runner,
        [
            "review-dispositions",
            "init",
            *_job_args(workspace, job),
            "--confirm-user-owned-write",
        ],
    )
    baseline = _artifact(initialized, "review_dispositions")
    assert baseline["privacy_tier"] == 2
    assert initialized["extensions"]["canisend.document_readiness"] == (
        "review_required"
    )

    review = json.loads((job / "review_findings.json").read_text(encoding="utf-8"))
    snapshot = baseline
    revision = 0
    for index, finding in enumerate(review["findings"]):
        patch = tmp_path / f"review-disposition-{index}.yaml"
        patch.write_text(
            yaml.safe_dump(
                {
                    "operation": "set_finding_disposition",
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
                "review-dispositions",
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
        snapshot = _artifact(updated, "review_dispositions")
        revision += 1

    assert updated["workflow"]["readiness"] == "ready_for_next_stage"
    assert updated["extensions"]["canisend.document_readiness"] == "reviewed"
    resumed = _invoke_json(
        runner,
        ["review-dispositions", "status", *_job_args(workspace, job)],
    )
    assert resumed["workflow"]["readiness"] == "ready_for_next_stage"


def test_agent_capabilities_advertise_review_disposition_operations() -> None:
    response = CliRunner().invoke(app, ["agent", "capabilities", "--format", "json"])

    assert response.exit_code == 0, response.output
    operations = json.loads(response.output)["capabilities"]["operations"]
    assert "review.dispositions_status" in operations
    assert "review.dispositions_initialize" in operations
    assert "review.dispositions_update" in operations
