from __future__ import annotations

import json
import re
from typing import Any

from canisend.llm import LLMProvider


REQUIRED_PARSED_JOB_FIELDS = [
    "title",
    "institution",
    "department",
    "location",
    "deadline",
    "salary",
    "contract_type",
    "role_type",
    "research_fields",
    "teaching_fields",
    "essential_criteria",
    "desirable_criteria",
    "required_documents",
    "application_url",
    "unknown_fields",
    "notes",
]


class ParsedJobValidationError(ValueError):
    pass


def parse_job_advert(advert_text: str, metadata: dict[str, Any]) -> dict[str, Any]:
    return {
        "title": _heading_title(advert_text) or _metadata_value(metadata, "title"),
        "institution": _metadata_value(metadata, "institution"),
        "department": _field(advert_text, "Department") or _metadata_value(metadata, "department"),
        "location": _field(advert_text, "Location") or _metadata_value(metadata, "location"),
        "deadline": _metadata_value(metadata, "deadline"),
        "salary": _field(advert_text, "Salary") or "unknown",
        "contract_type": _field(advert_text, "Contract") or "unknown",
        "role_type": _field(advert_text, "Role type") or "unknown",
        "research_fields": _comma_list(_field(advert_text, "Research fields")),
        "teaching_fields": _comma_list(_field(advert_text, "Teaching fields")),
        "essential_criteria": _criteria(advert_text, "essential"),
        "desirable_criteria": _criteria(advert_text, "desirable"),
        "required_documents": _comma_list(_field(advert_text, "Required documents")),
        "application_url": _metadata_value(metadata, "source_url"),
        "unknown_fields": [],
        "notes": "",
    }


def parse_job_advert_with_provider(
    *,
    advert_text: str,
    metadata: dict[str, Any],
    provider: LLMProvider,
    prompt_text: str,
) -> dict[str, Any]:
    prompt = prompt_text.replace("{job_metadata}", json.dumps(metadata, indent=2, default=str))
    prompt = prompt.replace("{job_advert}", advert_text)
    response = provider.complete(prompt)
    parsed = _loads_llm_json(response.content)
    validate_parsed_job(parsed)
    return parsed


def validate_parsed_job(parsed_job: dict[str, Any]) -> None:
    for field in REQUIRED_PARSED_JOB_FIELDS:
        if field not in parsed_job:
            raise ParsedJobValidationError(f"missing required field: {field}")

    for field in [
        "research_fields",
        "teaching_fields",
        "essential_criteria",
        "desirable_criteria",
        "required_documents",
        "unknown_fields",
    ]:
        if not isinstance(parsed_job[field], list):
            raise ParsedJobValidationError(f"field must be a list: {field}")

    for field in ["essential_criteria", "desirable_criteria"]:
        for index, criterion in enumerate(parsed_job[field]):
            if not isinstance(criterion, dict):
                raise ParsedJobValidationError(f"{field}[{index}] must be an object")
            if "criterion" not in criterion:
                raise ParsedJobValidationError(f"{field}[{index}] missing criterion")
            if "source_text" not in criterion:
                raise ParsedJobValidationError(f"{field}[{index}] missing source_text")


def _loads_llm_json(content: str) -> dict[str, Any]:
    stripped = content.strip()
    if stripped.startswith("```"):
        stripped = _strip_json_fence(stripped)
    try:
        parsed = json.loads(stripped)
    except json.JSONDecodeError as exc:
        raise ParsedJobValidationError(f"LLM parser returned invalid JSON: {exc.msg}") from exc
    if not isinstance(parsed, dict):
        raise ParsedJobValidationError("LLM parser returned JSON that is not an object")
    return parsed


def _strip_json_fence(content: str) -> str:
    lines = content.splitlines()
    if lines and lines[0].startswith("```"):
        lines = lines[1:]
    if lines and lines[-1].startswith("```"):
        lines = lines[:-1]
    return "\n".join(lines).strip()


def _metadata_value(metadata: dict[str, Any], key: str) -> str:
    value = metadata.get(key, "")
    if value is None or value == "":
        return "unknown"
    return str(value)


def _heading_title(text: str) -> str:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("# "):
            return stripped[2:].strip()
    return ""


def _plain_line(line: str) -> str:
    stripped = line.strip()
    stripped = re.sub(r"^#+\s*", "", stripped).strip()
    stripped = re.sub(r"^(?:[-*+]\s+|\d+[\.)]\s+)", "", stripped).strip()
    stripped = re.sub(r"^#+\s*", "", stripped).strip()
    return stripped


def _bullet_text(line: str) -> str:
    stripped = line.strip()
    match = re.match(r"^(?:[-*+]\s+|\d+[\.)]\s+)(.+)$", stripped)
    if not match:
        return ""
    return _plain_line(match.group(1))


def _field(text: str, label: str) -> str:
    prefix = f"{label.lower()}:"
    for line in text.splitlines():
        stripped = _plain_line(line)
        if stripped.lower().startswith(prefix):
            return stripped[len(prefix) :].strip()
    return ""


def _comma_list(value: str) -> list[str]:
    if not value:
        return []
    return [item.strip() for item in value.split(",") if item.strip()]


def _criteria(text: str, section: str) -> list[dict[str, str]]:
    active = False
    criteria: list[dict[str, str]] = []
    section_prefix = f"{section} criteria"

    for line in text.splitlines():
        stripped = _plain_line(line)
        lowered = stripped.lower()
        if lowered.startswith("essential criteria"):
            active = section == "essential"
            continue
        if lowered.startswith("desirable criteria"):
            active = section == "desirable"
            continue
        if lowered.endswith(":") and not lowered.startswith(section_prefix):
            active = False
        bullet = _bullet_text(line)
        if active and bullet:
            criterion = bullet
            criteria.append({"criterion": criterion, "source_text": criterion})

    return criteria
