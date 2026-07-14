from __future__ import annotations

from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import stat
import time
from typing import Any

import yaml

from canisend.stage_store import (
    StageStoreError,
    UnsafeStagePathError,
    atomic_write_bytes,
    resolve_job_relative_path,
    sha256_bytes,
)


USER_FILE_MAX_BYTES = 1024 * 1024
USER_FILE_MAX_YAML_EVENTS = 20_000
USER_FILE_MAX_YAML_DEPTH = 100
_INTERRUPTED_PUBLICATION_RE = re.compile(
    r"^\.canisend-[1-9][0-9]*-[0-9a-f]{16}\.tmp$"
)


class UserFileStoreError(RuntimeError):
    """A static failure at the user-owned file boundary."""


class UnsafeUserFileError(UserFileStoreError):
    """A user-owned file path or physical file is unsafe."""


class InvalidUserFileError(UserFileStoreError):
    """A user-owned file is not strict bounded YAML or JSON."""


class UserFileConflictError(UserFileStoreError):
    """An exclusive-create target already exists."""


@dataclass(frozen=True)
class SafeFileSnapshot:
    path: Path
    relative_path: str
    data: bytes
    sha256: str
    interrupted_publication: bool = False


def read_safe_bytes(
    job_dir: Path,
    relative_path: str,
    *,
    max_bytes: int = USER_FILE_MAX_BYTES,
    allow_interrupted_publication: bool = False,
) -> SafeFileSnapshot:
    """Read one bounded job file through a no-follow descriptor.

    The opt-in interrupted-publication path accepts only CanISend's private,
    same-directory two-link crash marker; ordinary hard links remain unsafe.
    """

    if max_bytes < 1:
        raise ValueError("max_bytes must be positive")
    try:
        target = resolve_job_relative_path(job_dir, relative_path)
    except (StageStoreError, UnsafeStagePathError) as exc:
        raise UnsafeUserFileError("The user-owned file path is unsafe.") from exc
    try:
        initial = target.lstat()
    except OSError as exc:
        raise UnsafeUserFileError("The user-owned file could not be inspected safely.") from exc
    interrupted_publication = (
        allow_interrupted_publication
        and initial.st_nlink == 2
        and has_interrupted_safe_publication(job_dir, relative_path)
    )
    expected_nlink = 2 if interrupted_publication else 1
    if (
        not stat.S_ISREG(initial.st_mode)
        or initial.st_nlink != expected_nlink
        or initial.st_size > max_bytes
    ):
        raise UnsafeUserFileError("The user-owned file is not a bounded unaliased regular file.")
    descriptor: int | None = None
    try:
        descriptor = _open_safe_job_file(
            job_dir,
            relative_path,
            expected_nlink=expected_nlink,
        )
        before = os.fstat(descriptor)
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_nlink != expected_nlink
            or before.st_size > max_bytes
        ):
            raise UnsafeUserFileError("The user-owned file is not a bounded unaliased regular file.")
        chunks: list[bytes] = []
        remaining = max_bytes + 1
        while remaining > 0:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        data = b"".join(chunks)
        if len(data) > max_bytes:
            raise UnsafeUserFileError("The user-owned file exceeds the size limit.")
        after = os.fstat(descriptor)
        if _file_identity(before) != _file_identity(after) or len(data) != after.st_size:
            raise UnsafeUserFileError("The user-owned file changed while it was read.")
        if interrupted_publication and not has_interrupted_safe_publication(
            job_dir,
            relative_path,
        ):
            raise UnsafeUserFileError("The interrupted private publication changed while it was read.")
        return SafeFileSnapshot(
            path=target,
            relative_path=relative_path,
            data=data,
            sha256=sha256_bytes(data),
            interrupted_publication=interrupted_publication,
        )
    except UnsafeUserFileError:
        raise
    except OSError as exc:
        raise UnsafeUserFileError("The user-owned file could not be read safely.") from exc
    finally:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass


