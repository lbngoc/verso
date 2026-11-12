#!/usr/bin/env python3
"""Generate the Verso tray marks (spec 008, "two-way lines" mark) with rsvg-convert.

Each nominal size is drawn on its own pixel grid (horizontal strokes on whole pixels) instead of being
scaled down from the 24-unit master in src-tauri/icons/source/mark.svg, so 16 px stays crisp (FR-008, FR-014).
@2x files reuse the same geometry at twice the scale, so they stay on the grid too.

Outputs, in src-tauri/icons/tray/:
  tray-{S}.png, tray-{S}@2x.png              black glyph (macOS template image, light Windows taskbar)
  tray-{S}-light.png, tray-{S}-light@2x.png  white glyph for dark Windows taskbars
  tray-{S}-busy*.png                         the translating state (FR-010), same two colours
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src-tauri" / "icons" / "tray"


@dataclass(frozen=True)
class Grid:
    size: int      # canvas, px at 1x
    w: float       # stroke width
    rows: tuple[float, float, float]
    x0: float      # left end of the long lines
    x1: float      # right end of the long lines
    mid: tuple[float, float]  # middle (short) line
    a: float       # arrowhead depth = half its height (45° wings)


# Hand-tuned per size. Even stroke widths sit on whole-pixel centres, odd ones on half pixels.
GRIDS = [
    Grid(16, 2, (4, 8, 12), 2, 14, (6.5, 9.5), 2),
    Grid(18, 2, (4, 9, 14), 2, 16, (7.5, 10.5), 3),
    Grid(22, 2, (6, 11, 16), 3, 19, (9.5, 12.5), 3),
    Grid(32, 3, (7.5, 15.5, 23.5), 4, 28, (13, 19), 5),
]


def paths(g: Grid, busy: bool) -> str:
    top, mid_y, bot = g.rows
    a = g.a
    if busy:
        # Translating: the two arrows shorten and pass each other, the middle line drops out.
        span = (g.x1 - g.x0) * 0.55
        top_line = f"M{g.x1 - span} {top}H{g.x1}"
        bot_line = f"M{g.x0 + span} {bot}H{g.x0}"
        middle = ""
    else:
        top_line = f"M{g.x0} {top}H{g.x1}"
        bot_line = f"M{g.x1} {bot}H{g.x0}"
        middle = f"M{g.mid[0]} {mid_y}H{g.mid[1]}"
    top_head = f"M{g.x1 - a} {top - a}L{g.x1} {top}L{g.x1 - a} {top + a}"
    bot_head = f"M{g.x0 + a} {bot - a}L{g.x0} {bot}L{g.x0 + a} {bot + a}"
    return top_line + top_head + middle + bot_line + bot_head


def svg(g: Grid, scale: int, colour: str, busy: bool) -> str:
    px = g.size * scale
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{px}" height="{px}" '
        f'viewBox="0 0 {g.size} {g.size}"><path d="{paths(g, busy)}" fill="none" stroke="{colour}" '
        f'stroke-width="{g.w}" stroke-linecap="round" stroke-linejoin="round"/></svg>'
    )


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
    for g in GRIDS:
        for busy in (False, True):
            for colour, suffix in (("#000000", ""), ("#ffffff", "-light")):
                for scale in (1, 2):
                    name = f"tray-{g.size}{'-busy' if busy else ''}{suffix}{'@2x' if scale == 2 else ''}.png"
                    render(svg(g, scale, colour, busy), OUT / name)
    print(f"wrote tray icons to {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
