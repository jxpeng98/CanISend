import hashlib
import json
from pathlib import Path
import sys
import unicodedata

import typer

from canisend import __version__
from canisend.evidence import EvidenceAugmentationError, extract_profile_evidence
from canisend.examples import run_packaged_example
from canisend.git_tracking import GitTrackingError, git_add_application_materials
from canisend.jobs import create_job, create_job_from_lead, list_jobs as list_job_folders, slugify
from canisend.orchestrator import OrchestrationError, run_orchestration
from canisend.pipeline import run_pipeline as run_job_pipeline
from canisend.profile import init_profile as create_profile
from canisend.ready_check import check_application_package
from canisend.rss import (
    JobFeedError,
    fetch_rss_text,
    filter_job_leads,
    parse_job_feed,
    parse_jobs_ac_uk_rss,
    write_job_leads,
)
from canisend.skill_distribution import export_skill_distribution
from canisend.typst import render_typst_files
from canisend.versioning import fetch_remote_versions, format_version_report
from canisend.workspace import (
    doctor_lines,
    init_workspace as create_workspace,
    load_workspace_config,
    prune_deprecated_workspace_files,
    update_workspace_defaults,
)


APP_HELP = """Prepare evidence-backed academic and professional job application materials from local files.

Common workflow:
  canisend init-workspace --workspace ~/CanISendWorkspace
  canisend doctor --workspace ~/CanISendWorkspace
  canisend init-profile --workspace ~/CanISendWorkspace --mode typst
  canisend extract-profile-evidence --workspace ~/CanISendWorkspace
  canisend new-job --workspace ~/CanISendWorkspace --title ... --institution ...
  canisend run --workspace ~/CanISendWorkspace --job jobs/<job-slug>
  canisend check-package --workspace ~/CanISendWorkspace --job jobs/<job-slug>

Version checks:
  canisend --version
  canisend version
"""

app = typer.Typer(
    help=APP_HELP,
    no_args_is_help=True,
)


def _version_callback(value: bool) -> None:
    if not value:
        return
    _echo_version_report()
    raise typer.Exit()


def _echo_version_report() -> None:
    try:
        remote = fetch_remote_versions()
        error = None
    except Exception as exc:
        remote = None
        error = str(exc)

    for line in format_version_report(local_version=__version__, remote=remote, error=error):
        typer.echo(line)


def _stdin_is_interactive() -> bool:
    return sys.stdin.isatty()


def _should_git_add_materials(value: bool | None) -> bool:
    if value is not None:
        return value
    if not _stdin_is_interactive():
        return False
    return typer.confirm("Add generated application materials to git?", default=False)


def _validate_lead_limit(limit: int) -> None:
    if limit < 0:
        raise typer.BadParameter("--limit must be zero or greater.")


def _validate_feed_input(feed_url: str, rss_file: Path | None) -> None:
    if rss_file is None and not feed_url:
        raise typer.BadParameter("Provide exactly one of --feed-url or --rss-file.")
    if rss_file is not None and feed_url:
        raise typer.BadParameter("Use either --feed-url or --rss-file, not both.")


def _feed_source_slug(source_label: str) -> str:
    if any(unicodedata.category(character).startswith("C") for character in source_label):
        raise typer.BadParameter("--source-name must not contain control characters.")
    if not any(character.isalnum() for character in source_label):
        raise typer.BadParameter("--source-name must contain a letter or number.")
    ascii_slug = slugify(source_label)
    if ascii_slug:
        return ascii_slug
    digest = hashlib.sha256(source_label.encode("utf-8")).hexdigest()[:10]
    return f"source-{digest}"


def _ensure_default_feed_output_matches_source(path: Path, source_label: str) -> None:
    if not path.exists():
        return
    try:
        existing = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise typer.BadParameter(
            f"Default output {path} already exists and cannot be verified; use --output explicitly."
        ) from exc
    if not isinstance(existing, list):
        raise typer.BadParameter(
            f"Default output {path} is not a lead list; use --output explicitly."
        )
    existing_sources = {
        str(item.get("source", "")).strip()
        for item in existing
        if isinstance(item, dict) and str(item.get("source", "")).strip()
    }
    if existing_sources and existing_sources != {source_label}:
        labels = ", ".join(sorted(existing_sources))
        raise typer.BadParameter(
            f"Default output {path} already belongs to source {labels}; use --output explicitly."
        )


