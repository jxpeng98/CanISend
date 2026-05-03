from pathlib import Path

import typer

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
) -> None:
    """Create a local job folder and advert file."""
    typer.echo(f"Job creation is not implemented yet: {title} at {institution} ({deadline}, {source_url})")


@app.command("run")
def run_pipeline(
    job: Path = typer.Option(..., "--job", help="Path to the job folder."),
) -> None:
    """Run the application preparation pipeline for one job."""
    typer.echo(f"Pipeline execution is not implemented yet: {job}")


@app.command("render-typst")
def render_typst(
    job: Path = typer.Option(..., "--job", help="Path to the job folder."),
) -> None:
    """Render generated Typst files for one job."""
    typer.echo(f"Typst rendering is not implemented yet: {job}")
