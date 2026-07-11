from __future__ import annotations

import builtins
import importlib
import json
from hashlib import sha256
import os
from pathlib import Path
import socket
import subprocess
import sys

import pytest
import yaml
from typer.testing import CliRunner

import canisend.stage_runtime as stage_runtime_module
from canisend.cli import app
from canisend.stage_adapters import get_stage_adapter
from canisend.stage_agent import stage_run_agent_response, stage_status_agent_response
from canisend.stage_runtime import (
    StageRuntimeError,
    apply_stage_result,
    cancel_stage_task,
    inspect_stage_status,
    prepare_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.stage_store import StageStoreError, sha256_file
from canisend.user_mutations import (
    SetDecisionPatch,
    apply_user_patch,
    initialize_application_decision,
    initialize_confirmed_corrections,
)


PRIVATE_EVIDENCE = "PRIVATE-EVIDENCE-SENTINEL-9427"
PRIVATE_SECTION = "PRIVATE-SECTION-SENTINEL-5174"
PRIVATE_LOCATOR = "private-locator-sentinel-6381"
PRIVATE_KIND = "teaching-private-kind-sentinel-2704"


def _write_workspace(tmp_path: Path) -> tuple[Path, Path, Path]:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    profile_dir = workspace / "profile"
    generated_dir = profile_dir / "generated"
    typst_dir = profile_dir / "typst"
    job_dir.mkdir(parents=True)
    generated_dir.mkdir(parents=True)
    typst_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (profile_dir / "profile.yaml").write_text(
        "sources:\n"
        "  cv: typst/cv.typ\n"
        "generated:\n"
        "  cv_evidence: generated/cv.evidence.md\n",
        encoding="utf-8",
    )
    source_bytes = b"= Private profile source\n"
    (typst_dir / "cv.typ").write_bytes(source_bytes)
    evidence_path = generated_dir / "cv.evidence.md"
    evidence_path.write_text(
        "# Evidence: cv\n\n"
        f"<!-- canisend-source-sha256: {sha256(source_bytes).hexdigest()} -->\n\n"
        "## Education\n\n"
        "- [cv-001] `education`: PhD in Economics\n\n"
        f"## Teaching {PRIVATE_SECTION}\n\n"
        f"- [{PRIVATE_LOCATOR}] `{PRIVATE_KIND}`: Led econometrics seminars. {PRIVATE_EVIDENCE}\n",
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

Essential criteria:
- PhD in Economics
- Evidence of teaching excellence

Desirable criteria:
- Experience teaching econometrics
""",
        encoding="utf-8",
    )
    return workspace, job_dir, evidence_path


def _run_to_match(workspace: Path, job_dir: Path) -> None:
    run_deterministic_stage(workspace, job_dir, stage="evidence")
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    run_deterministic_stage(workspace, job_dir, stage="match")


def _prepare_stage_dependencies(workspace: Path, job_dir: Path, stage: str) -> None:
    if stage == "match":
        run_deterministic_stage(workspace, job_dir, stage="evidence")
        run_deterministic_stage(workspace, job_dir, stage="parse")
        run_deterministic_stage(workspace, job_dir, stage="confirm")


def _prepare_and_submit(
    workspace: Path,
    job_dir: Path,
    *,
    stage: str,
) -> tuple[object, object, dict[str, object]]:
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage=stage,  # type: ignore[arg-type]
        execution_mode="deterministic",
    )
    adapter = get_stage_adapter(stage)
    candidate = adapter.build_deterministic_candidate(
        workspace,
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
        inputs=prepared.task_spec.inputs,
    )
    submitted = submit_stage_candidate(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(candidate) + "\n").encode("utf-8"),
    )
    return prepared, submitted, candidate


def _fresh_cli(*args: str) -> dict[str, object]:
    completed = subprocess.run(
        [
            sys.executable,
            "-c",
            "from canisend.cli import app; app()",
            *args,
        ],
        cwd=Path(__file__).resolve().parents[1],
        check=False,
        text=True,
        capture_output=True,
    )
    assert completed.returncode == 0, completed.stderr or completed.stdout
    return json.loads(completed.stdout)


def _job_file_snapshot(job_dir: Path) -> dict[str, bytes]:
    return {
        path.relative_to(job_dir).as_posix(): path.read_bytes()
        for path in sorted(job_dir.rglob("*"))
        if path.is_file()
    }


def test_evidence_snapshot_and_match_use_truthful_job_local_inputs(tmp_path: Path) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)

    evidence = run_deterministic_stage(workspace, job_dir, stage="evidence")
    assert evidence.manifest is not None
    assert evidence.manifest_path is not None
    evidence_run = evidence.manifest_path.parent
    task = json.loads((evidence_run / "task-spec.json").read_text(encoding="utf-8"))

    assert task["allowed_reads"] == [
        f"workflow/runs/{evidence.manifest.run_id}/inputs/evidence-snapshot.json"
    ]
    assert task["inputs"][0]["path"] == task["allowed_reads"][0]
    assert all("profile/" not in path for path in task["allowed_reads"])
    snapshot = job_dir / task["allowed_reads"][0]
    assert PRIVATE_EVIDENCE in snapshot.read_text(encoding="utf-8")

    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    matched = run_deterministic_stage(workspace, job_dir, stage="match")
    assert matched.manifest is not None
    assert {item.path for item in matched.manifest.inputs} == {
        "criteria.json",
        "evidence_catalog.json",
    }
    rendered_matches = (job_dir / "criterion_matches.json").read_text(encoding="utf-8")
    assert PRIVATE_EVIDENCE not in rendered_matches
    assert PRIVATE_SECTION not in rendered_matches
    assert PRIVATE_LOCATOR not in rendered_matches
    assert "teaching_private_kind_sentinel_2704" not in rendered_matches
    assert '"review_state": "proposed"' in rendered_matches


def test_evidence_private_body_never_enters_control_plane_or_agent_response(
    tmp_path: Path,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    outcome = run_deterministic_stage(workspace, job_dir, stage="evidence")
    assert outcome.manifest is not None
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
    rendered_control = "\n".join(path.read_text(encoding="utf-8") for path in control_paths)
    response = stage_run_agent_response(workspace, outcome)

    assert PRIVATE_EVIDENCE not in rendered_control
    assert PRIVATE_EVIDENCE not in response.model_dump_json()
    assert PRIVATE_SECTION not in rendered_control
    assert PRIVATE_SECTION not in response.model_dump_json()
    assert PRIVATE_LOCATOR not in rendered_control
    assert PRIVATE_LOCATOR not in response.model_dump_json()
    assert "teaching_private_kind_sentinel_2704" not in rendered_control
    assert "teaching_private_kind_sentinel_2704" not in response.model_dump_json()
    assert PRIVATE_EVIDENCE in (job_dir / "evidence_catalog.json").read_text(encoding="utf-8")
    assert str(workspace) not in rendered_control


def test_match_requires_current_confirm_and_evidence_then_caches(tmp_path: Path) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    blocked = inspect_stage_status(workspace, job_dir, stage="match")
    assert blocked.stage.status == "blocked"
    assert set(blocked.reasons) == {
        "dependency_not_current:confirm",
        "dependency_not_current:evidence",
    }

    _run_to_match(workspace, job_dir)
    target = job_dir / "criterion_matches.json"
    before_hash = sha256_file(target)
    before_mtime = target.stat().st_mtime_ns
    cached = run_deterministic_stage(workspace, job_dir, stage="match")

    assert cached.cache_hit is True
    assert sha256_file(target) == before_hash
    assert target.stat().st_mtime_ns == before_mtime


def test_match_status_blocks_unknown_criteria_with_review_action(tmp_path: Path) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    (job_dir / "job_advert.md").write_text(
        "# Lecturer\n\nNo selection criteria were extracted.\n",
        encoding="utf-8",
    )
    run_deterministic_stage(workspace, job_dir, stage="evidence")
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")

    status = inspect_stage_status(workspace, job_dir, stage="match")
    response = stage_status_agent_response(workspace, job_dir, status)

    assert status.stage.status == "blocked"
    assert status.reasons == ("input_not_ready:criteria_review",)
    assert response.workflow is not None
    assert response.workflow.readiness == "blocked"
    assert [item.id for item in response.next_actions] == [
        "criteria.review_confirmations"
    ]


def test_profile_change_stales_only_evidence_match_and_descendants(tmp_path: Path) -> None:
    workspace, job_dir, evidence_path = _write_workspace(tmp_path)
    _run_to_match(workspace, job_dir)
    evidence_path.write_text(
        evidence_path.read_text(encoding="utf-8") + "\n- [cv-003] `research`: New paper.\n",
        encoding="utf-8",
    )

    assert inspect_stage_status(workspace, job_dir, stage="parse").stage.status == "succeeded"
    assert inspect_stage_status(workspace, job_dir, stage="confirm").stage.status == "succeeded"
    evidence = inspect_stage_status(workspace, job_dir, stage="evidence")
    matched = inspect_stage_status(workspace, job_dir, stage="match")

    assert evidence.stage.status == "stale"
    assert evidence.reasons == ("input_changed",)
    assert matched.stage.status == "stale"
    assert "dependency_not_current:evidence" in matched.reasons


def test_match_status_merges_staleness_from_both_dependency_branches(tmp_path: Path) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _run_to_match(workspace, job_dir)
    advert = job_dir / "job_advert.md"
    advert.write_text(
        advert.read_text(encoding="utf-8") + "\nChanged advert input.\n",
        encoding="utf-8",
    )

    matched = inspect_stage_status(workspace, job_dir, stage="match")
    records = {record.stage: record for record in matched.state.stages}

    assert "dependency_not_current:confirm" in matched.reasons
    assert records["parse"].status == "stale"
    assert records["confirm"].status == "stale"
    assert records["evidence"].status == "succeeded"
    assert records["match"].status == "stale"


def test_raw_profile_change_requires_evidence_reextraction(tmp_path: Path) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _run_to_match(workspace, job_dir)
    source = workspace / "profile" / "typst" / "cv.typ"
    source.write_text(
        source.read_text(encoding="utf-8") + "Changed private source.\n",
        encoding="utf-8",
    )

    evidence_status = inspect_stage_status(workspace, job_dir, stage="evidence")
    match_status = inspect_stage_status(workspace, job_dir, stage="match")
    refreshed = run_deterministic_stage(workspace, job_dir, stage="evidence")
    refreshed_catalog = json.loads(
        (job_dir / "evidence_catalog.json").read_text(encoding="utf-8")
    )
    response = stage_run_agent_response(workspace, refreshed)

    assert evidence_status.stage.status == "stale"
    assert evidence_status.reasons == ("input_changed",)
    assert match_status.stage.status == "stale"
    assert refreshed_catalog["state"] == "unavailable"
    assert refreshed_catalog["unavailable_reason"] == "evidence.source_receipt_stale"
    assert response.extensions["canisend.evidence_state"] == "unavailable"
    assert response.extensions["canisend.evidence_reason"] == "evidence.source_receipt_stale"
    assert [item.id for item in response.next_actions] == ["profile.extract_evidence"]


@pytest.mark.parametrize(
    ("scenario", "expected_state", "expected_reason", "expected_action"),
    [
        ("empty", "empty", None, "profile.add_evidence"),
        (
            "profile_missing",
            "unavailable",
            "evidence.profile_missing",
            "profile.initialize",
        ),
        (
            "source_stale",
            "unavailable",
            "evidence.source_receipt_stale",
            "profile.extract_evidence",
        ),
    ],
)
def test_evidence_agent_distinguishes_empty_unavailable_and_stale_sources(
    tmp_path: Path,
    scenario: str,
    expected_state: str,
    expected_reason: str | None,
    expected_action: str,
) -> None:
    workspace, job_dir, evidence_path = _write_workspace(tmp_path)
    if scenario == "empty":
        source_bytes = (workspace / "profile" / "typst" / "cv.typ").read_bytes()
        evidence_path.write_text(
            "# Evidence: cv\n\n"
            f"<!-- canisend-source-sha256: {sha256(source_bytes).hexdigest()} -->\n\n"
            "## Education\n",
            encoding="utf-8",
        )
    elif scenario == "profile_missing":
        (workspace / "profile").rename(workspace / "profile-away")
    else:
        source = workspace / "profile" / "typst" / "cv.typ"
        source.write_text(
            source.read_text(encoding="utf-8") + "Changed source.\n",
            encoding="utf-8",
        )

    run_deterministic_stage(workspace, job_dir, stage="evidence")
    status = inspect_stage_status(workspace, job_dir, stage="evidence")
    response = stage_status_agent_response(workspace, job_dir, status)

    assert response.workflow is not None
    assert response.workflow.readiness == "review_required"
    assert response.extensions["canisend.evidence_count"] == 0
    assert response.extensions["canisend.evidence_gap_count"] == 1
    assert response.extensions["canisend.evidence_state"] == expected_state
    assert response.extensions["canisend.evidence_reason"] == expected_reason
    assert [item.id for item in response.next_actions] == [expected_action]


def test_evidence_agent_distinguishes_runtime_staleness_from_catalog_unavailability(
    tmp_path: Path,
) -> None:
    workspace, job_dir, evidence_path = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="evidence")
    evidence_path.write_text(
        evidence_path.read_text(encoding="utf-8")
        + "\n- [cv-003] `service`: New private evidence.\n",
        encoding="utf-8",
    )

    stale = inspect_stage_status(workspace, job_dir, stage="evidence")
    response = stage_status_agent_response(workspace, job_dir, stale)

    assert stale.stage.status == "stale"
    assert stale.reasons == ("input_changed",)
    assert response.extensions["canisend.stage_status"] == "stale"
    assert "canisend.evidence_state" not in response.extensions
    assert [item.id for item in response.next_actions] == ["stage.run_evidence"]


def test_evidence_prepare_snapshot_becomes_stale_when_profile_changes(tmp_path: Path) -> None:
    workspace, job_dir, evidence_path = _write_workspace(tmp_path)
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="evidence",
        execution_mode="deterministic",
    )
    adapter = get_stage_adapter("evidence")
    candidate = adapter.build_deterministic_candidate(
        workspace,
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
        inputs=prepared.task_spec.inputs,
    )
    evidence_path.write_text(
        evidence_path.read_text(encoding="utf-8") + "\n- [cv-003] `service`: Committee.\n",
        encoding="utf-8",
    )

    with pytest.raises(StageRuntimeError) as stale:
        submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(candidate) + "\n").encode("utf-8"),
        )

    assert stale.value.code == "stage.stale_input"
    assert not (job_dir / "evidence_catalog.json").exists()


@pytest.mark.parametrize("damage", ["missing", "hardlink"])
def test_invalid_evidence_snapshot_requires_cancellation_with_stable_error(
    tmp_path: Path,
    damage: str,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="evidence",
        execution_mode="deterministic",
    )
    snapshot = job_dir / prepared.task_spec.inputs[0].path
    if damage == "missing":
        snapshot.unlink()
    else:
        os.link(snapshot, snapshot.with_suffix(".alias.json"))

    status = inspect_stage_status(workspace, job_dir, stage="evidence")
    response = stage_status_agent_response(workspace, job_dir, status)

    assert "prepared_input_changed" in status.reasons
    assert [item.id for item in response.next_actions] == ["stage.cancel_active_task"]
    with pytest.raises(StageRuntimeError) as failed:
        run_deterministic_stage(workspace, job_dir, stage="evidence")
    assert failed.value.code == "stage.stale_input"


def test_evidence_apply_rechecks_live_profile_immediately_before_claim(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir, evidence_path = _write_workspace(tmp_path)
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="evidence",
        execution_mode="deterministic",
    )
    adapter = get_stage_adapter("evidence")
    candidate = adapter.build_deterministic_candidate(
        workspace,
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
        inputs=prepared.task_spec.inputs,
    )
    submitted = submit_stage_candidate(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(candidate) + "\n").encode("utf-8"),
    )
    original = stage_runtime_module._load_or_write_passed_validation

    def mutate_after_validation(*args: object, **kwargs: object) -> object:
        validation = original(*args, **kwargs)
        evidence_path.write_text(
            evidence_path.read_text(encoding="utf-8")
            + "\n- [cv-003] `service`: Late profile change.\n",
            encoding="utf-8",
        )
        return validation

    monkeypatch.setattr(
        stage_runtime_module,
        "_load_or_write_passed_validation",
        mutate_after_validation,
    )

    with pytest.raises(StageRuntimeError) as stale:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=submitted.result_path,
        )

    assert stale.value.code == "stage.stale_input"
    assert not (job_dir / "evidence_catalog.json").exists()
    assert not (prepared.task_spec_path.parent / "terminal-claim.json").exists()


def test_evidence_and_match_cancel_reconstruct_and_resume_deterministically(
    tmp_path: Path,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="evidence",
        execution_mode="deterministic",
    )
    running = inspect_stage_status(workspace, job_dir, stage="evidence")
    response = stage_status_agent_response(workspace, job_dir, running)
    assert [item.id for item in response.next_actions] == ["stage.run_evidence"]
    assert prepared.task_spec_path.is_file()

    cancelled = cancel_stage_task(workspace, job_dir, stage="evidence")
    assert cancelled.manifest.status == "cancelled"
    _run_to_match(workspace, job_dir)
    (job_dir / "workflow" / "state.json").unlink()

    reconstructed = inspect_stage_status(workspace, job_dir, stage="match")
    records = {record.stage: record for record in reconstructed.state.stages}
    assert reconstructed.reconstructed is True
    assert records["evidence"].status == "succeeded"
    assert records["parse"].status == "succeeded"
    assert records["confirm"].status == "succeeded"
    assert records["match"].status == "succeeded"


@pytest.mark.parametrize(
    ("stage", "target_name"),
    [
        ("evidence", "evidence_catalog.json"),
        ("match", "criterion_matches.json"),
    ],
)
def test_evidence_and_match_output_drift_is_never_overwritten(
    tmp_path: Path,
    stage: str,
    target_name: str,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _run_to_match(workspace, job_dir)
    target = job_dir / target_name
    target.write_bytes(target.read_bytes() + b"\n")
    before = _job_file_snapshot(job_dir)

    status = inspect_stage_status(
        workspace,
        job_dir,
        stage=stage,  # type: ignore[arg-type]
    )
    response = stage_status_agent_response(workspace, job_dir, status)

    assert status.output_drift is True
    assert status.reasons == ("output_drift",)
    assert response.workflow is not None
    assert response.workflow.readiness == "review_required"
    with pytest.raises(StageRuntimeError) as conflict:
        run_deterministic_stage(
            workspace,
            job_dir,
            stage=stage,  # type: ignore[arg-type]
        )
    assert conflict.value.code == "stage.output_conflict"
    assert _job_file_snapshot(job_dir) == before


@pytest.mark.parametrize(
    ("stage", "target_name"),
    [
        ("evidence", "evidence_catalog.json"),
        ("match", "criterion_matches.json"),
    ],
)
def test_evidence_and_match_cancel_wins_competing_terminal_claim(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    stage: str,
    target_name: str,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _prepare_stage_dependencies(workspace, job_dir, stage)
    prepared, submitted, _ = _prepare_and_submit(
        workspace,
        job_dir,
        stage=stage,
    )
    original_validate = stage_runtime_module._validate_active_task
    calls = 0

    def cancel_after_second_check(*args: object, **kwargs: object) -> None:
        nonlocal calls
        calls += 1
        original_validate(*args, **kwargs)
        if calls == 2:
            cancel_stage_task(
                workspace,
                job_dir,
                stage=stage,  # type: ignore[arg-type]
            )

    monkeypatch.setattr(
        stage_runtime_module,
        "_validate_active_task",
        cancel_after_second_check,
    )

    with pytest.raises(StageRuntimeError) as conflict:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=submitted.result_path,
        )

    assert conflict.value.code == "stage.transition_conflict"
    assert not (job_dir / target_name).exists()
    assert not (prepared.task_spec_path.parent / "promotion.json").exists()
    claim = json.loads(
        (prepared.task_spec_path.parent / "terminal-claim.json").read_text(
            encoding="utf-8"
        )
    )
    assert claim["action"] == "cancel"


@pytest.mark.parametrize("stage", ["evidence", "match"])
def test_evidence_and_match_reject_semantic_candidate_without_mutation(
    tmp_path: Path,
    stage: str,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _prepare_stage_dependencies(workspace, job_dir, stage)
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage=stage,  # type: ignore[arg-type]
        execution_mode="deterministic",
    )
    adapter = get_stage_adapter(stage)
    candidate = adapter.build_deterministic_candidate(
        workspace,
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
        inputs=prepared.task_spec.inputs,
    )
    candidate["job_id"] = "different-role"
    before = _job_file_snapshot(job_dir)

    with pytest.raises(StageRuntimeError) as rejected:
        submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(candidate) + "\n").encode("utf-8"),
        )

    assert rejected.value.code == "stage.invalid_candidate"
    assert _job_file_snapshot(job_dir) == before


@pytest.mark.parametrize("stage", ["evidence", "match"])
def test_evidence_and_match_apply_in_a_fresh_process(
    tmp_path: Path,
    stage: str,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _prepare_stage_dependencies(workspace, job_dir, stage)
    prepared, submitted, _ = _prepare_and_submit(
        workspace,
        job_dir,
        stage=stage,
    )

    payload = _fresh_cli(
        "stage",
        "apply",
        "--workspace",
        str(workspace),
        "--job",
        "jobs/example-role",
        "--task",
        prepared.task_spec_path.relative_to(job_dir).as_posix(),
        "--result",
        submitted.result_path.relative_to(job_dir).as_posix(),
        "--format",
        "json",
    )

    assert payload["ok"] is True
    assert inspect_stage_status(
        workspace,
        job_dir,
        stage=stage,  # type: ignore[arg-type]
    ).stage.status == "succeeded"


@pytest.mark.parametrize("stage", ["evidence", "match"])
@pytest.mark.parametrize("failed_record", ["promotion.json", "manifest.json"])
def test_evidence_and_match_recover_promotion_fault_in_a_fresh_process(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    stage: str,
    failed_record: str,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _prepare_stage_dependencies(workspace, job_dir, stage)
    prepared, submitted, _ = _prepare_and_submit(
        workspace,
        job_dir,
        stage=stage,
    )
    original_write = stage_runtime_module.write_immutable_json
    failed_once = False

    def fail_terminal_record_once(path: Path, value: object) -> Path:
        nonlocal failed_once
        if path.name == failed_record and not failed_once:
            failed_once = True
            raise StageStoreError(f"simulated {failed_record} failure")
        return original_write(path, value)

    monkeypatch.setattr(
        stage_runtime_module,
        "write_immutable_json",
        fail_terminal_record_once,
    )
    with pytest.raises(StageRuntimeError) as interrupted:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=submitted.result_path,
        )
    assert interrupted.value.code == "stage.invalid_result"
    monkeypatch.setattr(stage_runtime_module, "write_immutable_json", original_write)

    status = inspect_stage_status(
        workspace,
        job_dir,
        stage=stage,  # type: ignore[arg-type]
    )
    assert "terminal_claim:promote" in status.reasons
    assert "promotion_recovery" in status.reasons
    with pytest.raises(StageRuntimeError) as cancellation:
        cancel_stage_task(
            workspace,
            job_dir,
            stage=stage,  # type: ignore[arg-type]
        )
    assert cancellation.value.code == (
        "stage.transition_conflict"
        if failed_record == "promotion.json"
        else "stage.no_active_run"
    )

    common = (
        "--workspace",
        str(workspace),
        "--job",
        "jobs/example-role",
        "--format",
        "json",
    )
    if failed_record == "promotion.json":
        payload = _fresh_cli(
            "stage",
            "apply",
            "--task",
            prepared.task_spec_path.relative_to(job_dir).as_posix(),
            "--result",
            submitted.result_path.relative_to(job_dir).as_posix(),
            *common,
        )
    else:
        payload = _fresh_cli("stage", "run", "--stage", stage, *common)

    assert payload["ok"] is True
    recovered = inspect_stage_status(
        workspace,
        job_dir,
        stage=stage,  # type: ignore[arg-type]
    )
    assert recovered.stage.status == "succeeded"
    assert recovered.reasons == ()
    assert (prepared.task_spec_path.parent / "manifest.json").is_file()


def test_stage1_v1_task_receipt_and_state_fixture_survives_registry_upgrade(
    tmp_path: Path,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    prepared = prepare_stage(
        workspace,
        job_dir,
        stage="parse",
        execution_mode="host_agent",
    )
    adapter = get_stage_adapter("parse")
    candidate = adapter.build_deterministic_candidate(
        workspace,
        job_dir,
        input_fingerprint=prepared.task_spec.input_fingerprint,
        inputs=prepared.task_spec.inputs,
    )
    submitted = submit_stage_candidate(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(candidate) + "\n").encode("utf-8"),
    )
    task = json.loads(prepared.task_spec_path.read_text(encoding="utf-8"))
    receipt = json.loads(
        (prepared.task_spec_path.parent / "preparation.json").read_text(
            encoding="utf-8"
        )
    )
    state_path = job_dir / "workflow" / "state.json"
    state = json.loads(state_path.read_text(encoding="utf-8"))

    # These are the frozen v1 wire fields emitted before Evidence and Match were
    # executable. Keeping this literal fixture shape prevents a registry upgrade
    # from silently invalidating an already-prepared cross-platform task.
    assert set(task) == {
        "schema_version",
        "task_id",
        "run_id",
        "job_id",
        "stage",
        "operation",
        "execution_mode",
        "created_at",
        "input_fingerprint",
        "inputs",
        "allowed_reads",
        "allowed_writes",
        "write_authority",
        "candidate_output",
        "result_output",
        "authoritative_target",
        "expected_output_sha256",
        "output_schema",
        "privacy_tier",
        "required_consents",
    }
    assert set(receipt) == {
        "schema_version",
        "run_id",
        "task_id",
        "job_id",
        "stage",
        "attempt",
        "execution_mode",
        "status",
        "created_at",
        "started_at",
        "completed_at",
        "inputs",
        "input_fingerprint",
        "task_spec_sha256",
        "candidate_outputs",
        "promoted_outputs",
        "validation_report_path",
        "error_code",
        "error_message",
    }
    assert set(state) == {
        "schema_version",
        "job_id",
        "revision",
        "created_at",
        "updated_at",
        "active_run_id",
        "stages",
    }
    assert task["schema_version"] == receipt["schema_version"] == "1.0.0"
    assert [item["stage"] for item in state["stages"]] == ["parse"]

    applied = _fresh_cli(
        "stage",
        "apply",
        "--workspace",
        str(workspace),
        "--job",
        "jobs/example-role",
        "--task",
        prepared.task_spec_path.relative_to(job_dir).as_posix(),
        "--result",
        submitted.result_path.relative_to(job_dir).as_posix(),
        "--format",
        "json",
    )
    assert applied["ok"] is True

    run_deterministic_stage(workspace, job_dir, stage="evidence")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    run_deterministic_stage(workspace, job_dir, stage="match")
    state_path.unlink()
    recovered = _fresh_cli(
        "stage",
        "status",
        "--workspace",
        str(workspace),
        "--job",
        "jobs/example-role",
        "--stage",
        "match",
        "--format",
        "json",
    )

    assert recovered["ok"] is True
    assert recovered["extensions"]["canisend.state_reconstructed"] is True
    assert recovered["extensions"]["canisend.stage_status"] == "succeeded"


@pytest.mark.parametrize("stage", ["evidence", "match"])
def test_evidence_and_match_reject_host_agent_mode(tmp_path: Path, stage: str) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    if stage == "match":
        run_deterministic_stage(workspace, job_dir, stage="evidence")
        run_deterministic_stage(workspace, job_dir, stage="parse")
        run_deterministic_stage(workspace, job_dir, stage="confirm")

    with pytest.raises(StageRuntimeError) as unsupported:
        prepare_stage(
            workspace,
            job_dir,
            stage=stage,  # type: ignore[arg-type]
            execution_mode="host_agent",
        )

    assert unsupported.value.code == "stage.unsupported_mode"


def test_evidence_and_match_never_invoke_provider_network_or_platform_api(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)

    def fail(*args: object, **kwargs: object) -> object:
        raise AssertionError("deterministic Evidence/Match must remain local")

    original_import = builtins.__import__

    def reject_platform_sdk_import(
        name: str,
        globals: object = None,
        locals: object = None,
        fromlist: object = (),
        level: int = 0,
    ) -> object:
        if name.partition(".")[0] in {"anthropic", "mcp", "openai"}:
            raise AssertionError("deterministic Evidence/Match must not load a platform SDK")
        return original_import(name, globals, locals, fromlist, level)

    original_import_module = importlib.import_module

    def reject_dynamic_platform_sdk_import(
        name: str,
        package: str | None = None,
    ) -> object:
        if name.partition(".")[0] in {"anthropic", "mcp", "openai"}:
            raise AssertionError("deterministic Evidence/Match must not load a platform SDK")
        return original_import_module(name, package)

    monkeypatch.setattr("canisend.llm.provider_from_config", fail)
    monkeypatch.setattr("canisend.llm.urlopen", fail)
    monkeypatch.setattr("canisend.llm.subprocess.run", fail)
    monkeypatch.setattr("canisend.pipeline.provider_from_config", fail)
    monkeypatch.setattr("canisend.versioning.urlopen", fail)
    monkeypatch.setattr("canisend.job_import.urlopen", fail)
    monkeypatch.setattr("canisend.rss.urlopen", fail)
    monkeypatch.setattr(socket, "create_connection", fail)
    monkeypatch.setattr(socket, "socket", fail)
    monkeypatch.setattr(builtins, "__import__", reject_platform_sdk_import)
    monkeypatch.setattr(importlib, "import_module", reject_dynamic_platform_sdk_import)
    monkeypatch.setenv("OPENAI_API_KEY", "must-not-be-used")
    monkeypatch.setenv("ANTHROPIC_API_KEY", "must-not-be-used")

    _run_to_match(workspace, job_dir)

    assert (job_dir / "evidence_catalog.json").is_file()
    assert (job_dir / "criterion_matches.json").is_file()


def test_legacy_pipeline_preserves_current_structured_decision_spine(tmp_path: Path) -> None:
    workspace, job_dir, _ = _write_workspace(tmp_path)
    _run_to_match(workspace, job_dir)
    initialize_confirmed_corrections(
        workspace,
        job_dir,
        consent_confirmed=True,
    )
    decision = initialize_application_decision(
        workspace,
        job_dir,
        consent_confirmed=True,
    )
    apply_user_patch(
        workspace,
        job_dir,
        SetDecisionPatch(
            decision="apply",
            rationale_mode="set",
            rationale="PRIVATE LEGACY DECISION MUST REMAIN BYTE-IDENTICAL",
        ),
        expected_revision=decision.snapshot.revision,
        expected_sha256=decision.snapshot.sha256,
        consent_confirmed=True,
    )
    structured = (
        "parsed_job.json",
        "criteria.json",
        "evidence_catalog.json",
        "criterion_matches.json",
        "confirmed_corrections.yaml",
        "application_decision.yaml",
    )
    before = {name: sha256_file(job_dir / name) for name in structured}

    result = CliRunner().invoke(
        app,
        [
            "run",
            "--workspace",
            str(workspace),
            "--job",
            "jobs/example-role",
            "--no-git-add-materials",
        ],
    )

    assert result.exit_code == 0, result.output
    assert {name: sha256_file(job_dir / name) for name in structured} == before
    for stage in ("evidence", "parse", "confirm", "match"):
        status = inspect_stage_status(
            workspace,
            job_dir,
            stage=stage,  # type: ignore[arg-type]
        )
        assert status.stage.status == "succeeded"
        assert status.reasons == ()
