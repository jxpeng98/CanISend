from pathlib import Path

import typer

from academic_prep.jobs import create_job
from academic_prep.pipeline import run_pipeline as run_job_pipeline
from academic_prep.profile import init_profile as create_profile
from academic_prep.rss import fetch_rss_text, filter_job_leads, parse_jobs_ac_uk_rss, write_job_leads
from academic_prep.typst import render_typst_files

app = typer.Typer(
    help="Prepare academic job application materials from local files.",
    no_args_is_help=True,
)


@app.command("init-profile")
def init_profile(
    profile_dir: Path = typer.Option(
        Path("profile"),
        "--profile-dir",
        help="Directory for Markdown profile evidence files.",
    ),
    mode: str = typer.Option(
        "hybrid",
        "--mode",
        help="Profile scaffold mode: markdown, typst, or hybrid.",
    ),
) -> None:
    """Create starter profile files."""
    try:
        created = create_profile(profile_dir, mode=mode)
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    typer.echo(f"Profile ready at {profile_dir}")
    if created:
        typer.echo(f"Created {len(created)} profile files.")
    else:
        typer.echo("No files created; existing profile files were left unchanged.")


@app.command("new-job")
def new_job(
    title: str = typer.Option(..., "--title", help="Job title."),
    institution: str = typer.Option(..., "--institution", help="Hiring institution."),
    deadline: str = typer.Option("unknown", "--deadline", help="Application deadline."),
    source_url: str = typer.Option("", "--source-url", help="Original job advert URL."),
    jobs_dir: Path = typer.Option(Path("jobs"), "--jobs-dir", help="Directory for job folders."),
    advert_file: Path | None = typer.Option(
        None,
        "--advert-file",
        help="Local .md or .txt job advert file to import.",
    ),
) -> None:
    """Create a local job folder and advert file."""
    try:
        job_dir = create_job(
            jobs_dir=jobs_dir,
            title=title,
            institution=institution,
            deadline=deadline,
            source_url=source_url,
            advert_file=advert_file,
        )
    except ValueError as exc:
        raise typer.BadParameter(str(exc)) from exc
    typer.echo(f"Created job at {job_dir}")


@app.command("fetch-jobs-ac-uk")
def fetch_jobs_ac_uk(
    feed_url: str = typer.Option("", "--feed-url", help="jobs.ac.uk RSS feed URL."),
    rss_file: Path | None = typer.Option(
        None,
        "--rss-file",
        help="Local RSS XML file for testing or offline import.",
    ),
    output: Path = typer.Option(Path("job_leads/jobs_ac_uk.json"), "--output", help="JSON output path."),
    include: list[str] = typer.Option([], "--include", help="Include jobs matching this keyword."),
    exclude: list[str] = typer.Option([], "--exclude", help="Exclude jobs matching this keyword."),
    limit: int = typer.Option(100, "--limit", help="Maximum number of leads to write."),
) -> None:
    """Fetch jobs.ac.uk RSS leads and apply local keyword filters."""
    if rss_file is None and not feed_url:
        raise typer.BadParameter("Provide --feed-url or --rss-file.")

    xml_text = rss_file.read_text(encoding="utf-8") if rss_file is not None else fetch_rss_text(feed_url)
    leads = parse_jobs_ac_uk_rss(xml_text, feed_url=feed_url)
    filtered = filter_job_leads(leads, include_keywords=include, exclude_keywords=exclude)
    limited = filtered[:limit]
    write_job_leads(output, limited)
    typer.echo(f"Wrote {len(limited)} jobs.ac.uk leads to {output}")


@app.command("run")
def run_pipeline(
    job: Path = typer.Option(..., "--job", help="Path to the job folder."),
) -> None:
    """Run the application preparation pipeline for one job."""
    written = run_job_pipeline(job)
    typer.echo(f"Generated {len(written)} files for {job}")


@app.command("render-typst")
def render_typst(
    job: Path = typer.Option(..., "--job", help="Path to the job folder."),
    typst_bin: str = typer.Option("typst", "--typst-bin", help="Typst executable path or command name."),
) -> None:
    """Render generated Typst files for one job."""
    try:
        rendered = render_typst_files(job, typst_bin=typst_bin)
    except FileNotFoundError as exc:
        typer.echo(str(exc))
        raise typer.Exit(code=1) from exc
    except RuntimeError as exc:
        typer.echo(str(exc))
        raise typer.Exit(code=1) from exc
    typer.echo(f"Rendered {len(rendered)} PDF files for {job}")
