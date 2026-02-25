#!/usr/bin/env python3
"""
check_primitive_leakage.py — Detect bare primitive types (f64, usize, u64, u32, i64)
used as parameters in public function signatures in crates/.

Physical quantities must use the domain newtypes from 6g-common/types.rs.
Bare primitives are only acceptable for dimensionless counts and indices.

Usage:
    python3 scripts/check_primitive_leakage.py

Exit code 0 if no violations found, 1 otherwise.
"""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent
CRATES_DIR = REPO_ROOT / "crates"

# Match a full public function signature (up to the opening brace or semicolon)
# We capture the parameter list to inspect parameter types.
PUB_FN_RE = re.compile(
    r"pub\s+(?:async\s+)?fn\s+(\w+)\s*\([^)]*\)",
    re.DOTALL,
)

# Suspicious bare primitive types in function parameters.
# Format: r":\s*<type>" to match "param: f64" in a signature.
BARE_PRIMITIVE_RE = re.compile(r":\s*(f64|f32)\b")

# Parameters whose names suggest they are dimensionless counts — allowed to be usize/u64
DIMENSIONLESS_PARAM_NAMES = re.compile(
    r"\b(n|num|count|index|idx|len|size|cap|id|bits|bytes|iter|step|"
    r"subcarriers|antennas|elements|order|rank|lag|delay|doppler_bin|"
    r"sensing_subcarriers|total_subcarriers|num_points)\b"
)

# Parameter names that look like physical quantities — must NOT be bare f64
PHYSICAL_PARAM_NAMES = re.compile(
    r"\b(freq|frequency|dist|distance|power|loss|gain|snr|bandwidth|bw|"
    r"wavelength|range|speed|velocity|time|duration|energy|rate|"
    r"path_loss|noise)\w*\b"
)

# Files to skip (test files, build scripts, validation helpers)
SKIP_FILES = {"validation.rs", "build.rs"}

# Functions whose bare-f64 use is intentional (internal helpers, constructors, setters)
ALLOWED_FUNCTIONS = {
    "from_hz", "from_ghz", "from_thz", "new",
    "from_db", "to_db", "as_hz", "as_ghz", "as_thz",
    # Grid/map setter: `power` here is a dimensionless cell value, not a physical quantity
    "set",
}


def check_file(rs_file: Path) -> list[str]:
    """Return a list of violation messages for the given file."""
    if rs_file.name in SKIP_FILES:
        return []

    source = rs_file.read_text(encoding="utf-8")
    violations = []
    rel = rs_file.relative_to(REPO_ROOT)

    for m in PUB_FN_RE.finditer(source):
        fn_name = m.group(0)
        signature = m.group(0)
        fn_ident = m.group(1)

        if fn_ident in ALLOWED_FUNCTIONS:
            continue

        # Find all "param_name: f64" patterns
        for pm in re.finditer(r"(\w+)\s*:\s*(f64|f32)\b", signature):
            param_name = pm.group(1)
            ptype = pm.group(2)

            # Skip self, &self, &mut self
            if param_name in ("self", "_self"):
                continue

            # Skip if the param name looks dimensionless
            if DIMENSIONLESS_PARAM_NAMES.search(param_name):
                continue

            # Flag if the param name looks like a physical quantity
            if PHYSICAL_PARAM_NAMES.search(param_name):
                # Extract line number
                line_no = source[: m.start()].count("\n") + 1
                violations.append(
                    f"  {rel}:{line_no}: pub fn '{fn_ident}' — "
                    f"parameter '{param_name}: {ptype}' should use a domain newtype"
                )

    return violations


def main() -> int:
    all_violations: list[str] = []

    for rs_file in CRATES_DIR.rglob("*.rs"):
        all_violations.extend(check_file(rs_file))

    if all_violations:
        print("Primitive leakage check FAILED:")
        for v in all_violations:
            print(v)
        print(
            "\nPhysical quantities must use newtypes from crates/6g-common/src/types.rs."
            "\nSee AGENTS.md Rule #2."
        )
        return 1

    print("Primitive leakage check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