def read_optional_safe_bytes(
    job_dir: Path,
    relative_path: str,
    *,
    max_bytes: int = USER_FILE_MAX_BYTES,
    allow_interrupted_publication: bool = False,
) -> SafeFileSnapshot | None:
    try:
        target = resolve_job_relative_path(job_dir, relative_path)
    except (StageStoreError, UnsafeStagePathError) as exc:
        raise UnsafeUserFileError("The user-owned file path is unsafe.") from exc
    try:
        target.lstat()
    except FileNotFoundError:
        return None
    except OSError as exc:
        raise UnsafeUserFileError("The user-owned file could not be inspected safely.") from exc
    return read_safe_bytes(
        job_dir,
        relative_path,
        max_bytes=max_bytes,
        allow_interrupted_publication=allow_interrupted_publication,
    )


def load_strict_yaml(
    data: bytes,
    *,
    max_bytes: int = USER_FILE_MAX_BYTES,
) -> dict[str, Any]:
    """Load a small YAML mapping without aliases, anchors, merges, or explicit tags."""

    if max_bytes < 1:
        raise ValueError("max_bytes must be positive")
    if len(data) > max_bytes:
        raise InvalidUserFileError("The user-owned YAML exceeds the size limit.")
    try:
        text = data.decode("utf-8")
    except UnicodeError as exc:
        raise InvalidUserFileError("The user-owned YAML is not valid UTF-8.") from exc
    try:
        _validate_yaml_events(text)
        loaded = yaml.load(text, Loader=_StrictSafeLoader)
    except (yaml.YAMLError, ValueError) as exc:
        raise InvalidUserFileError("The user-owned file is not valid strict YAML.") from exc
    if not isinstance(loaded, dict):
        raise InvalidUserFileError("The user-owned YAML must contain a mapping.")
    return loaded


def load_strict_json(
    data: bytes,
    *,
    max_bytes: int = USER_FILE_MAX_BYTES,
) -> dict[str, Any]:
    if max_bytes < 1:
        raise ValueError("max_bytes must be positive")
    if len(data) > max_bytes:
        raise InvalidUserFileError("The structured input exceeds the size limit.")
    try:
        loaded = json.loads(
            data.decode("utf-8"),
            parse_constant=_reject_json_constant,
            object_pairs_hook=_unique_json_object,
        )
    except (UnicodeError, json.JSONDecodeError, ValueError) as exc:
        raise InvalidUserFileError("The structured input is not valid UTF-8 JSON.") from exc
    if not isinstance(loaded, dict):
        raise InvalidUserFileError("The structured input must contain a JSON object.")
    return loaded


def dump_yaml_mapping(value: dict[str, Any]) -> bytes:
    try:
        rendered = yaml.safe_dump(
            value,
            allow_unicode=True,
            default_flow_style=False,
            sort_keys=False,
        )
    except yaml.YAMLError as exc:
        raise InvalidUserFileError("The user-owned YAML could not be serialized.") from exc
    data = rendered.replace("\r\n", "\n").encode("utf-8")
    load_strict_yaml(data)
    return data


def create_safe_file(job_dir: Path, relative_path: str, data: bytes) -> Path:
    repair_interrupted_safe_publication(job_dir, relative_path)
    target = _safe_write_target(job_dir, relative_path, require_existing=False)
    if _supports_descriptor_write():
        _descriptor_write(job_dir, relative_path, data, mode="exclusive")
        _verify_written_file(job_dir, relative_path, data)
        return target
    target = _portable_write_guard(
        job_dir,
        relative_path,
        expected_target=target,
        require_existing=False,
    )
    created = _portable_link_complete_bytes(
        job_dir,
        relative_path,
        target,
        data,
        exclusive=True,
    )
    _verify_written_file(job_dir, relative_path, data)
    return created


