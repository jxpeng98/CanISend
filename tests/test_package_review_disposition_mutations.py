from __future__ import annotations

from pathlib import Path

import pytest
import yaml

import canisend.user_mutations as user_mutations
from canisend.stage_runtime import run_deterministic_stage
from canisend.stage_store import read_json_object
from canisend.user_file_store import UserFileStoreError
from canisend.user_mutations import (
    ClearPackageFindingDispositionPatch,
    ResetForCurrentPackageReviewPatch,
    SetPackageFindingDispositionPatch,
    UserMutationError,
    apply_user_patch,
    initialize_package_review_dispositions,
    inspect_package_review_dispositions,
    recover_user_mutation,
)
from tests.test_package_review_stage import _reviewed_dual_documents


def _current_reviewable_package(
    tmp_path: Path,
) -> tuple[Path, Path, dict[str, object]]:
    workspace, job, _cover, _research = _reviewed_dual_documents(
        tmp_path,
        include_cv=False,
    )
    run_deterministic_stage(workspace, job, stage="package_review")
    review = read_json_object(job / "package_review_findings.json")
    assert review["blocker_finding_ids"] == []
    assert review["findings"]
    return workspace, job, review


def test_package_dispositions_use_guarded_cas_and_derive_reviewed(
    tmp_path: Path,
) -> None:
    workspace, job, review = _current_reviewable_package(tmp_path)
    protected = {
        name: (job / name).read_bytes()
        for name in (
            "cover_letter_draft.json",
            "research_statement_draft.json",
            "review_findings.json",
            "research_statement_review_findings.json",
            "package_review_findings.json",
        )
    }

    initialized = initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    inspection = inspect_package_review_dispositions(workspace, job)

    assert initialized.snapshot.revision == 0
    assert initialized.snapshot.artifact == "package_review_dispositions"
    assert inspection.basis_status == "current"
    assert inspection.readiness is not None
    assert inspection.readiness.state == "review_required"

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

    final = inspect_package_review_dispositions(workspace, job)
    assert final.readiness is not None
    assert final.readiness.state == "reviewed"
    assert final.readiness.required_document_ids == tuple(
        sorted(item["document_id"] for item in review["documents"])
    )
    assert len(final.readiness.accepted_finding_ids) == len(review["findings"])
    assert {
        name: (job / name).read_bytes() for name in protected
    } == protected

    selected_finding_id = review["findings"][0]["finding_id"]
    cleared = apply_user_patch(
        workspace,
        job,
        ClearPackageFindingDispositionPatch(finding_id=selected_finding_id),
        expected_sha256=snapshot.sha256,
        expected_revision=snapshot.revision,
        consent_confirmed=True,
    )
    partial = inspect_package_review_dispositions(workspace, job)
    assert partial.readiness is not None
    assert partial.readiness.state == "review_required"
    assert selected_finding_id in partial.readiness.unresolved_finding_ids

    apply_user_patch(
        workspace,
        job,
        SetPackageFindingDispositionPatch(
            finding_id=selected_finding_id,
            disposition="revision_required",
        ),
        expected_sha256=cleared.snapshot.sha256,
        expected_revision=cleared.snapshot.revision,
        consent_confirmed=True,
    )
    revision = inspect_package_review_dispositions(workspace, job)
    assert revision.readiness is not None
    assert revision.readiness.state == "revision_required"
    assert revision.readiness.revision_required_finding_ids == (
        selected_finding_id,
    )


def test_package_blocker_disposition_is_nonwaivable(tmp_path: Path) -> None:
    workspace, job, _cover, _research = _reviewed_dual_documents(tmp_path)
    run_deterministic_stage(workspace, job, stage="package_review")
    initialized = initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    blocker_id = read_json_object(job / "package_review_findings.json")[
        "blocker_finding_ids"
    ][0]
    before = (job / "package_review_dispositions.yaml").read_bytes()

    with pytest.raises(UserMutationError) as failure:
        apply_user_patch(
            workspace,
            job,
            SetPackageFindingDispositionPatch(
                finding_id=blocker_id,
                disposition="accepted",
            ),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=initialized.snapshot.revision,
            consent_confirmed=True,
        )

    assert failure.value.code == "user_input.invalid"
    assert (job / "package_review_dispositions.yaml").read_bytes() == before
    inspection = inspect_package_review_dispositions(workspace, job)
    assert inspection.readiness is not None
    assert inspection.readiness.state == "blocked"


def test_stale_package_basis_is_preserved_until_explicit_reset(
    tmp_path: Path,
) -> None:
    workspace, job, review = _current_reviewable_package(tmp_path)
    initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    path = job / "package_review_dispositions.yaml"
    payload = yaml.safe_load(path.read_text(encoding="utf-8"))
    payload["package_review_findings_sha256"] = "0" * 64
    payload["revision"] = 1
    path.write_text(yaml.safe_dump(payload, sort_keys=True), encoding="utf-8")

    stale = inspect_package_review_dispositions(workspace, job)
    assert stale.snapshot is not None
    assert stale.basis_status == "review_required"
    assert stale.readiness is not None
    assert stale.readiness.state == "review_required"
    stale_bytes = path.read_bytes()

    with pytest.raises(UserMutationError) as failure:
        apply_user_patch(
            workspace,
            job,
            SetPackageFindingDispositionPatch(
                finding_id=review["findings"][0]["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=stale.snapshot.sha256,
            expected_revision=stale.snapshot.revision,
            consent_confirmed=True,
        )
    assert failure.value.code == "user_input.dependency_not_current"
    assert path.read_bytes() == stale_bytes

    reset = apply_user_patch(
        workspace,
        job,
        ResetForCurrentPackageReviewPatch(),
        expected_sha256=stale.snapshot.sha256,
        expected_revision=stale.snapshot.revision,
        consent_confirmed=True,
    )
    assert reset.snapshot.model.package_review_findings_sha256 != "0" * 64
    assert reset.snapshot.model.dispositions == ()
    assert inspect_package_review_dispositions(workspace, job).basis_status == "current"


def test_package_disposition_receipt_failure_recovers_without_replay(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job, _review = _current_reviewable_package(tmp_path)
    original_store_receipt = user_mutations._store_receipt
    monkeypatch.setattr(
        user_mutations,
        "_store_receipt",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            UserFileStoreError("injected receipt failure")
        ),
    )

    pending = initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    assert pending.status == "committed_receipt_pending"
    assert pending.mutation_id is not None
    committed = (job / "package_review_dispositions.yaml").read_bytes()

    monkeypatch.setattr(user_mutations, "_store_receipt", original_store_receipt)
    recovered = recover_user_mutation(
        workspace,
        job,
        pending.mutation_id,
        consent_confirmed=True,
    )

    assert recovered.status == "committed"
    assert recovered.changed is False
    assert recovered.receipt_path is not None
    assert (job / "package_review_dispositions.yaml").read_bytes() == committed
