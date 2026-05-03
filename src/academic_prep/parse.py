from __future__ import annotations

from typing import Any


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


def _field(text: str, label: str) -> str:
    prefix = f"{label.lower()}:"
    for line in text.splitlines():
        stripped = line.strip()
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
        stripped = line.strip()
        lowered = stripped.lower()
        if lowered.startswith("essential criteria"):
            active = section == "essential"
            continue
        if lowered.startswith("desirable criteria"):
            active = section == "desirable"
            continue
        if lowered.endswith(":") and not lowered.startswith(section_prefix):
            active = False
        if active and stripped.startswith("- "):
            criterion = stripped[2:].strip()
            criteria.append({"criterion": criterion, "source_text": criterion})

    return criteria
