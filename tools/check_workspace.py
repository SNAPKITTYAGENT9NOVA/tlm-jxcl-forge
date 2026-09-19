#!/usr/bin/env python3
"""Workspace validator for the 100-crate tlm-jxcl-forge expansion.

Deliberately a standalone script (not a workspace member crate) so it
has no chicken-and-egg dependency on the workspace it validates, and so
it doesn't perturb the "exactly 100 crates" count it's checking for.
Run with: python3 tools/check_workspace.py

Gates implemented (see docs/CRATE_ARCHITECTURE.md rule references):
  - exactly 100 crates registered
  - no duplicate package names
  - every dependency target exists in the registry
  - no dependency cycles
  - no forbidden category edges (ISA/execution/memory/toolchain/debug/
    hardware must never depend on storage or network category crates)
  - [when --disk is passed] every registered crate has a manifest,
    lib.rs/main.rs, and README on disk, and its Cargo.toml
    [dependencies] matches its registry path-dependency list
"""
import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
REGISTRY_PATH = REPO_ROOT / "docs" / "crates.toml"
EXPECTED_CRATES = 100

# Categories that make up the "ISA/execution kernel" layer per
# docs/CRATE_ARCHITECTURE.md's dependency-direction diagram. These must
# never reach up into storage or network concerns.
LOW_LEVEL_CATEGORIES = {
    "foundation",
    "isa",
    "execution",
    "memory",
    "toolchain",
    "debug",
    "hardware",
}
FORBIDDEN_TARGET_CATEGORIES = {"storage", "network"}


def load_registry():
    data = tomllib.loads(REGISTRY_PATH.read_text())
    crates = data.get("crate", [])
    by_name = {}
    for c in crates:
        name = c["name"]
        if name in by_name:
            raise SystemExit(f"FAIL: duplicate crate name in registry: {name}")
        by_name[name] = c
    return by_name


def check_count(by_name):
    actual = len(by_name)
    ok = actual == EXPECTED_CRATES
    print(f"[{'OK' if ok else 'FAIL'}] crate count: expected {EXPECTED_CRATES}, got {actual}")
    return ok


def check_dependency_targets(by_name):
    missing = []
    for name, c in by_name.items():
        for dep in c.get("dependencies", []):
            if dep not in by_name:
                missing.append((name, dep))
    ok = not missing
    print(f"[{'OK' if ok else 'FAIL'}] all dependency targets exist" + (f": missing {missing}" if missing else ""))
    return ok


def check_no_cycles(by_name):
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {n: WHITE for n in by_name}
    cycles = []

    def dfs(node, stack):
        color[node] = GRAY
        stack.append(node)
        for dep in by_name[node].get("dependencies", []):
            if dep not in by_name:
                continue
            if color[dep] == GRAY:
                cycles.append(stack[stack.index(dep):] + [dep])
            elif color[dep] == WHITE:
                dfs(dep, stack)
        stack.pop()
        color[node] = BLACK

    for n in by_name:
        if color[n] == WHITE:
            dfs(n, [])

    ok = not cycles
    print(f"[{'OK' if ok else 'FAIL'}] no dependency cycles" + (f": {cycles}" if cycles else ""))
    return ok


def check_forbidden_edges(by_name):
    violations = []
    for name, c in by_name.items():
        if c["category"] not in LOW_LEVEL_CATEGORIES:
            continue
        for dep in c.get("dependencies", []):
            dep_category = by_name.get(dep, {}).get("category")
            if dep_category in FORBIDDEN_TARGET_CATEGORIES:
                violations.append((name, dep, dep_category))
    ok = not violations
    print(f"[{'OK' if ok else 'FAIL'}] no ISA/execution/kernel -> storage/network edges" + (f": {violations}" if violations else ""))
    return ok


def check_disk(by_name):
    problems = []
    for name, c in by_name.items():
        crate_dir = REPO_ROOT / "crates" / name
        manifest = crate_dir / "Cargo.toml"
        readme = crate_dir / "README.md"
        lib_rs = crate_dir / "src" / "lib.rs"
        main_rs = crate_dir / "src" / "main.rs"
        if not manifest.exists():
            problems.append(f"{name}: missing Cargo.toml")
            continue
        if not readme.exists():
            problems.append(f"{name}: missing README.md")
        if not lib_rs.exists() and not main_rs.exists():
            problems.append(f"{name}: missing src/lib.rs and src/main.rs")

        manifest_text = manifest.read_text()
        m = re.search(r"\[dependencies\](.*?)(\n\[|\Z)", manifest_text, re.S)
        deps_section = m.group(1) if m else ""
        expected_path_deps = {d for d in c.get("dependencies", [])}
        for dep in expected_path_deps:
            if dep not in deps_section:
                problems.append(f"{name}: Cargo.toml does not reference registry dependency '{dep}'")

    ok = not problems
    print(f"[{'OK' if ok else 'FAIL'}] on-disk scaffolding matches registry" + ("" if ok else ""))
    for p in problems:
        print(f"    - {p}")
    return ok


def check_root_workspace_members(by_name):
    root_manifest = (REPO_ROOT / "Cargo.toml").read_text()
    m = re.search(r"members\s*=\s*\[(.*?)\]", root_manifest, re.S)
    members = set(re.findall(r'"crates/([^"]+)"', m.group(1))) if m else set()
    registry_names = set(by_name.keys())
    missing_from_workspace = registry_names - members
    orphaned_in_workspace = members - registry_names
    ok = not missing_from_workspace and not orphaned_in_workspace
    print(f"[{'OK' if ok else 'FAIL'}] workspace Cargo.toml members == registry"
          + (f": missing_from_workspace={missing_from_workspace} orphaned_in_workspace={orphaned_in_workspace}" if not ok else ""))
    return ok


def main():
    check_disk_flag = "--disk" in sys.argv
    by_name = load_registry()
    results = [
        check_count(by_name),
        check_dependency_targets(by_name),
        check_no_cycles(by_name),
        check_forbidden_edges(by_name),
        check_root_workspace_members(by_name),
    ]
    if check_disk_flag:
        results.append(check_disk(by_name))

    if all(results):
        print("\nALL GATES PASSED")
        sys.exit(0)
    else:
        print("\nGATE FAILURE")
        sys.exit(1)


if __name__ == "__main__":
    main()
