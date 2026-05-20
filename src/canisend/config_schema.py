from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml


VALID_CONFIG_KEYS = {
    "profile_dir",
    "jobs_dir",
    "job_leads_dir",
    "prompt_dir",
    "template_dir",
    "schema_dir",
    "agent_skills_dir",
}


def validate_workspace_config(config_path: Path) -> list[str]:
    if not config_path.exists():
        return ["canisend.yaml not found. Run `canisend init-workspace` to create it."]

    try:
        data = yaml.safe_load(config_path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        return [f"canisend.yaml is not valid YAML: {exc}"]

    if not isinstance(data, dict):
        return ["canisend.yaml must contain a mapping of key-value pairs."]

    warnings: list[str] = []
    for key, value in data.items():
        if key not in VALID_CONFIG_KEYS:
            warnings.append(f"Unknown key in canisend.yaml: '{key}'. Valid keys: {', '.join(sorted(VALID_CONFIG_KEYS))}")
        if not isinstance(value, (str, type(None))):
            warnings.append(f"Value for '{key}' must be a string or null, got {type(value).__name__}.")

    for key in VALID_CONFIG_KEYS:
        if key not in data:
            warnings.append(f"Missing key in canisend.yaml: '{key}'. Default will be used: '{_default_for(key)}'.")

    return warnings


def _default_for(key: str) -> str:
    defaults: dict[str, str] = {
        "profile_dir": "profile",
        "jobs_dir": "jobs",
        "job_leads_dir": "job_leads",
        "prompt_dir": "prompts",
        "template_dir": "templates",
        "schema_dir": "schemas",
        "agent_skills_dir": "agent-skills",
    }
    return defaults.get(key, "")
