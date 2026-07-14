from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re

import pytest
import yaml
from typer.testing import CliRunner

import canisend.user_mutation_agent as mutation_agent
import canisend.user_mutations as user_mutations
from canisend.cli import app
from canisend.examples import run_packaged_example
from canisend.stage_runtime import run_deterministic_stage
from canisend.user_file_store import UserFileStoreError
from canisend.user_mutations import (
    ConfirmCriterionPatch,
    ConfirmEmptyCriteriaPatch,
    CorrectCriterionPatch,
    MutationOutcome,
    ResetDecisionPatch,
    UserMutationError,
    apply_user_patch,
    initialize_confirmed_corrections,
    inspect_user_artifact,
)


PRIVATE_CORRECTION = "PRIVATE CORRECTION BODY MUST NOT APPEAR"
PRIVATE_RATIONALE = "PRIVATE DECISION RATIONALE MUST NOT APPEAR"
ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")


def _strip_ansi(value: str) -> str:
    return ANSI_ESCAPE_RE.sub("", value)


def test_recovery_conflict_routes_to_generic_control_review() -> None:
    response = mutation_agent.user_mutation_error_response(
        "user_mutation.recover",
        UserMutationError(
            "user_input.conflict",
            "private recovery conflict detail",
        ),
    ).model_dump(mode="json")

    assert response["workflow"]["readiness"] == "blocked"
    assert response["required_consents"] == []
    assert response["next_actions"] == [
        {
            "id": "user_mutation.review_controls",
            "label": "Review and coordinate the conflicting private mutation controls manually",
            "requires_consent": False,
            "consent_ids": [],
        }
    ]
    assert "decision.status" not in json.dumps(response)
    assert "private recovery conflict detail" not in json.dumps(response)


