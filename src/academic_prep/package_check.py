from __future__ import annotations

from pathlib import Path
import sys
import zipfile


REQUIRED_WHEEL_RESOURCES = [
    "academic_prep/resources/.env.example",
    "academic_prep/resources/prompts/job_parser.md",
    "academic_prep/resources/prompts/profile_matcher.md",
    "academic_prep/resources/prompts/cover_letter_writer.md",
    "academic_prep/resources/templates/typst/cover_letter.typ",
    "academic_prep/resources/templates/typst/application_package.typ",
    "academic_prep/resources/schemas/parsed_job.schema.json",
    "academic_prep/resources/agent-skills/academic-application-prep/SKILL.md",
    "academic_prep/resources/agent-skills/academic-application-prep/agents/openai.yaml",
    "academic_prep/resources/agent-skills/academic-application-prep/references/provider-config.md",
    "academic_prep/resources/agent-skills/academic-application-prep/references/quality-gates.md",
    "academic_prep/resources/agent-skills/academic-application-prep/references/job-lifecycle.md",
    "academic_prep/resources/agent-skills/academic-application-prep/references/platforms.md",
    "academic_prep/resources/platform-bridges/AGENTS.md",
    "academic_prep/resources/platform-bridges/CLAUDE.md",
    "academic_prep/resources/platform-bridges/GEMINI.md",
    "academic_prep/resources/examples/end_to_end/README.md",
]


def required_wheel_resources() -> list[str]:
    return list(REQUIRED_WHEEL_RESOURCES)


def missing_wheel_resources(wheel_path: Path) -> list[str]:
    with zipfile.ZipFile(wheel_path) as wheel:
        names = set(wheel.namelist())
    return [resource for resource in REQUIRED_WHEEL_RESOURCES if resource not in names]


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if not args:
        print("Usage: python -m academic_prep.package_check dist/*.whl", file=sys.stderr)
        return 2

    exit_code = 0
    for raw_path in args:
        wheel_path = Path(raw_path)
        missing = missing_wheel_resources(wheel_path)
        if missing:
            exit_code = 1
            print(f"{wheel_path}: missing packaged resources:", file=sys.stderr)
            for resource in missing:
                print(f"- {resource}", file=sys.stderr)
        else:
            print(f"{wheel_path}: packaged resources ok")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
