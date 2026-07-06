#!/usr/bin/env python3
"""Split styles.rs DESIGN_SYSTEM_CSS into styles/*.css fragments."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent  # burncloud-client-shared crate root
styles_rs = ROOT / "src" / "styles.rs"
out_dir = ROOT / "src" / "styles"

src = styles_rs.read_text(encoding="utf-8")
m = re.search(r'pub const DESIGN_SYSTEM_CSS: &str = r#"(.*)"#;\s*$', src, re.S)
if not m:
    raise SystemExit("Could not parse DESIGN_SYSTEM_CSS")
css = m.group(1)

parts = re.split(r"(?=/\* ═{10,})", css)
parts = [p for p in parts if p.strip()]

out_dir.mkdir(exist_ok=True)
files: list[str] = []
for i, part in enumerate(parts):
    title_m = re.search(r"/\* ═+\s*\n\s*(.+?)\s*\n\s*═", part, re.S)
    title = title_m.group(1).strip() if title_m else f"section_{i}"
    slug = re.sub(r"[^a-z0-9]+", "_", title.lower())[:48].strip("_")
    fname = f"{i:02d}_{slug}.css"
    files.append(fname)
    (out_dir / fname).write_text(part.lstrip("\n"), encoding="utf-8")
    print(f"{fname}: {len(part)} — {title[:70]}")

print(f"Wrote {len(files)} files to {out_dir}")
