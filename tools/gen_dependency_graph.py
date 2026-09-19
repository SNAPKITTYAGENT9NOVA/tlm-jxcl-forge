#!/usr/bin/env python3
"""Regenerates docs/DEPENDENCY_GRAPH.md from docs/crates.toml.

Keeps the dependency-graph documentation from drifting out of sync
with the registry it's supposed to describe -- run this instead of
hand-editing docs/DEPENDENCY_GRAPH.md whenever docs/crates.toml changes.
"""
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

LAYERING_DIAGRAM = """## Layering (high level)

```
                  applications
            (jxcl-cli, photo-cache-service)
                        |
                     services
        (jxcl-service, jxcl-rpc, jxcl-integration)
                        |
              integration / runtime
     (jxcl-simulator, jxcl-protocol, jxcl-runtime)
                        |
     crypto / storage / proof / network primitives
 (pq-*, jxcl-network, jxcl-http -- isolated from ISA internals)
                        |
              ISA / execution kernel
  (jxcl-execution, jxcl-memory, jxcl-assembler, jxcl-hardware, ...)
                        |
                    primitives
      (jxcl-types, jxcl-errors, jxcl-bitops, jxcl-endian, ...)
```

Enforced mechanically: no crate in `foundation`, `isa`, `execution`,
`memory`, `toolchain`, `debug`, or `hardware` may depend on a crate in
`storage` or `network` (`tools/check_workspace.py`'s
`check_forbidden_edges`). Crypto/proof crates depend only on
`std` + audited crates.io crates, never on service-layer crates.
"""


def main():
    data = tomllib.loads((REPO_ROOT / "docs" / "crates.toml").read_text())
    crates = data["crate"]
    by_cat = defaultdict(list)
    for c in crates:
        by_cat[c["category"]].append(c)

    lines = [
        "# Dependency Graph\n",
        "Generated from `docs/crates.toml` -- do not hand-edit; regenerate with",
        "`python3 tools/gen_dependency_graph.py` if the registry changes.\n",
        "Validated acyclic and free of forbidden ISA-to-storage/network edges by",
        "`tools/check_workspace.py` (see its output in `docs/BUILD_MATRIX.md`).\n",
    ]

    for cat in CAT_ORDER:
        lines.append(f"## {CAT_TITLE[cat]} ({len(by_cat[cat])})\n")
        lines.append("| Crate | Depends on | External deps |")
        lines.append("|---|---|---|")
        for c in sorted(by_cat[cat], key=lambda x: x["name"]):
            deps = ", ".join(f"`{d}`" for d in c.get("dependencies", [])) or "*(none)*"
            ext = ", ".join(f"`{d}`" for d in c.get("external_dependencies", [])) or "*(none)*"
            lines.append(f"| `{c['name']}` | {deps} | {ext} |")
        lines.append("")

    lines.append(LAYERING_DIAGRAM)

    (REPO_ROOT / "docs" / "DEPENDENCY_GRAPH.md").write_text("\n".join(lines))
    print(f"wrote docs/DEPENDENCY_GRAPH.md, {len(lines)} lines")


if __name__ == "__main__":
    main()