@app.callback()
def main(
    version: bool = typer.Option(
        False,
        "--version",
        callback=_version_callback,
        is_eager=True,
        help="Show local, remote stable, and remote prerelease versions, then exit.",
    ),
) -> None:
    """CanISend command-line interface."""


@app.command("version")
def version_command() -> None:
    """Show local, remote stable, and remote prerelease version information."""
    _echo_version_report()


@app.command("init-profile")
def init_profile(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    profile_dir: Path | None = typer.Option(
        None,
        "--profile-dir",
        help="Directory for profile evidence files. Relative paths are resolved against --workspace.",
    ),
    mode: str = typer.Option(
        "hybrid",
        "--mode",
        help="Profile scaffold mode: markdown, typst, or hybrid.",
    ),
) -> None:
    """Create starter profile files."""
    config = load_workspace_config(workspace)
    resolved_profile_dir = config.path("profile_dir", profile_dir)
    try:
        created = create_profile(resolved_profile_dir, mode=mode)
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    typer.echo(f"Profile ready at {resolved_profile_dir}")
    if created:
        typer.echo(f"Created {len(created)} profile files.")
    else:
        typer.echo("No files created; existing profile files were left unchanged.")


@app.command("init-workspace")
def init_workspace(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory for private profile, jobs, local prompts, and agent skills.",
    ),
    profile_mode: str = typer.Option(
        "typst",
        "--profile-mode",
        help="Profile scaffold mode: markdown, typst, or hybrid.",
    ),
    overwrite: bool = typer.Option(
        False,
        "--overwrite",
        help="Overwrite existing default resources and config files.",
    ),
) -> None:
    """Create a productized user workspace without requiring a repository fork."""
    try:
        created = create_workspace(workspace, profile_mode=profile_mode, overwrite=overwrite)
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    typer.echo(f"Workspace ready at {workspace}")
    if created:
        typer.echo(f"Created or updated {len(created)} files.")
    else:
        typer.echo("No files changed; existing workspace files were left unchanged.")


@app.command("update-workspace")
def update_workspace(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory whose default resources should be refreshed.",
    ),
    overwrite: bool = typer.Option(
        False,
        "--overwrite",
        help="Overwrite local default-resource copies. Leave off to preserve local edits.",
    ),
    prune_deprecated: bool = typer.Option(
        False,
        "--prune-deprecated",
        help="Remove deprecated packaged workspace files such as retired platform bridges.",
    ),
) -> None:
    """Copy current packaged prompts, templates, schemas, and agent skills into a workspace."""
    try:
        copied = update_workspace_defaults(workspace, overwrite=overwrite)
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    removed = prune_deprecated_workspace_files(workspace) if prune_deprecated else []
    typer.echo(f"Workspace defaults checked at {workspace}")
    if copied:
        typer.echo(f"Created or updated {len(copied)} files.")
    else:
        typer.echo("No default files changed; existing local files were left unchanged.")
    if prune_deprecated:
        suffix = "file" if len(removed) == 1 else "files"
        typer.echo(f"Removed {len(removed)} deprecated {suffix}.")


@app.command("export-skills")
def export_skills(
    target: Path = typer.Option(
        ...,
        "--target",
        help="Directory to receive the CanISend skill distribution.",
    ),
    kind: str = typer.Option(
        "codex-plugin",
        "--kind",
        help="Export kind: codex-plugin or skills-only.",
    ),
    overwrite: bool = typer.Option(
        False,
        "--overwrite",
        help="Write into a non-empty target directory.",
    ),
) -> None:
    """Export packaged CanISend skills for Codex marketplace or Claude skills usage."""
    try:
        copied = export_skill_distribution(target, kind=kind, overwrite=overwrite)
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    typer.echo(f"Exported CanISend {kind} distribution to {target}")
    typer.echo(f"Wrote {len(copied)} files.")


