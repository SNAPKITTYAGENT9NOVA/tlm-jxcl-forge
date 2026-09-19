#!/usr/bin/env python3
"""Regenerates docs/CRATE_REGISTRY.md (human-readable) from docs/crates.toml."""
import tomllib
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CAT_ORDER = [
    "foundation", "isa", "execution", "memory", "toolchain", "debug",
    "hardware", "crypto", "storage", "proof", "network", "security",
]
CAT_TITLE = {
    "foundation": "Foundation", "isa": "ISA", "execution": "Execution",
    "memory": "Memory", "toolchain": "Binary/Toolchain",
    "debug": "Debug/Simulation", "hardware": "Hardware/RTL",
    "crypto": "Cryptography", "storage": "Storage/Data",
    "proof": "Zero-Knowledge/Proof", "network": "Network/Service",
    "security": "Security/Observability/Integration",
}


def main():
    data = tomllib.loads((REPO_ROOT / "docs" / "crates.toml").read_text())
    crates = data["crate"]
    by_cat = defaultdict(list)
    for c in crates:
        by_cat[c["category"]].append(c)

    lines = [
        "# Crate Registry (human-readable index)\n",
        "Generated view of `docs/crates.toml`, the authoritative machine-readable",
        "registry. Regenerate with `python3 tools/gen_crate_registry_doc.py`.\n",
        f"**Total: {len(crates)} crates.**\n",
    ]

    for cat in CAT_ORDER:
        items = sorted(by_cat[cat], key=lambda x: x["name"])
        lines.append(f"## {CAT_TITLE[cat]} ({len(items)})\n")
        for c in items:
            lines.append(f"### `{c['name']}`  _{c['source']}_")
            lines.append(f"{c['purpose']}")
            lines.append(f"- **Owns:** {c['owns']}")
            lines.append(f"- **Public API:** {', '.join(c['public_api'])}")
            lines.append(f"- **Tests:** {', '.join(c['tests'])}")
            lines.append("")

    (REPO_ROOT / "docs" / "CRATE_REGISTRY.md").write_text("\n".join(lines))
    print(f"wrote docs/CRATE_REGISTRY.md, {len(lines)} lines")


if __name__ == "__main__":
    main()