def replace_safe_file(job_dir: Path, relative_path: str, data: bytes) -> Path:
    repair_interrupted_safe_publication(job_dir, relative_path)
    target = _safe_write_target(job_dir, relative_path, require_existing=True)
    if _supports_descriptor_write():
        _descriptor_write(job_dir, relative_path, data, mode="replace")
        _verify_written_file(job_dir, relative_path, data)
        return target
    target = _portable_write_guard(
        job_dir,
        relative_path,
        expected_target=target,
        require_existing=True,
    )
    try:
        written = atomic_write_bytes(target, data)
    except StageStoreError as exc:
        raise UserFileStoreError("The user-owned file could not be replaced safely.") from exc
    _verify_written_file(job_dir, relative_path, data)
    return written


def write_safe_immutable_file(job_dir: Path, relative_path: str, data: bytes) -> Path:
    repair_interrupted_safe_publication(job_dir, relative_path)
    target = _safe_write_target(job_dir, relative_path, require_existing=False)
    if _supports_descriptor_write():
        _descriptor_write(job_dir, relative_path, data, mode="immutable")
        _verify_written_file(job_dir, relative_path, data)
        return target
    target = _portable_write_guard(
        job_dir,
        relative_path,
        expected_target=target,
        require_existing=False,
    )
    written = _portable_link_complete_bytes(
        job_dir,
        relative_path,
        target,
        data,
        exclusive=False,
    )
    _verify_written_file(job_dir, relative_path, data)
    return written


def _portable_link_complete_bytes(
    job_dir: Path,
    relative_path: str,
    target: Path,
    data: bytes,
    *,
    exclusive: bool,
) -> Path:
    """Portable no-replace publication using the recoverable temp-name contract."""

    temporary = target.parent / f".canisend-{os.getpid()}-{os.urandom(8).hex()}.tmp"
    descriptor: int | None = None
    temp_exists = False
    try:
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        descriptor = os.open(
            temporary,
            os.O_WRONLY
            | os.O_CREAT
            | os.O_EXCL
            | getattr(os, "O_BINARY", 0)
            | getattr(os, "O_NOINHERIT", 0),
            0o600,
        )
        temp_exists = True
        view = memoryview(data)
        while view:
            written = os.write(descriptor, view)
            if written < 1:
                raise OSError("short write")
            view = view[written:]
        os.fsync(descriptor)
        os.close(descriptor)
        descriptor = None
        try:
            os.link(temporary, target)
        except FileExistsError as exc:
            if exclusive:
                raise UserFileConflictError("The user-owned file already exists.") from exc
            existing = _read_or_repair_portable_publication(
                job_dir,
                relative_path,
                max_bytes=max(USER_FILE_MAX_BYTES, len(data)),
            )
            if existing.data != data:
                raise UserFileConflictError(
                    "The immutable private candidate already has different content."
                ) from exc
        else:
            os.unlink(temporary)
            temp_exists = False
        _fsync_directory_best_effort(target.parent)
        return target
    except (UnsafeUserFileError, UserFileConflictError):
        raise
    except OSError as exc:
        raise UserFileStoreError("The user-owned file could not be stored safely.") from exc
    finally:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass
        if temp_exists:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass


def _read_or_repair_portable_publication(
    job_dir: Path,
    relative_path: str,
    *,
    max_bytes: int,
) -> SafeFileSnapshot:
    for attempt in range(20):
        try:
            return read_safe_bytes(job_dir, relative_path, max_bytes=max_bytes)
        except UnsafeUserFileError:
            if not has_interrupted_safe_publication(job_dir, relative_path):
                raise
        if attempt < 19:
            time.sleep(0.001)
    # The peer did not remove its verified alias during the bounded live-writer
    # window. This write operation may now finish the exact crash marker.
    repair_interrupted_safe_publication(job_dir, relative_path)
    return read_safe_bytes(job_dir, relative_path, max_bytes=max_bytes)


