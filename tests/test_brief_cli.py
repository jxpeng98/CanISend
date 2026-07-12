from __future__ import annotations

import json
from hashlib import sha256
from pathlib import Path

import pytest
import yaml
from typer.testing import CliRunner

from canisend.cli import app
from canisend.stage_runtime import run_deterministic_stage
from canisend.user_mutations import (
    ConfirmCriterionPatch,
    SetDecisionPatch,
    apply_user_patch,
    initialize_application_decision,
    initialize_confirmed_corrections,
)


PRIVATE_LEGACY_STYLE = "PRIVATE-LEGACY-STYLE-CLI-6193"
PRIVATE_MOTIVATION = "PRIVATE-MOTIVATION-CLI-7248"
PRIVATE_EXCLUSION = "PRIVATE-EXCLUSION-CLI-8351"
PRIVATE_UPDATED_STYLE = "PRIVATE-UPDATED-STYLE-CLI-9462"


def test_brief_status_init_requires_current_apply_and_bootstraps_legacy_once(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace_to_match(tmp_path)
    runner = CliRunner()
    decision = initialize_application_decision(
        workspace,
        job,
        consent_confirmed=True,
    )
    held = apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(decision="hold"),
        expected_revision=decision.snapshot.revision,
        expected_sha256=decision.snapshot.sha256,
        consent_confirmed=True,
    )

    held_status = _invoke_json(runner, ["brief", "status", *_job_args(workspace, job)])
    assert held_status["workflow"]["readiness"] == "review_required"
    assert held_status["next_actions"][0]["id"] == "decision.status"
    blocked = _invoke_json(
        runner,
        [
            "brief",
            "init",
            *_job_args(workspace, job),
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )
    assert blocked["error"]["code"] == "user_input.dependency_not_current"
    assert not (job / "application_brief.yaml").exists()

    applied = apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(decision="apply"),
        expected_revision=held.snapshot.revision,
        expected_sha256=held.snapshot.sha256,
        consent_confirmed=True,
    )
    assert applied.snapshot.revision == 2
    missing = _invoke_json(runner, ["brief", "status", *_job_args(workspace, job)])
    assert missing["workflow"]["readiness"] == "action_required"
    assert missing["missing_fields"] == ["application_brief.yaml"]
    assert missing["next_actions"][0]["id"] == "brief.initialize"
    assert missing["next_actions"][0]["requires_consent"] is True
    assert missing["required_consents"][0]["privacy_tier"] == 2

    denied = _invoke_json(
        runner,
        ["brief", "init", *_job_args(workspace, job)],
        expected_exit=1,
    )
    assert denied["error"]["code"] == "user_input.consent_required"
    assert not (job / "application_brief.yaml").exists()

    initialized = _invoke_json(
        runner,
        [
            "brief",
            "init",
            *_job_args(workspace, job),
            "--confirm-user-owned-write",
        ],
    )
    brief_ref = _artifact(initialized, "application_brief")
    receipt_ref = _artifact(initialized, "user_mutation_receipt")
    assert brief_ref["privacy_tier"] == 2
    assert receipt_ref["privacy_tier"] == 1
    assert initialized["extensions"]["canisend.mutation_status"] == "committed"
    assert initialized["next_actions"][0]["id"] == "stage.run_brief"
    assert PRIVATE_LEGACY_STYLE not in json.dumps(initialized)

    brief_path = job / "application_brief.yaml"
    brief = yaml.safe_load(brief_path.read_text(encoding="utf-8"))
    assert brief["language"] == {"value": "uk", "confirmation_state": "confirmed"}
    assert brief["writing_style"] == {
        "value": PRIVATE_LEGACY_STYLE,
        "confirmation_state": "confirmed",
    }
    original = brief_path.read_bytes()

    metadata = yaml.safe_load((job / "job.yaml").read_text(encoding="utf-8"))
    metadata["english_variant"] = "us"
    metadata["writing_style"] = "CHANGED-LEGACY-STYLE-MUST-NOT-SYNC"
    (job / "job.yaml").write_text(
        yaml.safe_dump(metadata, sort_keys=False),
        encoding="utf-8",
    )
    reused = _invoke_json(
        runner,
        [
            "brief",
            "init",
            *_job_args(workspace, job),
            "--confirm-user-owned-write",
        ],
    )
    assert reused["extensions"]["canisend.mutation_status"] == "reused"
    assert reused["extensions"]["canisend.mutation_changed"] is False
    assert brief_path.read_bytes() == original

    receipt_path = workspace / receipt_ref["path"]
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    assert receipt["artifact"] == "brief"
    assert receipt["target_path"] == "application_brief.yaml"
    assert PRIVATE_LEGACY_STYLE not in json.dumps(receipt)
    # The Brief receipt is the third user-owned artifact kind, not a special
    # side path alongside Corrections and Decision receipts.
    receipts = tuple((job / "workflow/user-mutations/events").glob("*/receipt.json"))
    receipt_artifacts = {
        json.loads(path.read_text(encoding="utf-8"))["artifact"] for path in receipts
    }
    assert receipt_artifacts == {"corrections", "decision", "brief"}


