from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml

import canisend.user_mutations as user_mutations
from canisend.decision_models import ApplicationBriefV1, ApplicationDecisionV1
from canisend.stage_runtime import run_deterministic_stage
from canisend.user_mutations import (
    APPLICATION_BRIEF_PATH,
    ConfirmDocumentRequirementsPatch,
    ReconfirmApplicationBriefPatch,
    RemoveDocumentChoicePatch,
    ResetBriefFieldPatch,
    SetBriefEmphasisPatch,
    SetBriefExclusionsPatch,
    SetBriefTextPatch,
    SetDecisionPatch,
    SetDocumentChoicePatch,
    UserMutationError,
    apply_user_patch,
    initialize_application_brief,
    initialize_application_decision,
    inspect_application_brief,
    inspect_current_artifact_mutation,
    parse_brief_patch,
)


def _write_workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    job.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (job / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/job",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct and evidence-led",
                "created_at": "2026-07-11T10:00:00Z",
                "updated_at": "2026-07-11T10:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job / "job_advert.md").write_text(
        """# Lecturer in Economics

Department: Economics
Location: London
Required documents: CV, Cover letter

Essential criteria:
- PhD in Economics

Desirable criteria:
- Experience teaching econometrics
""",
        encoding="utf-8",
    )
    return workspace, job


def _run_to_match(workspace: Path, job: Path) -> None:
    run_deterministic_stage(workspace, job, stage="parse")
    run_deterministic_stage(workspace, job, stage="confirm")
    run_deterministic_stage(workspace, job, stage="evidence")
    run_deterministic_stage(workspace, job, stage="match")


def _create_apply_decision(workspace: Path, job: Path) -> None:
    initialized = initialize_application_decision(
        workspace,
        job,
        consent_confirmed=True,
    )
    apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(decision="apply"),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=initialized.snapshot.revision,
        consent_confirmed=True,
    )


def _apply_brief_patch(
    workspace: Path,
    job: Path,
    snapshot: object,
    patch: object,
):
    return apply_user_patch(
        workspace,
        job,
        patch,  # type: ignore[arg-type]
        expected_sha256=snapshot.sha256,  # type: ignore[attr-defined]
        expected_revision=snapshot.revision,  # type: ignore[attr-defined]
        consent_confirmed=True,
    )


def test_brief_initialization_bootstraps_legacy_preferences_once(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _create_apply_decision(workspace, job)

    initialized = initialize_application_brief(
        workspace,
        job,
        consent_confirmed=True,
    )

    assert initialized.snapshot.artifact == "brief"
    assert initialized.snapshot.relative_path == APPLICATION_BRIEF_PATH
    assert isinstance(initialized.snapshot.model, ApplicationBriefV1)
    assert initialized.snapshot.model.language.value == "uk"
    assert initialized.snapshot.model.language.confirmation_state == "confirmed"
    assert initialized.snapshot.model.writing_style.value == "direct and evidence-led"
    assert initialized.snapshot.model.writing_style.confirmation_state == "confirmed"
    assert initialized.snapshot.model.document_requirements_confirmation.state == "unconfirmed"
    assert inspect_current_artifact_mutation(workspace, job, "brief").status == "committed"

    original = (job / APPLICATION_BRIEF_PATH).read_bytes()
    metadata = yaml.safe_load((job / "job.yaml").read_text(encoding="utf-8"))
    metadata["english_variant"] = "us"
    metadata["writing_style"] = "warm"
    (job / "job.yaml").write_text(yaml.safe_dump(metadata), encoding="utf-8")
    reused = initialize_application_brief(workspace, job, consent_confirmed=True)

    assert reused.status == "reused"
    assert reused.changed is False
    assert (job / APPLICATION_BRIEF_PATH).read_bytes() == original


def test_missing_brief_reports_the_actual_decision_gate(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)

    missing_decision = inspect_application_brief(workspace, job)
    assert missing_decision.snapshot is None
    assert missing_decision.reason == "decision.not_initialized"

    undecided = initialize_application_decision(
        workspace,
        job,
        consent_confirmed=True,
    )
    assert inspect_application_brief(workspace, job).reason == "decision.undecided"

    held = apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(decision="hold"),
        expected_sha256=undecided.snapshot.sha256,
        expected_revision=undecided.snapshot.revision,
        consent_confirmed=True,
    )
    hold_inspection = inspect_application_brief(workspace, job)
    assert hold_inspection.basis_status == "unavailable"
    assert hold_inspection.reason == "decision.not_apply"

    apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(decision="apply"),
        expected_sha256=held.snapshot.sha256,
        expected_revision=held.snapshot.revision,
        consent_confirmed=True,
    )
    current_apply = inspect_application_brief(workspace, job)
    assert current_apply.basis_status == "unavailable"
    assert current_apply.reason == "user_input.not_initialized"


