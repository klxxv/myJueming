#!/usr/bin/env python3
"""Export transparent SVG slices from authored masters; --check verifies drift."""

import argparse
import copy
import html
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "apps/desktop/src/assets/companion/puppets/v1"
NS = "http://www.w3.org/2000/svg"
ET.register_namespace("", NS)


def export(actor: Path, check: bool) -> int:
    rig = json.loads((actor / "rig.json").read_text())
    master = ET.parse(actor / "master.svg").getroot()
    groups = {node.attrib["data-part"]: node for node in master if "data-part" in node.attrib}
    bone_ids = set()
    rest_positions = {}
    for bone in rig["bones"]:
        assert bone["id"] not in bone_ids, "Duplicate bone"
        assert bone["parent"] is None or bone["parent"] in bone_ids, "Parent must precede child"
        bone_ids.add(bone["id"])
        parent = rest_positions.get(bone["parent"], rig["origin"])
        rest_positions[bone["id"]] = [parent[0] + bone["x"], parent[1] + bone["y"]]
    assert len(groups) == len(rig["parts"]), "Master and manifest disagree"
    assert len({p["id"] for p in rig["parts"]}) == len(rig["parts"]), "Duplicate part"
    products = {}
    cells = []
    for index, part in enumerate(rig["parts"]):
        assert part["bone"] in bone_ids, f"Missing bone for {part['id']}"
        assert part["file"] == f"parts/{part['id']}.svg", "Slices must remain in parts/"
        assert set(part.get("expressions", [])) <= set(rig["expressions"])
        source = groups[part["id"]]
        assert source.attrib["data-bone"] == part["bone"], "Master binding mismatch"
        x, y = rest_positions[part["bone"]]
        assert source.attrib["transform"] == f"translate({x} {y})", "Master and rig rest positions disagree"
        fragment = "\n".join(ET.tostring(copy.deepcopy(n), encoding="unicode") for n in source if n.tag != f"{{{NS}}}title")
        # Explicit negative viewBox coordinates preserve the local joint at (0, 0).
        bounds = " ".join(map(str, part["bounds"]))
        assert part["bounds"][2] > 0 and part["bounds"][3] > 0
        title = html.escape(part["label"])
        products[actor / part["file"]] = (
            f'<svg xmlns="{NS}" viewBox="{bounds}" fill="none" role="img" aria-label="{title}">\n'
            f"  <title>{title}</title>\n{fragment}\n</svg>\n"
        )
        x, y = 26 + (index % 5) * 196, 112 + (index // 5) * 160
        bx, by, bw, bh = part["bounds"]
        scale = min(145 / bw, 91 / bh, 1.45)
        tx, ty = x + 89 - (bx + bw / 2) * scale, y + 49 - (by + bh / 2) * scale
        cells.append(
            f'<g><rect x="{x}" y="{y}" width="180" height="145" rx="12" fill="#FFFDF7" stroke="#DCDACB"/>'
            f'<g transform="translate({tx} {ty}) scale({scale})">{fragment}'
            '<circle cx="0" cy="0" r="3" fill="#397C64" stroke="#FFFFFF" stroke-width="1"/></g>'
            f'<text x="{x+12}" y="{y+117}" font-size="12" fill="#4C5549">{title}</text>'
            f'<text x="{x+12}" y="{y+133}" font-size="10" fill="#777F72">{part["id"]}</text></g>'
        )
    height = 132 + ((len(rig["parts"]) + 4) // 5) * 160
    products[actor / "parts-sheet.svg"] = (
        f'<svg xmlns="{NS}" viewBox="0 0 1020 {height}" fill="none" font-family="system-ui, sans-serif" role="img" aria-label="{rig["label"]}切片总览">'
        f'<rect width="1020" height="{height}" fill="#F1F1E7"/>'
        f'<text x="30" y="49" font-size="25" font-weight="600" fill="#304D3C">{rig["label"]} / 部件图谱</text>'
        f'<text x="30" y="80" font-size="13" fill="#66705E">{len(rig["parts"])} 个透明切片 · 绿点为局部旋转原点 · v1</text>'
        + "\n".join(cells) + "</svg>\n"
    )
    for path, text in products.items():
        if check:
            assert path.exists() and path.read_text() == text, f"Stale export: {path.relative_to(ROOT)}"
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
    actual = {p.name for p in (actor / "parts").glob("*.svg")}
    expected = {Path(p["file"]).name for p in rig["parts"]}
    assert actual == expected, "Unexpected slice files; reconcile explicitly instead of deleting silently"
    print(f"{actor.name}: {len(rig['parts'])} slices, master bindings and exports {'verified' if check else 'exported'}")
    return len(rig["parts"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for directory in sorted(ASSETS.iterdir()):
        if (directory / "rig.json").exists():
            export(directory, args.check)