def test_brief_update_is_strict_cas_guarded_and_body_free(tmp_path: Path) -> None:
    workspace, job = _workspace_with_current_apply(tmp_path)
    runner = CliRunner()
    initialized = _brief_init(runner, workspace, job)
    first = _artifact(initialized, "application_brief")

    motivation_patch = _write_patch(
        tmp_path / "motivation.yaml",
        {
            "operation": "set_brief_text",
            "field": "motivation",
            "value": PRIVATE_MOTIVATION,
        },
    )
    motivated = _brief_update(
        runner,
        workspace,
        job,
        motivation_patch,
        revision=0,
        sha256=first["sha256"],
    )
    assert motivated["extensions"]["canisend.user_artifact_revision"] == 1
    assert PRIVATE_MOTIVATION not in json.dumps(motivated)

    exclusion_patch = _write_patch(
        tmp_path / "exclusions.yaml",
        {
            "operation": "set_brief_exclusions",
            "items": [PRIVATE_EXCLUSION],
        },
    )
    motivated_ref = _artifact(motivated, "application_brief")
    excluded = _brief_update(
        runner,
        workspace,
        job,
        exclusion_patch,
        revision=1,
        sha256=motivated_ref["sha256"],
    )
    assert PRIVATE_EXCLUSION not in json.dumps(excluded)

    style_patch = _write_patch(
        tmp_path / "style.yaml",
        {
            "operation": "set_brief_text",
            "field": "writing_style",
            "value": PRIVATE_UPDATED_STYLE,
        },
    )
    excluded_ref = _artifact(excluded, "application_brief")
    styled = _brief_update(
        runner,
        workspace,
        job,
        style_patch,
        revision=2,
        sha256=excluded_ref["sha256"],
    )
    assert styled["extensions"]["canisend.user_artifact_revision"] == 3

    stale = _brief_update(
        runner,
        workspace,
        job,
        style_patch,
        revision=0,
        sha256=first["sha256"],
        expected_exit=1,
    )
    assert stale["error"]["code"] == "user_input.conflict"

    whole_file = _write_patch(
        tmp_path / "whole-file.yaml",
        {
            "operation": "replace_brief",
            "motivation": PRIVATE_MOTIVATION,
        },
    )
    invalid = _brief_update(
        runner,
        workspace,
        job,
        whole_file,
        revision=3,
        sha256=_artifact(styled, "application_brief")["sha256"],
        expected_exit=1,
    )
    assert invalid["error"]["code"] == "user_input.invalid"

    duplicate = tmp_path / "duplicate.yaml"
    duplicate.write_text(
        "operation: set_brief_language\noperation: reset_brief_field\nvalue: uk\n",
        encoding="utf-8",
    )
    duplicate_response = _brief_update(
        runner,
        workspace,
        job,
        duplicate,
        revision=3,
        sha256=_artifact(styled, "application_brief")["sha256"],
        expected_exit=1,
    )
    assert duplicate_response["error"]["code"] == "user_input.invalid"

    status = _invoke_json(runner, ["brief", "status", *_job_args(workspace, job)])
    context = _context(runner, workspace, job)
    rendered = json.dumps([styled, stale, invalid, duplicate_response, status, context])
    for marker in (
        PRIVATE_LEGACY_STYLE,
        PRIVATE_MOTIVATION,
        PRIVATE_EXCLUSION,
        PRIVATE_UPDATED_STYLE,
    ):
        assert marker not in rendered
    brief_text = (job / "application_brief.yaml").read_text(encoding="utf-8")
    assert PRIVATE_MOTIVATION in brief_text
    assert PRIVATE_EXCLUSION in brief_text
    assert PRIVATE_UPDATED_STYLE in brief_text


