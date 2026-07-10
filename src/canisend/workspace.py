from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from shutil import which
from typing import Literal, Mapping

import yaml

from canisend import __version__
from canisend.agent_protocol import AgentResponse, ArtifactReference, NextAction, success_response
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


WorkspaceCheckStatus = Literal[
    "ok",
    "missing",
    "configured",
    "unconfigured",
    "unsupported",
    "found",
    "current",
    "stale",
    "not_generated",
    "warning",
    "error",
]


@dataclass(frozen=True)
class WorkspaceCheck:
    id: str
    label: str
    status: WorkspaceCheckStatus
    path: str | None = None
    detail: str | None = None
    items: tuple[str, ...] = ()


@dataclass(frozen=True)
class WorkspaceReport:
    version: str
    root: Path
    checks: tuple[WorkspaceCheck, ...]

    def check(self, check_id: str) -> WorkspaceCheck:
        for check in self.checks:
            if check.id == check_id:
                return check
        raise KeyError(check_id)


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


def workspace_report(workspace: Path, *, env: Mapping[str, str] | None = None) -> WorkspaceReport:
    root = workspace.expanduser().resolve()
    config = load_llm_config(env)
    path_checks = [
        ("workspace_config", "workspace config", WORKSPACE_CONFIG),
        ("profile_manifest", "profile manifest", "profile/profile.yaml"),
        ("jobs_directory", "jobs directory", "jobs"),
        ("rss_leads_directory", "RSS leads directory", "job_leads"),
        ("prompt_defaults", "prompt defaults", "prompts/job_parser.md"),
        ("typst_template_defaults", "Typst template defaults", "templates/typst/cover_letter.typ"),
        ("agent_skill", "agent skill", "agent-skills/canisend/SKILL.md"),
    ]
    checks = [
        WorkspaceCheck(
            id=check_id,
            label=label,
            status="ok" if (root / relative_path).exists() else "missing",
            path=relative_path,
        )
        for check_id, label, relative_path in path_checks
    ]
    checks.extend(
        [
            _llm_status_check(config),
            WorkspaceCheck(
                id="typst_binary",
                label="Typst binary",
                status="found" if which("typst") else "missing",
            ),
            _evidence_staleness_check(root),
            _config_validation_check(root),
            _deprecated_files_check(root),
            _default_resources_check(root),
        ]
    )
    return WorkspaceReport(version=__version__, root=root, checks=tuple(checks))


def doctor_lines(workspace: Path, *, env: Mapping[str, str] | None = None) -> list[str]:
    report = workspace_report(workspace, env=env)
    lines = [f"canisend: {report.version}", f"Workspace: {report.root}"]
    for check in report.checks:
        lines.append(_doctor_check_line(check))
    return lines


def workspace_report_agent_response(
    report: WorkspaceReport,
    *,
    operation: str = "workspace.inspect",
) -> AgentResponse:
    artifact_policy = {
        "workspace_config": (1, "validated", "application/yaml"),
        "profile_manifest": (2, "trusted_local", "application/yaml"),
        "jobs_directory": (2, "trusted_local", "inode/directory"),
        "rss_leads_directory": (1, "untrusted_import", "inode/directory"),
        "prompt_defaults": (0, "trusted_local", "text/markdown"),
        "typst_template_defaults": (0, "trusted_local", "text/plain"),
        "agent_skill": (0, "trusted_local", "text/markdown"),
    }
    artifacts = []
    for check in report.checks:
        if check.path is None or check.id not in artifact_policy:
            continue
        privacy_tier, trust_level, media_type = artifact_policy[check.id]
        artifacts.append(
            ArtifactReference(
                kind=check.id,
                path=check.path,
                exists=check.status == "ok",
                privacy_tier=privacy_tier,
                trust_level=trust_level,
                media_type=media_type,
            )
        )

    missing_fields = [
        check.path
        for check in report.checks
        if check.path is not None and check.status == "missing"
    ]
    warnings = _agent_safe_workspace_warnings(report)
    actions = _workspace_next_actions(report)
    provider = report.check("llm_provider")
    typst = report.check("typst_binary")
    return success_response(
        operation=operation,
        artifacts=artifacts,
        missing_fields=missing_fields,
        warnings=warnings,
        next_actions=actions,
        extensions={
            "canisend.version": report.version,
            "canisend.workspace_initialized": report.check("workspace_config").status == "ok",
            "canisend.provider": provider.detail if provider.detail in {"command", "openai-compatible"} else "unsupported",
            "canisend.provider_configured": provider.status == "configured",
            "canisend.typst_available": typst.status == "found",
            "canisend.diagnostic_check_count": len(report.checks),
        },
    )


