from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re


@dataclass(frozen=True)
class EvidenceItem:
    source_file: str
    section: str
    kind: str
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


def _clean_typst_inline(value: str) -> str:
    value = value.replace("[", "").replace("]", "")
    value = value.replace('"', "")
    value = re.sub(r"\s+", " ", value)
    return value.strip()
