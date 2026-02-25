#!/usr/bin/env python3
"""
check_dep_graph.py — Enforce the allowed crate dependency graph.

Reads `cargo tree --workspace --edges normal` output and fails if any
dependency edge violates the layered architecture defined in AGENTS.md.

Usage:
    cargo tree --workspace --edges normal | python3 scripts/check_dep_graph.py
"""

import sys
import re

# Allowed dependency edges: (dependent_crate, dependency_crate)
# A crate may ONLY depend on crates listed as its allowed dependencies.
ALLOWED_DEPS: dict[str, set[str]] = {
    "sixg-common": set(),  # no deps — foundation layer
    "sixg-ai": {"sixg-common"},
    "sixg-ntn": {"sixg-common"},
    "sixg-pdcp": {"sixg-common"},
    "sixg-rlc": {"sixg-common"},
    "sixg-phy": {"sixg-common", "sixg-ai"},
    "sixg-semantic": {"sixg-ai", "sixg-common"},
    "sixg-isac": {"sixg-phy", "sixg-common", "sixg-ai"},
    "sixg-mac": {"sixg-phy", "sixg-common", "sixg-ai"},
    "sixg-rrc": {"sixg-mac", "sixg-pdcp", "sixg-rlc", "sixg-common"},
    "sixg-core": {"sixg-rrc", "sixg-common", "sixg-ai", "sixg-ntn", "sixg-semantic"},
    # Top-level binary may depend on everything
    "sixg": {
        "sixg-common", "sixg-phy", "sixg-mac", "sixg-rlc", "sixg-pdcp",
        "sixg-rrc", "sixg-isac", "sixg-ai", "sixg-ntn", "sixg-semantic",
        "sixg-core",
    },
}

# Third-party crates that are always allowed as dependencies
THIRD_PARTY_PREFIXES = (
    "thiserror", "serde", "tokio", "tracing", "log", "anyhow",
    "rand", "rayon", "num", "itertools",
)


def is_third_party(name: str) -> bool:
    return any(name.startswith(p) for p in THIRD_PARTY_PREFIXES)


def parse_cargo_tree(lines: list[str]) -> list[tuple[str, str]]:
    """
    Parse `cargo tree` output and extract (parent_crate, dep_crate) edges
    for workspace crates only.

    cargo tree indents with spaces/pipes to show depth; we track the current
    stack to determine the parent at each level.
    """
    edges: list[tuple[str, str]] = []
    # Stack of (indent_level, crate_name)
    stack: list[tuple[int, str]] = []

    crate_re = re.compile(r"^([\s│├└─ ]*)(sixg-\w+)\s")

    for line in lines:
        m = crate_re.match(line)
        if not m:
            continue
        prefix = m.group(1)
        crate = m.group(2)
        # Compute depth from the prefix length (each level adds characters)
        depth = len(prefix) // 4 if prefix else 0

        # Pop the stack back to the parent level
        while stack and stack[-1][0] >= depth:
            stack.pop()

        if stack:
            parent = stack[-1][1]
            edges.append((parent, crate))

        stack.append((depth, crate))

    return edges


def main() -> int:
    lines = sys.stdin.readlines()
    edges = parse_cargo_tree(lines)

    violations: list[str] = []
    for parent, dep in edges:
        if is_third_party(dep) or is_third_party(parent):
            continue
        allowed = ALLOWED_DEPS.get(parent)
        if allowed is None:
            # Unknown crate — skip (may be a test crate or example)
            continue
        if dep not in allowed and dep != parent:
            violations.append(
                f"  VIOLATION: '{parent}' depends on '{dep}' — "
                f"not in allowed set {sorted(allowed)}"
            )

    if violations:
        print("Dependency graph check FAILED:")
        for v in violations:
            print(v)
        print("\nSee AGENTS.md for the allowed dependency graph.")
        return 1

    print(f"Dependency graph check passed ({len(edges)} edges verified).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
