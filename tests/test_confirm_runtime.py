from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml

from canisend.decision_models import ConfirmedCorrectionsV1, CriteriaCatalogV1, CriterionCorrectionV1
from canisend.stage_agent import stage_status_agent_response
from canisend.stage_runtime import (
    StageRuntimeError,
    apply_stage_result,
    cancel_stage_task,
    inspect_stage_status,
    prepare_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.stage_store import sha256_file
from canisend.stages.confirm_stage import (
    build_deterministic_confirm_candidate,
    criterion_source_sha256,
    criterion_text_sha256,
)


def _write_workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/job?private=token",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct",
                "created_at": "2026-07-11T10:00:00Z",
                "updated_at": "2026-07-11T10:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job_advert.md").write_text(
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
    return workspace, job_dir


def _confirm_one_criterion(
    job_dir: Path,
    catalog: CriteriaCatalogV1,
    *,
    corrected_text: str | None = None,
) -> Path:
    criterion = catalog.criteria[0]
    correction = CriterionCorrectionV1(
        correction_id="correction_" + "a" * 32,
        criterion_id=criterion.criterion_id,
        target_source_sha256=criterion_source_sha256(criterion.source_text),
        target_criterion_sha256=criterion_text_sha256(criterion.text),
        confirmation="corrected" if corrected_text is not None else "confirmed",
        corrected_text=corrected_text,
        record_state="active",
        confirmed_at="2026-07-11T12:00:00Z",
    )
    overlay = ConfirmedCorrectionsV1(
        job_id=job_dir.name,
        revision=1,
        updated_at="2026-07-11T12:00:00Z",
        criteria=(correction,),
    )
    path = job_dir / "confirmed_corrections.yaml"
    path.write_text(
        yaml.safe_dump(overlay.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )
    return path


def _write_result(prepared: object, candidate: dict[str, object]) -> Path:
    candidate_bytes = (json.dumps(candidate, indent=2, sort_keys=True) + "\n").encode("utf-8")
    job_dir = prepared.task_spec_path.parents[3]
    workspace = job_dir.parents[1]
    submitted = submit_stage_candidate(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=candidate_bytes,
    )
    return submitted.result_path


def test_confirm_requires_current_parse_then_runs_through_cache(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)

    blocked = inspect_stage_status(workspace, job_dir, stage="confirm")
    assert blocked.stage.status == "blocked"
    assert blocked.input_fingerprint is None
    assert blocked.reasons == ("dependency_not_current:parse",)

    with pytest.raises(StageRuntimeError) as captured:
        run_deterministic_stage(workspace, job_dir, stage="confirm")
    assert captured.value.code == "stage.dependency_not_current"

    run_deterministic_stage(workspace, job_dir, stage="parse")
    ready = inspect_stage_status(workspace, job_dir, stage="confirm")
    assert ready.stage.status == "ready"
    assert ready.input_fingerprint is not None

    first = run_deterministic_stage(workspace, job_dir, stage="confirm")
    target = job_dir / "criteria.json"
    first_hash = sha256_file(target)
    first_mtime = target.stat().st_mtime_ns
    second = run_deterministic_stage(workspace, job_dir, stage="confirm")

    assert first.cache_hit is False
    assert first.stage == "confirm"
    assert first.manifest is not None
    assert first.manifest.stage == "confirm"
    assert second.cache_hit is True
    assert second.stage == "confirm"
    assert sha256_file(target) == first_hash
    assert target.stat().st_mtime_ns == first_mtime
    records = {record.stage: record for record in second.state.stages}
    assert records["parse"].status == "succeeded"
    assert records["confirm"].status == "succeeded"


def test_confirmation_change_stales_confirm_without_touching_parse_or_user_file(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    catalog = CriteriaCatalogV1.model_validate(
        json.loads((job_dir / "criteria.json").read_text(encoding="utf-8"))
    )
    correction_path = _confirm_one_criterion(job_dir, catalog)
    correction_hash = sha256_file(correction_path)
    correction_mtime = correction_path.stat().st_mtime_ns

    assert inspect_stage_status(workspace, job_dir, stage="parse").stage.status == "succeeded"
    stale = inspect_stage_status(workspace, job_dir, stage="confirm")
    assert stale.stage.status == "stale"
    assert stale.reasons == ("input_changed",)

    refreshed = run_deterministic_stage(workspace, job_dir, stage="confirm")
    assert refreshed.cache_hit is False
    updated = CriteriaCatalogV1.model_validate(
        json.loads((job_dir / "criteria.json").read_text(encoding="utf-8"))
    )
    assert updated.criteria[0].confirmation_state == "confirmed"
    assert sha256_file(correction_path) == correction_hash
    assert correction_path.stat().st_mtime_ns == correction_mtime


def test_multistage_state_reconstructs_parse_and_confirm_manifests(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    state_path = job_dir / "workflow" / "state.json"
    state_path.write_text("not json\n", encoding="utf-8")

    status = inspect_stage_status(workspace, job_dir, stage="confirm")
    records = {record.stage: record for record in status.state.stages}

    assert status.reconstructed is True
    assert records["parse"].status == "succeeded"
    assert records["confirm"].status == "succeeded"


def test_upstream_rerun_persists_and_reconstructs_descendant_staleness(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8") + "\nChanged source input.\n",
        encoding="utf-8",
    )

    run_deterministic_stage(workspace, job_dir, stage="parse")
    state_path = job_dir / "workflow" / "state.json"
    persisted = json.loads(state_path.read_text(encoding="utf-8"))
    persisted_records = {item["stage"]: item for item in persisted["stages"]}
    assert persisted_records["confirm"]["status"] == "stale"

    state_path.unlink()
    reconstructed = inspect_stage_status(workspace, job_dir, stage="confirm")
    reconstructed_records = {record.stage: record for record in reconstructed.state.stages}
    assert reconstructed.reconstructed is True
    assert reconstructed_records["confirm"].status == "stale"


def test_confirm_output_drift_is_preserved_and_host_mode_is_rejected(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    target = job_dir / "criteria.json"
    target.write_text('{"manual":"edit"}\n', encoding="utf-8")

    status = inspect_stage_status(workspace, job_dir, stage="confirm")
    assert status.output_drift is True
    with pytest.raises(StageRuntimeError) as conflict:
        run_deterministic_stage(workspace, job_dir, stage="confirm")
    assert conflict.value.code == "stage.output_conflict"
    assert target.read_text(encoding="utf-8") == '{"manual":"edit"}\n'

    with pytest.raises(StageRuntimeError) as unsupported:
        prepare_stage(workspace, job_dir, stage="confirm", execution_mode="host_agent")
    assert unsupported.value.code == "stage.unsupported_mode"


def test_confirm_control_records_do_not_copy_criteria_or_corrections(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    initial = build_deterministic_confirm_candidate(job_dir)
    private_marker = "PRIVATE-CORRECTION-MARKER-7319"
    _confirm_one_criterion(job_dir, initial, corrected_text=private_marker)
    outcome = run_deterministic_stage(workspace, job_dir, stage="confirm")
    assert outcome.manifest_path is not None
    run_dir = outcome.manifest_path.parent
    control_paths = [
        job_dir / "workflow" / "state.json",
        run_dir / "task-spec.json",
        run_dir / "preparation.json",
        run_dir / "submission.json",
        run_dir / "terminal-claim.json",
        run_dir / "tasks" / outcome.manifest.task_id / "result.json",
        run_dir / "validation" / "report.json",
        run_dir / "promotion.json",
        run_dir / "manifest.json",
    ]
    rendered = "\n".join(path.read_text(encoding="utf-8") for path in control_paths)

    assert "PhD in Economics" not in rendered
    assert "private=token" not in rendered
    assert private_marker not in rendered
    assert "correction_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" not in rendered
    assert str(workspace) not in rendered
    assert "criteria.json" in rendered
    assert private_marker in (job_dir / "criteria.json").read_text(encoding="utf-8")


def test_stale_or_invalid_confirm_candidate_never_promotes(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="confirm",
        execution_mode="deterministic",
    )
    candidate = build_deterministic_confirm_candidate(
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
    )
    result_path = _write_result(prepared, candidate.model_dump(mode="json"))
    _confirm_one_criterion(job_dir, candidate)

    with pytest.raises(StageRuntimeError) as stale:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert stale.value.code == "stage.stale_input"
    assert not (job_dir / "criteria.json").exists()

    workspace, job_dir = _write_workspace(tmp_path / "invalid")
    run_deterministic_stage(workspace, job_dir, stage="parse")
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="confirm",
        execution_mode="deterministic",
    )
    candidate = build_deterministic_confirm_candidate(
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
    ).model_dump(mode="json")
    candidate["private_body"] = "must be rejected"

    with pytest.raises(StageRuntimeError) as invalid:
        _write_result(prepared, candidate)

    assert invalid.value.code == "stage.invalid_candidate"
    assert not (job_dir / "criteria.json").exists()


def test_parse_change_blocks_confirm_without_rewriting_user_corrections(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    catalog = CriteriaCatalogV1.model_validate(
        json.loads((job_dir / "criteria.json").read_text(encoding="utf-8"))
    )
    correction_path = _confirm_one_criterion(job_dir, catalog)
    correction_bytes = correction_path.read_bytes()
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8") + "\nA changed advert fact.\n",
        encoding="utf-8",
    )

    status = inspect_stage_status(workspace, job_dir, stage="confirm")

    assert status.stage.status == "stale"
    assert status.input_fingerprint is None
    assert "dependency_not_current:parse" in status.reasons
    assert correction_path.read_bytes() == correction_bytes


def test_apply_rejects_task_target_forgery_before_user_owned_file_changes(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    catalog = build_deterministic_confirm_candidate(job_dir)
    correction_path = _confirm_one_criterion(job_dir, catalog)
    correction_bytes = correction_path.read_bytes()
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="confirm",
        execution_mode="deterministic",
    )
    candidate = build_deterministic_confirm_candidate(
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
    )
    result_path = _write_result(prepared, candidate.model_dump(mode="json"))
    forged = json.loads(prepared.task_spec_path.read_text(encoding="utf-8"))
    forged["authoritative_target"] = "confirmed_corrections.yaml"
    forged["expected_output_sha256"] = sha256_file(correction_path)
    prepared.task_spec_path.write_text(
        json.dumps(forged, indent=2) + "\n",
        encoding="utf-8",
    )

    with pytest.raises(StageRuntimeError) as captured:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert captured.value.code == "stage.task_contract_mismatch"
    assert correction_path.read_bytes() == correction_bytes
    assert not (job_dir / "criteria.json").exists()


def test_active_confirm_task_blocks_upstream_prepare_with_stable_error(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    prepare_stage(
        workspace,
        job_dir,
        stage="confirm",
        execution_mode="deterministic",
    )
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8") + "\nChanged after Confirm prepare.\n",
        encoding="utf-8",
    )

    with pytest.raises(StageRuntimeError) as captured:
        prepare_stage(
            workspace,
            job_dir,
            stage="parse",
            execution_mode="deterministic",
        )

    assert captured.value.code == "stage.concurrent_run"
    pending_specs = [
        path
        for path in (job_dir / "workflow" / "runs").glob("*/task-spec.json")
        if not (path.parent / "manifest.json").exists()
    ]
    assert len(pending_specs) == 1
    status = inspect_stage_status(workspace, job_dir, stage="confirm")
    response = stage_status_agent_response(workspace, job_dir, status)
    assert response.blockers
    assert [action.id for action in response.next_actions] == ["stage.cancel_active_task"]


def test_stale_active_confirm_task_can_be_cancelled_before_parse_rerun(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    prepare_stage(
        workspace,
        job_dir,
        stage="confirm",
        execution_mode="deterministic",
    )
    advert = job_dir / "job_advert.md"
    advert.write_text(advert.read_text(encoding="utf-8") + "\nChanged input.\n", encoding="utf-8")

    cancelled = cancel_stage_task(workspace, job_dir, stage="confirm")
    reparsed = run_deterministic_stage(workspace, job_dir, stage="parse")

    assert cancelled.manifest.status == "cancelled"
    assert reparsed.cache_hit is False
    assert inspect_stage_status(workspace, job_dir, stage="parse").stage.status == "succeeded"


def test_cancelled_confirm_status_advertises_deterministic_run(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="confirm",
        execution_mode="deterministic",
    )
    cancelled = cancel_stage_task(workspace, job_dir, stage="confirm")

    status = inspect_stage_status(workspace, job_dir, stage="confirm")
    response = stage_status_agent_response(workspace, job_dir, status)

    assert cancelled.manifest.status == "cancelled"
    assert response.workflow is not None
    assert response.workflow.readiness == "action_required"
    assert [item.id for item in response.next_actions] == ["stage.run_confirm"]
    assert prepared.task_spec_path.is_file()
    assert (prepared.task_spec_path.parent / "preparation.json").is_file()