def test_brief_stage_projects_unconfirmed_basis_then_accepts_scoped_confirmation(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace_with_current_apply(tmp_path)
    runner = CliRunner()
    initialized = _brief_init(runner, workspace, job)

    first_run = _invoke_json(
        runner,
        ["stage", "run", "--stage", "brief", *_job_args(workspace, job)],
    )
    first_plan = json.loads(
        (job / "required_document_plan.json").read_text(encoding="utf-8")
    )
    basis = first_plan["requirements_basis_sha256"]
    assert first_run["workflow"]["readiness"] == "blocked"
    assert first_run["extensions"]["canisend.document_requirements_state"] == "unconfirmed"
    assert first_run["extensions"]["canisend.document_requirements_basis_sha256"] == basis
    assert first_plan["requirements_state"] == "unconfirmed"
    assert "documents.requirements_unconfirmed" in first_plan["blockers"]
    assert set(first_plan["blocking_document_ids"]) == set(
        first_plan["unresolved_document_ids"]
    )
    assert all(task["action"] == "needs_confirmation" for task in first_plan["tasks"])

    current = initialized
    for name, patch in (
        (
            "requirements-confirm.yaml",
            {
                "operation": "confirm_document_requirements",
                "state": "confirmed",
                "requirements_basis_sha256": basis,
            },
        ),
        (
            "motivation-confirm.yaml",
            {
                "operation": "set_brief_text",
                "field": "motivation",
                "value": PRIVATE_MOTIVATION,
            },
        ),
        (
            "emphasis-confirm.yaml",
            {
                "operation": "set_brief_emphasis",
                "criterion_ids": [],
                "evidence_ref_ids": [],
            },
        ),
        (
            "exclusions-confirm.yaml",
            {"operation": "set_brief_exclusions", "items": []},
        ),
    ):
        current_ref = _artifact(current, "application_brief")
        current = _brief_update(
            runner,
            workspace,
            job,
            _write_patch(tmp_path / name, patch),
            revision=current["extensions"]["canisend.user_artifact_revision"],
            sha256=current_ref["sha256"],
        )

    rerun = _invoke_json(
        runner,
        ["stage", "run", "--stage", "brief", *_job_args(workspace, job)],
    )
    plan = json.loads((job / "required_document_plan.json").read_text(encoding="utf-8"))
    assert plan["requirements_basis_sha256"] == basis
    assert plan["requirements_state"] == "confirmed"
    assert "documents.requirements_unconfirmed" not in plan["blockers"]
    assert all(task["action"] == "prepare" for task in plan["tasks"])
    assert rerun["workflow"]["readiness"] == "ready_for_next_stage"
    assert rerun["extensions"]["canisend.document_plan_blocker_count"] == 0
    assert PRIVATE_MOTIVATION not in json.dumps(rerun)
    assert PRIVATE_MOTIVATION not in json.dumps(plan)


def test_required_document_omit_remains_an_explicit_agent_blocker(tmp_path: Path) -> None:
    workspace, job = _workspace_with_current_apply(tmp_path)
    runner = CliRunner()
    initialized = _brief_init(runner, workspace, job)
    _invoke_json(
        runner,
        ["stage", "run", "--stage", "brief", *_job_args(workspace, job)],
    )
    initial_plan = json.loads(
        (job / "required_document_plan.json").read_text(encoding="utf-8")
    )
    basis = initial_plan["requirements_basis_sha256"]
    current = initialized
    for name, patch in (
        (
            "requirements.yaml",
            {
                "operation": "confirm_document_requirements",
                "state": "confirmed",
                "requirements_basis_sha256": basis,
            },
        ),
        ("motivation.yaml", {"operation": "set_brief_text", "field": "motivation", "value": ""}),
        (
            "emphasis.yaml",
            {
                "operation": "set_brief_emphasis",
                "criterion_ids": [],
                "evidence_ref_ids": [],
            },
        ),
        ("exclusions.yaml", {"operation": "set_brief_exclusions", "items": []}),
    ):
        ref = _artifact(current, "application_brief")
        current = _brief_update(
            runner,
            workspace,
            job,
            _write_patch(tmp_path / name, patch),
            revision=current["extensions"]["canisend.user_artifact_revision"],
            sha256=ref["sha256"],
        )
    _invoke_json(
        runner,
        ["stage", "run", "--stage", "brief", *_job_args(workspace, job)],
    )
    confirmed_plan = json.loads(
        (job / "required_document_plan.json").read_text(encoding="utf-8")
    )
    required_id = next(
        item["document_id"]
        for item in confirmed_plan["requirements"]
        if item["requirement"] == "required"
    )
    ref = _artifact(current, "application_brief")
    omitted = _brief_update(
        runner,
        workspace,
        job,
        _write_patch(
            tmp_path / "omit.yaml",
            {
                "operation": "set_document_choice",
                "document_id": required_id,
                "action": "omit",
            },
        ),
        revision=current["extensions"]["canisend.user_artifact_revision"],
        sha256=ref["sha256"],
    )
    assert omitted["next_actions"][0]["id"] == "stage.run_brief"

    rerun = _invoke_json(
        runner,
        ["stage", "run", "--stage", "brief", *_job_args(workspace, job)],
    )
    plan = json.loads((job / "required_document_plan.json").read_text(encoding="utf-8"))
    task = next(item for item in plan["tasks"] if item["document_id"] == required_id)
    assert task["action"] == "omit"
    assert task["blockers"] == ["documents.required_omitted"]
    assert plan["blocking_document_ids"] == [required_id]
    assert "documents.required_omitted" in plan["blockers"]
    assert rerun["workflow"]["readiness"] == "blocked"
    assert rerun["next_actions"][0]["id"] == "brief.status"

    status = _invoke_json(runner, ["brief", "status", *_job_args(workspace, job)])
    assert status["workflow"]["readiness"] == "blocked"
    assert status["extensions"]["canisend.document_plan_readiness"] == "blocked"
    assert status["extensions"]["canisend.document_plan_blocker_count"] > 0
    assert status["extensions"]["canisend.blocking_document_count"] == 1
    assert status["extensions"]["canisend.document_plan_primary_blocker"]
    assert status["next_actions"][0]["id"] == "brief.update"
    assert PRIVATE_MOTIVATION not in json.dumps(status)


def test_fresh_agent_context_keeps_tier2_refs_and_decision_state_honest(
    tmp_path: Path,
) -> None:
    runner = CliRunner()

    missing_workspace, missing_job = _workspace_to_match(tmp_path / "missing")
    missing = _context(runner, missing_workspace, missing_job)
    assert missing["next_actions"][0]["id"] == "decision.initialize"
    assert "application_brief.yaml" not in missing["missing_fields"]

    for value in ("hold", "skip"):
        workspace, job = _workspace_to_match(tmp_path / value)
        initialized = initialize_application_decision(
            workspace,
            job,
            consent_confirmed=True,
        )
        apply_user_patch(
            workspace,
            job,
            SetDecisionPatch(decision=value),
            expected_revision=initialized.snapshot.revision,
            expected_sha256=initialized.snapshot.sha256,
            consent_confirmed=True,
        )
        stopped = _context(runner, workspace, job)
        assert stopped["workflow"]["readiness"] == "review_required"
        assert stopped["extensions"]["canisend.decision_value"] == value
        assert stopped["next_actions"] == []
        assert "application_brief.yaml" not in stopped["missing_fields"]

    stale_workspace, stale_job = _workspace_with_current_apply(tmp_path / "stale")
    stale_job.joinpath("job_advert.md").write_text(
        stale_job.joinpath("job_advert.md").read_text(encoding="utf-8")
        + "\nAdditional essential criterion: experience supervising dissertations.\n",
        encoding="utf-8",
    )
    for stage in ("parse", "confirm", "match"):
        run_deterministic_stage(stale_workspace, stale_job, stage=stage)
    stale = _context(runner, stale_workspace, stale_job)
    assert stale["workflow"]["readiness"] == "review_required"
    assert stale["extensions"]["canisend.decision_value"] == "apply"
    assert stale["extensions"]["canisend.decision_basis_status"] == "review_required"
    assert stale["next_actions"][0]["id"] == "decision.update"
    assert "application_brief.yaml" not in stale["missing_fields"]

    current_workspace, current_job = _workspace_with_current_apply(tmp_path / "current")
    initialized = _brief_init(runner, current_workspace, current_job)
    current = _context(runner, current_workspace, current_job)
    brief_ref = _artifact(current, "application_brief")
    assert brief_ref["privacy_tier"] == 2
    assert _artifact(initialized, "user_mutation_receipt")["privacy_tier"] == 1
    assert PRIVATE_LEGACY_STYLE not in json.dumps(current)


def _write_workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    profile = workspace / "profile"
    generated = profile / "generated"
    typst = profile / "typst"
    job.mkdir(parents=True)
    generated.mkdir(parents=True)
    typst.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (profile / "profile.yaml").write_text(
        "sources:\n"
        "  cv: typst/cv.typ\n"
        "generated:\n"
        "  cv_evidence: generated/cv.evidence.md\n",
        encoding="utf-8",
    )
    source = b"= Academic CV\n"
    (typst / "cv.typ").write_bytes(source)
    (generated / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        f"<!-- canisend-source-sha256: {sha256(source).hexdigest()} -->\n\n"
        "## Education\n\n"
        "- [cv-001] `education`: PhD in Economics.\n\n"
        "## Teaching\n\n"
        "- [cv-002] `teaching`: Led econometrics seminars.\n",
        encoding="utf-8",
    )
    (job / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": job.name,
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/job",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": PRIVATE_LEGACY_STYLE,
                "created_at": "2026-07-12T10:00:00Z",
                "updated_at": "2026-07-12T10:00:00Z",
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


def _workspace_to_match(tmp_path: Path) -> tuple[Path, Path]:
    workspace, job = _write_workspace(tmp_path)
    for stage in ("evidence", "parse", "confirm"):
        run_deterministic_stage(workspace, job, stage=stage)
    corrections = initialize_confirmed_corrections(
        workspace,
        job,
        consent_confirmed=True,
    )
    criteria = json.loads((job / "criteria.json").read_text(encoding="utf-8"))
    for criterion in criteria["criteria"]:
        corrections = apply_user_patch(
            workspace,
            job,
            ConfirmCriterionPatch(criterion_id=criterion["criterion_id"]),
            expected_revision=corrections.snapshot.revision,
            expected_sha256=corrections.snapshot.sha256,
            consent_confirmed=True,
        )
        run_deterministic_stage(workspace, job, stage="confirm")
    run_deterministic_stage(workspace, job, stage="match")
    return workspace, job


def _workspace_with_current_apply(tmp_path: Path) -> tuple[Path, Path]:
    workspace, job = _workspace_to_match(tmp_path)
    initialized = initialize_application_decision(
        workspace,
        job,
        consent_confirmed=True,
    )
    apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(decision="apply"),
        expected_revision=initialized.snapshot.revision,
        expected_sha256=initialized.snapshot.sha256,
        consent_confirmed=True,
    )
    return workspace, job


def _brief_init(runner: CliRunner, workspace: Path, job: Path) -> dict[str, object]:
    return _invoke_json(
        runner,
        [
            "brief",
            "init",
            *_job_args(workspace, job),
            "--confirm-user-owned-write",
        ],
    )


def _brief_update(
    runner: CliRunner,
    workspace: Path,
    job: Path,
    patch: Path,
    *,
    revision: int,
    sha256: str,
    expected_exit: int = 0,
) -> dict[str, object]:
    return _invoke_json(
        runner,
        [
            "brief",
            "update",
            *_job_args(workspace, job),
            "--patch-file",
            str(patch),
            "--expected-revision",
            str(revision),
            "--expected-sha256",
            sha256,
            "--confirm-user-owned-write",
        ],
        expected_exit=expected_exit,
    )


def _write_patch(path: Path, payload: dict[str, object]) -> Path:
    path.write_text(yaml.safe_dump(payload, sort_keys=False), encoding="utf-8")
    return path


def _context(runner: CliRunner, workspace: Path, job: Path) -> dict[str, object]:
    return _invoke_json(runner, ["agent", "context", *_job_args(workspace, job)])


def _job_args(workspace: Path, job: Path) -> list[str]:
    return [
        "--workspace",
        str(workspace),
        "--job",
        job.relative_to(workspace).as_posix(),
        "--format",
        "json",
    ]


def _invoke_json(
    runner: CliRunner,
    args: list[str],
    *,
    expected_exit: int = 0,
) -> dict[str, object]:
    result = runner.invoke(app, args)
    assert result.exit_code == expected_exit, result.output
    assert result.stdout.endswith("\n")
    assert result.stdout.count("\n") == 1
    payload = json.loads(result.stdout)
    assert payload["protocol"] == "canisend.agent/v1"
    return payload


def _artifact(payload: dict[str, object], kind: str) -> dict[str, object]:
    return next(item for item in payload["artifacts"] if item["kind"] == kind)
