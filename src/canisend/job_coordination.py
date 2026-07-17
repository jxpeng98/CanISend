from __future__ import annotations

from contextlib import contextmanager
import errno
import os
from pathlib import Path
import stat
from threading import local
import time
from typing import Iterator


DEFAULT_COORDINATION_TIMEOUT_SECONDS = 5.0
DEFAULT_COORDINATION_POLL_SECONDS = 0.02
JOB_LOCK_RELATIVE_PATH = Path("workflow") / "job.lock"


class JobCoordinationError(RuntimeError):
    """A safe local coordination failure for one private job directory."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


class JobCoordinationBusy(JobCoordinationError):
    """Raised when another cooperative process owns the job mutation lock."""


_LOCAL = local()


@contextmanager
def coordinate_job(
    job_dir: Path,
    *,
    timeout_seconds: float = DEFAULT_COORDINATION_TIMEOUT_SECONDS,
    poll_seconds: float = DEFAULT_COORDINATION_POLL_SECONDS,
) -> Iterator[Path]:
    """Serialize cooperative mutations for one job with a crash-released OS lock."""

    if timeout_seconds < 0:
        raise ValueError("coordination timeout must not be negative")
    if poll_seconds <= 0:
        raise ValueError("coordination poll interval must be positive")

    raw_job = Path(job_dir).expanduser()
    if raw_job.is_symlink():
        raise JobCoordinationError(
            "stage.unsafe_job_lock",
            "The selected job directory must not be a symlink.",
        )
    try:
        job = raw_job.resolve(strict=True)
    except (OSError, RuntimeError) as exc:
        raise JobCoordinationError(
            "stage.unsafe_job_lock",
            "The selected job directory cannot be coordinated safely.",
        ) from exc
    if not job.is_dir():
        raise JobCoordinationError(
            "stage.unsafe_job_lock",
            "The selected job path is not a directory.",
        )

    lock_path = job / JOB_LOCK_RELATIVE_PATH
    key = str(lock_path)
    held = _held_locks()
    existing = held.get(key)
    if existing is not None:
        file_descriptor, depth = existing
        held[key] = (file_descriptor, depth + 1)
        try:
            yield lock_path
        finally:
            current_descriptor, current_depth = held[key]
            held[key] = (current_descriptor, current_depth - 1)
        return

    workflow_dir = lock_path.parent
    if workflow_dir.is_symlink():
        raise JobCoordinationError(
            "stage.unsafe_job_lock",
            "The job workflow directory must not be a symlink.",
        )
    try:
        workflow_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    except OSError as exc:
        raise JobCoordinationError(
            "stage.coordination_failed",
            "The job coordination directory could not be created.",
        ) from exc
    if lock_path.is_symlink():
        raise JobCoordinationError(
            "stage.unsafe_job_lock",
            "The job coordination file must not be a symlink.",
        )

    flags = os.O_RDWR | os.O_CREAT
    no_follow = getattr(os, "O_NOFOLLOW", 0)
    close_on_exec = getattr(os, "O_CLOEXEC", 0)
    file_descriptor: int | None = None
    try:
        file_descriptor = os.open(lock_path, flags | no_follow | close_on_exec, 0o600)
        metadata = os.fstat(file_descriptor)
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
            raise JobCoordinationError(
                "stage.unsafe_job_lock",
                "The job coordination file is not one unaliased regular file.",
            )
        if os.name != "nt":
            os.fchmod(file_descriptor, 0o600)
    except JobCoordinationError:
        if file_descriptor is not None:
            os.close(file_descriptor)
        raise
    except OSError as exc:
        if file_descriptor is not None:
            os.close(file_descriptor)
        raise JobCoordinationError(
            "stage.coordination_failed",
            "The job coordination file could not be opened safely.",
        ) from exc

    deadline = time.monotonic() + timeout_seconds
    try:
        while True:
            try:
                _try_lock(file_descriptor)
                break
            except BlockingIOError as exc:
                if time.monotonic() >= deadline:
                    raise JobCoordinationBusy(
                        "stage.coordination_busy",
                        "Another cooperative process is mutating this job.",
                    ) from exc
                time.sleep(min(poll_seconds, max(0.0, deadline - time.monotonic())))
        held[key] = (file_descriptor, 1)
        try:
            yield lock_path
        finally:
            held.pop(key, None)
            _unlock(file_descriptor)
    finally:
        os.close(file_descriptor)


def _held_locks() -> dict[str, tuple[int, int]]:
    held = getattr(_LOCAL, "held", None)
    if held is None:
        held = {}
        _LOCAL.held = held
    return held


def _try_lock(file_descriptor: int) -> None:
    if os.name == "nt":  # pragma: no cover - exercised by Windows CI
        import msvcrt

        if os.fstat(file_descriptor).st_size == 0:
            os.write(file_descriptor, b"\0")
            os.fsync(file_descriptor)
        os.lseek(file_descriptor, 0, os.SEEK_SET)
        try:
            msvcrt.locking(file_descriptor, msvcrt.LK_NBLCK, 1)
        except OSError as exc:
            if exc.errno in {errno.EACCES, errno.EAGAIN, errno.EDEADLK}:
                raise BlockingIOError from exc
            raise
        return

    import fcntl

    try:
        fcntl.flock(file_descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError as exc:
        if exc.errno in {errno.EACCES, errno.EAGAIN}:
            raise BlockingIOError from exc
        raise


def _unlock(file_descriptor: int) -> None:
    if os.name == "nt":  # pragma: no cover - exercised by Windows CI
        import msvcrt

        os.lseek(file_descriptor, 0, os.SEEK_SET)
        msvcrt.locking(file_descriptor, msvcrt.LK_UNLCK, 1)
        return

    import fcntl

    fcntl.flock(file_descriptor, fcntl.LOCK_UN)