def has_interrupted_safe_publication(job_dir: Path, relative_path: str) -> bool:
    """Recognize one complete CanISend publication left between link and unlink.

    The target must have exactly one private, same-directory sibling with the
    internal temporary-name shape and the same inode.  Because ``st_nlink`` is
    exactly two, accepting this state does not admit an additional external
    hard link.  This function is read-only so status remains non-mutating.
    """

    try:
        target = resolve_job_relative_path(job_dir, relative_path)
    except (StageStoreError, UnsafeStagePathError) as exc:
        raise UnsafeUserFileError("The interrupted publication path is unsafe.") from exc
    try:
        metadata = target.lstat()
    except FileNotFoundError:
        return False
    except OSError as exc:
        raise UnsafeUserFileError(
            "The interrupted publication could not be inspected safely."
        ) from exc
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 2:
        return False
    return _interrupted_alias_path(target, metadata) is not None


def repair_interrupted_safe_publication(job_dir: Path, relative_path: str) -> bool:
    """Remove only a verified internal temporary alias from a completed target."""

    try:
        target = resolve_job_relative_path(job_dir, relative_path)
    except (StageStoreError, UnsafeStagePathError) as exc:
        raise UnsafeUserFileError("The interrupted publication path is unsafe.") from exc
    try:
        metadata = target.lstat()
    except FileNotFoundError:
        return False
    except OSError as exc:
        raise UnsafeUserFileError(
            "The interrupted publication could not be inspected safely."
        ) from exc
    if not stat.S_ISREG(metadata.st_mode):
        raise UnsafeUserFileError("The interrupted publication target is not a regular file.")
    if metadata.st_nlink == 1:
        return False
    if metadata.st_nlink != 2:
        raise UnsafeUserFileError("The interrupted publication has unexpected hard links.")

    if _supports_descriptor_repair():
        return _repair_interrupted_publication_at(job_dir, relative_path)
    alias = _interrupted_alias_path(target, metadata)
    if alias is None:
        raise UnsafeUserFileError("The hard-linked file is not a CanISend interrupted publication.")
    stable = _stable_file_identity(metadata)
    try:
        alias.unlink()
        after = target.lstat()
    except OSError as exc:
        raise UserFileStoreError("The interrupted publication could not be repaired safely.") from exc
    if after.st_nlink != 1 or _stable_file_identity(after) != stable:
        raise UserFileStoreError("The interrupted publication changed during repair.")
    _fsync_directory_best_effort(target.parent)
    return True


def _safe_write_target(
    job_dir: Path,
    relative_path: str,
    *,
    require_existing: bool,
) -> Path:
    try:
        target = resolve_job_relative_path(job_dir, relative_path)
    except (StageStoreError, UnsafeStagePathError) as exc:
        raise UnsafeUserFileError("The user-owned file path is unsafe.") from exc
    if target.is_symlink():
        raise UnsafeUserFileError("The user-owned file must not be a symlink.")
    if target.exists():
        try:
            metadata = target.lstat()
        except OSError as exc:
            raise UnsafeUserFileError("The user-owned file could not be inspected safely.") from exc
        if not target.is_file() or metadata.st_nlink != 1:
            raise UnsafeUserFileError("The user-owned file must be one unaliased regular file.")
    elif require_existing:
        raise UnsafeUserFileError("The user-owned file is missing.")
    return target


def _portable_write_guard(
    job_dir: Path,
    relative_path: str,
    *,
    expected_target: Path,
    require_existing: bool,
) -> Path:
    """Best-effort second path check where descriptor-relative writes are unavailable.

    This closes ordinary check/use gaps such as a parent being replaced between
    the initial inspection and dispatch to the portable atomic primitive. It is
    not a lock against a same-user process that keeps changing directory
    topology; the public local-CAS contract treats that topology as cooperative.
    """

    guarded = _safe_write_target(
        job_dir,
        relative_path,
        require_existing=require_existing,
    )
    if guarded != expected_target:
        raise UnsafeUserFileError("The user-owned file path changed before it was written.")
    return guarded


