from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import stat
from typing import Any
import unicodedata

from pydantic import ValidationError
import yaml

from canisend.decision_models import (
    EvidenceCatalogItemV1,
    EvidenceCatalogState,
    EvidenceCatalogV1,
    EvidenceSourceReceiptV1,
)
from canisend.resource_files import read_resource_text
from canisend.schema_validation import (
    SchemaCompilationError,
    compiled_schema_validator,
)
from canisend.workspace import load_workspace_config


EVIDENCE_CONTRACT_VERSION = "1.0.0"
EVIDENCE_OUTPUT_PATH = "evidence_catalog.json"
EVIDENCE_SNAPSHOT_NAME = "evidence-snapshot.json"
EVIDENCE_MAX_SOURCE_BYTES_V1 = 4 * 1024 * 1024
EVIDENCE_MAX_TOTAL_BYTES_V1 = 16 * 1024 * 1024
_PROFILE_MANIFEST = "profile.yaml"
_EVIDENCE_ITEM = re.compile(
    r"^-\s+(?:\[([^\]]+)\]\s+)?(?:`([^`]+)`\s*:\s*)?(.*)$"
)
_GENERATED_SOURCE_RECEIPT = re.compile(
    rb"^<!--\s*canisend-source-sha256:\s*([0-9a-f]{64})\s*-->\r?$",
    re.MULTILINE,
)


class EvidenceStageError(ValueError):
    """Raised when profile evidence cannot form a safe semantic projection."""


class EvidenceStageValidationError(EvidenceStageError):
    """Raised when an Evidence candidate cannot be accepted."""


@dataclass(frozen=True)
class _RawEvidenceItem:
    path: str
    section: str
    item_locator: str | None
    kind: str
    text: str


@dataclass(frozen=True)
class _EvidenceSnapshot:
    state: EvidenceCatalogState
    unavailable_reason: str | None
    source_receipts: tuple[EvidenceSourceReceiptV1, ...]
    items: tuple[EvidenceCatalogItemV1, ...]


def stable_evidence_id(*, kind: str, text: str) -> str:
    canonical = json.dumps(
        {
            "kind": canonical_evidence_kind(kind),
            "text": normalized_evidence_text(text),
        },
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return f"evidence_{sha256(canonical).hexdigest()[:32]}"


def evidence_content_sha256(text: str) -> str:
    return sha256(normalized_evidence_text(text).encode("utf-8")).hexdigest()


def canonical_evidence_kind(kind: str) -> str:
    normalized = unicodedata.normalize("NFKC", str(kind)).strip().casefold()
    normalized = re.sub(r"[^a-z0-9]+", "_", normalized).strip("_")
    if not normalized:
        return "statement"
    if normalized[0].isdigit():
        normalized = f"kind_{normalized}"
    return normalized


def normalized_evidence_text(text: str) -> str:
    normalized = unicodedata.normalize("NFKC", str(text))
    return re.sub(r"\s+", " ", normalized).strip().casefold()


def evidence_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    evidence_schema_path: Path | None = None,
) -> dict[str, object]:
    snapshot = _load_evidence_snapshot(workspace)
    schema_text = _evidence_schema_text(evidence_schema_path)
    return _input_projection_from_snapshot(
        snapshot,
        job_id=job_dir.name,
        schema_text=schema_text,
    )


def _input_projection_from_snapshot(
    snapshot: _EvidenceSnapshot,
    *,
    job_id: str,
    schema_text: str,
) -> dict[str, object]:
    return {
        "stage": "evidence",
        "contract_version": EVIDENCE_CONTRACT_VERSION,
        "job_id": job_id,
        "state": snapshot.state,
        "unavailable_reason": snapshot.unavailable_reason,
        "input_limits": {
            "max_source_bytes": EVIDENCE_MAX_SOURCE_BYTES_V1,
            "max_total_bytes": EVIDENCE_MAX_TOTAL_BYTES_V1,
        },
        "source_receipts": tuple(
            receipt.model_dump(mode="json") for receipt in snapshot.source_receipts
        ),
        "schema_sha256": sha256(schema_text.encode("utf-8")).hexdigest(),
    }


