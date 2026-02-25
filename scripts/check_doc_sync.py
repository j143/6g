#!/usr/bin/env python3
"""
check_doc_sync.py — Verify that every pub struct and pub enum defined in
crates/ is mentioned in the corresponding docs/<crate>.md file.

Usage:
    python3 scripts/check_doc_sync.py

Exit code 0 if all checks pass, 1 if any public type is undocumented.
"""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent
CRATES_DIR = REPO_ROOT / "crates"
DOCS_DIR = REPO_ROOT / "docs"

# Pattern to find public structs and enums in Rust source files
PUB_TYPE_RE = re.compile(r"^pub\s+(?:struct|enum)\s+(\w+)", re.MULTILINE)

# Crate directory name → docs file name mapping
CRATE_TO_DOC: dict[str, str] = {
    "6g-common": "6g-common.md",
    "6g-phy": "6g-phy.md",
    "6g-isac": "6g-isac.md",
    "6g-mac": "6g-mac.md",
    "6g-ai": "6g-ai.md",
    "6g-ntn": "6g-ntn.md",
    "6g-pdcp": "6g-pdcp.md",
    "6g-rlc": "6g-rlc.md",
    "6g-rrc": "6g-rrc.md",
    "6g-core": "6g-core.md",
    "6g-semantic": "6g-semantic.md",
}

# Types that are intentionally not required to appear in docs
# (private/internal implementation details surfaced as pub for trait impls, etc.)
IGNORED_TYPES = {
    "ValidationCheck",
    "ValidationResult",
    "Error",
}


def collect_pub_types(crate_dir: Path) -> list[tuple[Path, str]]:
    """Return (source_file, type_name) for all pub structs/enums in a crate."""
    results = []
    for rs_file in (crate_dir / "src").rglob("*.rs"):
        source = rs_file.read_text(encoding="utf-8")
        for match in PUB_TYPE_RE.finditer(source):
            type_name = match.group(1)
            if type_name not in IGNORED_TYPES:
                results.append((rs_file, type_name))
    return results


def main() -> int:
    missing: list[str] = []

    for crate_name, doc_name in CRATE_TO_DOC.items():
        crate_dir = CRATES_DIR / crate_name
        doc_file = DOCS_DIR / doc_name

        if not crate_dir.exists():
            continue
        if not doc_file.exists():
            print(f"WARNING: docs/{doc_name} does not exist — skipping {crate_name}")
            continue

        doc_content = doc_file.read_text(encoding="utf-8")
        pub_types = collect_pub_types(crate_dir)

        for src_file, type_name in pub_types:
            if type_name not in doc_content:
                rel_src = src_file.relative_to(REPO_ROOT)
                missing.append(
                    f"  {type_name} (in {rel_src}) not mentioned in docs/{doc_name}"
                )

    if missing:
        print("Doc-code sync check FAILED — these public types are undocumented:")
        for m in missing:
            print(m)
        print(
            "\nAdd a mention of each type to the corresponding docs/<crate>.md file."
        )
        return 1

    print(f"Doc-code sync check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