def _interrupted_alias_path(
    target: Path,
    metadata: os.stat_result,
) -> Path | None:
    if not _publication_is_private(metadata):
        return None
    aliases: list[Path] = []
    try:
        entries = tuple(target.parent.iterdir())
    except OSError as exc:
        raise UnsafeUserFileError(
            "The interrupted publication directory could not be inspected safely."
        ) from exc
    for entry in entries:
        if entry.name == target.name or _INTERRUPTED_PUBLICATION_RE.fullmatch(entry.name) is None:
            continue
        try:
            sibling = entry.lstat()
        except OSError as exc:
            raise UnsafeUserFileError(
                "The interrupted publication alias could not be inspected safely."
            ) from exc
        if (
            stat.S_ISREG(sibling.st_mode)
            and sibling.st_dev == metadata.st_dev
            and sibling.st_ino == metadata.st_ino
            and sibling.st_nlink == 2
            and _publication_is_private(sibling)
        ):
            aliases.append(entry)
    return aliases[0] if len(aliases) == 1 else None


def _repair_interrupted_publication_at(job_dir: Path, relative_path: str) -> bool:
    parent_fd: int | None = None
    target_fd: int | None = None
    try:
        parent_fd, target_name = _open_safe_parent(job_dir, relative_path)
        target_fd = _open_at(parent_fd, target_name)
        before = os.fstat(target_fd)
        if not stat.S_ISREG(before.st_mode):
            raise UnsafeUserFileError(
                "The interrupted publication target is not a regular file."
            )
        if before.st_nlink == 1:
            return False
        if before.st_nlink != 2 or not _publication_is_private(before):
            raise UnsafeUserFileError(
                "The hard-linked file is not a CanISend interrupted publication."
            )
        aliases = _interrupted_alias_names_at(parent_fd, target_name, before)
        if len(aliases) != 1:
            raise UnsafeUserFileError(
                "The hard-linked file is not a unique CanISend interrupted publication."
            )
        stable = _stable_file_identity(before)
        os.unlink(aliases[0], dir_fd=parent_fd)
        after = os.fstat(target_fd)
        if after.st_nlink != 1 or _stable_file_identity(after) != stable:
            raise UserFileStoreError("The interrupted publication changed during repair.")
        try:
            os.fsync(parent_fd)
        except OSError:
            pass
        return True
    except (UnsafeUserFileError, UserFileStoreError):
        raise
    except OSError as exc:
        raise UserFileStoreError("The interrupted publication could not be repaired safely.") from exc
    finally:
        if target_fd is not None:
            try:
                os.close(target_fd)
            except OSError:
                pass
        if parent_fd is not None:
            try:
                os.close(parent_fd)
            except OSError:
                pass


def _interrupted_alias_names_at(
    parent_fd: int,
    target_name: str,
    target: os.stat_result,
) -> tuple[str, ...]:
    aliases: list[str] = []
    for name in os.listdir(parent_fd):
        if name == target_name or _INTERRUPTED_PUBLICATION_RE.fullmatch(name) is None:
            continue
        sibling_fd: int | None = None
        try:
            sibling_fd = _open_at(parent_fd, name)
            sibling = os.fstat(sibling_fd)
        except OSError:
            continue
        finally:
            if sibling_fd is not None:
                try:
                    os.close(sibling_fd)
                except OSError:
                    pass
        if (
            stat.S_ISREG(sibling.st_mode)
            and sibling.st_dev == target.st_dev
            and sibling.st_ino == target.st_ino
            and sibling.st_nlink == 2
            and _publication_is_private(sibling)
        ):
            aliases.append(name)
    return tuple(aliases)


def _publication_is_private(metadata: os.stat_result) -> bool:
    if os.name == "nt":
        return True
    if metadata.st_mode & (stat.S_IRWXG | stat.S_IRWXO):
        return False
    getuid = getattr(os, "getuid", None)
    return getuid is None or metadata.st_uid == getuid()


def _stable_file_identity(metadata: os.stat_result) -> tuple[int, ...]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
        metadata.st_size,
        metadata.st_mtime_ns,
    )


def _fsync_directory_best_effort(directory: Path) -> None:
    descriptor: int | None = None
    try:
        descriptor = os.open(
            directory,
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0),
        )
        os.fsync(descriptor)
    except OSError:
        pass
    finally:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass


