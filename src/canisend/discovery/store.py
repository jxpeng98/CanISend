from __future__ import annotations

import json
import os
from pathlib import Path
import tempfile
from typing import Any


class DiscoveryStoreError(ValueError):
    pass


def atomic_write_json(path: Path, value: Any) -> Path:
    """Serialize deterministic JSON and atomically replace one discovery artifact."""

    try:
        serialized = json.dumps(
            value,
            allow_nan=False,
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
        )
    except (TypeError, ValueError) as exc:
        raise DiscoveryStoreError("discovery artifact could not be serialized") from exc

    target = Path(path)
    if target.is_symlink():
        raise DiscoveryStoreError("discovery artifact target must not be a symlink")
    try:
        target.parent.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        raise DiscoveryStoreError("discovery artifact directory could not be created") from exc

    descriptor: int | None = None
    temporary: Path | None = None
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            dir=target.parent,
            prefix=f".{target.name}.",
            suffix=".tmp",
        )
        temporary = Path(temporary_name)
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as destination:
            descriptor = None
            destination.write(serialized)
            destination.write("\n")
            destination.flush()
            os.fsync(destination.fileno())
        os.replace(temporary, target)
        temporary = None
        _fsync_directory(target.parent)
    except OSError as exc:
        raise DiscoveryStoreError("discovery artifact could not be written atomically") from exc
    finally:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass
        if temporary is not None:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
    return target


def _fsync_directory(path: Path) -> None:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError:
        return
    try:
        os.fsync(descriptor)
    except OSError:
        pass
    finally:
        os.close(descriptor)
