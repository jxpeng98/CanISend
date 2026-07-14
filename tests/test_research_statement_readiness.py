from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml
from typer.testing import CliRunner

import canisend.user_mutations as user_mutations
from canisend.cli import app
from canisend.stage_runtime import run_deterministic_stage
from canisend.stage_store import read_json_object
from canisend.user_file_store import UserFileStoreError
from canisend.user_mutation_agent import mutation_outcome_agent_response
from canisend.user_mutations import (
    ResetForCurrentReviewPatch,
    SetFindingDispositionPatch,
    UserMutationError,
    apply_user_patch,
    initialize_review_dispositions,
    inspect_review_dispositions,
    recover_user_mutation,
)
from tests.test_draft_stage import _candidate, _workspace
from tests.test_research_statement_stage import (
    PRIVATE_RESEARCH_CLAIM,
    _promote,
    _research_candidate,
)
from tests.test_user_mutation_cli import _artifact, _invoke_json, _job_args


def _reviewed_cover_and_research(
    tmp_path: Path,
) -> tuple[Path, Path, dict[str, object], dict[str, object]]:
    workspace, job = _workspace(tmp_path, include_research_statement=True)
    cover = _candidate(workspace, job, factual=True)
    research = _research_candidate(workspace, job)
    _promote(workspace, job, cover)
    _promote(workspace, job, research)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(cover["document_id"]),
    )
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(research["document_id"]),
    )
    return workspace, job, cover, research


def test_research_dispositions_require_document_selection_and_are_independent(
    tmp_path: Path,
) -> None:
    workspace, job, cover, research = _reviewed_cover_and_research(tmp_path)
    research_id = str(research["document_id"])

    with pytest.raises(UserMutationError) as ambiguous:
        inspect_review_dispositions(workspace, job)
    assert ambiguous.value.code == "user_input.document_ambiguous"

    missing = inspect_review_dispositions(
        workspace,
        job,
        document_id=research_id,
    )
    assert missing.artifact == "research_statement_review_dispositions"
    assert missing.document_kind == "research_statement"
    assert missing.readiness is not None
    assert missing.readiness.state == "review_required"
    assert not (job / "research_statement_review_dispositions.yaml").exists()

    initialized = initialize_review_dispositions(
        workspace,
        job,
        document_id=research_id,
        consent_confirmed=True,
    )
    assert initialized.snapshot.artifact == (
        "research_statement_review_dispositions"
    )
    assert initialized.snapshot.model.document_kind == "research_statement"
    assert (job / "research_statement_review_dispositions.yaml").is_file()
    assert not (job / "review_dispositions.yaml").exists()

    research_review = read_json_object(
        job / "research_statement_review_findings.json"
    )
    snapshot = initialized.snapshot
    outcome = initialized
    for finding in research_review["findings"]:
        outcome = apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=snapshot.sha256,
            expected_revision=snapshot.revision,
            document_id=research_id,
            consent_confirmed=True,
        )
        snapshot = outcome.snapshot

    final = inspect_review_dispositions(
        workspace,
        job,
        document_id=research_id,
    )
    response = mutation_outcome_agent_response(
        workspace,
        job,
        outcome,
        operation="review.dispositions_update",
    )
    assert final.readiness is not None
    assert final.readiness.document_kind == "research_statement"
    assert final.readiness.state == "reviewed"
    assert response.workflow is not None
    assert response.workflow.readiness == "ready_for_next_stage"
    assert response.artifacts[0].kind == (
        "research_statement_review_dispositions"
    )
    assert response.extensions["canisend.document_id"] == research_id
    assert response.extensions["canisend.document_kind"] == "research_statement"
    assert PRIVATE_RESEARCH_CLAIM not in response.model_dump_json()
    assert str(cover["document_id"]) != research_id


