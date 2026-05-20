from pathlib import Path
import shutil
import subprocess


def render_typst_files(job_dir: Path, typst_bin: str = "typst") -> list[Path]:
    executable = shutil.which(typst_bin) if Path(typst_bin).name == typst_bin else typst_bin
    if executable is None or not Path(executable).exists():
        raise FileNotFoundError(f"Typst binary not found: {typst_bin}")

    typst_dir = job_dir / "typst"
    pdf_dir = job_dir / "pdf"
    typst_files = sorted(typst_dir.glob("*.typ"))
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
