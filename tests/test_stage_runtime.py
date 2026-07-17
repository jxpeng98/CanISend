from __future__ import annotations

import json
import multiprocessing
import os
from pathlib import Path

import pytest
import yaml

from canisend.stage_agent import stage_status_agent_response
from canisend.stage_models import ArtifactFingerprint, StageRecord, WorkflowStateV1
from canisend.stage_store import (
    StageStoreError,
    atomic_write_json,
    sha256_bytes,
    sha256_file,
)
from canisend.stage_runtime import (
    StageRuntimeError,
    apply_stage_result,
    cancel_stage_task,
    inspect_stage_status,
    prepare_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.stages.parse_stage import build_deterministic_parse_candidate


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


def _write_success_result(prepared: object, candidate: dict[str, object]) -> Path:
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


def _prepare_in_process(
    workspace: str,
    job_dir: str,
    start: object,
    results: object,
) -> None:
    start.wait(timeout=10)
    try:
        prepared = prepare_stage(
            Path(workspace),
            Path(job_dir),
            stage="parse",
            execution_mode="host_agent",
        )
        results.put(("ok", prepared.task_spec.run_id, prepared.reused))
    except StageRuntimeError as exc:
        results.put(("error", exc.code, False))


def _finish_in_process(
    action: str,
    workspace: str,
    job_dir: str,
    task_spec_path: str,
    task_result_path: str,
    start: object,
    results: object,
) -> None:
    start.wait(timeout=10)
    try:
        if action == "apply":
            apply_stage_result(
                Path(workspace),
                Path(job_dir),
                task_spec_path=Path(task_spec_path),
                task_result_path=Path(task_result_path),
            )
        else:
            cancel_stage_task(Path(workspace), Path(job_dir), stage="parse")
        results.put((action, "ok"))
    except StageRuntimeError as exc:
        results.put((action, exc.code))


def test_prepare_is_fresh_session_reusable_and_provider_free(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)

    def fail_provider(*args: object, **kwargs: object) -> object:
        raise AssertionError("host-agent prepare must not construct a provider")

    monkeypatch.setattr("canisend.llm.provider_from_config", fail_provider)
    first = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    second = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")

    assert first.reused is False
    assert second.reused is True
    assert second.task_spec == first.task_spec
    assert second.task_spec_path == first.task_spec_path
    assert first.task_spec_path.is_file()
    assert first.task_spec.privacy_tier == 2
    assert first.candidate_path.parent.is_dir()
    state = inspect_stage_status(workspace, job_dir)
    assert state.stage.status == "running"
    assert state.pending_task_path == first.task_spec_path


def test_concurrent_processes_prepare_exactly_one_stage_run(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    context = multiprocessing.get_context("spawn")
    start = context.Event()
    results = context.Queue()
    processes = [
        context.Process(
            target=_prepare_in_process,
            args=(str(workspace), str(job_dir), start, results),
        )
        for _ in range(2)
    ]
    for process in processes:
        process.start()
    start.set()
    for process in processes:
        process.join(timeout=15)
        assert process.exitcode == 0

    outcomes = [results.get(timeout=5) for _ in processes]
    assert {item[0] for item in outcomes} == {"ok"}
    assert len({item[1] for item in outcomes}) == 1
    assert {item[2] for item in outcomes} == {False, True}
    assert len(list((job_dir / "workflow" / "runs").glob("*/task-spec.json"))) == 1


def test_concurrent_apply_and_cancel_have_one_terminal_winner(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    context = multiprocessing.get_context("spawn")
    start = context.Event()
    results = context.Queue()
    processes = [
        context.Process(
            target=_finish_in_process,
            args=(
                action,
                str(workspace),
                str(job_dir),
                str(prepared.task_spec_path),
                str(result_path),
                start,
                results,
            ),
        )
        for action in ("apply", "cancel")
    ]
    for process in processes:
        process.start()
    start.set()
    for process in processes:
        process.join(timeout=15)
        assert process.exitcode == 0

    outcomes = [results.get(timeout=5) for _ in processes]
    assert [result for _, result in outcomes].count("ok") == 1
    assert len(list(prepared.task_spec_path.parent.glob("terminal-claim.json"))) == 1
    assert len(list(prepared.task_spec_path.parent.glob("manifest.json"))) == 1
    status = inspect_stage_status(workspace, job_dir, stage="parse")
    assert status.stage.status in {"succeeded", "cancelled"}
    assert status.state.active_run_id is None


def test_deterministic_parse_cache_is_true_noop(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)

    first = run_deterministic_stage(workspace, job_dir, stage="parse")
    authoritative = job_dir / "parsed_job.json"
    first_bytes = authoritative.read_bytes()
    first_mtime = authoritative.stat().st_mtime_ns
    second = run_deterministic_stage(workspace, job_dir, stage="parse")

    assert first.cache_hit is False
    assert first.manifest is not None
    assert first.manifest.status == "succeeded"
    assert second.cache_hit is True
    assert second.manifest is None
    assert authoritative.read_bytes() == first_bytes
    assert authoritative.stat().st_mtime_ns == first_mtime
    assert inspect_stage_status(workspace, job_dir).stage.status == "succeeded"


@pytest.mark.parametrize(
    "failure_point",
    (
        "after_terminal_claim",
        "after_authoritative_replace",
        "after_promotion_receipt",
        "after_manifest",
        "after_state_refresh",
    ),
)
def test_apply_failure_points_converge_without_duplicate_promotion(
    tmp_path: Path,
    failure_point: str,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    injected = False

    def fail_once(point: str) -> None:
        nonlocal injected
        if point == failure_point and not injected:
            injected = True
            raise RuntimeError(f"injected failure at {point}")

    with pytest.raises(RuntimeError, match="injected failure"):
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
            failure_injector=fail_once,
        )

    if failure_point in {
        "after_terminal_claim",
        "after_authoritative_replace",
    }:
        applied = apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )
        assert applied.manifest.status == "succeeded"
    else:
        resumed = run_deterministic_stage(workspace, job_dir, stage="parse")
        assert resumed.cache_hit is True

    status = inspect_stage_status(workspace, job_dir, stage="parse")
    assert status.stage.status == "succeeded"
    assert not status.reasons
    assert len(list((job_dir / "workflow" / "runs").glob("*/promotion.json"))) == 1


@pytest.mark.parametrize(
    "failure_point",
    ("after_terminal_claim", "after_manifest", "after_state_refresh"),
)
def test_cancel_failure_points_converge_to_one_cancelled_run(
    tmp_path: Path,
    failure_point: str,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")

    def fail_at(point: str) -> None:
        if point == failure_point:
            raise RuntimeError(f"injected failure at {point}")

    with pytest.raises(RuntimeError, match="injected failure"):
        cancel_stage_task(
            workspace,
            job_dir,
            stage="parse",
            failure_injector=fail_at,
        )

    if failure_point == "after_terminal_claim":
        cancelled = cancel_stage_task(workspace, job_dir, stage="parse")
        assert cancelled.manifest.status == "cancelled"

    status = inspect_stage_status(workspace, job_dir, stage="parse")
    assert status.stage.status == "cancelled"
    assert status.state.active_run_id is None
    manifests = list((job_dir / "workflow" / "runs").glob("*/manifest.json"))
    assert manifests == [prepared.task_spec_path.parent / "manifest.json"]


def test_parse_staleness_is_precise_for_advert_and_profile_changes(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    profile = workspace / "profile" / "generated" / "cv.evidence.md"
    profile.parent.mkdir(parents=True)
    profile.write_text("new private evidence\n", encoding="utf-8")

    assert inspect_stage_status(workspace, job_dir).stage.status == "succeeded"

    advert = job_dir / "job_advert.md"
    advert.write_text(advert.read_text() + "\n- Added requirement\n", encoding="utf-8")

    status = inspect_stage_status(workspace, job_dir)
    assert status.stage.status == "stale"
    assert "input_changed" in status.reasons


def test_parse_staleness_propagates_only_to_declared_descendant_records(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    status = inspect_stage_status(workspace, job_dir)
    output = job_dir / "confirmed.json"
    output.write_text("{}\n", encoding="utf-8")
    confirm = StageRecord(
        stage="confirm",
        status="succeeded",
        attempt_count=1,
        run_id="run_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        input_fingerprint="b" * 64,
        outputs=(
            ArtifactFingerprint(
                path="confirmed.json",
                sha256=sha256_bytes(output.read_bytes()),
                size_bytes=output.stat().st_size,
            ),
        ),
        started_at=status.stage.started_at,
        completed_at=status.stage.completed_at,
    )
    seeded = WorkflowStateV1(
        job_id=status.state.job_id,
        revision=status.state.revision + 1,
        created_at=status.state.created_at,
        updated_at=status.state.updated_at,
        stages=(status.stage, confirm),
    )
    atomic_write_json(
        job_dir / "workflow" / "state.json",
        seeded.model_dump(mode="json"),
    )
    advert = job_dir / "job_advert.md"
    advert.write_text(advert.read_text() + "\nChanged input\n", encoding="utf-8")

    changed = inspect_stage_status(workspace, job_dir)
    records = {record.stage: record for record in changed.state.stages}

    assert records["parse"].status == "stale"
    assert records["confirm"].status == "stale"


def test_stale_host_result_is_rejected_without_authoritative_write(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    candidate = build_deterministic_parse_candidate(job_dir)
    result_path = _write_success_result(prepared, candidate)
    (job_dir / "job_advert.md").write_text("# Changed after prepare\n", encoding="utf-8")

    with pytest.raises(StageRuntimeError) as captured:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert captured.value.code == "stage.stale_input"
    assert not (job_dir / "parsed_job.json").exists()


def test_invalid_candidate_and_wrong_hash_never_promote(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    candidate = build_deterministic_parse_candidate(job_dir)
    candidate.pop("deadline")
    with pytest.raises(StageRuntimeError) as invalid:
        submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(candidate) + "\n").encode("utf-8"),
        )

    assert invalid.value.code == "stage.invalid_candidate"
    assert not (job_dir / "parsed_job.json").exists()

    cancel_stage_task(workspace, job_dir, stage="parse")
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(prepared, build_deterministic_parse_candidate(job_dir))
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    payload["outputs"][0]["sha256"] = "f" * 64
    result_path.write_text(json.dumps(payload), encoding="utf-8")

    with pytest.raises(StageRuntimeError) as wrong_hash:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert wrong_hash.value.code == "stage.submission_conflict"
    assert not (job_dir / "parsed_job.json").exists()


@pytest.mark.skipif(os.name == "nt", reason="symlink creation may require elevated privileges")
def test_candidate_symlink_escape_is_rejected(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    candidate_parent = prepared.candidate_path.parent
    candidate_parent.rmdir()
    outside = tmp_path / "outside"
    outside.mkdir()
    candidate_parent.symlink_to(outside, target_is_directory=True)

    with pytest.raises(StageRuntimeError) as captured:
        submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(
                json.dumps(build_deterministic_parse_candidate(job_dir)) + "\n"
            ).encode("utf-8"),
        )

    assert captured.value.code == "stage.unsafe_path"
    assert not (job_dir / "parsed_job.json").exists()


def test_manual_output_drift_is_preserved_and_reported(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    authoritative = job_dir / "parsed_job.json"
    authoritative.write_text('{"manual":"edit"}\n', encoding="utf-8")

    status = inspect_stage_status(workspace, job_dir)
    assert status.output_drift is True
    assert "output_drift" in status.reasons

    with pytest.raises(StageRuntimeError) as captured:
        run_deterministic_stage(workspace, job_dir, stage="parse")

    assert captured.value.code == "stage.output_conflict"
    assert authoritative.read_text(encoding="utf-8") == '{"manual":"edit"}\n'


def test_second_execution_mode_cannot_create_a_parallel_task(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    host_task = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    with pytest.raises(StageRuntimeError) as second:
        prepare_stage(
            workspace,
            job_dir,
            stage="parse",
            execution_mode="deterministic",
        )

    assert second.value.code == "stage.concurrent_run"
    pending = list((job_dir / "workflow" / "runs").glob("*/task-spec.json"))
    assert pending == [host_task.task_spec_path]


def test_running_output_drift_blocks_rerun_and_is_cancellable(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    advert = job_dir / "job_advert.md"
    advert.write_text(advert.read_text(encoding="utf-8") + "\nChanged input.\n", encoding="utf-8")
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    authoritative = job_dir / "parsed_job.json"
    authoritative.write_text('{"manual":"preserve"}\n', encoding="utf-8")

    status = inspect_stage_status(workspace, job_dir, stage="parse")

    assert status.stage.status == "running"
    assert status.output_drift is True
    assert status.pending_task_path == prepared.task_spec_path
    with pytest.raises(StageRuntimeError) as captured:
        run_deterministic_stage(workspace, job_dir, stage="parse")
    assert captured.value.code == "stage.output_conflict"
    assert authoritative.read_text(encoding="utf-8") == '{"manual":"preserve"}\n'

    cancelled = cancel_stage_task(workspace, job_dir, stage="parse")
    assert cancelled.manifest.status == "cancelled"
    assert inspect_stage_status(workspace, job_dir, stage="parse").stage.status == "cancelled"


def test_modified_task_spec_cannot_forge_output_compare_and_swap(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    authoritative = job_dir / "parsed_job.json"
    authoritative.write_text('{"manual":"preserve"}\n', encoding="utf-8")
    forged = json.loads(prepared.task_spec_path.read_text(encoding="utf-8"))
    forged["expected_output_sha256"] = sha256_file(authoritative)
    prepared.task_spec_path.write_text(
        json.dumps(forged, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    with pytest.raises(StageRuntimeError) as captured:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert captured.value.code == "stage.task_integrity_mismatch"
    assert authoritative.read_text(encoding="utf-8") == '{"manual":"preserve"}\n'
    assert inspect_stage_status(workspace, job_dir, stage="parse").stage.status == "ready"
    replacement = prepare_stage(
        workspace,
        job_dir,
        stage="parse",
        execution_mode="host_agent",
    )
    assert replacement.task_spec.run_id != prepared.task_spec.run_id


def test_task_and_preparation_dual_edit_cannot_forge_output_baseline(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    advert = job_dir / "job_advert.md"
    advert.write_text(advert.read_text(encoding="utf-8") + "\nChanged input.\n", encoding="utf-8")
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    authoritative = job_dir / "parsed_job.json"
    authoritative.write_text('{"manual":"preserve"}\n', encoding="utf-8")
    forged = json.loads(prepared.task_spec_path.read_text(encoding="utf-8"))
    forged["expected_output_sha256"] = sha256_file(authoritative)
    prepared.task_spec_path.write_text(
        json.dumps(forged, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    preparation_path = prepared.task_spec_path.parent / "preparation.json"
    preparation = json.loads(preparation_path.read_text(encoding="utf-8"))
    preparation["task_spec_sha256"] = sha256_file(prepared.task_spec_path)
    preparation_path.write_text(
        json.dumps(preparation, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    with pytest.raises(StageRuntimeError) as captured:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert captured.value.code == "stage.task_integrity_mismatch"
    assert authoritative.read_text(encoding="utf-8") == '{"manual":"preserve"}\n'


def test_cancelled_task_result_cannot_promote_late(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    cancelled = cancel_stage_task(workspace, job_dir, stage="parse")

    with pytest.raises(StageRuntimeError) as captured:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert captured.value.code == "stage.task_not_active"
    assert not (job_dir / "parsed_job.json").exists()
    assert not (prepared.task_spec_path.parent / "promotion.json").exists()
    assert cancelled.manifest_path.is_file()


def test_cancel_and_apply_compete_for_one_terminal_claim(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    from canisend import stage_runtime

    original_validate = stage_runtime._validate_active_task
    calls = 0

    def cancel_after_second_check(*args: object, **kwargs: object) -> None:
        nonlocal calls
        calls += 1
        original_validate(*args, **kwargs)
        if calls == 2:
            cancel_stage_task(workspace, job_dir, stage="parse")

    monkeypatch.setattr(stage_runtime, "_validate_active_task", cancel_after_second_check)

    with pytest.raises(StageRuntimeError) as captured:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert captured.value.code == "stage.transition_conflict"
    assert not (job_dir / "parsed_job.json").exists()
    assert not (prepared.task_spec_path.parent / "promotion.json").exists()
    claim = json.loads(
        (prepared.task_spec_path.parent / "terminal-claim.json").read_text(
            encoding="utf-8"
        )
    )
    assert claim["action"] == "cancel"


def test_promotion_claim_retries_after_authoritative_write_failure(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    from canisend import stage_runtime

    original_atomic_write = stage_runtime.atomic_write_bytes
    target = job_dir / "parsed_job.json"
    failed_once = False

    def fail_target_once(path: Path, payload: bytes) -> Path:
        nonlocal failed_once
        if path == target and not failed_once:
            failed_once = True
            raise StageStoreError("simulated authoritative write failure")
        return original_atomic_write(path, payload)

    monkeypatch.setattr(stage_runtime, "atomic_write_bytes", fail_target_once)
    with pytest.raises(StageRuntimeError) as first:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )
    assert first.value.code == "stage.invalid_result"
    assert not target.exists()
    assert not (prepared.task_spec_path.parent / "manifest.json").exists()
    with pytest.raises(StageRuntimeError) as cancellation:
        cancel_stage_task(workspace, job_dir, stage="parse")
    assert cancellation.value.code == "stage.transition_conflict"

    recovered = apply_stage_result(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        task_result_path=result_path,
    )

    assert recovered.manifest.status == "succeeded"
    assert target.is_file()


def test_fresh_status_resumes_claimed_promotion_after_target_write(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    from canisend import stage_runtime

    original_immutable_write = stage_runtime.write_immutable_json
    failed_once = False

    def fail_promotion_once(path: Path, value: object) -> Path:
        nonlocal failed_once
        if path.name == "promotion.json" and not failed_once:
            failed_once = True
            raise StageStoreError("simulated promotion receipt failure")
        return original_immutable_write(path, value)

    monkeypatch.setattr(stage_runtime, "write_immutable_json", fail_promotion_once)
    with pytest.raises(StageRuntimeError) as first:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )
    assert first.value.code == "stage.invalid_result"
    status = inspect_stage_status(workspace, job_dir, stage="parse")
    response = stage_status_agent_response(workspace, job_dir, status)
    assert status.output_drift is False
    assert "promotion_recovery" in status.reasons
    assert [item.id for item in response.next_actions] == [
        "stage.apply_parse_candidate"
    ]
    with pytest.raises(StageRuntimeError) as cancellation:
        cancel_stage_task(workspace, job_dir, stage="parse")
    assert cancellation.value.code == "stage.transition_conflict"

    recovered = apply_stage_result(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        task_result_path=result_path,
    )
    assert recovered.manifest.status == "succeeded"


def test_fresh_status_retries_claimed_cancellation(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    from canisend import stage_runtime

    original_immutable_write = stage_runtime.write_immutable_json
    failed_once = False

    def fail_cancel_manifest_once(path: Path, value: object) -> Path:
        nonlocal failed_once
        if path.name == "manifest.json" and not failed_once:
            failed_once = True
            raise StageStoreError("simulated cancellation manifest failure")
        return original_immutable_write(path, value)

    monkeypatch.setattr(stage_runtime, "write_immutable_json", fail_cancel_manifest_once)
    with pytest.raises(StageRuntimeError) as first:
        cancel_stage_task(workspace, job_dir, stage="parse")
    assert first.value.code == "stage.store_failed"
    status = inspect_stage_status(workspace, job_dir, stage="parse")
    response = stage_status_agent_response(workspace, job_dir, status)
    assert "terminal_claim:cancel" in status.reasons
    assert [item.id for item in response.next_actions] == [
        "stage.cancel_active_task"
    ]

    cancelled = cancel_stage_task(workspace, job_dir, stage="parse")
    assert cancelled.manifest.status == "cancelled"
    assert prepared.task_spec_path.is_file()


def test_resubmitting_different_candidate_is_zero_mutation_conflict(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    first = build_deterministic_parse_candidate(job_dir)
    submitted = submit_stage_candidate(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(first) + "\n").encode("utf-8"),
    )
    before = {
        path: path.read_bytes()
        for path in (
            submitted.candidate_path,
            submitted.result_path,
            submitted.submission_path,
        )
    }
    second = dict(first)
    second["salary"] = "A different but valid salary display"

    with pytest.raises(StageRuntimeError) as captured:
        submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(second) + "\n").encode("utf-8"),
        )

    assert captured.value.code == "stage.submission_conflict"
    assert {path: path.read_bytes() for path in before} == before


def test_preparation_without_state_write_is_reconstructed_as_single_active_task(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    advert = job_dir / "job_advert.md"
    advert.write_text(advert.read_text(encoding="utf-8") + "\nChanged input.\n", encoding="utf-8")
    from canisend import stage_runtime

    original_write_state = stage_runtime._write_state

    def fail_state_write(*args: object, **kwargs: object) -> None:
        raise StageRuntimeError("stage.state_write_failed", "simulated state failure")

    monkeypatch.setattr(stage_runtime, "_write_state", fail_state_write)
    with pytest.raises(StageRuntimeError):
        prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    monkeypatch.setattr(stage_runtime, "_write_state", original_write_state)

    reconstructed = inspect_stage_status(workspace, job_dir, stage="parse")
    assert reconstructed.reconstructed is True
    assert reconstructed.stage.status == "running"
    assert reconstructed.pending_task_path is not None
    with pytest.raises(StageRuntimeError) as captured:
        prepare_stage(workspace, job_dir, stage="parse", execution_mode="deterministic")
    assert captured.value.code == "stage.concurrent_run"
    pending = [
        path
        for path in (job_dir / "workflow" / "runs").glob("*/task-spec.json")
        if not (path.parent / "manifest.json").exists()
    ]
    assert len(pending) == 1


def test_missing_preparation_receipt_discards_untrusted_running_view(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    (prepared.task_spec_path.parent / "preparation.json").unlink()

    status = inspect_stage_status(workspace, job_dir, stage="parse")

    assert status.reconstructed is True
    assert status.stage.status == "ready"
    assert status.pending_task_path is None
    replacement = prepare_stage(
        workspace,
        job_dir,
        stage="parse",
        execution_mode="host_agent",
    )
    assert replacement.task_spec.run_id != prepared.task_spec.run_id


@pytest.mark.skipif(os.name == "nt", reason="symlink creation may require elevated privileges")
def test_candidate_symlink_to_user_owned_job_file_is_rejected(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    protected = job_dir / "application_decision.yaml"
    protected.write_text("decision: hold\n", encoding="utf-8")
    prepared.candidate_path.symlink_to(protected)

    with pytest.raises(StageRuntimeError) as captured:
        submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(
                json.dumps(build_deterministic_parse_candidate(job_dir)) + "\n"
            ).encode("utf-8"),
        )

    assert captured.value.code == "stage.unsafe_path"
    assert protected.read_text(encoding="utf-8") == "decision: hold\n"


@pytest.mark.skipif(os.name == "nt", reason="hard links vary across filesystems")
def test_candidate_hard_link_to_user_owned_job_file_is_rejected(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    protected = job_dir / "application_decision.yaml"
    protected.write_text("decision: hold\n", encoding="utf-8")
    os.link(protected, prepared.candidate_path)

    with pytest.raises(StageRuntimeError) as captured:
        submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(
                json.dumps(build_deterministic_parse_candidate(job_dir)) + "\n"
            ).encode("utf-8"),
        )

    assert captured.value.code == "stage.unsafe_path"
    assert protected.read_text(encoding="utf-8") == "decision: hold\n"


def test_core_submission_timestamp_preserves_reconstructed_dependency_order(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(
        prepared,
        build_deterministic_parse_candidate(job_dir),
    )
    applied = apply_stage_result(
        workspace,
        job_dir,
        task_spec_path=prepared.task_spec_path,
        task_result_path=result_path,
    )
    assert applied.manifest.completed_at.year < 2099
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    (job_dir / "workflow" / "state.json").unlink()

    reconstructed = inspect_stage_status(workspace, job_dir, stage="confirm")

    assert reconstructed.reconstructed is True
    assert reconstructed.stage.status == "succeeded"


def test_state_is_reconstructed_from_immutable_manifest(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    result = run_deterministic_stage(workspace, job_dir, stage="parse")
    state_path = job_dir / "workflow" / "state.json"
    state_path.write_text("not json\n", encoding="utf-8")

    status = inspect_stage_status(workspace, job_dir)

    assert result.manifest is not None
    assert status.reconstructed is True
    assert status.stage.status == "succeeded"
    assert state_path.read_text(encoding="utf-8") == "not json\n"


@pytest.mark.skipif(os.name == "nt", reason="symlink creation may require elevated privileges")
def test_state_reconstruction_ignores_symlinked_run_directory(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    result = run_deterministic_stage(workspace, job_dir, stage="parse")
    assert result.manifest_path is not None
    run_dir = result.manifest_path.parent
    external_run = tmp_path / "external-run"
    run_dir.rename(external_run)
    run_dir.symlink_to(external_run, target_is_directory=True)
    (job_dir / "workflow" / "state.json").unlink()

    status = inspect_stage_status(workspace, job_dir, stage="parse")

    assert status.reconstructed is True
    assert status.stage.status == "ready"
    assert status.pending_task_path is None


def test_promotion_without_manifest_is_reconciled_on_next_mutating_session(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    prepared = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    result_path = _write_success_result(prepared, build_deterministic_parse_candidate(job_dir))
    from canisend import stage_runtime

    original_write = stage_runtime.write_immutable_json

    def fail_manifest(path: Path, value: object) -> Path:
        if path.name == "manifest.json":
            raise StageStoreError("simulated terminal manifest failure")
        return original_write(path, value)

    monkeypatch.setattr(stage_runtime, "write_immutable_json", fail_manifest)
    with pytest.raises(StageRuntimeError):
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )
    authoritative = job_dir / "parsed_job.json"
    promoted_hash = sha256_bytes(authoritative.read_bytes())
    assert (prepared.task_spec_path.parent / "promotion.json").is_file()
    assert not (prepared.task_spec_path.parent / "manifest.json").exists()

    monkeypatch.setattr(stage_runtime, "write_immutable_json", original_write)
    resumed = run_deterministic_stage(workspace, job_dir, stage="parse")

    assert resumed.cache_hit is True
    assert sha256_bytes(authoritative.read_bytes()) == promoted_hash
    assert (prepared.task_spec_path.parent / "manifest.json").is_file()
    assert inspect_stage_status(workspace, job_dir).stage.status == "succeeded"


def test_stage_runtime_refuses_job_outside_workspace(tmp_path: Path) -> None:
    workspace, _ = _write_workspace(tmp_path)
    external = tmp_path / "external-job"
    external.mkdir()

    with pytest.raises(StageRuntimeError) as captured:
        prepare_stage(workspace, external, stage="parse", execution_mode="host_agent")

    assert captured.value.code == "stage.job_outside_workspace"


def test_runtime_control_records_exclude_private_bodies_queries_and_absolute_paths(
    tmp_path: Path,
) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    result = run_deterministic_stage(workspace, job_dir, stage="parse")
    assert result.manifest_path is not None
    run_dir = result.manifest_path.parent
    control_paths = [
        job_dir / "workflow" / "state.json",
        run_dir / "task-spec.json",
        run_dir / "preparation.json",
        run_dir / "submission.json",
        run_dir / "terminal-claim.json",
        run_dir / "tasks" / result.manifest.task_id / "result.json",
        run_dir / "validation" / "report.json",
        run_dir / "promotion.json",
        run_dir / "manifest.json",
    ]
    rendered = "\n".join(path.read_text(encoding="utf-8") for path in control_paths)

    assert "PhD in Economics" not in rendered
    assert "private=token" not in rendered
    assert str(workspace) not in rendered
    assert "job_advert.md" in rendered
    assert "parsed_job.json" in rendered
