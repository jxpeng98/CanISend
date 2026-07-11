from __future__ import annotations

from collections.abc import Mapping
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import tempfile
from typing import Any


class StageStoreError(RuntimeError):
    """Raised when a stage record cannot be safely stored or loaded."""


class UnsafeStagePathError(StageStoreError):
    """Raised when a stage path can escape or alias the selected job directory."""


class ImmutableRecordError(StageStoreError):
    """Raised when an immutable record would be replaced with different content."""


def resolve_job_relative_path(job_dir: Path, relative_path: str | Path) -> Path:
    """Resolve a normalized job-relative path while rejecting path escapes."""
    raw_path = str(relative_path)
    if not raw_path or raw_path == "." or "\x00" in raw_path:
        raise UnsafeStagePathError("stage path must be a non-empty job-relative path")
    if "\\" in raw_path:
        raise UnsafeStagePathError("stage path must use POSIX separators")

    posix_path = PurePosixPath(raw_path)
    windows_path = PureWindowsPath(raw_path)
    raw_parts = raw_path.split("/")
    if (
        posix_path.is_absolute()
        or windows_path.is_absolute()
        or bool(windows_path.drive)
        or any(part in {"", ".", ".."} for part in raw_parts)
    ):
        raise UnsafeStagePathError("stage path must be normalized and job-relative")

    try:
        job_root = job_dir.expanduser().resolve()
        resolved = (job_root / Path(*posix_path.parts)).resolve()
        resolved.relative_to(job_root)
    except (OSError, RuntimeError, ValueError) as exc:
        raise UnsafeStagePathError("stage path escapes the selected job directory") from exc
    return resolved


def sha256_bytes(data: bytes) -> str:
    """Return a lowercase SHA-256 digest for bytes."""
    if not isinstance(data, bytes):
        raise TypeError("sha256_bytes requires bytes")
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    """Return a lowercase SHA-256 digest without loading the whole file at once."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as exc:
        raise StageStoreError(f"stage file could not be hashed: {path.name}") from exc
    return digest.hexdigest()


def atomic_write_bytes(path: Path, data: bytes) -> Path:
    """Atomically replace a file with fully flushed bytes from the same directory."""
    if not isinstance(data, bytes):
        raise TypeError("atomic_write_bytes requires bytes")

    target = Path(path)
    if target.is_symlink():
        raise UnsafeStagePathError("stage record target must not be a symlink")
    try:
        target.parent.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        raise StageStoreError(f"atomic write failed for stage record: {target.name}") from exc

    file_descriptor: int | None = None
    temporary: Path | None = None
    try:
        file_descriptor, temporary_name = tempfile.mkstemp(
            dir=target.parent,
            prefix=f".{target.name}.",
            suffix=".tmp",
        )
        temporary = Path(temporary_name)
        with os.fdopen(file_descriptor, "wb") as destination:
            file_descriptor = None
            destination.write(data)
            destination.flush()
            os.fsync(destination.fileno())
        os.replace(temporary, target)
        temporary = None
        _fsync_directory(target.parent)
    except (OSError, ValueError) as exc:
        raise StageStoreError(f"atomic write failed for stage record: {target.name}") from exc
    finally:
        if file_descriptor is not None:
            try:
                os.close(file_descriptor)
            except OSError:
                pass
        if temporary is not None:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
    return target


def atomic_write_json(path: Path, value: Mapping[str, Any]) -> Path:
    """Serialize a JSON object deterministically and atomically replace its target."""
    return atomic_write_bytes(Path(path), _json_object_bytes(value))


def write_immutable_json(path: Path, value: Mapping[str, Any]) -> Path:
    """Create an immutable JSON object, allowing only same-content retries."""
    target = Path(path)
    requested = _json_object_bytes(value)
    if target.is_symlink():
        raise UnsafeStagePathError("immutable stage record must not be a symlink")
    if target.exists():
        _require_same_immutable_content(target, requested)
        return target

    try:
        target.parent.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        raise StageStoreError(
            f"immutable stage record could not be created: {target.name}"
        ) from exc

    file_descriptor: int | None = None
    temporary: Path | None = None
    try:
        file_descriptor, temporary_name = tempfile.mkstemp(
            dir=target.parent,
            prefix=f".{target.name}.",
            suffix=".tmp",
        )
        temporary = Path(temporary_name)
        with os.fdopen(file_descriptor, "wb") as destination:
            file_descriptor = None
            destination.write(requested)
            destination.flush()
            os.fsync(destination.fileno())
        try:
            os.link(temporary, target)
        except FileExistsError:
            _require_same_immutable_content(target, requested)
        _fsync_directory(target.parent)
    except ImmutableRecordError:
        raise
    except (OSError, ValueError) as exc:
        raise StageStoreError(
            f"immutable stage record could not be created: {target.name}"
        ) from exc
    finally:
        if file_descriptor is not None:
            try:
                os.close(file_descriptor)
            except OSError:
                pass
        if temporary is not None:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
    return target


def read_json_object(path: Path) -> dict[str, Any]:
    """Read a JSON record and require an object at the document root."""
    target = Path(path)
    if target.is_symlink():
        raise UnsafeStagePathError("stage record must not be a symlink")
    try:
        serialized = target.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        raise StageStoreError(f"stage record could not be read: {target.name}") from exc
    try:
        loaded = json.loads(serialized, parse_constant=_reject_json_constant)
    except (json.JSONDecodeError, ValueError) as exc:
        raise StageStoreError(f"stage record is not valid JSON: {target.name}") from exc
    if not isinstance(loaded, dict):
        raise StageStoreError(f"stage record must contain a JSON object: {target.name}")
    return loaded


def _json_object_bytes(value: Mapping[str, Any]) -> bytes:
    if not isinstance(value, Mapping):
        raise StageStoreError("stage record must contain a JSON object")
    try:
        serialized = json.dumps(
            dict(value),
            allow_nan=False,
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
        )
    except (TypeError, ValueError) as exc:
        raise StageStoreError("stage record could not be serialized as JSON") from exc
    return f"{serialized}\n".encode("utf-8")


def _require_same_immutable_content(target: Path, requested: bytes) -> None:
    try:
        current = read_json_object(target)
        current_bytes = _json_object_bytes(current)
    except StageStoreError as exc:
        raise ImmutableRecordError(
            f"existing immutable stage record is not a valid JSON object: {target.name}"
        ) from exc
    if current_bytes != requested:
        raise ImmutableRecordError(
            f"immutable stage record already exists with different content: {target.name}"
        )


def _reject_json_constant(value: str) -> None:
    raise ValueError(f"unsupported JSON constant: {value}")


def _fsync_directory(directory: Path) -> None:
    """Best-effort directory sync; opening directories is not portable to Windows."""
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    try:
        file_descriptor = os.open(directory, flags)
    except OSError:
        return
    try:
        try:
            os.fsync(file_descriptor)
        except OSError:
            pass
    finally:
        try:
            os.close(file_descriptor)
        except OSError:
            pass