def test_cover_and_research_disposition_cas_namespaces_do_not_cross(
    tmp_path: Path,
) -> None:
    workspace, job, cover, research = _reviewed_cover_and_research(tmp_path)
    cover_id = str(cover["document_id"])
    research_id = str(research["document_id"])
    cover_initialized = initialize_review_dispositions(
        workspace,
        job,
        document_id=cover_id,
        consent_confirmed=True,
    )
    research_initialized = initialize_review_dispositions(
        workspace,
        job,
        document_id=research_id,
        consent_confirmed=True,
    )
    cover_bytes = (job / "review_dispositions.yaml").read_bytes()
    research_bytes = (
        job / "research_statement_review_dispositions.yaml"
    ).read_bytes()
    finding_id = read_json_object(
        job / "research_statement_review_findings.json"
    )["findings"][0]["finding_id"]
    patch = SetFindingDispositionPatch(
        finding_id=finding_id,
        disposition="accepted",
    )

    with pytest.raises(UserMutationError) as conflict:
        apply_user_patch(
            workspace,
            job,
            patch,
            expected_sha256=cover_initialized.snapshot.sha256,
            expected_revision=cover_initialized.snapshot.revision,
            document_id=research_id,
            consent_confirmed=True,
        )
    assert conflict.value.code == "user_input.conflict"
    assert (job / "review_dispositions.yaml").read_bytes() == cover_bytes
    assert (
        job / "research_statement_review_dispositions.yaml"
    ).read_bytes() == research_bytes

    updated = apply_user_patch(
        workspace,
        job,
        patch,
        expected_sha256=research_initialized.snapshot.sha256,
        expected_revision=research_initialized.snapshot.revision,
        document_id=research_id,
        consent_confirmed=True,
    )
    assert updated.snapshot.artifact == "research_statement_review_dispositions"
    assert (job / "review_dispositions.yaml").read_bytes() == cover_bytes
    assert (
        job
        / "workflow"
        / "user-mutations"
        / "claims"
        / "review_dispositions"
    ).is_dir()
    assert (
        job
        / "workflow"
        / "user-mutations"
        / "claims"
        / "research_statement_review_dispositions"
    ).is_dir()


def test_research_disposition_cli_is_explicit_body_free_and_resumable(
    tmp_path: Path,
) -> None:
    workspace, job, _cover, research = _reviewed_cover_and_research(tmp_path)
    research_id = str(research["document_id"])
    runner = CliRunner()

    ambiguous = _invoke_json(
        runner,
        ["review-dispositions", "status", *_job_args(workspace, job)],
        expected_exit=1,
    )
    assert ambiguous["error"]["code"] == "user_input.document_ambiguous"
    assert ambiguous["next_actions"][0]["id"] == "documents.status"
    assert PRIVATE_RESEARCH_CLAIM not in json.dumps(ambiguous)

    missing = _invoke_json(
        runner,
        [
            "review-dispositions",
            "status",
            *_job_args(workspace, job),
            "--document-id",
            research_id,
        ],
    )
    assert missing["missing_fields"] == [
        "research_statement_review_dispositions.yaml"
    ]
    assert missing["extensions"]["canisend.document_kind"] == (
        "research_statement"
    )

    denied = _invoke_json(
        runner,
        [
            "review-dispositions",
            "init",
            *_job_args(workspace, job),
            "--document-id",
            research_id,
        ],
        expected_exit=1,
    )
    assert denied["error"]["code"] == "user_input.consent_required"
    assert denied["required_consents"][0]["artifact_kinds"] == [
        "research_statement_review_dispositions"
    ]

    initialized = _invoke_json(
        runner,
        [
            "review-dispositions",
            "init",
            *_job_args(workspace, job),
            "--document-id",
            research_id,
            "--confirm-user-owned-write",
        ],
    )
    baseline = _artifact(
        initialized,
        "research_statement_review_dispositions",
    )
    finding_id = read_json_object(
        job / "research_statement_review_findings.json"
    )["findings"][0]["finding_id"]
    patch_path = tmp_path / "research-disposition.yaml"
    patch_path.write_text(
        yaml.safe_dump(
            {
                "operation": "set_finding_disposition",
                "finding_id": finding_id,
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
            "--document-id",
            research_id,
            "--patch-file",
            str(patch_path),
            "--expected-revision",
            "0",
            "--expected-sha256",
            baseline["sha256"],
            "--confirm-user-owned-write",
        ],
    )
    assert _artifact(
        updated,
        "research_statement_review_dispositions",
    )["privacy_tier"] == 2
    assert PRIVATE_RESEARCH_CLAIM not in json.dumps(updated)


def test_research_disposition_receipt_failure_is_recoverable(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _workspace(
        tmp_path,
        include_cover_letter=False,
        include_research_statement=True,
    )
    research = _research_candidate(workspace, job)
    research_id = str(research["document_id"])
    _promote(workspace, job, research)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=research_id,
    )
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
        document_id=research_id,
        consent_confirmed=True,
    )
    assert pending.status == "committed_receipt_pending"
    assert pending.mutation_id is not None
    committed_bytes = (
        job / "research_statement_review_dispositions.yaml"
    ).read_bytes()

    monkeypatch.setattr(user_mutations, "_store_receipt", original_store_receipt)
    recovered = recover_user_mutation(
        workspace,
        job,
        pending.mutation_id,
        consent_confirmed=True,
    )
    assert recovered.status == "committed"
    assert recovered.changed is False
    assert (
        job / "research_statement_review_dispositions.yaml"
    ).read_bytes() == committed_bytes