def test_brief_scoped_fields_and_reference_validation(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _create_apply_decision(workspace, job)
    outcome = initialize_application_brief(workspace, job, consent_confirmed=True)

    for patch in (
        SetBriefTextPatch(field="motivation", value="A private motivation sentinel."),
        SetBriefExclusionsPatch(items=("Do not overstate publication leadership.",)),
    ):
        outcome = _apply_brief_patch(workspace, job, outcome.snapshot, patch)

    criteria = json.loads((job / "criteria.json").read_text(encoding="utf-8"))
    criterion_id = criteria["criteria"][0]["criterion_id"]
    outcome = _apply_brief_patch(
        workspace,
        job,
        outcome.snapshot,
        SetBriefEmphasisPatch(criterion_ids=(criterion_id,), evidence_ref_ids=()),
    )

    assert isinstance(outcome.snapshot.model, ApplicationBriefV1)
    assert outcome.snapshot.model.motivation.confirmation_state == "confirmed"
    assert outcome.snapshot.model.exclusions.confirmation_state == "confirmed"
    assert outcome.snapshot.model.emphasis.criterion_ids == (criterion_id,)

    with pytest.raises(UserMutationError) as orphaned:
        _apply_brief_patch(
            workspace,
            job,
            outcome.snapshot,
            SetBriefEmphasisPatch(
                criterion_ids=("criterion_" + "f" * 32,),
                evidence_ref_ids=(),
            ),
        )
    assert orphaned.value.code == "user_input.invalid"

    reset = _apply_brief_patch(
        workspace,
        job,
        outcome.snapshot,
        ResetBriefFieldPatch(field="motivation"),
    )
    assert reset.snapshot.model.motivation.value is None
    assert "motivation" in inspect_application_brief(workspace, job).unresolved_fields


def test_document_requirement_confirmation_and_choice_are_basis_scoped(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _create_apply_decision(workspace, job)
    brief = initialize_application_brief(workspace, job, consent_confirmed=True)
    run_deterministic_stage(workspace, job, stage="brief")
    plan = json.loads((job / "required_document_plan.json").read_text(encoding="utf-8"))

    confirmed = _apply_brief_patch(
        workspace,
        job,
        brief.snapshot,
        ConfirmDocumentRequirementsPatch(
            state="confirmed",
            requirements_basis_sha256=plan["requirements_basis_sha256"],
        ),
    )
    assert confirmed.snapshot.model.document_requirements_confirmation.state == "confirmed"

    run_deterministic_stage(workspace, job, stage="brief")
    current_plan = json.loads(
        (job / "required_document_plan.json").read_text(encoding="utf-8")
    )
    document_id = current_plan["requirements"][0]["document_id"]
    chosen = _apply_brief_patch(
        workspace,
        job,
        confirmed.snapshot,
        SetDocumentChoicePatch(document_id=document_id, action="omit"),
    )
    assert chosen.snapshot.model.document_choices[0].document_id == document_id

    removed = _apply_brief_patch(
        workspace,
        job,
        chosen.snapshot,
        RemoveDocumentChoicePatch(document_id=document_id),
    )
    assert removed.snapshot.model.document_choices == ()

    with pytest.raises(UserMutationError) as stale_basis:
        _apply_brief_patch(
            workspace,
            job,
            removed.snapshot,
            ConfirmDocumentRequirementsPatch(
                state="confirmed",
                requirements_basis_sha256="f" * 64,
            ),
        )
    assert stale_basis.value.code in {
        "user_input.conflict",
        "user_input.dependency_not_current",
    }


def test_unknown_document_sources_cannot_be_confirmed(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    advert_path = job / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            "Required documents: CV, Cover letter\n",
            "Required documents: CV, Cover letter\nPlease submit a CV.\n",
        ),
        encoding="utf-8",
    )
    _run_to_match(workspace, job)
    _create_apply_decision(workspace, job)
    brief = initialize_application_brief(workspace, job, consent_confirmed=True)
    run_deterministic_stage(workspace, job, stage="brief")
    plan = json.loads((job / "required_document_plan.json").read_text(encoding="utf-8"))
    assert any(item["source_state"] == "unknown" for item in plan["requirements"])

    with pytest.raises(UserMutationError) as unknown_source:
        _apply_brief_patch(
            workspace,
            job,
            brief.snapshot,
            ConfirmDocumentRequirementsPatch(
                state="confirmed",
                requirements_basis_sha256=plan["requirements_basis_sha256"],
            ),
        )

    assert unknown_source.value.code == "user_input.invalid"
    assert inspect_application_brief(
        workspace,
        job,
    ).snapshot.model.document_requirements_confirmation.state == "unconfirmed"


@pytest.mark.parametrize("patch_kind", ["requirements", "choice"])
def test_plan_based_brief_patch_rechecks_plan_before_commit(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    patch_kind: str,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _create_apply_decision(workspace, job)
    brief = initialize_application_brief(workspace, job, consent_confirmed=True)
    run_deterministic_stage(workspace, job, stage="brief")
    plan = json.loads((job / "required_document_plan.json").read_text(encoding="utf-8"))
    patch = (
        ConfirmDocumentRequirementsPatch(
            state="confirmed",
            requirements_basis_sha256=plan["requirements_basis_sha256"],
        )
        if patch_kind == "requirements"
        else SetDocumentChoicePatch(
            document_id=plan["requirements"][0]["document_id"],
            action="prepare",
        )
    )
    original_brief = (job / APPLICATION_BRIEF_PATH).read_bytes()
    original_plan_reader = user_mutations._current_required_document_plan

    def replace_plan_after_snapshot(workspace_path: Path, job_path: Path):
        current = original_plan_reader(workspace_path, job_path)
        _plan, snapshot = current
        snapshot.path.write_bytes(snapshot.data + b"\n")
        return current

    monkeypatch.setattr(
        user_mutations,
        "_current_required_document_plan",
        replace_plan_after_snapshot,
    )

    with pytest.raises(UserMutationError) as stale_plan:
        _apply_brief_patch(workspace, job, brief.snapshot, patch)

    assert stale_plan.value.code == "user_input.dependency_not_current"
    assert (job / APPLICATION_BRIEF_PATH).read_bytes() == original_brief


def test_brief_reconfirmation_binds_the_current_apply_decision(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _create_apply_decision(workspace, job)
    brief = initialize_application_brief(workspace, job, consent_confirmed=True)
    decision_path = job / "application_decision.yaml"
    decision_payload = yaml.safe_load(decision_path.read_text(encoding="utf-8"))
    decision = ApplicationDecisionV1.model_validate(decision_payload)
    from canisend.user_mutations import inspect_user_artifact

    decision_snapshot = inspect_user_artifact(workspace, job, "decision")
    assert decision_snapshot is not None
    apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(decision="apply", rationale_mode="set", rationale="Updated basis."),
        expected_sha256=decision_snapshot.sha256,
        expected_revision=decision.revision,
        consent_confirmed=True,
    )
    assert inspect_application_brief(workspace, job).basis_status == "review_required"

    edited_while_stale = _apply_brief_patch(
        workspace,
        job,
        brief.snapshot,
        SetBriefTextPatch(field="motivation", value="Updated while basis remains stale."),
    )
    assert (
        edited_while_stale.snapshot.model.decision_sha256
        == brief.snapshot.model.decision_sha256
    )
    assert inspect_application_brief(workspace, job).basis_status == "review_required"

    refreshed = _apply_brief_patch(
        workspace,
        job,
        edited_while_stale.snapshot,
        ReconfirmApplicationBriefPatch(),
    )
    assert refreshed.snapshot.model.decision_sha256 != brief.snapshot.model.decision_sha256
    assert inspect_application_brief(workspace, job).basis_status == "current"


def test_brief_patch_parser_rejects_whole_file_replacement() -> None:
    with pytest.raises(UserMutationError) as invalid:
        parse_brief_patch(
            {
                "operation": "replace_brief",
                "motivation": "private body",
            }
        )
    assert invalid.value.code == "user_input.invalid"
