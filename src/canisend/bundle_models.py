from __future__ import annotations

import base64
import binascii
from pathlib import PurePosixPath, PureWindowsPath
import re
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator


BUNDLE_SCHEMA_VERSION = "1.0.0"
PROJECTION_JOURNAL_SCHEMA_VERSION = "1.0.0"

BundleStage = Literal["package", "render"]
BundleMode = Literal["guarded", "legacy_compatibility"]
ProjectionOutcome = Literal[
    "created",
    "replaced",
    "unchanged",
    "candidate_created",
    "candidate_replaced",
]

_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
PACKAGE_PROJECTION_PATHS = frozenset(
    {
        "00_preparation_questions.md",
        "01_job_summary.md",
        "02_fit_report.md",
        "03_cover_letter_draft.md",
        "04_cv_tailoring_notes.md",
        "05_criteria_checklist.md",
        "06_final_application_package.md",
        "07_material_review_checklist.md",
        "08_research_statement.md",
        "typst/application_package.typ",
        "typst/application_package_content.json",
        "typst/cover_letter.typ",
        "typst/cover_letter_content.json",
        "typst/research_statement.typ",
        "typst/research_statement_content.json",
    }
)


class BundleContractModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )


class BundleEntryV1(BundleContractModel):
    path: str
    media_type: str = Field(min_length=1, max_length=255)
    content_base64: str
    sha256: str
    size_bytes: int = Field(ge=0)

    @field_validator("path")
    @classmethod
    def _safe_path(cls, value: str) -> str:
        return _job_relative_path(value)

    @field_validator("sha256")
    @classmethod
    def _sha256(cls, value: str) -> str:
        if _SHA256_RE.fullmatch(value) is None:
            raise ValueError("sha256 must be a lowercase 64-character digest")
        return value

    @model_validator(mode="after")
    def _content_matches_receipt(self) -> BundleEntryV1:
        from canisend.stage_store import sha256_bytes

        data = self.decoded_bytes()
        if len(data) != self.size_bytes or sha256_bytes(data) != self.sha256:
            raise ValueError("bundle entry bytes do not match their receipt")
        return self

    def decoded_bytes(self) -> bytes:
        try:
            data = base64.b64decode(self.content_base64, validate=True)
        except (binascii.Error, ValueError) as exc:
            raise ValueError("bundle entry content is not canonical base64") from exc
        if base64.b64encode(data).decode("ascii") != self.content_base64:
            raise ValueError("bundle entry content is not canonical base64")
        return data

    @classmethod
    def from_bytes(cls, *, path: str, media_type: str, data: bytes) -> BundleEntryV1:
        from canisend.stage_store import sha256_bytes

        return cls(
            path=path,
            media_type=media_type,
            content_base64=base64.b64encode(data).decode("ascii"),
            sha256=sha256_bytes(data),
            size_bytes=len(data),
        )


class ArtifactBundleV1(BundleContractModel):
    schema_version: Literal["1.0.0"] = BUNDLE_SCHEMA_VERSION
    job_id: str = Field(pattern=r"^[a-z0-9][a-z0-9_.-]*$")
    stage: BundleStage
    mode: BundleMode = "guarded"
    input_fingerprint: str
    entries: tuple[BundleEntryV1, ...] = Field(min_length=1)

    @field_validator("input_fingerprint")
    @classmethod
    def _input_fingerprint(cls, value: str) -> str:
        if _SHA256_RE.fullmatch(value) is None:
            raise ValueError("input_fingerprint must be a lowercase SHA-256 digest")
        return value

    @model_validator(mode="after")
    def _ordered_unique_entries(self) -> ArtifactBundleV1:
        paths = tuple(entry.path for entry in self.entries)
        if paths != tuple(sorted(paths)):
            raise ValueError("bundle entries must be sorted by path")
        if len(paths) != len(set(paths)):
            raise ValueError("bundle entry paths must be unique")
        if self.stage == "render" and any(
            entry.media_type != "application/pdf" or not entry.path.startswith("pdf/")
            for entry in self.entries
        ):
            raise ValueError("Render bundles may contain only pdf/*.pdf entries")
        if self.stage == "package" and any(
            entry.path not in PACKAGE_PROJECTION_PATHS for entry in self.entries
        ):
            raise ValueError("Package bundle entries exceed the static projection scope")
        return self


class ProjectionEntryV1(BundleContractModel):
    source_path: str
    target_path: str
    source_sha256: str
    projected_sha256: str
    outcome: ProjectionOutcome

    @field_validator("source_path", "target_path")
    @classmethod
    def _safe_paths(cls, value: str) -> str:
        return _job_relative_path(value)

    @field_validator("source_sha256", "projected_sha256")
    @classmethod
    def _hashes(cls, value: str) -> str:
        if _SHA256_RE.fullmatch(value) is None:
            raise ValueError("projection hashes must be lowercase SHA-256 digests")
        return value


class ProjectionJournalV1(BundleContractModel):
    schema_version: Literal["1.0.0"] = PROJECTION_JOURNAL_SCHEMA_VERSION
    job_id: str = Field(pattern=r"^[a-z0-9][a-z0-9_.-]*$")
    stage: BundleStage
    bundle_sha256: str
    entries: tuple[ProjectionEntryV1, ...]

    @field_validator("bundle_sha256")
    @classmethod
    def _bundle_hash(cls, value: str) -> str:
        if _SHA256_RE.fullmatch(value) is None:
            raise ValueError("bundle_sha256 must be a lowercase SHA-256 digest")
        return value

    @model_validator(mode="after")
    def _unique_sources(self) -> ProjectionJournalV1:
        sources = tuple(entry.source_path for entry in self.entries)
        if sources != tuple(sorted(sources)) or len(sources) != len(set(sources)):
            raise ValueError("projection entries must have sorted unique sources")
        return self


def _job_relative_path(value: str) -> str:
    if "\\" in value or PureWindowsPath(value).drive:
        raise ValueError("path must use job-relative POSIX syntax")
    path = PurePosixPath(value)
    if path.is_absolute() or value in {"", "."} or any(
        part in {"", ".", ".."} for part in path.parts
    ):
        raise ValueError("path must be a safe job-relative path")
    return path.as_posix()
