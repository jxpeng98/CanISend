from pathlib import Path

import typer

from academic_prep.jobs import create_job
from academic_prep.pipeline import run_pipeline as run_job_pipeline
from academic_prep.profile import init_profile as create_profile

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
) -> None:
    """Create starter profile files."""
    created = create_profile(profile_dir)
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
) -> None:
    """Render generated Typst files for one job."""
    typer.echo(f"Typst rendering is not implemented yet: {job}")
