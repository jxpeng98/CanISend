from __future__ import annotations

from pathlib import Path
import shutil


REPO_ROOT = Path(__file__).resolve().parents[1]
CANONICAL_MAIN_SKILL = REPO_ROOT / "skills" / "canisend"
COMPATIBILITY_MIRROR = REPO_ROOT / "agent-skills" / "canisend"


def main() -> int:
    """Refresh the one-release compatibility mirror from the canonical main skill."""
    shutil.copytree(CANONICAL_MAIN_SKILL, COMPATIBILITY_MIRROR, dirs_exist_ok=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