def evidence_input_fingerprint(
    workspace: Path,
    job_dir: Path,
    *,
    evidence_schema_path: Path | None = None,
) -> str:
    projection = evidence_input_projection(
        workspace,
        job_dir,
        evidence_schema_path=evidence_schema_path,
    )
    return _projection_sha256(projection)


def build_deterministic_evidence_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    input_fingerprint: str,
    evidence_schema_path: Path | None = None,
) -> EvidenceCatalogV1:
    snapshot = _load_evidence_snapshot(workspace)
    projection = _input_projection_from_snapshot(
        snapshot,
        job_id=job_dir.name,
        schema_text=_evidence_schema_text(evidence_schema_path),
    )
    current_fingerprint = _projection_sha256(projection)
    if input_fingerprint != current_fingerprint:
        raise EvidenceStageError("Evidence input fingerprint is stale.")
    return _catalog_from_snapshot(
        snapshot,
        job_id=job_dir.name,
        input_fingerprint=input_fingerprint,
    )


def validate_evidence_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    evidence_schema_path: Path | None = None,
) -> EvidenceCatalogV1:
    if not isinstance(candidate, dict):
        raise EvidenceStageValidationError("Evidence candidate must be a JSON object.")
    try:
        validator = compiled_schema_validator(
            _evidence_schema_text(evidence_schema_path)
        )
    except SchemaCompilationError as exc:
        raise EvidenceStageValidationError("The configured Evidence schema is invalid.") from exc
    if list(validator.iter_errors(candidate)):
        raise EvidenceStageValidationError("Evidence candidate failed schema validation.")
    try:
        validated = EvidenceCatalogV1.model_validate(candidate)
    except ValidationError as exc:
        raise EvidenceStageValidationError("Evidence candidate failed semantic validation.") from exc

    snapshot = _load_evidence_snapshot(workspace)
    projection = _input_projection_from_snapshot(
        snapshot,
        job_id=job_dir.name,
        schema_text=_evidence_schema_text(evidence_schema_path),
    )
    current_fingerprint = _projection_sha256(projection)
    if input_fingerprint != current_fingerprint or validated.input_fingerprint != input_fingerprint:
        raise EvidenceStageValidationError("Evidence candidate input fingerprint is stale.")
    expected = _catalog_from_snapshot(
        snapshot,
        job_id=job_dir.name,
        input_fingerprint=input_fingerprint,
    )
    if validated.model_dump(mode="json") != expected.model_dump(mode="json"):
        raise EvidenceStageValidationError(
            "Evidence candidate does not match the current deterministic projection."
        )
    return validated


def _catalog_from_snapshot(
    snapshot: _EvidenceSnapshot,
    *,
    job_id: str,
    input_fingerprint: str,
) -> EvidenceCatalogV1:
    return EvidenceCatalogV1(
        job_id=job_id,
        input_fingerprint=input_fingerprint,
        state=snapshot.state,
        unavailable_reason=snapshot.unavailable_reason,
        source_receipts=snapshot.source_receipts,
        items=snapshot.items,
    )


