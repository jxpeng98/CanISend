from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

from canisend.stage_store import (
    ImmutableRecordError,
    StageStoreError,
    UnsafeStagePathError,
    atomic_write_bytes,
    atomic_write_json,
    read_json_object,
    resolve_job_relative_path,
    sha256_bytes,
    sha256_file,
    write_immutable_json,
)


def test_resolve_job_relative_path_accepts_nested_normalized_path(tmp_path: Path) -> None:
    job_dir = tmp_path / "jobs" / "example-role"
    job_dir.mkdir(parents=True)

    resolved = resolve_job_relative_path(job_dir, "workflow/runs/run-1/manifest.json")

    assert resolved == job_dir / "workflow" / "runs" / "run-1" / "manifest.json"


@pytest.mark.parametrize(
    "unsafe_path",
    [
        "",
        ".",
        "../outside.json",
        "workflow/../outside.json",
        "/private/tmp/outside.json",
        "C:/Users/example/outside.json",
        "C:outside.json",
        r"workflow\outside.json",
    ],
)
def test_resolve_job_relative_path_rejects_unsafe_paths(
    tmp_path: Path,
    unsafe_path: str,
) -> None:
    job_dir = tmp_path / "jobs" / "example-role"
    job_dir.mkdir(parents=True)

    with pytest.raises(UnsafeStagePathError):
        resolve_job_relative_path(job_dir, unsafe_path)


@pytest.mark.skipif(os.name == "nt", reason="symlink creation may require elevated privileges")
def test_resolve_job_relative_path_rejects_symlink_escape(tmp_path: Path) -> None:
    job_dir = tmp_path / "jobs" / "example-role"
    outside = tmp_path / "private"
    job_dir.mkdir(parents=True)
    outside.mkdir()
    (job_dir / "linked").symlink_to(outside, target_is_directory=True)

    with pytest.raises(UnsafeStagePathError):
        resolve_job_relative_path(job_dir, "linked/manifest.json")


def test_sha256_helpers_hash_bytes_and_file_consistently(tmp_path: Path) -> None:
    payload = b"stage payload\n"
    path = tmp_path / "payload.bin"
    path.write_bytes(payload)

    assert sha256_bytes(payload) == sha256_file(path)
    assert sha256_bytes(payload) == (
        "de93dcac4ace8cfa2ef923cba2c743b592e6638776aac682575716abd53f2861"
    )


def test_atomic_write_bytes_replaces_target_and_cleans_temp_files(tmp_path: Path) -> None:
    target = tmp_path / "workflow" / "state.json"
    target.parent.mkdir()
    target.write_bytes(b"old")

    written = atomic_write_bytes(target, b"new")

    assert written == target
    assert target.read_bytes() == b"new"
    assert list(target.parent.glob(f".{target.name}.*.tmp")) == []


def test_atomic_write_bytes_preserves_target_when_file_fsync_fails(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    target = tmp_path / "state.json"
    target.write_bytes(b"old")

    def fail_fsync(file_descriptor: int) -> None:
        raise OSError("simulated fsync failure")

    monkeypatch.setattr("canisend.stage_store.os.fsync", fail_fsync)

    with pytest.raises(StageStoreError, match="atomic write failed"):
        atomic_write_bytes(target, b"new")

    assert target.read_bytes() == b"old"
    assert list(tmp_path.glob(f".{target.name}.*.tmp")) == []


def test_atomic_write_bytes_preserves_target_when_replace_fails(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    target = tmp_path / "state.json"
    target.write_bytes(b"old")

    def fail_replace(source: str | Path, destination: str | Path) -> None:
        raise OSError("simulated replace failure")

    monkeypatch.setattr("canisend.stage_store.os.replace", fail_replace)

    with pytest.raises(StageStoreError, match="atomic write failed"):
        atomic_write_bytes(target, b"new")

    assert target.read_bytes() == b"old"
    assert list(tmp_path.glob(f".{target.name}.*.tmp")) == []


def test_atomic_write_json_uses_stable_encoding(tmp_path: Path) -> None:
    target = tmp_path / "nested" / "state.json"

    atomic_write_json(target, {"z": 1, "a": "é"})

    assert target.read_text(encoding="utf-8") == '{\n  "a": "é",\n  "z": 1\n}\n'


def test_write_immutable_json_is_idempotent_for_same_json_content(tmp_path: Path) -> None:
    target = tmp_path / "manifest.json"
    target.write_text('{"stage":"parse", "attempt":1}\n', encoding="utf-8")
    before = target.stat().st_mtime_ns

    written = write_immutable_json(target, {"attempt": 1, "stage": "parse"})

    assert written == target
    assert target.stat().st_mtime_ns == before
    assert json.loads(target.read_text(encoding="utf-8")) == {
        "stage": "parse",
        "attempt": 1,
    }


def test_write_immutable_json_creates_complete_record_and_cleans_temp_files(
    tmp_path: Path,
) -> None:
    target = tmp_path / "runs" / "run-1" / "manifest.json"

    written = write_immutable_json(target, {"stage": "parse", "status": "succeeded"})

    assert written == target
    assert read_json_object(target) == {"stage": "parse", "status": "succeeded"}
    assert list(target.parent.glob(f".{target.name}.*.tmp")) == []


def test_write_immutable_json_rejects_different_or_invalid_existing_record(
    tmp_path: Path,
) -> None:
    target = tmp_path / "manifest.json"
    target.write_text('{"stage":"parse"}\n', encoding="utf-8")

    with pytest.raises(ImmutableRecordError, match="different content"):
        write_immutable_json(target, {"stage": "match"})
    assert json.loads(target.read_text(encoding="utf-8")) == {"stage": "parse"}

    target.write_text("not json\n", encoding="utf-8")
    with pytest.raises(ImmutableRecordError, match="not a valid JSON object"):
        write_immutable_json(target, {"stage": "parse"})
    assert target.read_text(encoding="utf-8") == "not json\n"


def test_read_json_object_accepts_only_json_objects(tmp_path: Path) -> None:
    target = tmp_path / "record.json"
    target.write_text('{"status":"succeeded"}\n', encoding="utf-8")
    assert read_json_object(target) == {"status": "succeeded"}

    target.write_text("[]\n", encoding="utf-8")
    with pytest.raises(StageStoreError, match="must contain a JSON object"):
        read_json_object(target)

    target.write_text("not json\n", encoding="utf-8")
    with pytest.raises(StageStoreError, match="valid JSON"):
        read_json_object(target)

    target.unlink()
    with pytest.raises(StageStoreError, match="could not be read"):
        read_json_object(target)