@app.command("doctor")
def doctor(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory to inspect.",
    ),
) -> None:
    """Report local workspace, provider, and rendering readiness."""
    for line in doctor_lines(workspace):
        typer.echo(line)


@app.command("run-example")
def run_example(
    workspace: Path = typer.Option(
        Path("/tmp/canisend-example"),
        "--workspace",
        help="Directory where the packaged fake-data example workspace should be created.",
    ),
    overwrite: bool = typer.Option(
        False,
        "--overwrite",
        help="Replace an existing non-empty example workspace.",
    ),
) -> None:
    """Run the packaged end-to-end fake-data workflow locally."""
    try:
        result = run_packaged_example(workspace, overwrite=overwrite)
    except ValueError as exc:
        typer.echo(str(exc), err=True)
        raise typer.Exit(code=1) from exc

    typer.echo(f"Example workflow complete at {result.workspace}")
    typer.echo(f"Job: {result.job_dir.relative_to(result.workspace)}")
    typer.echo(f"RSS leads: {result.leads_file.relative_to(result.workspace)}")
    typer.echo("Key outputs:")
    for output in [
        "parsed_job.json",
        "00_preparation_questions.md",
        "02_fit_report.md",
        "03_cover_letter_draft.md",
        "05_criteria_checklist.md",
        "07_material_review_checklist.md",
        "typst/cover_letter_content.json",
        "typst/cover_letter.typ",
        "typst/application_package_content.json",
        "typst/application_package.typ",
    ]:
        typer.echo(f"  - {result.job_dir.relative_to(result.workspace) / output}")
    typer.echo("Next: inspect the generated job folder, then try the same workflow with your private workspace.")


@app.command("check-package")
def check_package(
    job: Path = typer.Option(
        ...,
        "--job",
        help="Job folder path or slug. Slugs are resolved under the workspace jobs directory.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    profile_dir: Path | None = typer.Option(
        None,
        "--profile-dir",
        help="Directory containing generated profile evidence. Relative paths are resolved against --workspace.",
    ),
    write_report: bool = typer.Option(
        False,
        "--write-report",
        help="Write application_gate_report.json to the job directory after checking.",
    ),
) -> None:
    """Check whether a generated application package has unresolved issues."""
    config = load_workspace_config(workspace)
    result = check_application_package(
        job_dir=config.job_dir(job),
        profile_dir=config.path("profile_dir", profile_dir),
    )
    for line in result.output_lines():
        typer.echo(line)
    if write_report:
        try:
            report_path = result.write_report()
        except ValueError as exc:
            typer.echo(str(exc), err=True)
        else:
            typer.echo(f"Application gate report: {report_path}")
    if not result.ok:
        raise typer.Exit(code=1)


