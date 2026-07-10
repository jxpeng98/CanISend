from pathlib import Path
import shutil
import subprocess


PRIMARY_TYPST_FILENAMES = ("cover_letter.typ", "application_package.typ")


def render_typst_files(job_dir: Path, typst_bin: str = "typst") -> list[Path]:
    typst_dir = job_dir / "typst"
    pending_candidates = [
        typst_dir / filename.replace(".typ", ".generated.typ")
        for filename in PRIMARY_TYPST_FILENAMES
        if (typst_dir / filename.replace(".typ", ".generated.typ")).exists()
    ]
    if pending_candidates:
        names = ", ".join(path.name for path in pending_candidates)
        raise RuntimeError(
            "Refusing to render while generated Typst candidates require reconciliation: "
            f"{names}"
        )

    executable = shutil.which(typst_bin) if Path(typst_bin).name == typst_bin else typst_bin
    if executable is None or not Path(executable).exists():
        raise FileNotFoundError(f"Typst binary not found: {typst_bin}")

    pdf_dir = job_dir / "pdf"
    typst_files = [
        typst_dir / filename
        for filename in PRIMARY_TYPST_FILENAMES
        if (typst_dir / filename).is_file()
    ]
    rendered: list[Path] = []

    for source in typst_files:
        output = pdf_dir / f"{source.stem}.pdf"
        output.parent.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [str(executable), "compile", str(source), str(output)],
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            raise RuntimeError(result.stderr.strip() or f"Typst failed for {source}")
        rendered.append(output)

    return rendered