def _projection_sha256(projection: dict[str, object]) -> str:
    canonical = json.dumps(
        projection,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(canonical).hexdigest()


def _load_evidence_snapshot(workspace: Path) -> _EvidenceSnapshot:
    root = workspace.expanduser().resolve()
    profile_dir = load_workspace_config(root).path("profile_dir")
    _require_workspace_directory(root, profile_dir, allow_missing=True)
    if not profile_dir.exists():
        return _unavailable_snapshot("evidence.profile_missing")

    manifest_path = profile_dir / _PROFILE_MANIFEST
    manifest_receipt: EvidenceSourceReceiptV1 | None = None
    declared_paths: tuple[Path, ...] | None = None
    profile_source_paths: tuple[Path, ...] = ()
    source_bindings: dict[Path, Path] = {}
    if manifest_path.exists() or manifest_path.is_symlink():
        manifest_bytes = _read_safe_workspace_file(root, manifest_path)
        manifest = _load_profile_manifest(manifest_bytes)
        manifest_receipt = _source_receipt(
            root,
            manifest_path,
            source_type="manifest",
            content=manifest_bytes,
            item_count=0,
        )
        declared_paths = _declared_generated_paths(profile_dir, manifest)
        profile_source_paths, source_bindings = _declared_profile_source_bindings(
            profile_dir,
            manifest,
        )

    if declared_paths is None:
        generated_dir = profile_dir / "generated"
        if generated_dir.exists() or generated_dir.is_symlink():
            _require_workspace_directory(root, generated_dir)
            declared_paths = tuple(sorted(generated_dir.glob("*.evidence.md")))
        else:
            declared_paths = ()

    if not declared_paths:
        receipts = (manifest_receipt,) if manifest_receipt is not None else ()
        return _unavailable_snapshot(
            "evidence.generated_missing",
            source_receipts=receipts,
        )

    for path in profile_source_paths:
        _require_safe_workspace_file(root, path, allow_missing=True)
    if any(not path.exists() for path in profile_source_paths):
        receipts = (manifest_receipt,) if manifest_receipt is not None else ()
        return _unavailable_snapshot(
            "evidence.profile_source_missing",
            source_receipts=receipts,
        )

    for path in declared_paths:
        _require_safe_workspace_file(root, path, allow_missing=True)
    if any(not path.exists() for path in declared_paths):
        receipts = (manifest_receipt,) if manifest_receipt is not None else ()
        return _unavailable_snapshot(
            "evidence.generated_missing",
            source_receipts=receipts,
        )

    total_read_bytes = manifest_receipt.size_bytes if manifest_receipt is not None else 0
    profile_source_receipts: list[EvidenceSourceReceiptV1] = []
    profile_source_hashes: dict[Path, str] = {}
    for path in profile_source_paths:
        content = _read_safe_workspace_file(root, path)
        total_read_bytes += len(content)
        if total_read_bytes > EVIDENCE_MAX_TOTAL_BYTES_V1:
            raise EvidenceStageError("Evidence inputs exceed the versioned total size limit.")
        receipt = _source_receipt(
            root,
            path,
            source_type="profile_source",
            content=content,
            item_count=0,
        )
        profile_source_receipts.append(receipt)
        profile_source_hashes[path] = receipt.content_sha256

    raw_items: list[_RawEvidenceItem] = []
    generated_receipts: list[EvidenceSourceReceiptV1] = []
    unavailable_reason: str | None = None
    for path in declared_paths:
        content = _read_safe_workspace_file(root, path)
        total_read_bytes += len(content)
        if total_read_bytes > EVIDENCE_MAX_TOTAL_BYTES_V1:
            raise EvidenceStageError("Evidence inputs exceed the versioned total size limit.")
        items = _parse_generated_evidence(root, path, content)
        raw_items.extend(items)
        bound_source = source_bindings.get(path)
        if bound_source is not None:
            declared_source_hash = _generated_source_sha256(content)
            if declared_source_hash is None:
                unavailable_reason = unavailable_reason or "evidence.source_receipt_missing"
            elif declared_source_hash != profile_source_hashes[bound_source]:
                unavailable_reason = unavailable_reason or "evidence.source_receipt_stale"
        generated_receipts.append(
            _source_receipt(
                root,
                path,
                source_type="generated_evidence",
                content=content,
                item_count=len(items),
            )
        )

    receipts = tuple(
        sorted(
            (
                *generated_receipts,
                *profile_source_receipts,
                *((manifest_receipt,) if manifest_receipt is not None else ()),
            ),
            key=_receipt_order_key,
        )
    )
    _verify_source_receipts(root, receipts)
    if unavailable_reason is not None:
        return _unavailable_snapshot(
            unavailable_reason,
            source_receipts=receipts,
        )
    items = _project_catalog_items(raw_items)
    return _EvidenceSnapshot(
        state="available" if items else "empty",
        unavailable_reason=None,
        source_receipts=receipts,
        items=items,
    )


def _unavailable_snapshot(
    reason: str,
    *,
    source_receipts: tuple[EvidenceSourceReceiptV1, ...] = (),
) -> _EvidenceSnapshot:
    return _EvidenceSnapshot(
        state="unavailable",
        unavailable_reason=reason,
        source_receipts=tuple(sorted(source_receipts, key=_receipt_order_key)),
        items=(),
    )


def _load_profile_manifest(content: bytes) -> dict[str, object]:
    try:
        loaded = yaml.load(content.decode("utf-8"), Loader=_UniqueKeySafeLoader)
    except (UnicodeError, yaml.YAMLError) as exc:
        raise EvidenceStageError("The profile evidence manifest is not valid safe YAML.") from exc
    if loaded is None:
        return {}
    if not isinstance(loaded, dict):
        raise EvidenceStageError("The profile evidence manifest must contain a mapping.")
    for field in ("sources", "generated"):
        value = loaded.get(field)
        if value is not None and not isinstance(value, dict):
            raise EvidenceStageError(f"Profile manifest {field} must contain a mapping.")
        if isinstance(value, dict):
            for key, path in value.items():
                if not isinstance(key, str) or not isinstance(path, str):
                    raise EvidenceStageError(
                        f"Profile manifest {field} entries must be string paths."
                    )
                _safe_manifest_path(path)
    return loaded


def _declared_generated_paths(
    profile_dir: Path,
    manifest: dict[str, object],
) -> tuple[Path, ...] | None:
    sources = manifest.get("sources")
    generated = manifest.get("generated")
    if sources is None and generated is None:
        return None
    source_map = sources if isinstance(sources, dict) else {}
    generated_map = generated if isinstance(generated, dict) else {}
    values: list[str] = []
    for source_key, source_value in source_map.items():
        if not str(source_value).endswith(".typ"):
            continue
        default = f"generated/{source_key}.evidence.md"
        configured = generated_map.get(f"{source_key}_evidence", default)
        values.append(str(configured))
    values.extend(str(value) for value in generated_map.values())
    paths = {
        profile_dir.joinpath(*_safe_manifest_path(value).parts)
        for value in values
    }
    return tuple(sorted(paths))


def _declared_profile_source_bindings(
    profile_dir: Path,
    manifest: dict[str, object],
) -> tuple[tuple[Path, ...], dict[Path, Path]]:
    sources = manifest.get("sources")
    generated = manifest.get("generated")
    source_map = sources if isinstance(sources, dict) else {}
    generated_map = generated if isinstance(generated, dict) else {}
    source_paths: set[Path] = set()
    bindings: dict[Path, Path] = {}
    for source_key, source_value in source_map.items():
        if not str(source_value).endswith(".typ"):
            continue
        source_path = profile_dir.joinpath(
            *_safe_manifest_path(str(source_value)).parts
        )
        generated_value = generated_map.get(
            f"{source_key}_evidence",
            f"generated/{source_key}.evidence.md",
        )
        generated_path = profile_dir.joinpath(
            *_safe_manifest_path(str(generated_value)).parts
        )
        existing = bindings.get(generated_path)
        if existing is not None and existing != source_path:
            raise EvidenceStageError(
                "Profile sources must not share one generated evidence path."
            )
        source_paths.add(source_path)
        bindings[generated_path] = source_path
    return tuple(sorted(source_paths)), bindings


def _generated_source_sha256(content: bytes) -> str | None:
    receipts = _GENERATED_SOURCE_RECEIPT.findall(content)
    if len(receipts) > 1:
        raise EvidenceStageError(
            "Generated evidence contains duplicate source receipts."
        )
    return receipts[0].decode("ascii") if receipts else None


def _safe_manifest_path(value: str) -> PurePosixPath:
    if not value or value == "." or "\\" in value or "\x00" in value:
        raise EvidenceStageError("Profile manifest paths must be normalized relative POSIX paths.")
    posix = PurePosixPath(value)
    windows = PureWindowsPath(value)
    if (
        posix.is_absolute()
        or windows.is_absolute()
        or bool(windows.drive)
        or any(part in {"", ".", ".."} for part in value.split("/"))
    ):
        raise EvidenceStageError("Profile manifest paths must be normalized relative POSIX paths.")
    return posix


def _parse_generated_evidence(
    workspace: Path,
    path: Path,
    content: bytes,
) -> tuple[_RawEvidenceItem, ...]:
    try:
        lines = content.decode("utf-8").splitlines()
    except UnicodeError as exc:
        raise EvidenceStageError("Generated evidence must be valid UTF-8 text.") from exc
    section = "Unsectioned"
    relative_path = path.relative_to(workspace).as_posix()
    items: list[_RawEvidenceItem] = []
    citations: set[tuple[str, str | None]] = set()
    for raw_line in lines:
        line = raw_line.strip()
        if line.startswith("## "):
            section = _display_text(line[3:])
            if not section:
                raise EvidenceStageError("Generated evidence contains an empty section heading.")
            continue
        if not line.startswith("- "):
            continue
        match = _EVIDENCE_ITEM.fullmatch(line)
        if match is None:
            raise EvidenceStageError("Generated evidence contains a malformed evidence item.")
        raw_locator, raw_kind, raw_text = match.groups()
        item_locator = raw_locator.strip() if raw_locator is not None else None
        if item_locator == "":
            item_locator = None
        if item_locator is not None and any(character in item_locator for character in "#/\\"):
            raise EvidenceStageError("Generated evidence contains an unsafe item locator.")
        text = _display_text(raw_text)
        if not text:
            raise EvidenceStageError("Generated evidence contains an empty evidence item.")
        citation_key = (section, item_locator)
        if citation_key in citations:
            raise EvidenceStageError("Generated evidence contains a duplicate item citation.")
        citations.add(citation_key)
        items.append(
            _RawEvidenceItem(
                path=relative_path,
                section=section,
                item_locator=item_locator,
                kind=canonical_evidence_kind(raw_kind or "statement"),
                text=text,
            )
        )
    return tuple(items)


def _project_catalog_items(
    raw_items: list[_RawEvidenceItem],
) -> tuple[EvidenceCatalogItemV1, ...]:
    by_id: dict[str, EvidenceCatalogItemV1] = {}
    for raw in raw_items:
        evidence_id = stable_evidence_id(kind=raw.kind, text=raw.text)
        item = EvidenceCatalogItemV1(
            evidence_id=evidence_id,
            path=raw.path,
            section=raw.section,
            item_locator=raw.item_locator,
            kind=raw.kind,
            text=raw.text,
            content_sha256=evidence_content_sha256(raw.text),
        )
        current = by_id.get(evidence_id)
        if current is None or _item_locator_key(item) < _item_locator_key(current):
            by_id[evidence_id] = item
    return tuple(by_id[evidence_id] for evidence_id in sorted(by_id))


def _item_locator_key(item: EvidenceCatalogItemV1) -> tuple[str, str, str, str]:
    return (item.path, item.section, item.item_locator or "", item.text)


def _display_text(value: str) -> str:
    normalized = unicodedata.normalize("NFKC", value)
    return re.sub(r"\s+", " ", normalized).strip()


def _source_receipt(
    workspace: Path,
    path: Path,
    *,
    source_type: str,
    content: bytes,
    item_count: int,
) -> EvidenceSourceReceiptV1:
    return EvidenceSourceReceiptV1(
        path=path.relative_to(workspace).as_posix(),
        source_type=source_type,
        content_sha256=sha256(content).hexdigest(),
        size_bytes=len(content),
        item_count=item_count,
    )


def _verify_source_receipts(
    workspace: Path,
    receipts: tuple[EvidenceSourceReceiptV1, ...],
) -> None:
    for receipt in receipts:
        content = _read_safe_workspace_file(workspace, workspace / receipt.path)
        if len(content) != receipt.size_bytes or sha256(content).hexdigest() != receipt.content_sha256:
            raise EvidenceStageError(
                "Evidence inputs changed while the catalog snapshot was being assembled."
            )


def _receipt_order_key(receipt: EvidenceSourceReceiptV1) -> tuple[int, str]:
    return (
        {
            "manifest": 0,
            "profile_source": 1,
            "generated_evidence": 2,
        }[receipt.source_type],
        receipt.path,
    )


def _require_workspace_directory(
    workspace: Path,
    path: Path,
    *,
    allow_missing: bool = False,
) -> None:
    _require_inside_workspace(workspace, path)
    _reject_symlink_components(workspace, path)
    if not path.exists():
        if allow_missing:
            return
        raise EvidenceStageError("The configured evidence directory is missing.")
    try:
        mode = path.stat().st_mode
    except OSError as exc:
        raise EvidenceStageError("The configured evidence directory cannot be inspected.") from exc
    if not stat.S_ISDIR(mode):
        raise EvidenceStageError("The configured evidence directory is not a safe directory.")


def _require_safe_workspace_file(
    workspace: Path,
    path: Path,
    *,
    allow_missing: bool = False,
) -> None:
    _require_inside_workspace(workspace, path)
    _reject_symlink_components(workspace, path)
    if not path.exists():
        if allow_missing:
            return
        raise EvidenceStageError("A configured evidence file is missing.")
    try:
        file_stat = path.stat()
    except OSError as exc:
        raise EvidenceStageError("A configured evidence file cannot be inspected.") from exc
    if not stat.S_ISREG(file_stat.st_mode):
        raise EvidenceStageError("A configured evidence path is not a regular file.")
    if file_stat.st_nlink != 1:
        raise EvidenceStageError("Evidence inputs must not use hard-link aliases.")
    if file_stat.st_size > EVIDENCE_MAX_SOURCE_BYTES_V1:
        raise EvidenceStageError("An evidence input exceeds the versioned source size limit.")


def _read_safe_workspace_file(workspace: Path, path: Path) -> bytes:
    descriptor: int | None = None
    try:
        descriptor = _open_safe_workspace_file(workspace, path)
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
            raise EvidenceStageError("Evidence inputs must be unaliased regular files.")
        if before.st_size > EVIDENCE_MAX_SOURCE_BYTES_V1:
            raise EvidenceStageError("An evidence input exceeds the versioned source size limit.")
        chunks: list[bytes] = []
        remaining = EVIDENCE_MAX_SOURCE_BYTES_V1 + 1
        while remaining > 0:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        content = b"".join(chunks)
        if len(content) > EVIDENCE_MAX_SOURCE_BYTES_V1:
            raise EvidenceStageError("An evidence input exceeds the versioned source size limit.")
        after = os.fstat(descriptor)
        if _file_identity(before) != _file_identity(after) or len(content) != after.st_size:
            raise EvidenceStageError("An evidence input changed while it was being read.")
        return content
    except EvidenceStageError:
        raise
    except OSError as exc:
        raise EvidenceStageError("A configured evidence file cannot be read.") from exc
    finally:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass


def _open_safe_workspace_file(workspace: Path, path: Path) -> int:
    """Open through no-follow directory descriptors so parent swaps cannot escape."""

    root = workspace.expanduser().resolve()
    relative = _workspace_relative_path(root, path)
    if not relative.parts:
        raise EvidenceStageError("A configured evidence file path is invalid.")
    if not _supports_descriptor_walk():
        return _open_safe_workspace_file_fallback(root, path)
    no_follow = getattr(os, "O_NOFOLLOW", 0)
    directory_only = getattr(os, "O_DIRECTORY", 0)
    close_on_exec = getattr(os, "O_CLOEXEC", 0)
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
            os.O_RDONLY | no_follow | close_on_exec,
            dir_fd=directory_fd,
        )
        file_stat = os.fstat(descriptor)
        if not stat.S_ISREG(file_stat.st_mode) or file_stat.st_nlink != 1:
            os.close(descriptor)
            descriptor = None
            raise EvidenceStageError("Evidence inputs must be unaliased regular files.")
        if file_stat.st_size > EVIDENCE_MAX_SOURCE_BYTES_V1:
            os.close(descriptor)
            descriptor = None
            raise EvidenceStageError("An evidence input exceeds the versioned source size limit.")
        return descriptor
    except EvidenceStageError:
        raise
    except OSError as exc:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass
        raise EvidenceStageError("A configured evidence file cannot be opened safely.") from exc
    finally:
        if directory_fd is not None:
            try:
                os.close(directory_fd)
            except OSError:
                pass


