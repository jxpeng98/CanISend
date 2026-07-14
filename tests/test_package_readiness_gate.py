from __future__ import annotations

from pathlib import Path

import yaml

from canisend.ready_check import APP_Q5, check_application_package
from canisend.stage_runtime import run_deterministic_stage
from canisend.stage_store import read_json_object
from canisend.user_mutations import (
    SetPackageFindingDispositionPatch,
    apply_user_patch,
    initialize_package_review_dispositions,
    inspect_package_review_dispositions,
)
from tests.test_package_review_stage import _reviewed_dual_documents


def _aggregate_gate_messages(workspace: Path, job: Path) -> list[str]:
    result = check_application_package(
        job,
        workspace / "profile",
        workspace=workspace,
    )
    return [issue.message for issue in result.issues if issue.gate == APP_Q5]


def test_app_q5_requires_current_complete_aggregate_review_receipts(
    tmp_path: Path,
) -> None:
    workspace, job, _cover, _research = _reviewed_dual_documents(
        tmp_path,
        include_cv=False,
    )
    run_deterministic_stage(workspace, job, stage="package_review")
    review = read_json_object(job / "package_review_findings.json")

    assert _aggregate_gate_messages(workspace, job) == [
        "application package requires current complete package finding decisions"
    ]

    initialized = initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    snapshot = initialized.snapshot
    for finding in review["findings"]:
        updated = apply_user_patch(
            workspace,
            job,
            SetPackageFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=snapshot.sha256,
            expected_revision=snapshot.revision,
            consent_confirmed=True,
        )
        snapshot = updated.snapshot

    inspection = inspect_package_review_dispositions(workspace, job)
    assert inspection.readiness is not None
    assert inspection.readiness.state == "reviewed"
    assert _aggregate_gate_messages(workspace, job) == []

    disposition_path = job / "package_review_dispositions.yaml"
    stale = yaml.safe_load(disposition_path.read_text(encoding="utf-8"))
    stale["package_review_findings_sha256"] = "0" * 64
    disposition_path.write_text(
        yaml.safe_dump(stale, sort_keys=True),
        encoding="utf-8",
    )

    assert _aggregate_gate_messages(workspace, job) == [
        "application package requires current complete package finding decisions"
    ]