def _verify_written_file(job_dir: Path, relative_path: str, expected: bytes) -> None:
    snapshot = read_safe_bytes(job_dir, relative_path, max_bytes=max(USER_FILE_MAX_BYTES, len(expected)))
    if snapshot.data != expected:
        raise UserFileStoreError("The user-owned file does not match the committed bytes.")


def _descriptor_write(
    job_dir: Path,
    relative_path: str,
    data: bytes,
    *,
    mode: str,
) -> None:
    """Write through a no-follow parent descriptor so parent swaps cannot redirect it."""

    parent_fd: int | None = None
    temporary_name = f".canisend-{os.getpid()}-{os.urandom(8).hex()}.tmp"
    temp_fd: int | None = None
    temp_exists = False
    try:
        parent_fd, target_name = _open_safe_parent(job_dir, relative_path)
        if mode == "replace":
            current_fd = _open_at(parent_fd, target_name)
            try:
                metadata = os.fstat(current_fd)
                if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
                    raise UnsafeUserFileError(
                        "The user-owned file must be one unaliased regular file."
                    )
            finally:
                os.close(current_fd)
        temp_fd = os.open(
            temporary_name,
            os.O_WRONLY
            | os.O_CREAT
            | os.O_EXCL
            | getattr(os, "O_CLOEXEC", 0)
            | getattr(os, "O_NOFOLLOW", 0),
            0o600,
            dir_fd=parent_fd,
        )
        temp_exists = True
        view = memoryview(data)
        while view:
            written = os.write(temp_fd, view)
            if written < 1:
                raise OSError("short write")
            view = view[written:]
        os.fsync(temp_fd)
        os.close(temp_fd)
        temp_fd = None
        if mode in {"exclusive", "immutable"}:
            linked = False
            try:
                os.link(
                    temporary_name,
                    target_name,
                    src_dir_fd=parent_fd,
                    dst_dir_fd=parent_fd,
                    follow_symlinks=False,
                )
                linked = True
            except FileExistsError as exc:
                if mode == "exclusive":
                    raise UserFileConflictError("The user-owned file already exists.") from exc
                existing = _read_published_immutable(
                    parent_fd,
                    target_name,
                    max_bytes=max(USER_FILE_MAX_BYTES, len(data)),
                )
                if existing != data:
                    raise UserFileConflictError(
                        "The immutable private candidate already has different content."
                    ) from exc
            if linked:
                os.unlink(temporary_name, dir_fd=parent_fd)
                temp_exists = False
        else:
            os.rename(
                temporary_name,
                target_name,
                src_dir_fd=parent_fd,
                dst_dir_fd=parent_fd,
            )
            temp_exists = False
        try:
            os.fsync(parent_fd)
        except OSError:
            pass
    except (UnsafeUserFileError, UserFileConflictError):
        raise
    except OSError as exc:
        raise UserFileStoreError("The user-owned file could not be stored safely.") from exc
    finally:
        if temp_fd is not None:
            try:
                os.close(temp_fd)
            except OSError:
                pass
        if temp_exists and parent_fd is not None:
            try:
                os.unlink(temporary_name, dir_fd=parent_fd)
            except OSError:
                pass
        if parent_fd is not None:
            try:
                os.close(parent_fd)
            except OSError:
                pass


def _open_safe_parent(job_dir: Path, relative_path: str) -> tuple[int, str]:
    root = job_dir.expanduser().resolve()
    target = resolve_job_relative_path(root, relative_path)
    relative = target.relative_to(root)
    flags = (
        os.O_RDONLY
        | getattr(os, "O_DIRECTORY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_CLOEXEC", 0)
    )
    directory_fd = os.open(root, flags)
    try:
        for component in relative.parts[:-1]:
            try:
                next_fd = os.open(component, flags, dir_fd=directory_fd)
            except FileNotFoundError:
                try:
                    os.mkdir(component, mode=0o700, dir_fd=directory_fd)
                except FileExistsError:
                    pass
                next_fd = os.open(component, flags, dir_fd=directory_fd)
            os.close(directory_fd)
            directory_fd = next_fd
        return directory_fd, relative.parts[-1]
    except BaseException:
        try:
            os.close(directory_fd)
        except OSError:
            pass
        raise


