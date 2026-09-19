#!/usr/bin/env python3
"""Mass-scaffolds the 94 new crates from docs/crates.toml.

Skips the 6 pre-expansion crates entirely for Cargo.toml/src -- those
already have real, tested code and must not be overwritten (only a
README.md is added if missing). For every other registry entry, this
generates a compiling skeleton (Cargo.toml with real path dependencies
matching the registry's DAG, a placeholder src/lib.rs, and a README.md)
so `cargo check --workspace` is green immediately, before real
implementation lands per docs/CRATE_GENERATION_PLAN.md's batches.

Also rewrites the root Cargo.toml's [workspace] members list to
contain exactly the 100 registry crates (preserving any members not in
the registry is not expected -- the registry IS the member list).

Idempotent: re-running only creates files that don't already exist; it
never overwrites a crate's Cargo.toml/lib.rs/main.rs once real
implementation has replaced the placeholder (detected by the presence
of the file at all -- once a batch lands real content, re-running this
script leaves it untouched).
"""
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
REGISTRY_PATH = REPO_ROOT / "docs" / "crates.toml"
CRATES_DIR = REPO_ROOT / "crates"

PRE_EXISTING = {
    "jxcl", "pq-crypto", "pq-cache", "pq-sql-vault", "pq-error-proof",
    "photo-cache-service",
}


def toml_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def cargo_toml_for(c: dict) -> str:
    lines = [
        "[package]",
        f'name = "{c["name"]}"',
        'version = "0.1.0"',
        'edition = "2021"',
        f'description = "{toml_escape(c["purpose"])}"',
        'license = "MIT"',
        "",
        "[dependencies]",
    ]
    for dep in c.get("dependencies", []):
        lines.append(f'{dep} = {{ path = "../{dep}" }}')
    if not c.get("dependencies"):
        lines.append("# (none yet -- external dependencies for this crate's")
        lines.append("# real implementation are added when that batch lands,")
        lines.append("# per docs/crates.toml's external_dependencies field)")
    lines.append("")
    return "\n".join(lines)


def lib_rs_for(c: dict) -> str:
    api = ", ".join(c.get("public_api", [])) or "(none yet)"
    return f'''//! {c["purpose"]}
//!
//! Owns: {c["owns"]}
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `{c["category"]}` category. Planned public API: {api}.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {{
    #[test]
    fn scaffold_placeholder() {{
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // {", ".join(c.get("tests", []))}.
    }}
}}
'''


def readme_for(c: dict) -> str:
    deps = "\n".join(f"- `{d}`" for d in c.get("dependencies", [])) or "*(none)*"
    ext = "\n".join(f"- `{d}`" for d in c.get("external_dependencies", [])) or "*(none)*"
    return f'''# {c["name"]}

{c["purpose"]}

## Architecture

**Owns:** {c["owns"]}

**Category:** {c["category"]} · **Source:** {c["source"]}

## Public API

{", ".join(f"`{a}`" for a in c.get("public_api", []))}

## Dependencies

Workspace crates:

{deps}

External crates:

{ext}

## Testing

Planned test kinds: {", ".join(c.get("tests", []))}.

## Status

`{c["status"]}` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
'''


def main():
    data = tomllib.loads(REGISTRY_PATH.read_text())
    crates = data["crate"]
    created, skipped_existing, readme_added = [], [], []

    for c in crates:
        name = c["name"]
        crate_dir = CRATES_DIR / name

        if name in PRE_EXISTING:
            skipped_existing.append(name)
            readme_path = crate_dir / "README.md"
            if not readme_path.exists():
                readme_path.write_text(readme_for(c))
                readme_added.append(name)
            continue

        crate_dir.mkdir(parents=True, exist_ok=True)
        (crate_dir / "src").mkdir(exist_ok=True)

        manifest = crate_dir / "Cargo.toml"
        if not manifest.exists():
            manifest.write_text(cargo_toml_for(c))

        lib_rs = crate_dir / "src" / "lib.rs"
        if not lib_rs.exists():
            lib_rs.write_text(lib_rs_for(c))

        readme = crate_dir / "README.md"
        if not readme.exists():
            readme.write_text(readme_for(c))

        created.append(name)

    # Rewrite the root workspace member list to match the registry exactly.
    root_manifest_path = REPO_ROOT / "Cargo.toml"
    root_manifest = root_manifest_path.read_text()
    members_block = "\n".join(f'    "crates/{c["name"]}",' for c in sorted(crates, key=lambda x: x["name"]))
    new_members_section = f'[workspace]\nresolver = "2"\nmembers = [\n{members_block}\n]'
    import re
    new_manifest = re.sub(
        r'\[workspace\]\nresolver = "2"\nmembers = \[.*?\]',
        new_members_section,
        root_manifest,
        flags=re.S,
    )
    root_manifest_path.write_text(new_manifest)

    print(f"Created {len(created)} new crate scaffolds.")
    print(f"Skipped {len(skipped_existing)} pre-existing crates (README added to {len(readme_added)} of them).")
    print(f"Rewrote root Cargo.toml workspace members ({len(crates)} entries).")


if __name__ == "__main__":
    main()