def _evidence_staleness_check(workspace: Path) -> WorkspaceCheck:
    profile_dir = workspace / "profile"
    manifest_path = profile_dir / "profile.yaml"
    if not manifest_path.exists():
        return WorkspaceCheck(
            id="evidence_freshness",
            label="Evidence staleness",
            status="missing",
            detail="profile manifest missing",
        )
    try:
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
            return WorkspaceCheck(
                id="evidence_freshness",
                label="Evidence staleness",
                status="stale",
                items=tuple(stale),
            )
        if sources and any(_generated_evidence_path(profile_dir, k, generated).exists() for k in sources):
            return WorkspaceCheck(id="evidence_freshness", label="Evidence staleness", status="current")
        return WorkspaceCheck(id="evidence_freshness", label="Evidence staleness", status="not_generated")
    except Exception:
        return WorkspaceCheck(id="evidence_freshness", label="Evidence staleness", status="error")


def _generated_evidence_path(profile_dir: Path, source_key: str, generated: object) -> Path:
    default_output = f"generated/{source_key}.evidence.md"
    if isinstance(generated, dict):
        output = generated.get(f"{source_key}_evidence", default_output)
    else:
        output = default_output
    return profile_dir / Path(str(output))


def _config_validation_check(workspace: Path) -> WorkspaceCheck:
    from canisend.config_schema import validate_workspace_config

    warnings = validate_workspace_config(workspace / WORKSPACE_CONFIG)
    if not warnings:
        return WorkspaceCheck(id="config_validation", label="Config validation", status="ok")
    return WorkspaceCheck(
        id="config_validation",
        label="Config validation",
        status="warning",
        items=tuple(warnings),
    )


def _deprecated_files_check(workspace: Path) -> WorkspaceCheck:
    deprecated = deprecated_workspace_files(workspace)
    if not deprecated:
        return WorkspaceCheck(id="deprecated_files", label="Deprecated files", status="ok")
    return WorkspaceCheck(
        id="deprecated_files",
        label="Deprecated files",
        status="warning",
        items=tuple(path.name for path in deprecated),
    )


def _default_resources_check(workspace: Path) -> WorkspaceCheck:
    stale = _stale_default_resources(workspace)
    if not stale:
        return WorkspaceCheck(id="default_resources", label="Default resources", status="current")
    return WorkspaceCheck(
        id="default_resources",
        label="Default resources",
        status="stale",
        items=tuple(stale),
    )


def _stale_default_resources(workspace: Path) -> list[str]:
    stale: list[str] = []
    for local_relative, resource_relative in DEFAULT_RESOURCE_CHECKS.items():
        local_path = workspace / local_relative
        if not local_path.exists():
            continue
        if local_path.read_text(encoding="utf-8") != read_resource_text(resource_relative):
            stale.append(local_relative)
    return stale


def _llm_status_check(config: LLMConfig) -> WorkspaceCheck:
    if config.provider == "command":
        status: WorkspaceCheckStatus = "configured" if config.command.strip() else "unconfigured"
        state = "configured" if status == "configured" else "missing command"
    elif config.provider == "openai-compatible":
        status = "configured" if config.openai_api_key and config.openai_model else "unconfigured"
        state = "configured" if status == "configured" else "missing API key or model"
    else:
        status = "unsupported"
        state = "unsupported provider"
    return WorkspaceCheck(
        id="llm_provider",
        label="LLM provider",
        status=status,
        detail=config.provider,
        items=(state,),
    )


