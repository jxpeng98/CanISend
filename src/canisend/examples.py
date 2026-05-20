from __future__ import annotations

from contextlib import contextmanager
from dataclasses import dataclass
import os
from pathlib import Path
import shlex
import shutil
import sys
from collections.abc import Iterator, Mapping

from canisend.evidence import extract_profile_evidence
from canisend.jobs import create_job_from_lead
from canisend.pipeline import run_pipeline
from canisend.resource_files import copy_resource_tree
from canisend.rss import filter_job_leads, parse_jobs_ac_uk_rss, write_job_leads
from canisend.workspace import init_workspace


EXAMPLE_MARKER = ".canisend-example-workspace"
SAFE_EXAMPLE_ENV_KEYS = ("PATH", "TMPDIR", "TEMP", "TMP", "PYTHONIOENCODING", "LANG", "LC_ALL", "SYSTEMROOT", "WINDIR")


@dataclass(frozen=True)
class ExampleRunResult:
    workspace: Path
    job_dir: Path
    leads_file: Path
    written_files: list[Path]


def run_packaged_example(workspace: Path, *, overwrite: bool = False) -> ExampleRunResult:
    """Run the packaged fake-data workflow in a standalone workspace."""
    workspace = workspace.expanduser()
    if workspace.exists() and any(workspace.iterdir()):
        if not overwrite:
            raise ValueError("Example workspace already exists and is not empty. Use --overwrite or choose another --workspace.")
        _remove_workspace_for_example(workspace)

    workspace.mkdir(parents=True, exist_ok=True)
    (workspace / EXAMPLE_MARKER).write_text("CanISend packaged example workspace\n", encoding="utf-8")
    example_inputs = workspace / "example_inputs"
    profile_dir = workspace / "profile"
    jobs_dir = workspace / "jobs"
    leads_file = workspace / "job_leads" / "jobs_ac_uk.json"

    copy_resource_tree("examples/end_to_end/profile", profile_dir, overwrite=True)
    copy_resource_tree("examples/end_to_end/jobs_ac_uk_sample.xml", example_inputs / "jobs_ac_uk_sample.xml", overwrite=True)
    copy_resource_tree("examples/end_to_end/full_job_advert.md", example_inputs / "full_job_advert.md", overwrite=True)
    copy_resource_tree("examples/end_to_end/fake_llm_provider.py", example_inputs / "fake_llm_provider.py", overwrite=True)
    init_workspace(workspace, profile_mode="typst", overwrite=False)

    rss_text = (example_inputs / "jobs_ac_uk_sample.xml").read_text(encoding="utf-8")
    leads = parse_jobs_ac_uk_rss(rss_text, feed_url="packaged-example")
    filtered_leads = filter_job_leads(leads, include_keywords=["economics"], exclude_keywords=[])
    write_job_leads(leads_file, filtered_leads)

    job_dir = create_job_from_lead(
        leads_file=leads_file,
        lead_index=0,
        jobs_dir=jobs_dir,
        institution="Example University",
        deadline="2026-06-15",
    )
    (job_dir / "job_advert.md").write_text((example_inputs / "full_job_advert.md").read_text(encoding="utf-8"), encoding="utf-8")

    written_files = extract_profile_evidence(profile_dir)
    env = {
        "ACADEMIC_PREP_LLM_PROVIDER": "command",
        "ACADEMIC_PREP_LLM_COMMAND": f"{shlex.quote(sys.executable)} {shlex.quote(str(example_inputs / 'fake_llm_provider.py'))}",
        "ACADEMIC_PREP_LLM_TIMEOUT_SECONDS": "300",
    }
    with _temporary_example_env(env):
        written_files.extend(
            run_pipeline(
                job_dir,
                profile_dir=profile_dir,
                use_llm_parser=True,
                use_llm_drafts=True,
                prompt_dir=workspace / "prompts",
            )
        )

    return ExampleRunResult(
        workspace=workspace,
        job_dir=job_dir,
        leads_file=leads_file,
        written_files=written_files,
    )


def _remove_workspace_for_example(workspace: Path) -> None:
    resolved = workspace.resolve()
    if resolved == Path("/") or resolved == Path.home().resolve() or len(resolved.parts) < 3:
        raise ValueError(f"Refusing to overwrite unsafe example workspace path: {workspace}")
    if not (resolved / EXAMPLE_MARKER).exists():
        raise ValueError(f"Refusing to overwrite {workspace}: not a CanISend example workspace.")
    shutil.rmtree(resolved)


@contextmanager
def _temporary_example_env(values: Mapping[str, str]) -> Iterator[None]:
    old_values = dict(os.environ)
    safe_values = {key: old_values[key] for key in SAFE_EXAMPLE_ENV_KEYS if key in old_values}
    os.environ.clear()
    os.environ.update(safe_values)
    os.environ.update(values)
    try:
        yield
    finally:
        os.environ.clear()
        os.environ.update(old_values)
