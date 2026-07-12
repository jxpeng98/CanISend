from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import stat
import sys
import tempfile
from typing import Iterable, Sequence


REPO_ROOT = Path(__file__).resolve().parents[1]
CANONICAL_MAIN_SKILL = REPO_ROOT / "skills" / "canisend"
COMPATIBILITY_MIRROR = REPO_ROOT / "agent-skills" / "canisend"


class MirrorBoundaryError(RuntimeError):
    """Raised when a mirror operation would cross the repository boundary."""


def _absolute(path: Path) -> Path:
    return Path(os.path.abspath(os.fspath(path)))


def _repository_paths() -> tuple[Path, Path, Path]:
    repo_root = REPO_ROOT.resolve(strict=True)
    canonical = _absolute(CANONICAL_MAIN_SKILL)
    mirror = _absolute(COMPATIBILITY_MIRROR)

    for label, path in (("canonical skill", canonical), ("compatibility mirror", mirror)):
        try:
            relative = path.relative_to(repo_root)
        except ValueError as exc:
            raise MirrorBoundaryError(f"{label} is outside the repository: {path}") from exc

        current = repo_root
        for part in relative.parts:
            current /= part
            if current.exists() or current.is_symlink():
                if current.is_symlink():
                    raise MirrorBoundaryError(f"{label} crosses an unsafe symlink: {current}")

    if canonical == mirror or canonical in mirror.parents or mirror in canonical.parents:
        raise MirrorBoundaryError("canonical skill and compatibility mirror must be separate trees")

    return repo_root, canonical, mirror


def _scan_tree(root: Path) -> dict[Path, tuple[str, bytes | None]]:
    if not root.exists():
        return {}
    if not root.is_dir():
        raise MirrorBoundaryError(f"skill tree is not a directory: {root}")

    entries: dict[Path, tuple[str, bytes | None]] = {}

    def visit(directory: Path, relative_directory: Path) -> None:
        with os.scandir(directory) as children:
            for child in sorted(children, key=lambda entry: entry.name):
                relative = relative_directory / child.name
                path = Path(child.path)
                if child.is_symlink():
                    raise MirrorBoundaryError(f"skill tree contains an unsafe symlink: {path}")
                if child.is_dir(follow_symlinks=False):
                    entries[relative] = ("directory", None)
                    visit(path, relative)
                    continue
                if child.is_file(follow_symlinks=False):
                    entries[relative] = ("file", path.read_bytes())
                    continue
                raise MirrorBoundaryError(f"skill tree contains an unsupported entry: {path}")

    visit(root, Path())
    return entries


def _format_paths(paths: Iterable[Path]) -> str:
    return ", ".join(path.as_posix() for path in paths)


def check_mirror() -> bool:
    """Return whether the compatibility mirror exactly matches the canonical skill."""
    _, canonical, mirror = _repository_paths()
    if not canonical.is_dir():
        raise MirrorBoundaryError(f"canonical skill is missing: {canonical}")

    canonical_entries = _scan_tree(canonical)
    mirror_entries = _scan_tree(mirror)
    canonical_paths = set(canonical_entries)
    mirror_paths = set(mirror_entries)
    missing = sorted(canonical_paths - mirror_paths)
    extra = sorted(mirror_paths - canonical_paths)
    drifted = sorted(
        path
        for path in canonical_paths & mirror_paths
        if canonical_entries[path] != mirror_entries[path]
    )

    if not (missing or extra or drifted):
        return True

    print("CanISend skill compatibility mirror is out of sync:", file=sys.stderr)
    if missing:
        print(f"  missing: {_format_paths(missing)}", file=sys.stderr)
    if extra:
        print(f"  extra: {_format_paths(extra)}", file=sys.stderr)
    if drifted:
        print(f"  content/type drift: {_format_paths(drifted)}", file=sys.stderr)
    print("Run: python scripts/sync_workspace_skill_mirror.py", file=sys.stderr)
    return False


def _remove_tree_without_following_symlinks(root: Path) -> None:
    mode = root.lstat().st_mode if root.exists() or root.is_symlink() else None
    if mode is None:
        return
    if not stat.S_ISDIR(mode):
        root.unlink()
        return
    with os.scandir(root) as children:
        for child in children:
            path = Path(child.path)
            if child.is_symlink() or child.is_file(follow_symlinks=False):
                path.unlink()
            elif child.is_dir(follow_symlinks=False):
                _remove_tree_without_following_symlinks(path)
            else:
                # FIFOs, sockets, and devices are unlinked as entries; they are never opened.
                path.unlink()
    root.rmdir()


def sync_mirror() -> None:
    """Safely rebuild the compatibility mirror from the canonical skill."""
    _, canonical, mirror = _repository_paths()
    if not canonical.is_dir():
        raise MirrorBoundaryError(f"canonical skill is missing: {canonical}")

    # Validate the whole source before copying so copytree never follows a link.
    _scan_tree(canonical)
    mirror.parent.mkdir(parents=True, exist_ok=True)

    temporary_root = Path(tempfile.mkdtemp(prefix=".canisend-mirror-", dir=mirror.parent))
    staged = temporary_root / "canisend"
    previous = temporary_root / "previous"
    moved_previous = False
    installed_staged = False
    try:
        shutil.copytree(canonical, staged, symlinks=True)
        _scan_tree(staged)

        if mirror.exists():
            os.replace(mirror, previous)
            moved_previous = True
        os.replace(staged, mirror)
        installed_staged = True

        if moved_previous:
            _remove_tree_without_following_symlinks(previous)
            moved_previous = False
    except BaseException:
        if installed_staged and mirror.exists():
            _remove_tree_without_following_symlinks(mirror)
        if moved_previous and previous.exists():
            os.replace(previous, mirror)
        raise
    finally:
        if temporary_root.exists():
            _remove_tree_without_following_symlinks(temporary_root)


def _parse_args(argv: Sequence[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Synchronize or check the canonical CanISend skill compatibility mirror."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="report missing, extra, or drifted mirror entries without changing files",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(argv)
    try:
        if args.check:
            return 0 if check_mirror() else 1
        sync_mirror()
        return 0
    except (MirrorBoundaryError, OSError) as exc:
        print(f"skill mirror operation failed: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