def _open_at(parent_fd: int, name: str) -> int:
    return os.open(
        name,
        os.O_RDONLY
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NONBLOCK", 0),
        dir_fd=parent_fd,
    )


def _read_descriptor_bytes(descriptor: int, *, max_bytes: int) -> bytes:
    metadata = os.fstat(descriptor)
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_size > max_bytes
    ):
        raise UnsafeUserFileError("The user-owned file is not a bounded unaliased regular file.")
    chunks: list[bytes] = []
    remaining = max_bytes + 1
    while remaining > 0:
        chunk = os.read(descriptor, min(1024 * 1024, remaining))
        if not chunk:
            break
        chunks.append(chunk)
        remaining -= len(chunk)
    data = b"".join(chunks)
    after = os.fstat(descriptor)
    if (
        len(data) > max_bytes
        or len(data) != after.st_size
        or _file_identity(metadata) != _file_identity(after)
    ):
        raise UnsafeUserFileError("The user-owned file changed while it was read.")
    return data


def _read_published_immutable(
    parent_fd: int,
    name: str,
    *,
    max_bytes: int,
) -> bytes:
    """Wait briefly for a peer writer to drop its atomic temporary hard link."""

    for attempt in range(20):
        descriptor = _open_at(parent_fd, name)
        try:
            metadata = os.fstat(descriptor)
            if stat.S_ISREG(metadata.st_mode) and metadata.st_nlink == 1:
                return _read_descriptor_bytes(descriptor, max_bytes=max_bytes)
            transient_publication = stat.S_ISREG(metadata.st_mode) and metadata.st_nlink == 2
        finally:
            os.close(descriptor)
        if not transient_publication or attempt == 19:
            raise UnsafeUserFileError(
                "The immutable private candidate is not one unaliased regular file."
            )
        time.sleep(0.001)
    raise AssertionError("unreachable immutable publication retry")


def _supports_descriptor_write() -> bool:
    supports = getattr(os, "supports_dir_fd", set())
    return (
        os.open in supports
        and os.mkdir in supports
        and os.link in supports
        and os.unlink in supports
        and os.rename in supports
        and hasattr(os, "O_DIRECTORY")
        and hasattr(os, "O_NOFOLLOW")
    )


def _supports_descriptor_repair() -> bool:
    return (
        _supports_descriptor_write()
        and os.listdir in getattr(os, "supports_fd", set())
    )


def _open_safe_job_file(
    job_dir: Path,
    relative_path: str,
    *,
    expected_nlink: int = 1,
) -> int:
    root = job_dir.expanduser().resolve()
    target = resolve_job_relative_path(root, relative_path)
    relative = target.relative_to(root)
    if not _supports_descriptor_walk():
        return _open_safe_job_file_fallback(
            root,
            target,
            expected_nlink=expected_nlink,
        )
    no_follow = getattr(os, "O_NOFOLLOW", 0)
    directory_only = getattr(os, "O_DIRECTORY", 0)
    close_on_exec = getattr(os, "O_CLOEXEC", 0)
    nonblocking = getattr(os, "O_NONBLOCK", 0)
    directory_fd: int | None = None
    descriptor: int | None = None
    try:
        directory_fd = os.open(root, os.O_RDONLY | directory_only | close_on_exec)
        for component in relative.parts[:-1]:
            next_fd = os.open(
                component,
                os.O_RDONLY | directory_only | no_follow | close_on_exec,
                dir_fd=directory_fd,
            )
            os.close(directory_fd)
            directory_fd = next_fd
        descriptor = os.open(
            relative.parts[-1],
            os.O_RDONLY | no_follow | close_on_exec | nonblocking,
            dir_fd=directory_fd,
        )
        metadata = os.fstat(descriptor)
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != expected_nlink:
            os.close(descriptor)
            descriptor = None
            raise UnsafeUserFileError("The user-owned file must be one unaliased regular file.")
        return descriptor
    except UnsafeUserFileError:
        raise
    except OSError as exc:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass
        raise UnsafeUserFileError("The user-owned file could not be opened safely.") from exc
    finally:
        if directory_fd is not None:
            try:
                os.close(directory_fd)
            except OSError:
                pass


