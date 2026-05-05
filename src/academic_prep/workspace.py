from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from shutil import which
from typing import Mapping

import yaml

from academic_prep.llm import LLMConfig, load_llm_config
from academic_prep.profile import init_profile
from academic_prep.resource_files import copy_resource_tree


WORKSPACE_CONFIG = "academic-prep.yaml"


@dataclass(frozen=True)
class WorkspaceStatus:
    label: str
    path: str
    ok: bool


def init_workspace(workspace: Path, *, profile_mode: str = "typst", overwrite: bool = False) -> list[Path]:
    workspace.mkdir(parents=True, exist_ok=True)
    created: list[Path] = []

    created.extend(_write_text_if_needed(workspace / WORKSPACE_CONFIG, _workspace_config_text(), overwrite=overwrite))
    created.extend(copy_resource_tree(".env.example", workspace / ".env.example", overwrite=overwrite))
    created.extend(_write_text_if_needed(workspace / ".gitignore", _workspace_gitignore_text(), overwrite=overwrite))
    created.extend(_ensure_marker(workspace / "jobs" / ".gitkeep"))
    created.extend(_ensure_marker(workspace / "job_leads" / ".gitkeep"))
    created.extend(init_profile(workspace / "profile", mode=profile_mode))
    created.extend(update_workspace_defaults(workspace, overwrite=overwrite))
    return created


def update_workspace_defaults(workspace: Path, *, overwrite: bool = False) -> list[Path]:
    workspace.mkdir(parents=True, exist_ok=True)
    copied: list[Path] = []
    copied.extend(copy_resource_tree("prompts", workspace / "prompts", overwrite=overwrite))
    copied.extend(copy_resource_tree("templates", workspace / "templates", overwrite=overwrite))
    copied.extend(copy_resource_tree("schemas", workspace / "schemas", overwrite=overwrite))
    copied.extend(copy_resource_tree("agent-skills", workspace / "agent-skills", overwrite=overwrite))
    return copied


def doctor_lines(workspace: Path, *, env: Mapping[str, str] | None = None) -> list[str]:
    config = load_llm_config(env)
    statuses = [
        WorkspaceStatus("workspace config", WORKSPACE_CONFIG, (workspace / WORKSPACE_CONFIG).exists()),
        WorkspaceStatus("profile manifest", "profile/profile.yaml", (workspace / "profile" / "profile.yaml").exists()),
        WorkspaceStatus("jobs directory", "jobs", (workspace / "jobs").exists()),
        WorkspaceStatus("RSS leads directory", "job_leads", (workspace / "job_leads").exists()),
        WorkspaceStatus("prompt defaults", "prompts/job_parser.md", (workspace / "prompts" / "job_parser.md").exists()),
        WorkspaceStatus(
            "Typst template defaults",
            "templates/typst/cover_letter.typ",
            (workspace / "templates" / "typst" / "cover_letter.typ").exists(),
        ),
        WorkspaceStatus(
            "agent skill",
            "agent-skills/academic-application-prep/SKILL.md",
            (workspace / "agent-skills" / "academic-application-prep" / "SKILL.md").exists(),
        ),
    ]
    lines = [f"Workspace: {workspace.resolve()}"]
    for status in statuses:
        marker = "ok" if status.ok else "missing"
        lines.append(f"- {status.path}: {marker} ({status.label})")
    lines.append(_llm_status_line(config))
    lines.append(f"- Typst binary: {'found' if which('typst') else 'missing'}")
    return lines


def _llm_status_line(config: LLMConfig) -> str:
    if config.provider == "command":
        state = "configured" if config.command.strip() else "missing command"
    elif config.provider == "openai-compatible":
        state = "configured" if config.openai_api_key and config.openai_model else "missing API key or model"
    else:
        state = "unsupported provider"
    return f"- LLM provider: {config.provider} ({state})"


def _write_text_if_needed(path: Path, text: str, *, overwrite: bool) -> list[Path]:
    if path.exists() and not overwrite:
        return []

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return [path]


def _ensure_marker(path: Path) -> list[Path]:
    if path.exists():
        return []

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("", encoding="utf-8")
    return [path]


def _workspace_config_text() -> str:
    return yaml.safe_dump(
        {
            "profile_dir": "profile",
            "jobs_dir": "jobs",
            "job_leads_dir": "job_leads",
            "prompt_dir": "prompts",
            "template_dir": "templates",
            "schema_dir": "schemas",
            "agent_skills_dir": "agent-skills",
        },
        sort_keys=False,
    )


def _workspace_gitignore_text() -> str:
    return """# Private application data
.env
profile/
jobs/
job_leads/
*.pdf

# Local tool output
.DS_Store
__pycache__/
.pytest_cache/
"""