def test_user_owned_status_and_init_are_typed_consent_guarded_and_idempotent(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _minimal_workspace(tmp_path)
    runner = CliRunner()
    before = _tree_digest(workspace)

    missing = _invoke_json(
        runner,
        ["corrections", "status", *_job_args(workspace, job_dir)],
    )

    assert missing["workflow"] == {
        "phase": "unknown",
        "readiness": "action_required",
        "derived": True,
    }
    assert missing["missing_fields"] == ["confirmed_corrections.yaml"]
    assert missing["next_actions"][0]["id"] == "criteria.corrections_initialize"
    assert missing["next_actions"][0]["requires_consent"] is True
    assert _tree_digest(workspace) == before
    assert not (job_dir / "confirmed_corrections.yaml").exists()

    denied = _invoke_json(
        runner,
        ["corrections", "init", *_job_args(workspace, job_dir)],
        expected_exit=1,
    )
    assert denied["error"]["code"] == "user_input.consent_required"
    assert denied["required_consents"][0]["privacy_tier"] == 2
    assert not (job_dir / "confirmed_corrections.yaml").exists()

    created = _invoke_json(
        runner,
        [
            "corrections",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    artifact = next(item for item in created["artifacts"] if item["kind"] == "confirmed_corrections")
    receipt = next(item for item in created["artifacts"] if item["kind"] == "user_mutation_receipt")
    assert artifact["privacy_tier"] == 2
    assert receipt["privacy_tier"] == 1
    assert created["extensions"]["canisend.mutation_status"] == "committed"
    assert created["next_actions"][0]["id"] == "criteria.corrections_update"
    original = (job_dir / "confirmed_corrections.yaml").read_bytes()

    reused = _invoke_json(
        runner,
        [
            "corrections",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    assert reused["extensions"]["canisend.mutation_status"] == "reused"
    assert reused["extensions"]["canisend.mutation_changed"] is False
    assert (job_dir / "confirmed_corrections.yaml").read_bytes() == original


def test_corrections_update_accepts_only_strict_bounded_patch_and_cas_baseline(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _workspace_through_initial_confirm(tmp_path)
    runner = CliRunner()
    needs_overlay = _context(runner, workspace, job_dir)
    assert needs_overlay["workflow"]["phase"] == "unknown"
    assert needs_overlay["next_actions"][0]["id"] == "criteria.corrections_initialize"
    assert needs_overlay["next_actions"][0]["requires_consent"] is True
    initialized = _invoke_json(
        runner,
        [
            "corrections",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    baseline = _artifact(initialized, "confirmed_corrections")
    criterion_id = json.loads((job_dir / "criteria.json").read_text(encoding="utf-8"))["criteria"][0][
        "criterion_id"
    ]
    patch = tmp_path / "correction-patch.yaml"
    patch.write_text(
        yaml.safe_dump(
            {
                "operation": "correct_criterion",
                "criterion_id": criterion_id,
                "corrected_text": PRIVATE_CORRECTION,
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )

    updated = _invoke_json(
        runner,
        [
            "corrections",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(patch),
            "--expected-revision",
            "0",
            "--expected-sha256",
            baseline["sha256"],
            "--confirm-user-owned-write",
        ],
    )

    assert updated["workflow"]["phase"] == "unknown"
    assert updated["workflow"]["readiness"] == "action_required"
    assert updated["next_actions"] == [
        {
            "id": "stage.run_confirm",
            "label": "Rerun Confirm against the accepted corrections",
            "requires_consent": False,
            "consent_ids": [],
        }
    ]
    assert _artifact(updated, "confirmed_corrections")["privacy_tier"] == 2
    assert _artifact(updated, "user_mutation_receipt")["privacy_tier"] == 1
    assert PRIVATE_CORRECTION not in json.dumps(updated)
    assert PRIVATE_CORRECTION in (job_dir / "confirmed_corrections.yaml").read_text(encoding="utf-8")
    rerun = _context(runner, workspace, job_dir)
    assert rerun["next_actions"][0]["id"] == "stage.run_confirm"
    accepted_bytes = (job_dir / "confirmed_corrections.yaml").read_bytes()

    conflict = _invoke_json(
        runner,
        [
            "corrections",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(patch),
            "--expected-revision",
            "0",
            "--expected-sha256",
            baseline["sha256"],
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )
    assert conflict["error"]["code"] == "user_input.recovery_required"
    assert conflict["extensions"]["canisend.mutation_id"].startswith("mutation_")
    assert conflict["next_actions"][0]["id"] == "user_mutation.recover"
    assert (job_dir / "confirmed_corrections.yaml").read_bytes() == accepted_bytes

    duplicate = tmp_path / "duplicate.yaml"
    duplicate.write_text(
        f"operation: confirm_criterion\noperation: withdraw_criterion\ncriterion_id: {criterion_id}\n",
        encoding="utf-8",
    )
    invalid = _invoke_json(
        runner,
        [
            "corrections",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(duplicate),
            "--expected-revision",
            "1",
            "--expected-sha256",
            _sha256(accepted_bytes),
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )
    assert invalid["error"]["code"] == "user_input.invalid"
    assert PRIVATE_CORRECTION not in json.dumps(invalid)

    malformed_revision = _invoke_json(
        runner,
        [
            "corrections",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(patch),
            "--expected-revision",
            "not-a-revision",
            "--expected-sha256",
            _sha256(accepted_bytes),
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )
    assert malformed_revision["error"]["code"] == "user_input.invalid"
    assert (job_dir / "confirmed_corrections.yaml").read_bytes() == accepted_bytes

    oversized_revision = _invoke_json(
        runner,
        [
            "corrections",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(patch),
            "--expected-revision",
            str(2**63),
            "--expected-sha256",
            _sha256(accepted_bytes),
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )
    assert oversized_revision["error"]["code"] == "user_input.invalid"
    assert (job_dir / "confirmed_corrections.yaml").read_bytes() == accepted_bytes


def test_patch_file_symlink_and_user_artifact_aliases_fail_without_leaking_body(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _minimal_workspace(tmp_path)
    runner = CliRunner()
    secret = tmp_path / "private-patch.yaml"
    secret.write_text(f"operation: reset_decision\nnote: {PRIVATE_RATIONALE}\n", encoding="utf-8")
    alias = tmp_path / "patch-link.yaml"
    alias.symlink_to(secret)

    rejected_patch = _invoke_json(
        runner,
        [
            "decision",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(alias),
            "--expected-revision",
            "0",
            "--expected-sha256",
            "0" * 64,
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )
    assert rejected_patch["error"]["code"] == "user_input.unsafe_path"
    assert PRIVATE_RATIONALE not in json.dumps(rejected_patch)
    assert str(tmp_path) not in json.dumps(rejected_patch)

    if os.name != "nt":
        hard_patch = tmp_path / "patch-hardlink.yaml"
        os.link(secret, hard_patch)
        rejected_hard_patch = _invoke_json(
            runner,
            [
                "decision",
                "update",
                *_job_args(workspace, job_dir),
                "--patch-file",
                str(hard_patch),
                "--expected-revision",
                "0",
                "--expected-sha256",
                "0" * 64,
                "--confirm-user-owned-write",
            ],
            expected_exit=1,
        )
        assert rejected_hard_patch["error"]["code"] == "user_input.unsafe_path"
        assert PRIVATE_RATIONALE not in json.dumps(rejected_hard_patch)
        hard_patch.unlink()

    decision_alias = job_dir / "application_decision.yaml"
    decision_alias.symlink_to(secret)
    rejected_status = _invoke_json(
        runner,
        ["decision", "status", *_job_args(workspace, job_dir)],
        expected_exit=1,
    )
    assert rejected_status["error"]["code"] == "user_input.unsafe_path"
    assert rejected_status["artifacts"] == []
    assert PRIVATE_RATIONALE not in json.dumps(rejected_status)

    if os.name != "nt":
        decision_alias.unlink()
        os.link(secret, decision_alias)
        hardlinked = _invoke_json(
            runner,
            ["decision", "status", *_job_args(workspace, job_dir)],
            expected_exit=1,
        )
        assert hardlinked["error"]["code"] == "user_input.unsafe_path"
        assert PRIVATE_RATIONALE not in json.dumps(hardlinked)
        decision_alias.unlink()
    else:
        decision_alias.unlink()

    decision_alias.write_text(
        f"decision: apply\nrationale: {PRIVATE_RATIONALE}\n",
        encoding="utf-8",
    )
    invalid_body = _invoke_json(
        runner,
        ["decision", "status", *_job_args(workspace, job_dir)],
        expected_exit=1,
    )
    assert invalid_body["error"]["code"] == "user_input.invalid"
    assert PRIVATE_RATIONALE not in json.dumps(invalid_body)


def test_decision_cli_and_context_cover_missing_undecided_current_stale_hold_and_skip(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    runner = CliRunner()

    missing = _context(runner, workspace, job_dir)
    assert missing["workflow"] == {
        "phase": "unknown",
        "readiness": "action_required",
        "derived": True,
    }
    assert missing["next_actions"][0]["id"] == "decision.initialize"
    assert missing["next_actions"][0]["requires_consent"] is True
    assert missing["extensions"]["canisend.proposed_count"] > 0
    assert any("remain proposed" in warning for warning in missing["warnings"])
    assert not (job_dir / "application_decision.yaml").exists()

    initialized = _invoke_json(
        runner,
        [
            "decision",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    undecided = _context(runner, workspace, job_dir)
    assert undecided["workflow"]["readiness"] == "review_required"
    assert undecided["extensions"]["canisend.decision_value"] == "undecided"
    assert undecided["next_actions"][0]["id"] == "decision.update"

    baseline = _artifact(initialized, "application_decision")
    apply_patch = tmp_path / "apply.yaml"
    apply_patch.write_text(
        yaml.safe_dump(
            {
                "operation": "set_decision",
                "decision": "apply",
                "rationale_mode": "set",
                "rationale": PRIVATE_RATIONALE,
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    applied = _decision_update(
        runner,
        workspace,
        job_dir,
        apply_patch,
        revision=0,
        sha256=baseline["sha256"],
    )
    assert applied["workflow"]["readiness"] == "action_required"
    assert applied["extensions"]["canisend.decision_value"] == "apply"
    assert applied["next_actions"][0]["id"] == "brief.initialize"
    assert PRIVATE_RATIONALE not in json.dumps(applied)

    current = _context(runner, workspace, job_dir)
    assert current["workflow"] == {
        "phase": "unknown",
        "readiness": "action_required",
        "derived": True,
    }
    assert current["missing_fields"] == ["application_brief.yaml"]
    assert current["next_actions"][0]["id"] == "brief.initialize"
    assert current["next_actions"][0]["requires_consent"] is True
    assert current["extensions"]["canisend.decision_basis_status"] == "current"
    assert PRIVATE_RATIONALE not in json.dumps(current)
    decision_bytes = (job_dir / "application_decision.yaml").read_bytes()

    (job_dir / "job_advert.md").write_text(
        (job_dir / "job_advert.md").read_text(encoding="utf-8") + "\nUnrelated footer.\n",
        encoding="utf-8",
    )
    for stage in ("parse", "confirm", "match"):
        run_deterministic_stage(workspace, job_dir, stage=stage)  # type: ignore[arg-type]
    stale = _context(runner, workspace, job_dir)
    assert stale["workflow"]["readiness"] == "review_required"
    assert stale["extensions"]["canisend.decision_value"] == "apply"
    assert stale["extensions"]["canisend.decision_basis_status"] == "review_required"
    assert stale["next_actions"][0]["id"] == "decision.update"
    assert (job_dir / "application_decision.yaml").read_bytes() == decision_bytes

    status = _invoke_json(runner, ["decision", "status", *_job_args(workspace, job_dir)])
    refreshed = _artifact(status, "application_decision")
    hold_patch = tmp_path / "hold.yaml"
    hold_patch.write_text("operation: set_decision\ndecision: hold\n", encoding="utf-8")
    held = _decision_update(
        runner,
        workspace,
        job_dir,
        hold_patch,
        revision=status["extensions"]["canisend.user_artifact_revision"],
        sha256=refreshed["sha256"],
    )
    held_context = _context(runner, workspace, job_dir)
    assert held["extensions"]["canisend.decision_value"] == "hold"
    assert held_context["workflow"]["readiness"] == "review_required"
    assert held_context["next_actions"] == []

    held_artifact = _artifact(held, "application_decision")
    skip_patch = tmp_path / "skip.yaml"
    skip_patch.write_text("operation: set_decision\ndecision: skip\n", encoding="utf-8")
    skipped = _decision_update(
        runner,
        workspace,
        job_dir,
        skip_patch,
        revision=held["extensions"]["canisend.user_artifact_revision"],
        sha256=held_artifact["sha256"],
    )
    skipped_context = _context(runner, workspace, job_dir)
    assert skipped["extensions"]["canisend.decision_value"] == "skip"
    assert skipped_context["workflow"]["readiness"] == "review_required"
    assert skipped_context["next_actions"] == []


def test_confirmed_empty_match_also_routes_context_to_user_owned_decision(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path, confirmed_empty=True)
    matches = json.loads((job_dir / "criterion_matches.json").read_text(encoding="utf-8"))

    context = _context(CliRunner(), workspace, job_dir)

    assert matches["matches"] == []
    assert context["extensions"]["canisend.match_count"] == 0
    assert context["next_actions"][0]["id"] == "decision.initialize"
    assert context["workflow"]["phase"] == "unknown"


def test_recover_command_is_typed_idempotent_and_does_not_rewrite_user_yaml(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _minimal_workspace(tmp_path)
    runner = CliRunner()
    initialized = _invoke_json(
        runner,
        [
            "decision",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    mutation_id = initialized["extensions"]["canisend.mutation_id"]
    before = (job_dir / "application_decision.yaml").read_bytes()

    recovered = _invoke_json(
        runner,
        [
            "user-mutation",
            "recover",
            *_job_args(workspace, job_dir),
            "--mutation-id",
            mutation_id,
            "--confirm-user-owned-write",
        ],
    )

    assert recovered["operation"] == "user_mutation.recover"
    assert recovered["extensions"]["canisend.mutation_status"] == "committed"
    assert recovered["extensions"]["canisend.mutation_changed"] is False
    assert _artifact(recovered, "application_decision")["privacy_tier"] == 2
    assert _artifact(recovered, "user_mutation_receipt")["privacy_tier"] == 1
    assert (job_dir / "application_decision.yaml").read_bytes() == before


def test_idempotent_corrections_recovery_does_not_rerun_current_confirm(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    runner = CliRunner()
    status = _invoke_json(
        runner,
        ["corrections", "status", *_job_args(workspace, job_dir)],
    )
    baseline = _artifact(status, "confirmed_corrections")
    criteria = json.loads((job_dir / "criteria.json").read_text(encoding="utf-8"))
    patch = tmp_path / "withdraw.yaml"
    patch.write_text(
        yaml.safe_dump(
            {
                "operation": "withdraw_criterion",
                "criterion_id": criteria["criteria"][0]["criterion_id"],
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    updated = _invoke_json(
        runner,
        [
            "corrections",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(patch),
            "--expected-revision",
            str(status["extensions"]["canisend.user_artifact_revision"]),
            "--expected-sha256",
            baseline["sha256"],
            "--confirm-user-owned-write",
        ],
    )
    run_deterministic_stage(workspace, job_dir, stage="confirm")

    recovered = _invoke_json(
        runner,
        [
            "user-mutation",
            "recover",
            *_job_args(workspace, job_dir),
            "--mutation-id",
            updated["extensions"]["canisend.mutation_id"],
            "--confirm-user-owned-write",
        ],
    )

    assert recovered["extensions"]["canisend.mutation_changed"] is False
    assert recovered["workflow"]["readiness"] == "ready_for_next_stage"
    assert all(item["id"] != "stage.run_confirm" for item in recovered["next_actions"])


def test_tampered_receipt_is_never_reported_as_validated_success(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _minimal_workspace(tmp_path)
    original_inspect = mutation_agent.inspect_user_mutation

    def tamper_then_inspect(root: Path, job: Path, mutation_id: str):
        receipt = (
            job
            / "workflow"
            / "user-mutations"
            / "events"
            / mutation_id
            / "receipt.json"
        )
        receipt.write_text("{}\n", encoding="utf-8")
        return original_inspect(root, job, mutation_id)

    monkeypatch.setattr(mutation_agent, "inspect_user_mutation", tamper_then_inspect)
    response = _invoke_json(
        CliRunner(),
        [
            "decision",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )

    assert response["error"]["code"] == "user_input.recovery_required"
    assert response["extensions"]["canisend.mutation_id"].startswith("mutation_")
    assert response["next_actions"][0]["id"] == "user_mutation.recover"
    assert all(item["kind"] != "user_mutation_receipt" for item in response["artifacts"])
    assert (job_dir / "application_decision.yaml").is_file()


def test_committed_apply_with_pending_receipt_is_action_required(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    runner = CliRunner()
    initialized = _invoke_json(
        runner,
        [
            "decision",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    baseline = _artifact(initialized, "application_decision")
    patch = tmp_path / "apply-pending.yaml"
    patch.write_text("operation: set_decision\ndecision: apply\n", encoding="utf-8")

    def fail_receipt(*args, **kwargs):
        raise UserFileStoreError("simulated receipt failure")

    monkeypatch.setattr(user_mutations, "_store_receipt", fail_receipt)
    response = _decision_update(
        runner,
        workspace,
        job_dir,
        patch,
        revision=0,
        sha256=baseline["sha256"],
    )

    assert response["extensions"]["canisend.decision_value"] == "apply"
    assert response["extensions"]["canisend.mutation_status"] == "committed_receipt_pending"
    assert response["workflow"]["readiness"] == "action_required"
    assert response["next_actions"][0]["id"] == "user_mutation.recover"
    receipt = _artifact(response, "user_mutation_receipt")
    assert receipt["exists"] is False
    assert receipt["privacy_tier"] == 1
    receipt_refs = [
        item
        for item in response["artifacts"]
        if item["kind"] == "user_mutation_receipt" and item["path"] == receipt["path"]
    ]
    assert len(receipt_refs) == 1


def test_failed_promotion_exposes_recoverable_mutation_id(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _minimal_workspace(tmp_path)
    runner = CliRunner()
    initialized = _invoke_json(
        runner,
        [
            "decision",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    baseline = _artifact(initialized, "application_decision")
    patch = tmp_path / "reset.yaml"
    patch.write_text("operation: reset_decision\n", encoding="utf-8")
    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated promotion failure")
        ),
    )
    failed = _invoke_json(
        runner,
        [
            "decision",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(patch),
            "--expected-revision",
            "0",
            "--expected-sha256",
            baseline["sha256"],
            "--confirm-user-owned-write",
        ],
        expected_exit=1,
    )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)

    mutation_id = failed["extensions"]["canisend.mutation_id"]
    assert failed["error"]["code"] == "user_input.recovery_required"
    assert failed["next_actions"][0]["id"] == "user_mutation.recover"
    recovered = _invoke_json(
        runner,
        [
            "user-mutation",
            "recover",
            *_job_args(workspace, job_dir),
            "--mutation-id",
            mutation_id,
            "--confirm-user-owned-write",
        ],
    )
    assert recovered["extensions"]["canisend.mutation_id"] == mutation_id
    assert recovered["extensions"]["canisend.mutation_status"] == "committed"


def test_fresh_decision_status_and_context_discover_pre_promotion_claim(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    initialized = user_mutations.initialize_application_decision(
        workspace,
        job_dir,
        consent_confirmed=True,
    )
    mutation_id = "mutation_" + "d" * 32
    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated promotion failure")
        ),
    )
    with pytest.raises(UserMutationError):
        apply_user_patch(
            workspace,
            job_dir,
            ResetDecisionPatch(),
            expected_revision=initialized.snapshot.revision,
            expected_sha256=initialized.snapshot.sha256,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)
    before = _tree_digest(workspace)

    status = _invoke_json(
        CliRunner(),
        ["decision", "status", *_job_args(workspace, job_dir)],
    )
    context = _context(CliRunner(), workspace, job_dir)

    for response in (status, context):
        assert response["workflow"]["readiness"] == "action_required"
        assert response["next_actions"][0]["id"] == "user_mutation.recover"
        assert response["extensions"]["canisend.mutation_id"] == mutation_id
        receipt = _artifact(response, "user_mutation_receipt")
        assert mutation_id in receipt["path"]
        assert receipt["exists"] is False
    assert _tree_digest(workspace) == before

    job_args = [
        "--workspace",
        str(workspace),
        "--job",
        job_dir.relative_to(workspace).as_posix(),
    ]
    for command in (["decision", "status"], ["agent", "context"]):
        rendered = CliRunner().invoke(app, [*command, *job_args])
        assert rendered.exit_code == 0
        assert mutation_id in rendered.stdout
        assert "user_mutation.recover" in rendered.stdout

    patch_file = tmp_path / "retry-reset.yaml"
    patch_file.write_text("operation: reset_decision\n", encoding="utf-8")
    rejected = CliRunner().invoke(
        app,
        [
            "decision",
            "update",
            *job_args,
            "--patch-file",
            str(patch_file),
            "--expected-revision",
            str(initialized.snapshot.revision),
            "--expected-sha256",
            initialized.snapshot.sha256,
            "--confirm-user-owned-write",
        ],
    )
    assert rejected.exit_code == 1
    assert f"Hint: Recover with mutation ID {mutation_id}." in rejected.stdout


def test_conflicting_claim_with_diagnostic_id_requires_manual_control_review(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    initialized = user_mutations.initialize_application_decision(
        workspace,
        job_dir,
        consent_confirmed=True,
    )
    mutation_id = "mutation_" + "a" * 32
    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated promotion failure")
        ),
    )
    with pytest.raises(UserMutationError):
        apply_user_patch(
            workspace,
            job_dir,
            ResetDecisionPatch(),
            expected_revision=initialized.snapshot.revision,
            expected_sha256=initialized.snapshot.sha256,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)
    candidate = (
        job_dir
        / "workflow"
        / "user-mutations"
        / "events"
        / mutation_id
        / "candidate.yaml"
    )
    candidate.unlink()

    response = _invoke_json(
        CliRunner(),
        ["decision", "status", *_job_args(workspace, job_dir)],
    )

    assert response["workflow"]["readiness"] == "blocked"
    assert response["extensions"]["canisend.mutation_id"] == mutation_id
    assert response["extensions"]["canisend.mutation_audit_status"] == "conflict"
    assert response["required_consents"] == []
    assert response["next_actions"] == [
        {
            "id": "user_mutation.review_controls",
            "label": "Review and coordinate the conflicting private mutation controls manually",
            "requires_consent": False,
            "consent_ids": [],
        }
    ]
    assert "user_mutation.recover" not in json.dumps(response)
    assert all(item["kind"] != "user_mutation_receipt" for item in response["artifacts"])


def test_mutation_outcome_preserves_pending_id_discovered_by_projection(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    initialized = user_mutations.initialize_application_decision(
        workspace,
        job_dir,
        consent_confirmed=True,
    )
    mutation_id = "mutation_" + "b" * 32
    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated promotion failure")
        ),
    )
    with pytest.raises(UserMutationError):
        apply_user_patch(
            workspace,
            job_dir,
            ResetDecisionPatch(),
            expected_revision=initialized.snapshot.revision,
            expected_sha256=initialized.snapshot.sha256,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)

    response = mutation_agent.mutation_outcome_agent_response(
        workspace,
        job_dir,
        MutationOutcome(
            status="reused",
            snapshot=initialized.snapshot,
            mutation_id=None,
            claim_path=None,
            receipt_path=None,
            changed=False,
        ),
        operation="decision.initialize",
    ).model_dump(mode="json")

    assert response["extensions"]["canisend.mutation_id"] == mutation_id
    assert response["next_actions"][0]["id"] == "user_mutation.recover"


def test_fresh_corrections_status_and_context_discover_post_promotion_claim(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    snapshot = inspect_user_artifact(workspace, job_dir, "corrections")
    assert snapshot is not None
    criteria = json.loads((job_dir / "criteria.json").read_text(encoding="utf-8"))
    mutation_id = "mutation_" + "e" * 32
    monkeypatch.setattr(
        user_mutations,
        "_store_receipt",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated receipt failure")
        ),
    )
    outcome = apply_user_patch(
        workspace,
        job_dir,
        ConfirmCriterionPatch(criterion_id=criteria["criteria"][0]["criterion_id"]),
        expected_revision=snapshot.revision,
        expected_sha256=snapshot.sha256,
        mutation_id=mutation_id,
        consent_confirmed=True,
    )
    assert outcome.status == "committed_receipt_pending"
    before = _tree_digest(workspace)

    status = _invoke_json(
        CliRunner(),
        ["corrections", "status", *_job_args(workspace, job_dir)],
    )
    context = _context(CliRunner(), workspace, job_dir)

    for response in (status, context):
        assert response["workflow"]["readiness"] == "action_required"
        assert response["next_actions"][0]["id"] == "user_mutation.recover"
        assert response["extensions"]["canisend.mutation_id"] == mutation_id
        assert mutation_id in _artifact(response, "user_mutation_receipt")["path"]
    assert _tree_digest(workspace) == before


@pytest.mark.parametrize("group", ["corrections", "decision"])
def test_update_cli_exposes_only_scoped_patch_and_cas_inputs(group: str) -> None:
    result = CliRunner().invoke(app, [group, "update", "--help"])
    output = _strip_ansi(result.stdout)

    assert result.exit_code == 0, result.output
    for option in (
        "--patch-file",
        "--expected-revision",
        "--expected-sha256",
        "--confirm-user-owned-write",
    ):
        assert option in output
    for forbidden in (
        "--decision",
        "--rationale",
        "--corrected-text",
        "--replacement-file",
        "--mutation-id",
    ):
        assert forbidden not in output


def test_private_correction_and_rationale_stay_on_declared_tier_two_data_paths(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _workspace_with_current_match(tmp_path)
    correction_marker = "PRIVATE-CORRECTION-WHITELIST-4937"
    rationale_marker = "PRIVATE-RATIONALE-WHITELIST-6281"
    correction_snapshot = inspect_user_artifact(workspace, job_dir, "corrections")
    assert correction_snapshot is not None
    criterion_id = json.loads((job_dir / "criteria.json").read_text(encoding="utf-8"))["criteria"][0][
        "criterion_id"
    ]

    apply_user_patch(
        workspace,
        job_dir,
        CorrectCriterionPatch(
            criterion_id=criterion_id,
            corrected_text=correction_marker,
        ),
        expected_revision=correction_snapshot.revision,
        expected_sha256=correction_snapshot.sha256,
        consent_confirmed=True,
    )
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    run_deterministic_stage(workspace, job_dir, stage="match")

    runner = CliRunner()
    initialized = _invoke_json(
        runner,
        [
            "decision",
            "init",
            *_job_args(workspace, job_dir),
            "--confirm-user-owned-write",
        ],
    )
    baseline = _artifact(initialized, "application_decision")
    decision_patch = tmp_path / "private-decision-patch.yaml"
    decision_patch.write_text(
        yaml.safe_dump(
            {
                "operation": "set_decision",
                "decision": "apply",
                "rationale_mode": "set",
                "rationale": rationale_marker,
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    updated = _decision_update(
        runner,
        workspace,
        job_dir,
        decision_patch,
        revision=0,
        sha256=baseline["sha256"],
    )
    responses = [
        updated,
        _invoke_json(runner, ["corrections", "status", *_job_args(workspace, job_dir)]),
        _invoke_json(runner, ["decision", "status", *_job_args(workspace, job_dir)]),
        _context(runner, workspace, job_dir),
    ]
    rendered_responses = json.dumps(responses, sort_keys=True)
    assert correction_marker not in rendered_responses
    assert rationale_marker not in rendered_responses

    correction_paths: set[str] = set()
    rationale_paths: set[str] = set()
    for path in job_dir.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(job_dir).as_posix()
        body = path.read_bytes()
        if correction_marker.encode() in body:
            correction_paths.add(relative)
        if rationale_marker.encode() in body:
            rationale_paths.add(relative)

    assert {"confirmed_corrections.yaml", "criteria.json"} <= correction_paths
    assert len(correction_paths) == 4
    correction_private = correction_paths - {"confirmed_corrections.yaml", "criteria.json"}
    assert sum(
        path.startswith("workflow/user-mutations/events/") and path.endswith("/candidate.yaml")
        for path in correction_private
    ) == 1
    assert sum(
        path.startswith("workflow/runs/") and path.endswith("/candidates/criteria.json")
        for path in correction_private
    ) == 1
    assert "application_decision.yaml" in rationale_paths
    assert len(rationale_paths) == 2
    rationale_private = rationale_paths - {"application_decision.yaml"}
    assert all(
        path.startswith("workflow/user-mutations/events/") and path.endswith("/candidate.yaml")
        for path in rationale_private
    )


def _minimal_workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n", encoding="utf-8")
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "deadline": "2026-08-01",
                "status": "advert_imported",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    return workspace, job_dir


def _workspace_through_initial_confirm(tmp_path: Path) -> tuple[Path, Path]:
    result = run_packaged_example(tmp_path / "workspace", overwrite=True)
    for stage in ("evidence", "parse", "confirm"):
        run_deterministic_stage(result.workspace, result.job_dir, stage=stage)  # type: ignore[arg-type]
    return result.workspace, result.job_dir


def _workspace_with_current_match(
    tmp_path: Path,
    *,
    confirmed_empty: bool = False,
) -> tuple[Path, Path]:
    result = run_packaged_example(tmp_path / "workspace", overwrite=True)
    if confirmed_empty:
        (result.job_dir / "job_advert.md").write_text(
            "# General role\n\nApplications are invited.\n",
            encoding="utf-8",
        )
    for stage in ("evidence", "parse", "confirm"):
        run_deterministic_stage(result.workspace, result.job_dir, stage=stage)  # type: ignore[arg-type]

    outcome = initialize_confirmed_corrections(
        result.workspace,
        result.job_dir,
        consent_confirmed=True,
    )
    criteria = json.loads((result.job_dir / "criteria.json").read_text(encoding="utf-8"))
    patches = (
        [ConfirmEmptyCriteriaPatch()]
        if confirmed_empty
        else [ConfirmCriterionPatch(criterion_id=item["criterion_id"]) for item in criteria["criteria"]]
    )
    snapshot = outcome.snapshot
    for patch in patches:
        outcome = apply_user_patch(
            result.workspace,
            result.job_dir,
            patch,
            expected_revision=snapshot.revision,
            expected_sha256=snapshot.sha256,
            consent_confirmed=True,
        )
        snapshot = outcome.snapshot
        run_deterministic_stage(result.workspace, result.job_dir, stage="confirm")
    run_deterministic_stage(result.workspace, result.job_dir, stage="match")
    return result.workspace, result.job_dir


def _decision_update(
    runner: CliRunner,
    workspace: Path,
    job_dir: Path,
    patch: Path,
    *,
    revision: int,
    sha256: str,
) -> dict[str, object]:
    return _invoke_json(
        runner,
        [
            "decision",
            "update",
            *_job_args(workspace, job_dir),
            "--patch-file",
            str(patch),
            "--expected-revision",
            str(revision),
            "--expected-sha256",
            sha256,
            "--confirm-user-owned-write",
        ],
    )


def _context(runner: CliRunner, workspace: Path, job_dir: Path) -> dict[str, object]:
    return _invoke_json(
        runner,
        ["agent", "context", *_job_args(workspace, job_dir)],
    )


def _job_args(workspace: Path, job_dir: Path) -> list[str]:
    return [
        "--workspace",
        str(workspace),
        "--job",
        job_dir.relative_to(workspace).as_posix(),
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


def _tree_digest(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): _sha256(path.read_bytes())
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()
