from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from shutil import which
from typing import Mapping

import yaml

from canisend import __version__
from canisend.llm import LLMConfig, load_llm_config
from canisend.profile import init_profile
from canisend.resource_files import copy_resource_tree, read_resource_text


WORKSPACE_CONFIG = "canisend.yaml"
DEFAULT_WORKSPACE_CONFIG = {
    "profile_dir": "profile",
    "jobs_dir": "jobs",
    "job_leads_dir": "job_leads",
    "prompt_dir": "prompts",
    "template_dir": "templates",
    "schema_dir": "schemas",
    "agent_skills_dir": "agent-skills",
}
DEPRECATED_WORKSPACE_FILES = ("GEMINI.md",)
DEFAULT_RESOURCE_CHECKS = {
    "prompts/job_parser.md": "prompts/job_parser.md",
    "prompts/profile_evidence_augmenter.md": "prompts/profile_evidence_augmenter.md",
    "templates/typst/cover_letter.typ": "templates/typst/cover_letter.typ",
    "schemas/parsed_job.schema.json": "schemas/parsed_job.schema.json",
    "agent-skills/canisend/SKILL.md": "agent-skills/canisend/SKILL.md",
    "AGENTS.md": "platform-bridges/AGENTS.md",
    "CLAUDE.md": "platform-bridges/CLAUDE.md",
}


@dataclass(frozen=True)
class WorkspaceStatus:
    label: str
    path: str
    ok: bool


@dataclass(frozen=True)
class WorkspaceConfig:
    root: Path
    values: dict[str, str]

    def path(self, key: str, override: Path | None = None) -> Path:
        raw_value = override if override is not None else Path(self.values[key])
        return _resolve_under_workspace(self.root, raw_value)

    def lead_file(self, override: Path | None = None) -> Path:
        if override is not None:
            return _resolve_under_workspace(self.root, override)
        return self.path("job_leads_dir") / "jobs_ac_uk.json"

    def job_dir(self, job: Path) -> Path:
        expanded = job.expanduser()
        if expanded.is_absolute():
            return expanded
        if len(expanded.parts) == 1:
            return self.path("jobs_dir") / expanded
        return self.root / expanded


def load_workspace_config(workspace: Path = Path(".")) -> WorkspaceConfig:
    root = workspace.expanduser().resolve()
    config_path = root / WORKSPACE_CONFIG
    values = dict(DEFAULT_WORKSPACE_CONFIG)
    if config_path.exists():
        loaded = yaml.safe_load(config_path.read_text(encoding="utf-8")) or {}
        for key in DEFAULT_WORKSPACE_CONFIG:
            raw_value = loaded.get(key)
            if raw_value is not None and str(raw_value).strip():
                values[key] = str(raw_value)
    return WorkspaceConfig(root=root, values=values)


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
    copied.extend(copy_resource_tree("platform-bridges", workspace, overwrite=overwrite))
    return copied


def deprecated_workspace_files(workspace: Path) -> list[Path]:
    return [workspace / filename for filename in DEPRECATED_WORKSPACE_FILES if (workspace / filename).exists()]


def prune_deprecated_workspace_files(workspace: Path) -> list[Path]:
    removed: list[Path] = []
    for path in deprecated_workspace_files(workspace):
        path.unlink()
        removed.append(path)
    return removed


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
            "agent-skills/canisend/SKILL.md",
            (workspace / "agent-skills" / "canisend" / "SKILL.md").exists(),
        ),
    ]
    lines = [f"canisend: {__version__}", f"Workspace: {workspace.resolve()}"]
    for status in statuses:
        marker = "ok" if status.ok else "missing"
        lines.append(f"- {status.path}: {marker} ({status.label})")
    lines.append(_llm_status_line(config))
    lines.append(f"- Typst binary: {'found' if which('typst') else 'missing'}")
    lines.append(_evidence_staleness_line(workspace))
    lines.append(_config_validation_line(workspace))
    lines.append(_deprecated_files_line(workspace))
    lines.append(_default_resources_line(workspace))
    return lines


def _evidence_staleness_line(workspace: Path) -> str:
    profile_dir = workspace / "profile"
    manifest_path = profile_dir / "profile.yaml"
    if not manifest_path.exists():
        return "- Evidence staleness: cannot check (profile/profile.yaml missing)"
    try:
        import yaml

        manifest = yaml.safe_load(manifest_path.read_text(encoding="utf-8"))
        sources = manifest.get("sources", {})
        generated = manifest.get("generated", {})
        stale: list[str] = []
        for source_key, source_value in sources.items():
            source_path = profile_dir / source_value
            if not source_path.exists():
                continue
            evidence_path = _generated_evidence_path(profile_dir, source_key, generated)
            if evidence_path.exists() and source_path.stat().st_mtime > evidence_path.stat().st_mtime:
                stale.append(source_key)
        if stale:
            return f"- Evidence staleness: STALE ({', '.join(stale)} source(s) newer than generated evidence)"
        if sources and any(_generated_evidence_path(profile_dir, k, generated).exists() for k in sources):
            return "- Evidence staleness: up to date"
        return "- Evidence staleness: no generated evidence found (run extract-profile-evidence)"
    except Exception:
        return "- Evidence staleness: check failed"


def _generated_evidence_path(profile_dir: Path, source_key: str, generated: object) -> Path:
    default_output = f"generated/{source_key}.evidence.md"
    if isinstance(generated, dict):
        output = generated.get(f"{source_key}_evidence", default_output)
    else:
        output = default_output
    return profile_dir / Path(str(output))


def _config_validation_line(workspace: Path) -> str:
    from canisend.config_schema import validate_workspace_config

    warnings = validate_workspace_config(workspace / WORKSPACE_CONFIG)
    if not warnings:
        return "- Config validation: ok"
    return f"- Config validation: {'; '.join(warnings)}"


def _deprecated_files_line(workspace: Path) -> str:
    deprecated = deprecated_workspace_files(workspace)
    if not deprecated:
        return "- Deprecated files: none"
    names = ", ".join(path.name for path in deprecated)
    return f"- Deprecated files: {names} (run `canisend update-workspace --prune-deprecated`)"


def _default_resources_line(workspace: Path) -> str:
    stale = _stale_default_resources(workspace)
    if not stale:
        return "- Default resources: up to date"
    return f"- Default resources: stale/local edits ({', '.join(stale)})"


def _stale_default_resources(workspace: Path) -> list[str]:
    stale: list[str] = []
    for local_relative, resource_relative in DEFAULT_RESOURCE_CHECKS.items():
        local_path = workspace / local_relative
        if not local_path.exists():
            continue
        if local_path.read_text(encoding="utf-8") != read_resource_text(resource_relative):
            stale.append(local_relative)
    return stale


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
    return yaml.safe_dump(DEFAULT_WORKSPACE_CONFIG, sort_keys=False)


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


def _resolve_under_workspace(root: Path, path: Path) -> Path:
    expanded = path.expanduser()
    if expanded.is_absolute():
        return expanded
    return root / expanded