def _open_safe_job_file_fallback(
    job_dir: Path,
    target: Path,
    *,
    expected_nlink: int = 1,
) -> int:
    descriptor: int | None = None
    try:
        before = target.resolve(strict=True)
        before.relative_to(job_dir)
        descriptor = os.open(
            before,
            os.O_RDONLY
            | getattr(os, "O_BINARY", 0)
            | getattr(os, "O_NOINHERIT", 0)
            | getattr(os, "O_NONBLOCK", 0),
        )
        metadata = os.fstat(descriptor)
        after = target.resolve(strict=True)
        if (
            before != after
            or not stat.S_ISREG(metadata.st_mode)
            or metadata.st_nlink != expected_nlink
        ):
            os.close(descriptor)
            descriptor = None
            raise UnsafeUserFileError("The user-owned file changed or aliased during open.")
        return descriptor
    except UnsafeUserFileError:
        raise
    except (OSError, ValueError) as exc:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass
        raise UnsafeUserFileError("The user-owned file could not be opened safely.") from exc


def _supports_descriptor_walk() -> bool:
    return (
        os.open in getattr(os, "supports_dir_fd", set())
        and hasattr(os, "O_DIRECTORY")
        and hasattr(os, "O_NOFOLLOW")
    )


def _file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_nlink,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


class _StrictSafeLoader(yaml.SafeLoader):
    pass


def _construct_unique_mapping(
    loader: _StrictSafeLoader,
    node: yaml.MappingNode,
    deep: bool = False,
) -> dict[object, object]:
    mapping: dict[object, object] = {}
    for key_node, value_node in node.value:
        if key_node.tag == "tag:yaml.org,2002:merge" or key_node.value == "<<":
            raise yaml.constructor.ConstructorError(
                "while constructing a mapping",
                node.start_mark,
                "merge keys are not supported",
                key_node.start_mark,
            )
        key = loader.construct_object(key_node, deep=deep)
        try:
            duplicate = key in mapping
        except TypeError as exc:
            raise yaml.constructor.ConstructorError(
                "while constructing a mapping",
                node.start_mark,
                "mapping keys must be scalar",
                key_node.start_mark,
            ) from exc
        if duplicate:
            raise yaml.constructor.ConstructorError(
                "while constructing a mapping",
                node.start_mark,
                "duplicate mapping key",
                key_node.start_mark,
            )
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


_StrictSafeLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
    _construct_unique_mapping,
)


def _validate_yaml_events(text: str) -> None:
    count = 0
    depth = 0
    for event in yaml.parse(text, Loader=yaml.SafeLoader):
        count += 1
        if count > USER_FILE_MAX_YAML_EVENTS:
            raise ValueError("too many YAML events")
        if isinstance(event, (yaml.events.MappingStartEvent, yaml.events.SequenceStartEvent)):
            depth += 1
            if depth > USER_FILE_MAX_YAML_DEPTH:
                raise ValueError("YAML nesting is too deep")
        elif isinstance(event, (yaml.events.MappingEndEvent, yaml.events.SequenceEndEvent)):
            depth -= 1
        if isinstance(event, yaml.events.AliasEvent) or getattr(event, "anchor", None) is not None:
            raise ValueError("YAML aliases and anchors are not supported")
        if getattr(event, "tag", None) is not None:
            raise ValueError("explicit YAML tags are not supported")


def _reject_json_constant(value: str) -> None:
    raise ValueError(f"unsupported JSON constant: {value}")


def _unique_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object key")
        result[key] = value
    return result
