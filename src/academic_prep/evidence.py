from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re

import yaml


@dataclass(frozen=True)
class EvidenceItem:
    source_file: str
    section: str
    kind: str
    text: str


@dataclass(frozen=True)
class EvidenceReference:
    source_file: str
    section: str
    text: str


def extract_typst_evidence(path: Path) -> list[EvidenceItem]:
    section = "Unsectioned"
    evidence: list[EvidenceItem] = []

    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue

        section_match = re.match(r'#section\("([^"]+)"\)', line)
        if section_match:
            section = section_match.group(1)
            continue

        heading_match = re.match(r"=+\s+(.+)", line)
        if heading_match:
            section = heading_match.group(1).strip()
            continue

        block_match = re.match(r"#(education|job|award|references?)\((.*)\)", line)
        if block_match:
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind=block_match.group(1),
                    text=_clean_typst_inline(block_match.group(2)),
                )
            )
            continue

        if line.startswith("+ @"):
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind="publication",
                    text=line[2:].strip(),
                )
            )

    return evidence


def extract_profile_evidence(profile_dir: Path) -> list[Path]:
    manifest_path = profile_dir / "profile.yaml"
    manifest = yaml.safe_load(manifest_path.read_text(encoding="utf-8"))
    sources = manifest.get("sources", {})
    generated = manifest.get("generated", {})
    written: list[Path] = []

    for source_key, source_value in sources.items():
        if not str(source_value).endswith(".typ"):
            continue
        source_path = profile_dir / source_value
        output_path = _output_path_for_source(profile_dir, source_key, generated)
        evidence = extract_typst_evidence(source_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(_evidence_markdown(source_key, evidence), encoding="utf-8")
        written.append(output_path)

    return written


def load_generated_evidence(profile_dir: Path) -> list[EvidenceReference]:
    generated_dir = profile_dir / "generated"
    if not generated_dir.exists():
        return []

    references: list[EvidenceReference] = []
    for path in sorted(generated_dir.glob("*.evidence.md")):
        section = "Unsectioned"
        relative = path.relative_to(profile_dir.parent) if profile_dir.parent in path.parents else path
        for raw_line in path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if line.startswith("## "):
                section = line[3:].strip()
            elif line.startswith("- "):
                references.append(
                    EvidenceReference(
                        source_file=str(relative),
                        section=section,
                        text=line[2:].strip(),
                    )
                )
    return references


def _clean_typst_inline(value: str) -> str:
    value = value.replace("[", "").replace("]", "")
    value = value.replace('"', "")
    value = re.sub(r"\s+", " ", value)
    return value.strip()


def _output_path_for_source(profile_dir: Path, source_key: str, generated: dict[str, str]) -> Path:
    manifest_key = f"{source_key}_evidence"
    output = generated.get(manifest_key, f"generated/{source_key}.evidence.md")
    return profile_dir / output


def _evidence_markdown(source_key: str, evidence: list[EvidenceItem]) -> str:
    lines = [f"# Evidence: {source_key}", ""]
    current_section = ""

    for item in evidence:
        if item.section != current_section:
            current_section = item.section
            lines.extend([f"## {current_section}", ""])
        lines.append(f"- `{item.kind}`: {item.text}")

    if not evidence:
        lines.append("_No evidence extracted._")

    return "\n".join(lines).rstrip() + "\n"
