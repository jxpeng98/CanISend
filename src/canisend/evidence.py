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
    item_id: str = ""

    @property
    def citation(self) -> str:
        section_citation = f"{self.source_file}#{self.section}"
        if not self.item_id:
            return section_citation
        return f"{section_citation}/{self.item_id}"

    @property
    def section_citation(self) -> str:
        return f"{self.source_file}#{self.section}"


EVIDENCE_BLOCK_FUNCTIONS = {
    "award",
    "conference",
    "dated-entry",
    "education",
    "employment",
    "entry",
    "event",
    "experience",
    "grant",
    "job",
    "presentation",
    "project",
    "publication",
    "reference",
    "references",
    "research",
    "service",
    "skill",
    "skills",
    "supervision",
    "talk",
    "teaching",
}


def extract_typst_evidence(path: Path) -> list[EvidenceItem]:
    section = "Unsectioned"
    evidence: list[EvidenceItem] = []
    statement_lines: list[str] = []
    content_started = False

    def flush_statement() -> None:
        nonlocal statement_lines
        text = _clean_statement_lines(statement_lines)
        if text:
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind="statement",
                    text=text,
                )
            )
        statement_lines = []

    lines = path.read_text(encoding="utf-8").splitlines()
    index = 0
    while index < len(lines):
        raw_line = lines[index]
        line = raw_line.strip()
        if not line:
            index += 1
            continue

        section_match = re.match(r'#section\("([^"]+)"\)', line)
        if section_match:
            flush_statement()
            section = section_match.group(1)
            content_started = True
            index += 1
            continue

        heading_match = re.match(r"=+\s+(.+)", line)
        if heading_match:
            flush_statement()
            section = heading_match.group(1).strip()
            content_started = True
            index += 1
            continue

        block_name = _typst_call_name(line)
        if block_name in EVIDENCE_BLOCK_FUNCTIONS:
            flush_statement()
            block_text, index = _collect_typst_call(lines, index)
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind=block_name,
                    text=_clean_typst_inline(_strip_typst_call(block_name, block_text)),
                )
            )
            continue

        if line.startswith("#"):
            if "(" in line and _paren_delta(line) > 0:
                _, index = _collect_typst_call(lines, index)
            else:
                index += 1
            continue

        if line.startswith("+ @"):
            flush_statement()
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind="publication",
                    text=line[2:].strip(),
                )
            )
            index += 1
            continue

        if content_started and _is_statement_line(line):
            statement_lines.append(line)

        index += 1

    flush_statement()

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
    references: list[EvidenceReference] = []
    for path in _generated_evidence_paths(profile_dir):
        section = "Unsectioned"
        relative = path.relative_to(profile_dir.parent) if profile_dir.parent in path.parents else path
        for raw_line in path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if line.startswith("## "):
                section = line[3:].strip()
            elif line.startswith("- "):
                item_id, text = _parse_evidence_markdown_item(line)
                references.append(
                    EvidenceReference(
                        source_file=str(relative),
                        section=section,
                        item_id=item_id,
                        text=text,
                    )
                )
    return references


def _generated_evidence_paths(profile_dir: Path) -> list[Path]:
    manifest_paths = _manifest_generated_evidence_paths(profile_dir)
    if manifest_paths is not None:
        return manifest_paths

    generated_dir = profile_dir / "generated"
    if not generated_dir.exists():
        return []
    return sorted(generated_dir.glob("*.evidence.md"))


def _manifest_generated_evidence_paths(profile_dir: Path) -> list[Path] | None:
    manifest_path = profile_dir / "profile.yaml"
    if not manifest_path.exists():
        return None

    try:
        manifest = yaml.safe_load(manifest_path.read_text(encoding="utf-8"))
    except yaml.YAMLError:
        return None
    if not isinstance(manifest, dict):
        return None

    sources = manifest.get("sources")
    generated = manifest.get("generated")
    if sources is None and generated is None:
        return None
    if sources is not None and not isinstance(sources, dict):
        return None
    if generated is not None and not isinstance(generated, dict):
        return None

    source_map = sources or {}
    generated_map = generated or {}
    paths: list[Path] = []

    for source_key, source_value in source_map.items():
        if not str(source_value).endswith(".typ"):
            continue
        paths.append(_output_path_for_source(profile_dir, str(source_key), generated_map))

    paths.extend(profile_dir / Path(str(path)) for path in generated_map.values())

    unique_paths = sorted({path for path in paths if path.exists()})
    return unique_paths


def _typst_call_name(line: str) -> str:
    match = re.match(r"#([A-Za-z][\w-]*)\s*\(", line)
    return match.group(1) if match else ""


def _collect_typst_call(lines: list[str], start_index: int) -> tuple[str, int]:
    collected: list[str] = []
    balance = 0

    for index in range(start_index, len(lines)):
        line = lines[index].strip()
        collected.append(line)
        balance += _paren_delta(line)
        if balance <= 0:
            return "\n".join(collected), index + 1

    return "\n".join(collected), len(lines)


def _paren_delta(line: str) -> int:
    return line.count("(") - line.count(")")


def _strip_typst_call(name: str, call_text: str) -> str:
    text = call_text.strip()
    text = re.sub(rf"^#{re.escape(name)}\s*\(", "", text, count=1)
    if text.endswith(")"):
        text = text[:-1]
    return text


def _is_statement_line(line: str) -> bool:
    if line.startswith(("//", "%", "|")):
        return False
    if re.match(r"^[A-Za-z0-9_-]+\s*:", line):
        return False
    return bool(re.search(r"[A-Za-z]", line))


def _clean_statement_lines(lines: list[str]) -> str:
    cleaned: list[str] = []
    for line in lines:
        if line.startswith(("- ", "+ ")):
            line = line[2:].strip()
        cleaned.append(line)
    return _clean_typst_inline(" ".join(cleaned))


def _clean_typst_inline(value: str) -> str:
    value = value.replace("[", "").replace("]", "")
    value = value.replace('"', "")
    value = value.replace("\n", " ")
    value = re.sub(r",\s*", ", ", value)
    value = re.sub(r"\s+", " ", value)
    return value.strip(" ,")


def _output_path_for_source(profile_dir: Path, source_key: str, generated: dict[str, str]) -> Path:
    manifest_key = f"{source_key}_evidence"
    output = generated.get(manifest_key, f"generated/{source_key}.evidence.md")
    return profile_dir / output


def _evidence_markdown(source_key: str, evidence: list[EvidenceItem]) -> str:
    lines = [f"# Evidence: {source_key}", ""]
    current_section = ""

    item_prefix = _evidence_item_prefix(source_key)
    for index, item in enumerate(evidence, start=1):
        if item.section != current_section:
            current_section = item.section
            lines.extend([f"## {current_section}", ""])
        item_id = f"{item_prefix}-{index:03d}"
        lines.append(f"- [{item_id}] `{item.kind}`: {item.text}")

    if not evidence:
        lines.append("_No evidence extracted._")

    return "\n".join(lines).rstrip() + "\n"


def _evidence_item_prefix(source_key: str) -> str:
    prefix = re.sub(r"[^A-Za-z0-9]+", "-", source_key).strip("-").lower()
    return prefix or "item"


def _parse_evidence_markdown_item(line: str) -> tuple[str, str]:
    item_match = re.match(r"- \[([^\]]+)\]\s+(.*)", line)
    if item_match:
        return item_match.group(1).strip(), item_match.group(2).strip()
    return "", line[2:].strip()