def test_stale_research_dispositions_are_preserved_until_explicit_reset(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(
        tmp_path,
        include_cover_letter=False,
        include_research_statement=True,
    )
    research = _research_candidate(workspace, job)
    research_id = str(research["document_id"])
    _promote(workspace, job, research)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=research_id,
    )
    initialize_review_dispositions(
        workspace,
        job,
        document_id=research_id,
        consent_confirmed=True,
    )
    path = job / "research_statement_review_dispositions.yaml"
    payload = yaml.safe_load(path.read_text(encoding="utf-8"))
    payload["review_findings_sha256"] = "0" * 64
    payload["revision"] = 1
    path.write_text(yaml.safe_dump(payload, sort_keys=True), encoding="utf-8")

    stale = inspect_review_dispositions(
        workspace,
        job,
        document_id=research_id,
    )
    assert stale.snapshot is not None
    assert stale.basis_status == "review_required"
    stale_bytes = path.read_bytes()
    finding_id = read_json_object(
        job / "research_statement_review_findings.json"
    )["findings"][0]["finding_id"]

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
            document_id=research_id,
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
        document_id=research_id,
        consent_confirmed=True,
    )
    assert reset.snapshot.model.review_findings_sha256 != "0" * 64
    assert inspect_review_dispositions(
        workspace,
        job,
        document_id=research_id,
    ).basis_status == "current"


def test_research_statement_blockers_cannot_be_accepted(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(
        tmp_path,
        include_cover_letter=False,
        include_research_statement=True,
    )
    research = _research_candidate(workspace, job)
    research["sections"] = [research["sections"][0]]
    research_id = str(research["document_id"])
    _promote(workspace, job, research)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=research_id,
    )
    initialized = initialize_review_dispositions(
        workspace,
        job,
        document_id=research_id,
        consent_confirmed=True,
    )
    review = read_json_object(job / "research_statement_review_findings.json")
    blocker_id = review["blocker_finding_ids"][0]
    path = job / "research_statement_review_dispositions.yaml"
    before = path.read_bytes()

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
            document_id=research_id,
            consent_confirmed=True,
        )
    assert failure.value.code == "user_input.invalid"
    assert path.read_bytes() == before
    inspection = inspect_review_dispositions(
        workspace,
        job,
        document_id=research_id,
    )
    assert inspection.readiness is not None
    assert inspection.readiness.state == "blocked"