def _supports_descriptor_walk() -> bool:
    return (
        os.open in getattr(os, "supports_dir_fd", set())
        and hasattr(os, "O_DIRECTORY")
        and hasattr(os, "O_NOFOLLOW")
    )


def _open_safe_workspace_file_fallback(workspace: Path, path: Path) -> int:
    """Use pre/post resolution checks on platforms without descriptor-relative open."""

    _require_safe_workspace_file(workspace, path)
    descriptor: int | None = None
    try:
        before = path.resolve(strict=True)
        before.relative_to(workspace)
        descriptor = os.open(
            before,
            os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOINHERIT", 0),
        )
        file_stat = os.fstat(descriptor)
        after = path.resolve(strict=True)
        if (
            before != after
            or not stat.S_ISREG(file_stat.st_mode)
            or file_stat.st_nlink != 1
            or file_stat.st_size > EVIDENCE_MAX_SOURCE_BYTES_V1
        ):
            os.close(descriptor)
            descriptor = None
            raise EvidenceStageError("A configured evidence file changed or aliased during open.")
        return descriptor
    except EvidenceStageError:
        raise
    except (OSError, ValueError) as exc:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass
        raise EvidenceStageError("A configured evidence file cannot be opened safely.") from exc


def _workspace_relative_path(workspace: Path, path: Path) -> Path:
    candidate = path.expanduser()
    if not candidate.is_absolute():
        candidate = workspace / candidate
    try:
        relative = candidate.relative_to(workspace)
    except ValueError as exc:
        raise EvidenceStageError("Evidence inputs must remain inside the workspace.") from exc
    if any(part in {"", ".", ".."} for part in relative.parts):
        raise EvidenceStageError("Evidence inputs must use normalized workspace paths.")
    return relative


