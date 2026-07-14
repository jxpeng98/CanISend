from __future__ import annotations

from pathlib import Path

import pytest
import yaml

import canisend.user_mutations as user_mutations
from canisend.stage_store import read_json_object
from canisend.user_file_store import UserFileStoreError
from canisend.user_mutations import (
    ResetForCurrentReviewPatch,
    SetFindingDispositionPatch,
    UserMutationError,
    apply_user_patch,
    initialize_review_dispositions,
    inspect_review_dispositions,
    recover_user_mutation,
)
from tests.test_draft_views import _reviewed_draft


def test_review_dispositions_use_guarded_cas_and_derive_reviewed(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    protected = {
        name: (job / name).read_bytes()
        for name in ("cover_letter_draft.json", "review_findings.json")
    }

    initialized = initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    inspection = inspect_review_dispositions(workspace, job)

    assert initialized.snapshot.revision == 0
    assert inspection.basis_status == "current"
    assert inspection.readiness is not None
    assert inspection.readiness.state == "review_required"
    assert initialized.receipt_path is not None

    review = read_json_object(job / "review_findings.json")
    snapshot = initialized.snapshot
    for finding in review["findings"]:
        accepted = apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=snapshot.sha256,
            expected_revision=snapshot.revision,
            consent_confirmed=True,
        )
        snapshot = accepted.snapshot

    final = inspect_review_dispositions(workspace, job)
    assert final.readiness is not None
    assert final.readiness.state == "reviewed"
    assert len(final.readiness.accepted_finding_ids) == len(review["findings"])
    assert {
        name: (job / name).read_bytes()
        for name in ("cover_letter_draft.json", "review_findings.json")
    } == protected


def test_blocker_disposition_is_nonwaivable(tmp_path: Path) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path, unsupported=True)
    initialized = initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    blocker_id = read_json_object(job / "review_findings.json")[
        "blocker_finding_ids"
    ][0]
    before = (job / "review_dispositions.yaml").read_bytes()

    with pytest.raises(UserMutationError) as failure:
        apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=blocker_id,
                disposition="accepted",
            ),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=initialized.snapshot.revision,
            consent_confirmed=True,
        )

    assert failure.value.code == "user_input.invalid"
    assert (job / "review_dispositions.yaml").read_bytes() == before
    inspection = inspect_review_dispositions(workspace, job)
    assert inspection.readiness is not None
    assert inspection.readiness.state == "blocked"


def test_explicit_reset_rebinds_and_clears_current_dispositions(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    initialized = initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    finding_id = read_json_object(job / "review_findings.json")["findings"][0][
        "finding_id"
    ]
    accepted = apply_user_patch(
        workspace,
        job,
        SetFindingDispositionPatch(
            finding_id=finding_id,
            disposition="accepted",
        ),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=initialized.snapshot.revision,
        consent_confirmed=True,
    )

    reset = apply_user_patch(
        workspace,
        job,
        ResetForCurrentReviewPatch(),
        expected_sha256=accepted.snapshot.sha256,
        expected_revision=accepted.snapshot.revision,
        consent_confirmed=True,
    )

    assert reset.snapshot.revision == 2
    assert reset.snapshot.model.dispositions == ()
    inspection = inspect_review_dispositions(workspace, job)
    assert inspection.readiness is not None
    assert inspection.readiness.state == "review_required"


def test_review_disposition_receipt_failure_recovers_without_replaying_patch(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    original_store_receipt = user_mutations._store_receipt
    monkeypatch.setattr(
        user_mutations,
        "_store_receipt",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            UserFileStoreError("injected receipt failure")
        ),
    )

    pending = initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )

    assert pending.status == "committed_receipt_pending"
    assert pending.mutation_id is not None
    committed_bytes = (job / "review_dispositions.yaml").read_bytes()

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
    assert (job / "review_dispositions.yaml").read_bytes() == committed_bytes


def test_stale_manual_basis_is_preserved_until_explicit_reset(tmp_path: Path) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    path = job / "review_dispositions.yaml"
    payload = yaml.safe_load(path.read_text(encoding="utf-8"))
    payload["review_findings_sha256"] = "0" * 64
    payload["revision"] = 1
    path.write_text(yaml.safe_dump(payload, sort_keys=True), encoding="utf-8")

    stale = inspect_review_dispositions(workspace, job)
    assert stale.snapshot is not None
    assert stale.basis_status == "review_required"
    assert stale.readiness is not None
    assert stale.readiness.state == "review_required"
    stale_bytes = path.read_bytes()
    finding_id = read_json_object(job / "review_findings.json")["findings"][0][
        "finding_id"
    ]

    with pytest.raises(UserMutationError) as failure:
        apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=finding_id,
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
        ResetForCurrentReviewPatch(),
        expected_sha256=stale.snapshot.sha256,
        expected_revision=stale.snapshot.revision,
        consent_confirmed=True,
    )
    assert reset.snapshot.model.review_findings_sha256 != "0" * 64
    assert inspect_review_dispositions(workspace, job).basis_status == "current"


def test_orphaned_manual_disposition_requires_reset(tmp_path: Path) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    path = job / "review_dispositions.yaml"
    payload = yaml.safe_load(path.read_text(encoding="utf-8"))
    payload["revision"] = 1
    payload["dispositions"] = [
        {
            "finding_id": "finding_" + "0" * 32,
            "disposition": "accepted",
            "decided_at": payload["updated_at"],
        }
    ]
    path.write_text(yaml.safe_dump(payload, sort_keys=True), encoding="utf-8")

    inspection = inspect_review_dispositions(workspace, job)

    assert inspection.snapshot is not None
    assert inspection.basis_status == "review_required"
    assert inspection.reason == "review.disposition_orphaned"
    assert inspection.readiness is not None
    assert inspection.readiness.state == "review_required"
