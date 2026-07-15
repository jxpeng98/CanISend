import hashlib
import json
from pathlib import Path
import sys
import unicodedata

import typer

from canisend import __version__
from canisend.agent_protocol import (
    AgentResponse,
    WorkflowSnapshotReference,
    agent_response_lines,
    artifact_reference_from_path,
    default_agent_capabilities,
    dumps_agent_response,
    error_response,
    success_response,
)
from canisend.evidence import EvidenceAugmentationError, extract_profile_evidence
from canisend.decision_models import MAX_USER_REVISION
from canisend.document_execution import (
    document_execution_status_agent_response,
    inspect_document_execution,
)
from canisend.discovery.agent import (
    discovery_catalog_agent_response,
    discovery_refresh_agent_response,
)
from canisend.discovery.catalog import (
    DiscoveryInputError,
    DiscoveryWriteError,
    build_catalog_from_files,
    write_lead_catalog,
)
from canisend.discovery.catalog_models import normalized_ranking_policy
from canisend.discovery.refresh import (
    DiscoveryRefreshInputError,
    DiscoveryRefreshWriteError,
    load_discovery_sources,
    refresh_discovery_sources,
)
from canisend.examples import run_packaged_example
from canisend.git_tracking import GitTrackingError, git_add_application_materials
from canisend.jobs import create_job, create_job_from_lead, list_jobs as list_job_folders, slugify
from canisend.orchestrator import OrchestrationError, run_orchestration
from canisend.pipeline import run_pipeline as run_job_pipeline
from canisend.profile import init_profile as create_profile
from canisend.ready_check import check_application_package, package_check_agent_response
from canisend.rss import (
    JobFeedError,
    fetch_rss_text,
    filter_job_leads,
    parse_job_feed,
    parse_jobs_ac_uk_rss,
    write_job_leads,
)
from canisend.skill_distribution import export_skill_distribution
from canisend.stage_agent import (
    stage_apply_agent_response,
    stage_cancel_agent_response,
    stage_error_response,
    stage_prepare_agent_response,
    stage_run_agent_response,
    stage_status_agent_response,
    stage_submit_agent_response,
)
from canisend.stage_runtime import (
    StageRuntimeError,
    apply_stage_result,
    cancel_stage_task,
    inspect_stage_status,
    prepare_stage,
    run_configured_provider_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.typst import render_typst_files
from canisend.user_mutation_agent import (
    application_brief_status_agent_response,
    application_decision_status_agent_response,
    corrections_status_agent_response,
    load_brief_patch_file,
    load_corrections_patch_file,
    load_decision_patch_file,
    load_package_review_disposition_patch_file,
    load_review_disposition_patch_file,
    mutation_outcome_agent_response,
    package_review_dispositions_status_agent_response,
    review_dispositions_status_agent_response,
    user_mutation_error_response,
)
from canisend.user_mutations import (
    UserMutationError,
    apply_user_patch,
    initialize_application_brief,
    initialize_application_decision,
    initialize_confirmed_corrections,
    initialize_package_review_dispositions,
    initialize_review_dispositions,
    inspect_application_brief,
    inspect_application_decision,
    inspect_package_review_dispositions,
    inspect_review_dispositions,
    inspect_user_artifact,
    recover_user_mutation,
    resolve_review_disposition_artifact,
)
from canisend.versioning import fetch_remote_versions, format_version_report
from canisend.workflow_state import (
    derive_workflow_snapshot,
    job_intake_agent_response,
    job_list_agent_response,
    workflow_snapshot_agent_response,
)
from canisend.workspace import (
    doctor_lines,
    init_workspace as create_workspace,
    load_workspace_config,
    prune_deprecated_workspace_files,
    update_workspace_defaults,
    workspace_report,
    workspace_report_agent_response,
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
agent_app = typer.Typer(
    help="Inspect CanISend's versioned host-agent contract and safe workspace context.",
    no_args_is_help=True,
)
app.add_typer(agent_app, name="agent")
stage_app = typer.Typer(
    help="Inspect and execute resumable workflow stages through versioned local contracts.",
    no_args_is_help=True,
)
app.add_typer(stage_app, name="stage")
corrections_app = typer.Typer(
    help="Inspect and update the user-owned confirmed corrections record.",
    no_args_is_help=True,
)
app.add_typer(corrections_app, name="corrections")
decision_app = typer.Typer(
    help="Inspect and update the user-owned apply, hold, or skip decision.",
    no_args_is_help=True,
)
app.add_typer(decision_app, name="decision")
brief_app = typer.Typer(
    help="Inspect and update the user-owned application brief.",
    no_args_is_help=True,
)
app.add_typer(brief_app, name="brief")
documents_app = typer.Typer(
    help="Inspect required-document fan-out and guarded executor availability.",
    no_args_is_help=True,
)
app.add_typer(documents_app, name="documents")
review_dispositions_app = typer.Typer(
    help="Inspect and update user-owned dispositions for structured Review findings.",
    no_args_is_help=True,
)
app.add_typer(review_dispositions_app, name="review-dispositions")
package_review_app = typer.Typer(
    help="Inspect and update user-owned decisions for aggregate package Review findings.",
    no_args_is_help=True,
)
app.add_typer(package_review_app, name="package-review")
user_mutation_app = typer.Typer(
    help="Recover a previously accepted user-owned mutation.",
    no_args_is_help=True,
)
app.add_typer(user_mutation_app, name="user-mutation")
discovery_app = typer.Typer(
    help="Refresh, merge, filter, and rank source-neutral job leads.",
    no_args_is_help=True,
)
app.add_typer(discovery_app, name="discovery")


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


@agent_app.command("capabilities")
def agent_capabilities(
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Report workspace-independent protocol and operation capabilities."""
    response = success_response(
        operation="agent.capabilities",
        capabilities=default_agent_capabilities(__version__),
    )
    _emit_agent_response(response, output_format=output_format)


@agent_app.command("context")
def agent_context(
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory to inspect.",
    ),
    job: Path | None = typer.Option(
        None,
        "--job",
        help="Optional job directory or workspace-relative job identifier to inspect.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Return privacy-safe workspace or job context for a host agent."""
    _validate_output_format(output_format)
    try:
        report = workspace_report(workspace)
        workspace_response = workspace_report_agent_response(
            report,
            operation="agent.context",
        )
        if report.check("workspace_config").status == "missing":
            response = error_response(
                operation="agent.context",
                code="workspace.not_initialized",
                message="The requested directory is not an initialized CanISend workspace.",
                artifacts=workspace_response.artifacts,
                missing_fields=["canisend.yaml"],
                warnings=workspace_response.warnings,
                next_actions=workspace_response.next_actions,
            )
        elif job is None:
            response = workspace_response
        else:
            response = workflow_snapshot_agent_response(
                derive_workflow_snapshot(workspace, job),
                operation="agent.context",
            )
    except Exception:
        if output_format != "json":
            raise
        response = error_response(
            operation="agent.context",
            code="operation.failed",
            message="CanISend could not derive the requested agent context.",
        )
    _emit_agent_response(response, output_format=output_format)


@stage_app.command("status")
def stage_status_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    stage: str = typer.Option(
        "parse",
        "--stage",
        help=(
            "Workflow stage to inspect: evidence, parse, confirm, match, brief, draft, "
            "review, or package_review."
        ),
    ),
    document_id: str | None = typer.Option(
        None,
        "--document-id",
        help="Stable Required Document Plan ID for a document-scoped Draft or Review stage.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Inspect one resumable stage without creating workflow state."""
    _validate_output_format(output_format)
    operation = "workflow.stage_status"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        inspection = inspect_stage_status(
            config.root,
            job_dir,
            stage=stage,  # type: ignore[arg-type]
            document_id=document_id,
        )
        response = stage_status_agent_response(config.root, job_dir, inspection)
    except StageRuntimeError as exc:
        response = stage_error_response(operation, exc)
    _emit_agent_response(response, output_format=output_format)


@stage_app.command("prepare")
def stage_prepare_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    stage: str = typer.Option(
        "parse",
        "--stage",
        help=(
            "Workflow stage to prepare: evidence, parse, confirm, match, brief, draft, "
            "review, or package_review."
        ),
    ),
    mode: str = typer.Option(
        "host-agent",
        "--mode",
        help="Executor mode: host-agent or deterministic.",
    ),
    document_id: str | None = typer.Option(
        None,
        "--document-id",
        help="Stable Required Document Plan ID for a document-scoped Draft or Review stage.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create or reuse an immutable task for the current host agent."""
    _validate_output_format(output_format)
    operation = "workflow.stage_prepare"
    try:
        normalized_mode = _stage_mode(mode)
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        prepared = prepare_stage(
            config.root,
            job_dir,
            stage=stage,  # type: ignore[arg-type]
            execution_mode=normalized_mode,
            document_id=document_id,
        )
        response = stage_prepare_agent_response(config.root, job_dir, prepared)
    except StageRuntimeError as exc:
        response = stage_error_response(operation, exc)
    _emit_agent_response(response, output_format=output_format)


@stage_app.command("submit")
def stage_submit_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    task: Path = typer.Option(..., "--task", help="Job-relative immutable TaskSpec path."),
    candidate_file: Path = typer.Option(
        ...,
        "--candidate-file",
        help="JSON candidate file to copy through the guarded stage boundary.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Validate and safely stage candidate JSON without direct run-path writes."""
    _validate_output_format(output_format)
    operation = "workflow.stage_submit"
    try:
        try:
            candidate_bytes = candidate_file.expanduser().read_bytes()
        except OSError as exc:
            raise StageRuntimeError(
                "stage.candidate_missing",
                "The candidate JSON file cannot be read.",
            ) from exc
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        submitted = submit_stage_candidate(
            config.root,
            job_dir,
            task_spec_path=task,
            candidate_bytes=candidate_bytes,
        )
        response = stage_submit_agent_response(config.root, submitted)
    except StageRuntimeError as exc:
        response = stage_error_response(operation, exc)
    _emit_agent_response(response, output_format=output_format)


@stage_app.command("apply")
def stage_apply_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    task: Path = typer.Option(..., "--task", help="Job-relative immutable TaskSpec path."),
    result: Path = typer.Option(..., "--result", help="Job-relative TaskResult path."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Validate and atomically promote one staged result."""
    _validate_output_format(output_format)
    operation = "workflow.stage_apply"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        applied = apply_stage_result(
            config.root,
            job_dir,
            task_spec_path=task,
            task_result_path=result,
        )
        response = stage_apply_agent_response(config.root, applied)
    except StageRuntimeError as exc:
        response = stage_error_response(operation, exc)
    _emit_agent_response(response, output_format=output_format)


@stage_app.command("cancel")
def stage_cancel_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    stage: str = typer.Option(
        "parse",
        "--stage",
        help=(
            "Active workflow stage to cancel: evidence, parse, confirm, match, brief, draft, "
            "review, or package_review."
        ),
    ),
    document_id: str | None = typer.Option(
        None,
        "--document-id",
        help="Stable Required Document Plan ID for the active Draft or Review task.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Cancel one prepared task without deleting its audit records or candidate."""
    _validate_output_format(output_format)
    operation = "workflow.stage_cancel"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        cancelled = cancel_stage_task(
            config.root,
            job_dir,
            stage=stage,  # type: ignore[arg-type]
            document_id=document_id,
        )
        response = stage_cancel_agent_response(config.root, cancelled)
    except StageRuntimeError as exc:
        response = stage_error_response(operation, exc)
    _emit_agent_response(response, output_format=output_format)


@stage_app.command("run")
def stage_run_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    stage: str = typer.Option(
        "parse",
        "--stage",
        help=(
            "Workflow stage to execute: evidence, parse, confirm, match, brief, draft, "
            "review, or package_review."
        ),
    ),
    mode: str = typer.Option(
        "deterministic",
        "--mode",
        help="Executor mode: deterministic or configured-provider.",
    ),
    allow_provider_backed: bool = typer.Option(
        False,
        "--allow-provider-backed",
        help="Explicitly allow this Tier 3 provider-backed execution request.",
    ),
    document_id: str | None = typer.Option(
        None,
        "--document-id",
        help="Stable Required Document Plan ID for a document-scoped Draft or Review stage.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Run one core-controlled stage through candidate validation and promotion."""
    _validate_output_format(output_format)
    operation = "workflow.stage_run"
    try:
        normalized_mode = _stage_mode(mode)
        if normalized_mode == "host_agent":
            raise StageRuntimeError(
                "stage.unsupported_mode",
                "Host-agent execution uses stage prepare, submit, and apply.",
            )
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        outcome = (
            run_configured_provider_stage(
                config.root,
                job_dir,
                stage=stage,  # type: ignore[arg-type]
                allow_provider_backed=allow_provider_backed,
                document_id=document_id,
            )
            if normalized_mode == "configured_provider"
            else run_deterministic_stage(
                config.root,
                job_dir,
                stage=stage,  # type: ignore[arg-type]
                document_id=document_id,
            )
        )
        response = stage_run_agent_response(config.root, outcome)
    except StageRuntimeError as exc:
        response = stage_error_response(operation, exc)
    _emit_agent_response(response, output_format=output_format)


@corrections_app.command("status")
def corrections_status_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Inspect the user-owned corrections record without reading its body into the response."""
    _validate_output_format(output_format)
    operation = "criteria.corrections_status"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        response = corrections_status_agent_response(
            config.root,
            job_dir,
            inspect_user_artifact(config.root, job_dir, "corrections"),
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@corrections_app.command("init")
def corrections_init_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize creation of the user-owned corrections record.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create the unresolved corrections template only when it is absent."""
    _validate_output_format(output_format)
    operation = "criteria.corrections_initialize"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        outcome = initialize_confirmed_corrections(
            config.root,
            job_dir,
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@corrections_app.command("update")
def corrections_update_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    patch_file: Path = typer.Option(
        ...,
        "--patch-file",
        help="Strict bounded YAML or JSON containing one supported corrections patch.",
    ),
    expected_revision: str = typer.Option(
        ...,
        "--expected-revision",
        help="Current corrections revision used as the compare-and-swap baseline.",
    ),
    expected_sha256: str = typer.Option(
        ...,
        "--expected-sha256",
        help="Current corrections SHA-256 used as the compare-and-swap baseline.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize this one scoped corrections update.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Apply one scoped corrections patch through revision/hash compare-and-swap."""
    _validate_output_format(output_format)
    operation = "criteria.corrections_update"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        patch = load_corrections_patch_file(patch_file)
        outcome = apply_user_patch(
            config.root,
            job_dir,
            patch,
            expected_sha256=expected_sha256,
            expected_revision=_user_owned_expected_revision(expected_revision),
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@decision_app.command("status")
def decision_status_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Inspect the user-owned decision and its current basis without returning rationale."""
    _validate_output_format(output_format)
    operation = "decision.status"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        response = application_decision_status_agent_response(
            config.root,
            job_dir,
            inspect_application_decision(config.root, job_dir),
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@decision_app.command("init")
def decision_init_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize creation of the user-owned application decision.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create the explicitly undecided application decision only when absent."""
    _validate_output_format(output_format)
    operation = "decision.initialize"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        outcome = initialize_application_decision(
            config.root,
            job_dir,
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@decision_app.command("update")
def decision_update_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    patch_file: Path = typer.Option(
        ...,
        "--patch-file",
        help="Strict bounded YAML or JSON containing one supported decision patch.",
    ),
    expected_revision: str = typer.Option(
        ...,
        "--expected-revision",
        help="Current decision revision used as the compare-and-swap baseline.",
    ),
    expected_sha256: str = typer.Option(
        ...,
        "--expected-sha256",
        help="Current decision SHA-256 used as the compare-and-swap baseline.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize this one scoped decision update.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Apply one scoped decision patch through revision/hash compare-and-swap."""
    _validate_output_format(output_format)
    operation = "decision.update"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        patch = load_decision_patch_file(patch_file)
        outcome = apply_user_patch(
            config.root,
            job_dir,
            patch,
            expected_sha256=expected_sha256,
            expected_revision=_user_owned_expected_revision(expected_revision),
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@brief_app.command("status")
def brief_status_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Inspect Brief confirmation state without returning any private field body."""
    _validate_output_format(output_format)
    operation = "brief.status"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        response = application_brief_status_agent_response(
            config.root,
            job_dir,
            inspect_application_brief(config.root, job_dir),
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@brief_app.command("init")
def brief_init_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize create-if-absent initialization of the user-owned Brief.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create a Brief once and bootstrap concrete legacy language/style preferences."""
    _validate_output_format(output_format)
    operation = "brief.initialize"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        outcome = initialize_application_brief(
            config.root,
            job_dir,
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@brief_app.command("update")
def brief_update_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    patch_file: Path = typer.Option(
        ...,
        "--patch-file",
        help="Strict bounded YAML or JSON containing one supported Brief patch.",
    ),
    expected_revision: str = typer.Option(
        ...,
        "--expected-revision",
        help="Current Brief revision used as the compare-and-swap baseline.",
    ),
    expected_sha256: str = typer.Option(
        ...,
        "--expected-sha256",
        help="Current Brief SHA-256 used as the compare-and-swap baseline.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize this one scoped Brief update.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Apply one scoped Brief patch through revision/hash compare-and-swap."""
    _validate_output_format(output_format)
    operation = "brief.update"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        patch = load_brief_patch_file(patch_file)
        outcome = apply_user_patch(
            config.root,
            job_dir,
            patch,
            expected_sha256=expected_sha256,
            expected_revision=_user_owned_expected_revision(expected_revision),
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@documents_app.command("status")
def documents_status_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Inspect body-free fan-out state for every required-document task."""
    _validate_output_format(output_format)
    operation = "documents.status"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        response = document_execution_status_agent_response(
            config.root,
            job_dir,
            inspect_document_execution(config.root, job_dir),
        )
    except Exception:
        response = error_response(
            operation=operation,
            code="operation.failed",
            message="CanISend could not inspect document execution safely.",
            workflow=WorkflowSnapshotReference(phase="unknown", readiness="blocked"),
        )
    _emit_agent_response(response, output_format=output_format)


@review_dispositions_app.command("status")
def review_dispositions_status_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    document_id: str | None = typer.Option(
        None,
        "--document-id",
        help="Stable Required Document Plan ID for the Draft and Review being dispositioned.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Inspect body-free Review disposition and document-readiness state."""
    _validate_output_format(output_format)
    operation = "review.dispositions_status"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        response = review_dispositions_status_agent_response(
            config.root,
            job_dir,
            inspect_review_dispositions(
                config.root,
                job_dir,
                document_id=document_id,
            ),
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@review_dispositions_app.command("init")
def review_dispositions_init_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    document_id: str | None = typer.Option(
        None,
        "--document-id",
        help="Stable Required Document Plan ID for the Draft and Review being dispositioned.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize create-if-absent Review disposition initialization.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create dispositions bound to the current Draft and deterministic Review."""
    _validate_output_format(output_format)
    operation = "review.dispositions_initialize"
    selected_artifact = None
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        selected_artifact = resolve_review_disposition_artifact(
            config.root,
            job_dir,
            document_id=document_id,
        )
        outcome = initialize_review_dispositions(
            config.root,
            job_dir,
            document_id=document_id,
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(
            operation,
            exc,
            artifact=selected_artifact,
        )
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@review_dispositions_app.command("update")
def review_dispositions_update_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    document_id: str | None = typer.Option(
        None,
        "--document-id",
        help="Stable Required Document Plan ID for the Draft and Review being dispositioned.",
    ),
    patch_file: Path = typer.Option(
        ...,
        "--patch-file",
        help="Strict bounded YAML or JSON containing one Review disposition patch.",
    ),
    expected_revision: str = typer.Option(
        ...,
        "--expected-revision",
        help="Current disposition revision used as the compare-and-swap baseline.",
    ),
    expected_sha256: str = typer.Option(
        ...,
        "--expected-sha256",
        help="Current disposition SHA-256 used as the compare-and-swap baseline.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize this one scoped Review disposition update.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Apply one finding disposition through revision/hash compare-and-swap."""
    _validate_output_format(output_format)
    operation = "review.dispositions_update"
    selected_artifact = None
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        selected_artifact = resolve_review_disposition_artifact(
            config.root,
            job_dir,
            document_id=document_id,
        )
        patch = load_review_disposition_patch_file(patch_file)
        outcome = apply_user_patch(
            config.root,
            job_dir,
            patch,
            expected_sha256=expected_sha256,
            expected_revision=_user_owned_expected_revision(expected_revision),
            document_id=document_id,
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(
            operation,
            exc,
            artifact=selected_artifact,
        )
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@package_review_app.command("status")
def package_review_status_command(
    job: Path = typer.Option(
        ...,
        "--job",
        help="Job folder path or workspace-relative job identifier.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Inspect body-free aggregate dispositions and application-package readiness."""
    _validate_output_format(output_format)
    operation = "package_review.dispositions_status"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        response = package_review_dispositions_status_agent_response(
            config.root,
            job_dir,
            inspect_package_review_dispositions(config.root, job_dir),
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@package_review_app.command("init")
def package_review_init_command(
    job: Path = typer.Option(
        ...,
        "--job",
        help="Job folder path or workspace-relative job identifier.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize create-if-absent package disposition initialization.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create package decisions bound to the current aggregate Review."""
    _validate_output_format(output_format)
    operation = "package_review.dispositions_initialize"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        outcome = initialize_package_review_dispositions(
            config.root,
            job_dir,
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@package_review_app.command("update")
def package_review_update_command(
    job: Path = typer.Option(
        ...,
        "--job",
        help="Job folder path or workspace-relative job identifier.",
    ),
    patch_file: Path = typer.Option(
        ...,
        "--patch-file",
        help="Strict bounded YAML or JSON containing one package finding patch.",
    ),
    expected_revision: str = typer.Option(
        ...,
        "--expected-revision",
        help="Current package disposition revision used as the compare-and-swap baseline.",
    ),
    expected_sha256: str = typer.Option(
        ...,
        "--expected-sha256",
        help="Current package disposition SHA-256 used as the compare-and-swap baseline.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize this one scoped package disposition update.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Apply one package finding decision through revision/hash compare-and-swap."""
    _validate_output_format(output_format)
    operation = "package_review.dispositions_update"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        patch = load_package_review_disposition_patch_file(patch_file)
        outcome = apply_user_patch(
            config.root,
            job_dir,
            patch,
            expected_sha256=expected_sha256,
            expected_revision=_user_owned_expected_revision(expected_revision),
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(operation, exc)
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


@user_mutation_app.command("recover")
def user_mutation_recover_command(
    job: Path = typer.Option(..., "--job", help="Job folder path or workspace-relative job identifier."),
    mutation_id: str = typer.Option(
        ...,
        "--mutation-id",
        help="Opaque mutation identifier from a prior accepted write response.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="Initialized workspace containing the job.",
    ),
    confirm_user_owned_write: bool = typer.Option(
        False,
        "--confirm-user-owned-write",
        help="Explicitly authorize completion of the accepted mutation claim.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Complete a durable accepted mutation without accepting a new patch."""
    _validate_output_format(output_format)
    operation = "user_mutation.recover"
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        outcome = recover_user_mutation(
            config.root,
            job_dir,
            mutation_id,
            consent_confirmed=confirm_user_owned_write,
        )
        response = mutation_outcome_agent_response(
            config.root,
            job_dir,
            outcome,
            operation=operation,
        )
    except UserMutationError as exc:
        response = user_mutation_error_response(
            operation,
            exc,
            mutation_id=mutation_id,
        )
    except Exception:
        response = _unexpected_user_mutation_error_response(operation)
    _emit_agent_response(response, output_format=output_format)


def _unexpected_user_mutation_error_response(operation: str) -> AgentResponse:
    return error_response(
        operation=operation,
        code="operation.failed",
        message="CanISend could not complete the requested user-owned operation.",
        workflow=WorkflowSnapshotReference(phase="unknown", readiness="blocked"),
    )


def _user_owned_expected_revision(value: str) -> int:
    if (
        not value
        or len(value) > 20
        or not value.isascii()
        or not value.isdigit()
    ):
        raise UserMutationError(
            "user_input.invalid",
            "The expected user-owned revision is invalid.",
        )
    revision = int(value)
    if revision > MAX_USER_REVISION:
        raise UserMutationError(
            "user_input.invalid",
            "The expected user-owned revision is invalid.",
        )
    return revision


def _stage_mode(value: str) -> str:
    normalized = value.strip().lower().replace("-", "_")
    if normalized not in {
        "deterministic",
        "host_agent",
        "configured_provider",
    }:
        raise StageRuntimeError(
            "stage.unsupported_mode",
            "Stage mode must be deterministic, host-agent, or configured-provider.",
        )
    return normalized


def _emit_agent_response(response: AgentResponse, *, output_format: str) -> None:
    _validate_output_format(output_format)
    if output_format == "json":
        typer.echo(dumps_agent_response(response), nl=False)
    else:
        for line in agent_response_lines(response):
            typer.echo(line)
    if not response.ok:
        raise typer.Exit(code=1)


def _validate_output_format(output_format: str) -> None:
    if output_format not in {"text", "json"}:
        raise typer.BadParameter("--format must be text or json.")


def _emit_operation_error(
    *,
    operation: str,
    code: str,
    message: str,
    retryable: bool = False,
    artifacts=None,
) -> None:
    _emit_agent_response(
        error_response(
            operation=operation,
            code=code,
            message=message,
            retryable=retryable,
            artifacts=artifacts or [],
        ),
        output_format="json",
    )


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
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Report local workspace, provider, and rendering readiness."""
    if output_format not in {"text", "json"}:
        raise typer.BadParameter("--format must be text or json.")
    if output_format == "json":
        response = workspace_report_agent_response(workspace_report(workspace))
        typer.echo(dumps_agent_response(response), nl=False)
        return
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
    except Exception as exc:
        typer.echo(
            f"CanISend example failed [example.{type(exc).__name__}].",
            err=True,
        )
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
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Check whether a generated application package has unresolved issues."""
    _validate_output_format(output_format)
    try:
        config = load_workspace_config(workspace)
        job_dir = config.job_dir(job)
        if output_format == "json" and not job_dir.is_dir():
            reference = artifact_reference_from_path(
                workspace=config.root,
                path=job_dir,
                kind="job_directory",
                privacy_tier=2,
                trust_level="trusted_local",
                media_type="inode/directory",
            )
            _emit_operation_error(
                operation="package.check",
                code="job.not_found",
                message="The requested job directory does not exist.",
                artifacts=[reference],
            )
        result = check_application_package(
            job_dir=job_dir,
            profile_dir=config.path("profile_dir", profile_dir),
            workspace=config.root,
        )
    except typer.Exit:
        raise
    except Exception:
        if output_format != "json":
            raise
        _emit_operation_error(
            operation="package.check",
            code="operation.failed",
            message="The application package check could not be completed.",
        )
    report_path = None
    report_write_failed = False
    if write_report:
        try:
            report_path = result.write_report()
        except ValueError as exc:
            report_write_failed = True
            if output_format == "text":
                typer.echo(str(exc), err=True)
    response = package_check_agent_response(
        result,
        workspace=config.root,
        report_path=report_path,
        report_write_failed=report_write_failed,
    )
    if output_format == "json":
        _emit_agent_response(response, output_format="json")
    else:
        for line in result.output_lines():
            typer.echo(line)
        if report_path is not None:
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
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create a local job folder and advert file."""
    _validate_output_format(output_format)
    try:
        config = load_workspace_config(workspace)
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
    except FileExistsError:
        if output_format != "json":
            raise
        _emit_operation_error(
            operation="job.intake",
            code="input.invalid",
            message="A job with this identifier already exists.",
        )
    except ValueError as exc:
        if output_format == "json":
            invalid_source_options = advert_file is not None and fetch_url
            unsupported_file = advert_file is not None and advert_file.suffix.lower() not in {".md", ".txt", ".pdf"}
            if invalid_source_options or unsupported_file:
                _emit_operation_error(
                    operation="job.intake",
                    code="input.invalid",
                    message="The requested job intake options are invalid.",
                )
            if advert_file is not None or fetch_url:
                _emit_operation_error(
                    operation="job.intake",
                    code="source.import_failed",
                    message="The job advert source could not be imported safely.",
                    retryable=fetch_url,
                )
            _emit_operation_error(
                operation="job.intake",
                code="input.invalid",
                message="The requested job metadata is invalid.",
            )
        raise typer.BadParameter(str(exc)) from exc
    except Exception:
        if output_format != "json":
            raise
        _emit_operation_error(
            operation="job.intake",
            code="operation.failed",
            message="The job intake operation failed unexpectedly.",
        )
    response = job_intake_agent_response(config.root, job_dir, operation="job.intake")
    if output_format == "json":
        _emit_agent_response(response, output_format="json")
    else:
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
    lead_id: str | None = typer.Option(
        None,
        "--lead-id",
        help="Stable Lead v2 identifier. Use exactly one of --lead-id or --lead-index.",
    ),
    lead_index: int | None = typer.Option(
        None,
        "--lead-index",
        help="Legacy zero-based lead index. Use exactly one selector.",
    ),
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
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Create a local job folder from a selected RSS or Atom lead without crawling."""
    _validate_output_format(output_format)
    try:
        config = load_workspace_config(workspace)
        job_dir = create_job_from_lead(
            leads_file=config.lead_file(leads_file),
            lead_index=lead_index,
            lead_id=lead_id,
            jobs_dir=config.path("jobs_dir", jobs_dir),
            institution=institution,
            deadline=deadline,
            title=title,
            english_variant=english_variant,
            writing_style=writing_style,
        )
    except FileExistsError:
        if output_format != "json":
            raise
        _emit_operation_error(
            operation="job.intake_from_lead",
            code="input.invalid",
            message="A job with this identifier already exists.",
        )
    except ValueError as exc:
        if output_format == "json":
            _emit_operation_error(
                operation="job.intake_from_lead",
                code="input.invalid",
                message="The selected lead or job metadata is invalid.",
            )
        raise typer.BadParameter(str(exc)) from exc
    except Exception:
        if output_format != "json":
            raise
        _emit_operation_error(
            operation="job.intake_from_lead",
            code="operation.failed",
            message="The lead intake operation failed unexpectedly.",
        )
    response = job_intake_agent_response(
        config.root,
        job_dir,
        operation="job.intake_from_lead",
    )
    if output_format == "json":
        _emit_agent_response(response, output_format="json")
    else:
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
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """List all job folders with status, deadline, and institution."""
    _validate_output_format(output_format)
    try:
        config = load_workspace_config(workspace)
        jobs = list_job_folders(config.path("jobs_dir", jobs_dir))
        response = job_list_agent_response(config.root, jobs)
    except Exception:
        if output_format != "json":
            raise
        _emit_operation_error(
            operation="job.list",
            code="operation.failed",
            message="CanISend could not list the requested job workspace.",
        )
    if output_format == "json":
        _emit_agent_response(response, output_format="json")
        return
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


@discovery_app.command("merge")
def merge_discovery_catalog(
    inputs: list[Path] = typer.Option(
        [],
        "--input",
        help="Lead JSON list or CanISend catalog. Repeat for multiple sources.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    output: Path | None = typer.Option(
        None,
        "--output",
        help="Catalog output path. Defaults to job_leads/catalog.json.",
    ),
    include: list[str] = typer.Option(
        [],
        "--include",
        help="Retain leads matching this keyword and explain each match.",
    ),
    exclude: list[str] = typer.Option(
        [],
        "--exclude",
        help="Exclude leads matching this keyword and record the reason.",
    ),
    prefer_source: list[str] = typer.Option(
        [],
        "--prefer-source",
        help="Source preference in descending priority. Repeat in priority order.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Deterministically merge, deduplicate, filter, and rank local lead files."""
    _validate_output_format(output_format)
    try:
        config = load_workspace_config(workspace)
        input_paths = [
            path.expanduser() if path.expanduser().is_absolute() else config.root / path.expanduser()
            for path in inputs
        ]
        output_path = (
            config.lead_file(output)
            if output is not None
            else config.path("job_leads_dir") / "catalog.json"
        )
        policy = normalized_ranking_policy(
            include_keywords=include,
            exclude_keywords=exclude,
            source_preference=prefer_source,
        )
        catalog = build_catalog_from_files(input_paths, policy=policy)
        write_lead_catalog(output_path, catalog)
        response = discovery_catalog_agent_response(
            config.root,
            output_path,
            catalog,
        )
    except DiscoveryWriteError as exc:
        if output_format == "json":
            _emit_operation_error(
                operation="discovery.merge",
                code="operation.failed",
                message="CanISend could not write the discovery catalog.",
            )
        raise typer.BadParameter(str(exc)) from exc
    except (DiscoveryInputError, ValueError) as exc:
        if output_format == "json":
            _emit_operation_error(
                operation="discovery.merge",
                code="input.invalid",
                message="The discovery inputs or ranking policy are invalid.",
            )
        raise typer.BadParameter(str(exc)) from exc
    except Exception:
        if output_format != "json":
            raise
        _emit_operation_error(
            operation="discovery.merge",
            code="operation.failed",
            message="The discovery merge operation failed unexpectedly.",
        )

    if output_format == "json":
        _emit_agent_response(response, output_format="json")
        return
    _emit_agent_response(response, output_format="text")
    typer.echo(
        "Catalog: "
        f"{catalog.stats.input_records} input, "
        f"{catalog.stats.merged_records} merged, "
        f"{catalog.stats.retained_records} retained, "
        f"{catalog.stats.excluded_records} excluded"
    )


@discovery_app.command("refresh")
def refresh_discovery_catalog(
    sources: Path = typer.Option(
        Path("discovery-sources.yaml"),
        "--sources",
        help="Versioned discovery source configuration YAML.",
    ),
    workspace: Path = typer.Option(
        Path("."),
        "--workspace",
        help="User workspace directory containing canisend.yaml.",
    ),
    output_format: str = typer.Option(
        "text",
        "--format",
        help="Output format: text or json.",
    ),
) -> None:
    """Conditionally refresh configured sources and atomically promote a catalog."""
    _validate_output_format(output_format)
    try:
        config = load_workspace_config(workspace)
        source_path = sources.expanduser()
        if not source_path.is_absolute():
            source_path = config.root / source_path
        source_config = load_discovery_sources(source_path)
        execution = refresh_discovery_sources(
            config.root,
            source_config,
            lead_root=config.path("job_leads_dir"),
        )
        response = discovery_refresh_agent_response(config.root, execution)
    except DiscoveryRefreshInputError as exc:
        if output_format == "json":
            _emit_operation_error(
                operation="discovery.refresh",
                code="input.invalid",
                message="The discovery source configuration is invalid.",
            )
        raise typer.BadParameter(str(exc)) from exc
    except DiscoveryRefreshWriteError as exc:
        if output_format == "json":
            _emit_operation_error(
                operation="discovery.refresh",
                code="operation.failed",
                message="CanISend could not write the discovery refresh artifacts.",
            )
        raise typer.BadParameter(str(exc)) from exc
    except Exception as exc:
        if output_format != "json":
            raise typer.BadParameter(
                "The discovery refresh operation failed unexpectedly."
            ) from exc
        _emit_operation_error(
            operation="discovery.refresh",
            code="operation.failed",
            message="The discovery refresh operation failed unexpectedly.",
        )

    _emit_agent_response(response, output_format=output_format)
    if output_format == "json":
        return
    typer.echo(
        "Refresh: "
        f"{execution.report.status}, "
        f"{execution.report.successful_sources} current, "
        f"{execution.report.stale_sources} stale, "
        f"{execution.report.failed_sources} failed, "
        f"{execution.report.retained_records} retained"
    )


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
        from canisend.stages.parse_stage import build_deterministic_parse_candidate

        import yaml as _yaml

        metadata = _yaml.safe_load((job_dir / "job.yaml").read_text(encoding="utf-8"))
        evidence = load_generated_evidence(config.path("profile_dir", profile_dir))
        parsed_job = build_deterministic_parse_candidate(job_dir)
        if llm_parser:
            typer.echo("Parser: LLM-backed (planned; not executed in dry run)")
        else:
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
        draft_mode = (
            "LLM-backed (planned; not executed in dry run)"
            if llm_drafts
            else "deterministic"
        )
        typer.echo(f"\nDraft mode: {draft_mode}")
        return

    written = run_job_pipeline(
        job_dir,
        profile_dir=config.path("profile_dir", profile_dir),
        use_llm_parser=llm_parser,
        use_llm_drafts=llm_drafts,
        prompt_dir=config.path("prompt_dir", prompt_dir),
        workspace=config.root,
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
