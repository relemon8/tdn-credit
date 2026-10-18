#!/usr/bin/env python3
"""Development/release helper; Python is not required to play the game."""
import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
meta = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root))
output = root / "THIRD_PARTY_NOTICES.md"
lines = ["# Third-party notices\n", "Generated from Cargo.lock. Includes conditional/build dependencies.\n",
         "License texts are reproduced below where supplied in the crate root. SQLite is public domain.\n"]
lines += ["\n## Toolchain runtimes\n", "Rust standard library: MIT/Apache-2.0. Linux static builds include musl libc. LLVM compiler-runtime portions use Apache-2.0 with LLVM exceptions. Original license texts follow.\n"]
for file in sorted((root / "assets" / "licenses").glob("*.txt")):
    lines += [f"\n### {file.name}\n", "```text\n", file.read_text(), "\n```\n"]
for package in sorted(meta["packages"], key=lambda p: (p["name"], p["version"])):
    if package["source"] is None:
        continue
    lines += [f'\n## {package["name"]} {package["version"]}\n', f'License expression: {package.get("license", "UNKNOWN")}\n']
    directory = pathlib.Path(package["manifest_path"]).parent
    files = sorted(p for p in directory.iterdir() if p.is_file() and
                   (p.name.upper().startswith(("LICENSE", "COPYING", "NOTICE")) or p.name.lower() == "unlicense"))
    for file in files:
        lines += [f'\n### {file.name}\n', "```text\n", file.read_text(errors="replace"), "\n```\n"]
output.write_text("\n".join(lines))
print(f"Wrote {output.name}: {len(meta['packages'])} package records")
