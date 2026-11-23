#!/usr/bin/env python3
"""Generate the Verso tray marks with rsvg-convert: a V wrapped by a wide S whose ends are opposite arrows
(logo "S-flow", the same drawing as src-tauri/icons/source/mark.svg).

The glyph is drawn once on a 36-unit grid (the @2x pixel grid of the 18 pt macOS status item) and the same
geometry is rendered at every size, so the artwork is identical on both platforms. Only the @2x files are
embedded (src-tauri/src/tray.rs); the 1x files are the same drawing at half size.

Outputs, in src-tauri/icons/tray/:
  tray-{S}.png, tray-{S}@2x.png              black glyph (macOS template image, light Windows taskbar)
  tray-{S}-light.png, tray-{S}-light@2x.png  white glyph for dark Windows taskbars
  tray-{S}-busy*.png                         the translating state: the S-arrow alone, without the V
with S = 18 (macOS, 36 px @2x) and 16 (Windows, 32 px @2x).

The glyph spans about 4..32 units across and 1.5..34.5 down (0.1.7: 3.6..32.4; 0.1.5: 5.6..30.4). The macOS status
item is a fixed 18 pt high, so the only way to a larger mark is less padding inside the canvas (spec 004 R12):
the S stroke is 2.2 and the V is 22 units tall with 5-unit arms.

`--sheet` also writes target/tray-compare.png: 0.1.7 beside the new mark, on light and dark backgrounds,
at 36 px and at 32/24 px, for the owner's visual judgement (SC-007).
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src-tauri" / "icons" / "tray"

GRID = 36
GAP = 1.0  # clearance cut out of the S around the V
# Nominal size -> pixels at @2x.
SIZES = {18: 36, 16: 32}

HEAD = 2.4
ARC_L, ARC_R = 10, 26  # centres of the two bowls
TIP_R, TIP_L = 31.5, 4.5

# 0.1.7 drawing, kept for the comparison sheet.
OLD = {
    "s_w": 2.0,
    "top": 6,
    "bot": 30,
    "v": "11.6,9 15.6,9 18,18.6 20.4,9 24.4,9 20,27 16,27",
}
# 0.1.8 drawing: the glyph fills ~1.5..34.5 of the 36-unit canvas.
NEW = {
    "s_w": 2.2,
    "top": 5.0,
    "bot": 31.0,
    "v": "11,7 16,7 18,19.8 20,7 25,7 20.6,29 15.4,29",
}


def s_path(top: float, bot: float) -> str:
    mid = (top + bot) / 2
    r = (mid - top) / 2
    return (
        f"M{TIP_R} {top}H{ARC_L}A{r} {r} 0 0 0 {ARC_L} {mid}H{ARC_R}A{r} {r} 0 0 1 {ARC_R} {bot}H{TIP_L}"
        f"M{TIP_R - HEAD} {top - HEAD}L{TIP_R} {top}L{TIP_R - HEAD} {top + HEAD}"
        f"M{TIP_L + HEAD} {bot - HEAD}L{TIP_L} {bot}L{TIP_L + HEAD} {bot + HEAD}"
    )


def body(colour: str, busy: bool, geo: dict, ident: str) -> str:
    """The glyph's SVG elements on the 36-unit grid (no <svg> wrapper)."""
    path = s_path(geo["top"], geo["bot"])
    stroke = (
        f'fill="none" stroke="{colour}" stroke-width="{geo["s_w"]}" '
        'stroke-linecap="round" stroke-linejoin="round"'
    )
    if busy:
        return f'<path d="{path}" {stroke}/>'
    return (
        f'<defs><mask id="{ident}" maskUnits="userSpaceOnUse">'
        f'<rect width="{GRID}" height="{GRID}" fill="#fff"/>'
        f'<polygon points="{geo["v"]}" stroke="#000" stroke-width="{2 * GAP}" stroke-linejoin="round"/>'
        "</mask></defs>"
        f'<path mask="url(#{ident})" d="{path}" {stroke}/>'
        f'<polygon points="{geo["v"]}" fill="{colour}"/>'
    )


def svg(px: int, colour: str, busy: bool) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{px}" height="{px}" viewBox="0 0 {GRID} {GRID}">'
        f"{body(colour, busy, NEW, 'v')}</svg>"
    )


def sheet() -> str:
    """0.1.7 (left) and the new mark (right) on light and dark, at 36 px and 32/24 px."""
    cell = 80
    rows = [("#f2f2f2", "#000000"), ("#2b2b2b", "#ffffff")]
    sizes = [36, 32, 24]
    width = cell * len(sizes) * 2 + 20
    height = cell * len(rows) + 10
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">']
    for r, (background, colour) in enumerate(rows):
        out.append(f'<rect x="0" y="{r * cell}" width="{width}" height="{cell}" fill="{background}"/>')
        for g, (geo, name) in enumerate(((OLD, "o"), (NEW, "n"))):
            for i, size in enumerate(sizes):
                x = g * (cell * len(sizes) + 20) + i * cell + (cell - size) / 2
                y = r * cell + (cell - size) / 2
                out.append(
                    f'<svg x="{x}" y="{y}" width="{size}" height="{size}" viewBox="0 0 {GRID} {GRID}">'
                    f"{body(colour, False, geo, f'{name}{r}{i}')}</svg>"
                )
    out.append("</svg>")
    return "".join(out)


def render(svg_text: str, target: Path) -> None:
    subprocess.run(
        ["rsvg-convert", "--format", "png", "--output", str(target)],
        input=svg_text.encode(),
        check=True,
    )


def main() -> None:
    if shutil.which("rsvg-convert") is None:
        sys.exit("rsvg-convert not found (brew install librsvg)")
    OUT.mkdir(parents=True, exist_ok=True)
    for nominal, px2x in SIZES.items():
        for busy in (False, True):
            for colour, suffix in (("#000000", ""), ("#ffffff", "-light")):
                for scale, px in ((1, px2x // 2), (2, px2x)):
                    name = f"tray-{nominal}{'-busy' if busy else ''}{suffix}{'@2x' if scale == 2 else ''}.png"
                    render(svg(px, colour, busy), OUT / name)
    print(f"wrote tray icons to {OUT.relative_to(ROOT)}")
    if "--sheet" in sys.argv:
        target = ROOT / "src-tauri" / "target" / "tray-compare.png"
        target.parent.mkdir(parents=True, exist_ok=True)
        render(sheet(), target)
        print(f"wrote {target.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
