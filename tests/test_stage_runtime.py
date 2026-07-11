from __future__ import annotations

from datetime import UTC, datetime, timedelta
import json
import os
from pathlib import Path

import pytest
import yaml

from canisend.stage_models import ArtifactFingerprint, StageRecord, TaskResultV1, WorkflowStateV1
from canisend.stage_store import StageStoreError, atomic_write_json, sha256_bytes
from canisend.stage_runtime import (
    StageRuntimeError,
    apply_stage_result,
    inspect_stage_status,
    prepare_stage,
    run_deterministic_stage,
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
    prepared.candidate_path.write_bytes(candidate_bytes)
    started = prepared.task_spec.created_at
    result = TaskResultV1(
        task_id=prepared.task_spec.task_id,
        run_id=prepared.task_spec.run_id,
        job_id=prepared.task_spec.job_id,
        stage="parse",
        status="succeeded",
        input_fingerprint=prepared.task_spec.input_fingerprint,
        started_at=started,
        completed_at=max(datetime.now(UTC), started + timedelta(microseconds=1)),
        outputs=(
            ArtifactFingerprint(
                path=prepared.task_spec.candidate_output,
                sha256=sha256_bytes(candidate_bytes),
                size_bytes=len(candidate_bytes),
            ),
        ),
    )
    prepared.result_path.write_text(
        json.dumps(result.model_dump(mode="json"), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return prepared.result_path


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
    assert first.candidate_path.parent.is_dir()
    state = inspect_stage_status(workspace, job_dir)
    assert state.stage.status == "running"
    assert state.pending_task_path == first.task_spec_path


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
    result_path = _write_success_result(prepared, candidate)

    with pytest.raises(StageRuntimeError) as invalid:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=result_path,
        )

    assert invalid.value.code == "stage.invalid_candidate"
    assert not (job_dir / "parsed_job.json").exists()

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

    assert wrong_hash.value.code == "stage.candidate_hash_mismatch"
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
    prepared.candidate_path.write_text("{}", encoding="utf-8")
    result = TaskResultV1(
        task_id=prepared.task_spec.task_id,
        run_id=prepared.task_spec.run_id,
        job_id=prepared.task_spec.job_id,
        stage="parse",
        status="succeeded",
        input_fingerprint=prepared.task_spec.input_fingerprint,
        started_at=prepared.task_spec.created_at,
        completed_at=datetime.now(UTC) + timedelta(microseconds=1),
        outputs=(
            ArtifactFingerprint(
                path=prepared.task_spec.candidate_output,
                sha256=sha256_bytes(b"{}"),
                size_bytes=2,
            ),
        ),
    )
    prepared.result_path.write_text(
        json.dumps(result.model_dump(mode="json")),
        encoding="utf-8",
    )

    with pytest.raises(StageRuntimeError) as captured:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=prepared.result_path,
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


def test_two_prepared_attempts_use_optimistic_output_compare_and_swap(tmp_path: Path) -> None:
    workspace, job_dir = _write_workspace(tmp_path)
    host_task = prepare_stage(workspace, job_dir, stage="parse", execution_mode="host_agent")
    deterministic_task = prepare_stage(
        workspace,
        job_dir,
        stage="parse",
        execution_mode="deterministic",
    )
    candidate = build_deterministic_parse_candidate(job_dir)
    host_result = _write_success_result(host_task, candidate)
    deterministic_result = _write_success_result(deterministic_task, candidate)

    first = apply_stage_result(
        workspace,
        job_dir,
        task_spec_path=host_task.task_spec_path,
        task_result_path=host_result,
    )
    first_hash = sha256_bytes(first.authoritative_path.read_bytes())

    with pytest.raises(StageRuntimeError) as second:
        apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=deterministic_task.task_spec_path,
            task_result_path=deterministic_result,
        )

    assert second.value.code == "stage.output_conflict"
    assert sha256_bytes(first.authoritative_path.read_bytes()) == first_hash


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