@app.command("orchestrate")
def orchestrate(
    job: Path = typer.Option(
        ...,
        "--job",
        help="Job folder path or slug. Slugs are resolved under the workspace jobs directory.",
    ),
    plan: Path = typer.Option(..., "--plan", help="Local orchestration YAML plan."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    dry_run: bool = typer.Option(
        False,
        "--dry-run",
        help="Validate and print task readiness without launching workers.",
    ),
    allow_private_sources: bool = typer.Option(
        False,
        "--allow-private-sources",
        help="Allow Tier 2 private-source tasks declared in the plan.",
    ),
    allow_provider_backed: bool = typer.Option(
        False,
        "--allow-provider-backed",
        help="Allow Tier 3 provider-backed tasks declared in the plan.",
    ),
    allow_profile_input_edits: bool = typer.Option(
        False,
        "--allow-profile-input-edits",
        help="Allow declared profile input edit tasks that can modify original profile source files.",
    ),
    confirm_profile_input_edit: bool = typer.Option(
        False,
        "--confirm-profile-input-edit",
        help="First confirmation that original profile source files may be modified.",
    ),
    confirm_profile_input_edit_again: bool = typer.Option(
        False,
        "--confirm-profile-input-edit-again",
        help="Second confirmation that original profile source files may be modified.",
    ),
    fail_fast: bool = typer.Option(
        False,
        "--fail-fast",
        help="Stop scheduling unrelated tasks after the first failure.",
    ),
) -> None:
    """Coordinate multiple local agent CLI workers for one job."""
    config = load_workspace_config(workspace)
    try:
        result = run_orchestration(
            workspace=config.root,
            job_dir=config.job_dir(job),
            plan_path=plan,
            dry_run=dry_run,
            allow_private_sources=allow_private_sources,
            allow_provider_backed=allow_provider_backed,
            allow_profile_input_edits=allow_profile_input_edits,
            profile_input_edit_confirmations=(
                int(confirm_profile_input_edit) + int(confirm_profile_input_edit_again)
            ),
            fail_fast=fail_fast,
        )
    except OrchestrationError as exc:
        typer.echo(str(exc), err=True)
        raise typer.Exit(code=1) from exc
    if result.run_dir is not None:
        typer.echo(f"Orchestration run: {result.run_dir}")
    for task_id, status in sorted(result.task_statuses.items()):
        typer.echo(f"{task_id}: {status}")
    if not result.ok:
        raise typer.Exit(code=1)


@app.command("extract-profile-evidence")
def extract_profile_evidence_command(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    profile_dir: Path | None = typer.Option(
        None,
        "--profile-dir",
        help="Directory containing profile.yaml and Typst profile sources. Relative paths are resolved against --workspace.",
    ),
    llm_augment: bool = typer.Option(
        False,
        "--llm-augment",
        help="Use the configured LLM provider to supplement locally extracted profile evidence.",
    ),
    prompt_dir: Path | None = typer.Option(
        None,
        "--prompt-dir",
        help="Directory containing prompt files. Relative paths are resolved against --workspace.",
    ),
) -> None:
    """Generate normalized evidence Markdown from local profile sources."""
    config = load_workspace_config(workspace)
    provider = None
    if llm_augment:
        from canisend.llm import load_llm_config, provider_from_config

        provider = provider_from_config(load_llm_config())
        typer.echo("Evidence augmentation: LLM-backed")
    try:
        written = extract_profile_evidence(
            config.path("profile_dir", profile_dir),
            llm_provider=provider,
            prompt_dir=config.path("prompt_dir", prompt_dir),
        )
    except EvidenceAugmentationError as exc:
        typer.echo(str(exc), err=True)
        raise typer.Exit(code=1) from exc
    typer.echo(f"Generated {len(written)} evidence files.")


@app.command("new-job")
def new_job(
    title: str = typer.Option(..., "--title", help="Job title."),
    institution: str = typer.Option(..., "--institution", help="Hiring institution."),
    deadline: str = typer.Option("unknown", "--deadline", help="Application deadline."),
    source_url: str = typer.Option("", "--source-url", help="Original job advert URL."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    jobs_dir: Path | None = typer.Option(
        None,
        "--jobs-dir",
        help="Directory for job folders. Relative paths are resolved against --workspace.",
    ),
    advert_file: Path | None = typer.Option(
        None,
        "--advert-file",
        help="Local .md, .txt, or .pdf job advert file to import.",
    ),
    fetch_url: bool = typer.Option(
        False,
        "--fetch-url",
        help="Explicitly fetch --source-url and import readable HTML or PDF text into job_advert.md.",
    ),
    english_variant: str = typer.Option(
        "",
        "--english-variant",
        help="Preferred English variant for drafted materials: uk, us, or needs_confirmation.",
    ),
    writing_style: str = typer.Option(
        "",
        "--writing-style",
        help="Preferred writing style, e.g. 'direct, warm, evidence-led'.",
    ),
) -> None:
    """Create a local job folder and advert file."""
    config = load_workspace_config(workspace)
    try:
        job_dir = create_job(
            jobs_dir=config.path("jobs_dir", jobs_dir),
            title=title,
            institution=institution,
            deadline=deadline,
            source_url=source_url,
            advert_file=advert_file,
            fetch_url=fetch_url,
            english_variant=english_variant,
            writing_style=writing_style,
        )
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    typer.echo(f"Created job at {job_dir}")


@app.command("new-job-from-lead")
def new_job_from_lead(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    leads_file: Path | None = typer.Option(
        None,
        "--leads-file",
        help="Local lead JSON file created by fetch-jobs-ac-uk or fetch-job-feed. Relative paths are resolved against --workspace.",
    ),
    lead_index: int = typer.Option(..., "--lead-index", help="Zero-based index of the selected lead."),
    institution: str = typer.Option(..., "--institution", help="Hiring institution for the job workspace."),
    deadline: str = typer.Option("unknown", "--deadline", help="Application deadline."),
    title: str | None = typer.Option(None, "--title", help="Override the feed lead title."),
    english_variant: str = typer.Option(
        "",
        "--english-variant",
        help="Preferred English variant for drafted materials: uk, us, or needs_confirmation.",
    ),
    writing_style: str = typer.Option(
        "",
        "--writing-style",
        help="Preferred writing style, e.g. 'direct, warm, evidence-led'.",
    ),
    jobs_dir: Path | None = typer.Option(
        None,
        "--jobs-dir",
        help="Directory for job folders. Relative paths are resolved against --workspace.",
    ),
) -> None:
    """Create a local job folder from a selected RSS or Atom lead without crawling."""
    config = load_workspace_config(workspace)
    try:
        job_dir = create_job_from_lead(
            leads_file=config.lead_file(leads_file),
            lead_index=lead_index,
            jobs_dir=config.path("jobs_dir", jobs_dir),
            institution=institution,
            deadline=deadline,
            title=title,
            english_variant=english_variant,
            writing_style=writing_style,
        )
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    typer.echo(f"Created job from lead at {job_dir}")


@app.command("list-jobs")
def list_jobs_command(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    jobs_dir: Path | None = typer.Option(
        None,
        "--jobs-dir",
        help="Directory containing job folders. Relative paths are resolved against --workspace.",
    ),
) -> None:
    """List all job folders with status, deadline, and institution."""
    config = load_workspace_config(workspace)
    jobs = list_job_folders(config.path("jobs_dir", jobs_dir))
    if not jobs:
        typer.echo("No job folders found.")
        return
    typer.echo(f"{'Deadline':<12} {'Status':<18} {'Institution':<30} {'Title':<40} {'Next action'}")
    typer.echo("-" * 120)
    for job in jobs:
        typer.echo(
            f"{job['deadline']:<12} "
            f"{job['status']:<18} "
            f"{job['institution'][:28]:<30} "
            f"{job['title'][:40]:<40} "
            f"{job['next_action']}"
        )
    typer.echo(f"\n{len(jobs)} job(s) found.")


@app.command("fetch-jobs-ac-uk")
def fetch_jobs_ac_uk(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    feed_url: str = typer.Option("", "--feed-url", help="jobs.ac.uk RSS feed URL."),
    rss_file: Path | None = typer.Option(
        None,
        "--rss-file",
        help="Local RSS XML file for testing or offline import.",
    ),
    output: Path | None = typer.Option(
        None,
        "--output",
        help="JSON output path. Relative paths are resolved against --workspace.",
    ),
    include: list[str] = typer.Option([], "--include", help="Include jobs matching this keyword."),
    exclude: list[str] = typer.Option([], "--exclude", help="Exclude jobs matching this keyword."),
    limit: int = typer.Option(100, "--limit", help="Maximum number of leads to write."),
) -> None:
    """Fetch jobs.ac.uk RSS leads and apply local keyword filters."""
    _validate_feed_input(feed_url, rss_file)
    _validate_lead_limit(limit)

    config = load_workspace_config(workspace)
    output_path = config.lead_file(output)
    try:
        xml_text = rss_file.read_text(encoding="utf-8") if rss_file is not None else fetch_rss_text(feed_url)
        leads = parse_jobs_ac_uk_rss(xml_text, feed_url=feed_url)
    except (JobFeedError, OSError, UnicodeError) as exc:
        raise typer.BadParameter(str(exc)) from exc
    filtered = filter_job_leads(leads, include_keywords=include, exclude_keywords=exclude)
    limited = filtered[:limit]
    write_job_leads(output_path, limited)
    typer.echo(f"Wrote {len(limited)} jobs.ac.uk leads to {output_path}")


@app.command("fetch-job-feed")
def fetch_job_feed(
    source_name: str = typer.Option(..., "--source-name", help="Label for the job feed source."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    feed_url: str = typer.Option("", "--feed-url", help="RSS or Atom feed URL."),
    rss_file: Path | None = typer.Option(
        None,
        "--rss-file",
        help="Local RSS or Atom XML file for testing or offline import.",
    ),
    output: Path | None = typer.Option(
        None,
        "--output",
        help="JSON output path. Relative paths are resolved against --workspace.",
    ),
    include: list[str] = typer.Option([], "--include", help="Include jobs matching this keyword."),
    exclude: list[str] = typer.Option([], "--exclude", help="Exclude jobs matching this keyword."),
    limit: int = typer.Option(100, "--limit", help="Maximum number of leads to write."),
) -> None:
    """Fetch a generic RSS or Atom job feed and apply local keyword filters."""
    source_label = source_name.strip()
    if not source_label:
        raise typer.BadParameter("--source-name must not be empty.")
    source_slug = _feed_source_slug(source_label)
    _validate_feed_input(feed_url, rss_file)
    _validate_lead_limit(limit)

    config = load_workspace_config(workspace)
    default_output = output is None
    output_path = config.lead_file(output) if output is not None else (
        config.path("job_leads_dir") / f"{source_slug}.json"
    )
    if default_output:
        _ensure_default_feed_output_matches_source(output_path, source_label)
    try:
        xml_text = rss_file.read_text(encoding="utf-8") if rss_file is not None else fetch_rss_text(feed_url)
        leads = parse_job_feed(xml_text, feed_url=feed_url, source_name=source_label)
    except (JobFeedError, OSError, UnicodeError) as exc:
        raise typer.BadParameter(str(exc)) from exc
    filtered = filter_job_leads(leads, include_keywords=include, exclude_keywords=exclude)
    limited = filtered[:limit]
    write_job_leads(output_path, limited)
    typer.echo(f"Wrote {len(limited)} {source_label} leads to {output_path}")


@app.command("run")
def run_pipeline(
    job: Path = typer.Option(..., "--job", help="Path to the job folder."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    profile_dir: Path | None = typer.Option(
        None,
        "--profile-dir",
        help="Directory containing generated profile evidence. Relative paths are resolved against --workspace.",
    ),
    llm_parser: bool = typer.Option(
        False,
        "--llm-parser",
        help="Use configured LLM provider and prompts/job_parser.md instead of deterministic parsing.",
    ),
    llm_drafts: bool = typer.Option(
        False,
        "--llm-drafts",
        help="Use configured LLM provider for fit report, cover letter, CV notes, and criteria checklist.",
    ),
    prompt_dir: Path | None = typer.Option(
        None,
        "--prompt-dir",
        help="Directory containing application prompt files. Relative paths are resolved against --workspace.",
    ),
    dry_run: bool = typer.Option(
        False,
        "--dry-run",
        help="Preview what would be generated without writing any files.",
    ),
    git_add_materials: bool | None = typer.Option(
        None,
        "--git-add-materials/--no-git-add-materials",
        help="Ask git to stage generated application materials after a successful run.",
    ),
) -> None:
    """Run the application preparation pipeline for one job."""
    config = load_workspace_config(workspace)
    job_dir = config.job_dir(job)
    if dry_run:
        from canisend.evidence import load_generated_evidence
        from canisend.llm import load_llm_config, provider_from_config
        from canisend.parse import parse_job_advert, parse_job_advert_with_provider
        from canisend.resource_files import read_resource_text

        import yaml as _yaml

        metadata = _yaml.safe_load((job_dir / "job.yaml").read_text(encoding="utf-8"))
        advert_text = (job_dir / "job_advert.md").read_text(encoding="utf-8")
        evidence = load_generated_evidence(config.path("profile_dir", profile_dir))
        if llm_parser:
            prompt_text = read_resource_text("prompts/job_parser.md", local_path=config.path("prompt_dir", prompt_dir) / "job_parser.md")
            provider = provider_from_config(load_llm_config())
            parsed_job = parse_job_advert_with_provider(advert_text=advert_text, metadata=metadata, provider=provider, prompt_text=prompt_text)
            typer.echo("Parser: LLM-backed")
        else:
            parsed_job = parse_job_advert(advert_text, metadata)
            typer.echo("Parser: deterministic")

        typer.echo(f"  Title: {parsed_job['title']}")
        typer.echo(f"  Institution: {parsed_job['institution']}")
        typer.echo(f"  Essential criteria: {len(parsed_job['essential_criteria'])}")
        typer.echo(f"  Desirable criteria: {len(parsed_job['desirable_criteria'])}")
        typer.echo(f"  Required documents: {len(parsed_job['required_documents'])}")
        typer.echo(f"  Evidence items available: {len(evidence)}")
        typer.echo(f"\nOutputs that would be generated:")

        outputs = [
            "parsed_job.json", "00_preparation_questions.md", "01_job_summary.md", "02_fit_report.md",
            "03_cover_letter_draft.md", "04_cv_tailoring_notes.md",
            "05_criteria_checklist.md", "06_final_application_package.md",
            "07_material_review_checklist.md",
            "typst/cover_letter_content.json", "typst/cover_letter.typ",
            "typst/application_package_content.json", "typst/application_package.typ",
        ]
        for output in outputs:
            typer.echo(f"  - {job_dir}/{output}")
        typer.echo(f"\nDraft mode: {'LLM-backed' if llm_drafts else 'deterministic'}")
        return

    written = run_job_pipeline(
        job_dir,
        profile_dir=config.path("profile_dir", profile_dir),
        use_llm_parser=llm_parser,
        use_llm_drafts=llm_drafts,
        prompt_dir=config.path("prompt_dir", prompt_dir),
    )
    typer.echo(f"Generated {len(written)} files for {job_dir}")
    written_candidate_paths = {
        path for path in written if path.name.endswith(".generated.typ")
    }
    candidate_paths = sorted((job_dir / "typst").glob("*.generated.typ"))
    for candidate_path in candidate_paths:
        primary_path = candidate_path.with_name(
            candidate_path.name.removesuffix(".generated.typ") + ".typ"
        )
        if candidate_path in written_candidate_paths:
            message = (
                "WARNING: Preserved edited Typst source "
                f"{primary_path}; wrote the new generated candidate to {candidate_path}. "
                "Replace the primary file with the candidate to adopt it."
            )
        else:
            message = (
                f"WARNING: Pending Typst candidate {candidate_path} still requires reconciliation "
                f"with {primary_path}."
            )
        typer.echo(message, err=True)
    if candidate_paths:
        typer.echo(
            "Skipped git staging because the generated Markdown and preserved Typst sources need reconciliation.",
            err=True,
        )
        return
    if _should_git_add_materials(git_add_materials):
        try:
            git_result = git_add_application_materials(job_dir, repo_dir=config.root)
        except GitTrackingError as exc:
            typer.echo(f"Could not add generated application materials to git: {exc}", err=True)
            return
        suffix = "file" if len(git_result.files) == 1 else "files"
        typer.echo(f"Added {len(git_result.files)} generated application material {suffix} to git.")


@app.command("render-typst")
def render_typst(
    job: Path = typer.Option(..., "--job", help="Path to the job folder."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    typst_bin: str = typer.Option("typst", "--typst-bin", help="Typst executable path or command name."),
) -> None:
    """Render generated Typst files for one job."""
    config = load_workspace_config(workspace)
    job_dir = config.job_dir(job)
    try:
        rendered = render_typst_files(job_dir, typst_bin=typst_bin)
    except FileNotFoundError as exc:
        typer.echo(str(exc))
        raise typer.Exit(code=1) from exc
    except RuntimeError as exc:
        typer.echo(str(exc))
        raise typer.Exit(code=1) from exc
    typer.echo(f"Rendered {len(rendered)} PDF files for {job_dir}")
