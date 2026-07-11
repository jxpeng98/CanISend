from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path
import re
from typing import Any, Literal

from jsonschema import Draft202012Validator

from canisend.jobs import load_job_metadata
from canisend.parse import (
    REQUIRED_PARSED_JOB_FIELDS,
    ParsedJobValidationError,
    parse_job_advert,
    validate_parsed_job,
)
from canisend.resource_files import read_resource_text


PARSE_CONTRACT_VERSION = "1.0.0"
ParseExecutorMode = Literal["deterministic", "host_agent"]
PARSE_METADATA_FIELDS = (
    "title",
    "institution",
    "department",
    "location",
    "deadline",
    "source_url",
)


class ParseStageError(ValueError):
    """Raised when Parse inputs cannot produce a safe stage contract."""


class ParseStageValidationError(ParseStageError):
    """Raised when a candidate cannot be promoted as a Parsed Job."""


def parse_input_projection(
    job_dir: Path,
    *,
    executor_mode: ParseExecutorMode = "deterministic",
    schema_path: Path | None = None,
) -> dict[str, object]:
    metadata = load_job_metadata(job_dir)
    advert_path = job_dir / "job_advert.md"
    try:
        advert_bytes = advert_path.read_bytes()
    except OSError as exc:
        raise ParseStageError("The reviewed job advert is not readable.") from exc
    schema_text = _parsed_job_schema_text(schema_path)
    return {
        "stage": "parse",
        "contract_version": PARSE_CONTRACT_VERSION,
        # Executor choice is provenance, not a Parse input. Deterministic and current-host
        # candidates share this output contract and the same freshness boundary.
        "parser_mode": "parsed_job_v1",
        "metadata": {
            field: str(metadata.get(field) or "")
            for field in PARSE_METADATA_FIELDS
        },
        "advert_sha256": sha256(advert_bytes).hexdigest(),
        "schema_sha256": sha256(schema_text.encode("utf-8")).hexdigest(),
    }


def parse_input_fingerprint(
    job_dir: Path,
    *,
    executor_mode: ParseExecutorMode = "deterministic",
    schema_path: Path | None = None,
) -> str:
    projection = parse_input_projection(
        job_dir,
        executor_mode=executor_mode,
        schema_path=schema_path,
    )
    canonical = json.dumps(
        projection,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(canonical).hexdigest()


def build_deterministic_parse_candidate(job_dir: Path) -> dict[str, Any]:
    metadata = load_job_metadata(job_dir)
    try:
        advert_text = (job_dir / "job_advert.md").read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        raise ParseStageError("The reviewed job advert is not readable UTF-8 text.") from exc
    candidate = parse_job_advert(advert_text, metadata)
    return validate_parse_candidate(candidate, advert_text=advert_text)


def validate_parse_candidate(
    candidate: object,
    *,
    advert_text: str,
    schema_path: Path | None = None,
) -> dict[str, Any]:
    if not isinstance(candidate, dict):
        raise ParseStageValidationError("Parse candidate failed schema or semantic validation.")

    schema_text = _parsed_job_schema_text(schema_path)
    try:
        schema = json.loads(schema_text)
    except json.JSONDecodeError as exc:
        raise ParseStageValidationError("The configured Parsed Job schema is invalid.") from exc
    errors = list(Draft202012Validator(schema).iter_errors(candidate))
    try:
        validate_parsed_job(candidate)
    except ParsedJobValidationError as exc:
        raise ParseStageValidationError(
            "Parse candidate failed schema or semantic validation."
        ) from exc
    if errors or set(candidate) != set(REQUIRED_PARSED_JOB_FIELDS):
        raise ParseStageValidationError("Parse candidate failed schema or semantic validation.")

    normalized_advert = _normalized_source_text(advert_text)
    for field in ("essential_criteria", "desirable_criteria"):
        for criterion in candidate[field]:
            source_text = str(criterion.get("source_text") or "")
            normalized_source = _normalized_source_text(source_text)
            if not normalized_source or normalized_source not in normalized_advert:
                raise ParseStageValidationError(
                    "Parse candidate contains a criterion source receipt that does not resolve."
                )

    return json.loads(json.dumps(candidate, ensure_ascii=False))


def _parsed_job_schema_text(schema_path: Path | None) -> str:
    try:
        return read_resource_text(
            "schemas/parsed_job.schema.json",
            local_path=schema_path,
        )
    except (OSError, UnicodeError) as exc:
        raise ParseStageError("The Parsed Job schema is not readable.") from exc


def _normalized_source_text(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip().casefold()
