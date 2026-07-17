from __future__ import annotations

import multiprocessing
import os
from pathlib import Path

import pytest

from canisend.job_coordination import (
    JOB_LOCK_RELATIVE_PATH,
    JobCoordinationBusy,
    JobCoordinationError,
    coordinate_job,
)


def _hold_job_lock(job_dir: str, acquired: object, release: object) -> None:
    with coordinate_job(Path(job_dir)):
        acquired.set()
        release.wait(timeout=10)


def test_job_coordination_is_reentrant_for_nested_mutations(tmp_path: Path) -> None:
    job_dir = tmp_path / "job"
    job_dir.mkdir()

    with coordinate_job(job_dir) as outer:
        with coordinate_job(job_dir) as inner:
            assert inner == outer

    assert (job_dir / JOB_LOCK_RELATIVE_PATH).is_file()


def test_job_coordination_times_out_while_another_process_owns_lock(
    tmp_path: Path,
) -> None:
    job_dir = tmp_path / "job"
    job_dir.mkdir()
    context = multiprocessing.get_context("spawn")
    acquired = context.Event()
    release = context.Event()
    process = context.Process(
        target=_hold_job_lock,
        args=(str(job_dir), acquired, release),
    )
    process.start()
    try:
        assert acquired.wait(timeout=10)
        with pytest.raises(JobCoordinationBusy) as captured:
            with coordinate_job(job_dir, timeout_seconds=0):
                pytest.fail("a second process must not enter the mutation section")
        assert captured.value.code == "stage.coordination_busy"
    finally:
        release.set()
        process.join(timeout=10)
        if process.is_alive():
            process.terminate()
            process.join(timeout=5)

    assert process.exitcode == 0


def test_job_coordination_rejects_symlinked_lock_file(tmp_path: Path) -> None:
    job_dir = tmp_path / "job"
    workflow_dir = job_dir / "workflow"
    workflow_dir.mkdir(parents=True)
    external = tmp_path / "external.lock"
    external.write_bytes(b"")
    (job_dir / JOB_LOCK_RELATIVE_PATH).symlink_to(external)

    with pytest.raises(JobCoordinationError) as captured:
        with coordinate_job(job_dir):
            pytest.fail("a symlinked lock file must not be trusted")

    assert captured.value.code == "stage.unsafe_job_lock"


@pytest.mark.skipif(os.name == "nt", reason="hard-link semantics differ on Windows")
def test_job_coordination_rejects_hard_linked_lock_file(tmp_path: Path) -> None:
    job_dir = tmp_path / "job"
    workflow_dir = job_dir / "workflow"
    workflow_dir.mkdir(parents=True)
    lock_path = job_dir / JOB_LOCK_RELATIVE_PATH
    lock_path.write_bytes(b"")
    os.link(lock_path, tmp_path / "alias.lock")

    with pytest.raises(JobCoordinationError) as captured:
        with coordinate_job(job_dir):
            pytest.fail("a multiply linked lock file must not be trusted")

    assert captured.value.code == "stage.unsafe_job_lock"