def _doctor_check_line(check: WorkspaceCheck) -> str:
    if check.path is not None:
        return f"- {check.path}: {check.status} ({check.label})"
    if check.id == "llm_provider":
        return f"- LLM provider: {check.detail} ({check.items[0]})"
    if check.id == "typst_binary":
        return f"- Typst binary: {check.status}"
    if check.id == "evidence_freshness":
        if check.status == "missing":
            return "- Evidence staleness: cannot check (profile/profile.yaml missing)"
        if check.status == "stale":
            return f"- Evidence staleness: STALE ({', '.join(check.items)} source(s) newer than generated evidence)"
        if check.status == "current":
            return "- Evidence staleness: up to date"
        if check.status == "not_generated":
            return "- Evidence staleness: no generated evidence found (run extract-profile-evidence)"
        return "- Evidence staleness: check failed"
    if check.id == "config_validation":
        return "- Config validation: ok" if check.status == "ok" else f"- Config validation: {'; '.join(check.items)}"
    if check.id == "deprecated_files":
        if check.status == "ok":
            return "- Deprecated files: none"
        return (
            f"- Deprecated files: {', '.join(check.items)} "
            "(run `canisend update-workspace --prune-deprecated`)"
        )
    if check.id == "default_resources":
        if check.status == "current":
            return "- Default resources: up to date"
        return f"- Default resources: stale/local edits ({', '.join(check.items)})"
    raise ValueError(f"unsupported workspace check: {check.id}")


def _agent_safe_workspace_warnings(report: WorkspaceReport) -> list[str]:
    warnings: list[str] = []
    for check in report.checks:
        if check.path is not None and check.status == "missing":
            warnings.append(f"Missing workspace artifact: {check.path}")
        elif check.id == "llm_provider" and check.status != "configured":
            warnings.append("The configured model-provider integration is not ready.")
        elif check.id == "typst_binary" and check.status == "missing":
            warnings.append("The Typst binary is not available.")
        elif check.id == "evidence_freshness" and check.status != "current":
            warnings.append("Profile evidence is not current.")
        elif check.id == "config_validation" and check.status != "ok":
            warnings.append(f"Workspace configuration has {len(check.items)} validation warning(s).")
        elif check.id == "deprecated_files" and check.status != "ok":
            warnings.append("Deprecated workspace bridge files are present.")
        elif check.id == "default_resources" and check.status != "current":
            warnings.append("Packaged workspace defaults differ from local copies.")
    return warnings


def _workspace_next_actions(report: WorkspaceReport) -> list[NextAction]:
    actions: list[NextAction] = []
    if report.check("workspace_config").status == "missing":
        actions.append(NextAction(id="workspace.initialize", label="Initialize the CanISend workspace"))
    if report.check("profile_manifest").status == "missing":
        actions.append(NextAction(id="profile.initialize", label="Initialize the applicant profile"))
    if report.check("evidence_freshness").status in {"stale", "not_generated"}:
        actions.append(NextAction(id="profile.extract_evidence", label="Refresh profile evidence"))
    if report.check("default_resources").status == "stale":
        actions.append(NextAction(id="workspace.update_defaults", label="Review workspace default updates"))
    if report.check("deprecated_files").status == "warning":
        actions.append(NextAction(id="workspace.prune_deprecated", label="Remove deprecated workspace bridges"))
    if report.check("llm_provider").status != "configured":
        actions.append(NextAction(id="provider.configure", label="Configure a model-provider integration"))
    if report.check("typst_binary").status == "missing":
        actions.append(NextAction(id="runtime.install_typst", label="Install Typst for PDF rendering"))
    return actions


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