def _file_identity(file_stat: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        file_stat.st_dev,
        file_stat.st_ino,
        file_stat.st_nlink,
        file_stat.st_size,
        file_stat.st_mtime_ns,
        file_stat.st_ctime_ns,
    )


def _require_inside_workspace(workspace: Path, path: Path) -> None:
    root = workspace.expanduser().resolve()
    _workspace_relative_path(root, path)


def _reject_symlink_components(workspace: Path, path: Path) -> None:
    root = workspace.expanduser().resolve()
    candidate = path if path.is_absolute() else root / path
    relative = candidate.relative_to(root)
    current = root
    for part in relative.parts:
        current = current / part
        if current.is_symlink():
            raise EvidenceStageError("Evidence inputs must not contain symlink aliases.")


def _evidence_schema_text(path: Path | None) -> str:
    try:
        return read_resource_text(
            "schemas/evidence-catalog.schema.json",
            local_path=path or Path("schemas/evidence-catalog.schema.json"),
        )
    except (OSError, ValueError) as exc:
        raise EvidenceStageError("The Evidence schema could not be read.") from exc


class _UniqueKeySafeLoader(yaml.SafeLoader):
    pass


def _construct_unique_mapping(
    loader: yaml.SafeLoader,
    node: yaml.MappingNode,
    deep: bool = False,
) -> dict[object, object]:
    loader.flatten_mapping(node)
    mapping: dict[object, object] = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        try:
            duplicate = key in mapping
        except TypeError as exc:
            raise yaml.constructor.ConstructorError(
                "while constructing a mapping",
                node.start_mark,
                "found an unhashable mapping key",
                key_node.start_mark,
            ) from exc
        if duplicate:
            raise yaml.constructor.ConstructorError(
                "while constructing a mapping",
                node.start_mark,
                "found duplicate key",
                key_node.start_mark,
            )
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


_UniqueKeySafeLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
    _construct_unique_mapping,
)
