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

The glyph spans about 4..32 units across and 3.6..32.4 down (taller than 0.1.5's 5.6..30.4, spec 009 FR-054):
a thinner S and a bolder, taller V keep the letter readable at menu-bar size.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src-tauri" / "icons" / "tray"

GRID = 36
S_W = 2.0  # stroke of the S-arrow (2.4 in 0.1.5; thinner so the V stands out, FR-054)
GAP = 1.0  # clearance cut out of the S around the V
# Nominal size -> pixels at @2x.
SIZES = {18: 36, 16: 32}

HEAD = 2.4
TOP_Y, MID_Y, BOT_Y = 6, 18, 30
R = (MID_Y - TOP_Y) / 2
ARC_L, ARC_R = 10, 26  # centres of the two bowls
TIP_R, TIP_L = 31.5, 4.5

# The V: 18 units tall with 4-unit arms (0.1.5: 14.6 tall, 3.3-unit arms).
V = "11.6,9 15.6,9 18,18.6 20.4,9 24.4,9 20,27 16,27"

S_PATH = (
    f"M{TIP_R} {TOP_Y}H{ARC_L}A{R} {R} 0 0 0 {ARC_L} {MID_Y}H{ARC_R}A{R} {R} 0 0 1 {ARC_R} {BOT_Y}H{TIP_L}"
    f"M{TIP_R - HEAD} {TOP_Y - HEAD}L{TIP_R} {TOP_Y}L{TIP_R - HEAD} {TOP_Y + HEAD}"
    f"M{TIP_L + HEAD} {BOT_Y - HEAD}L{TIP_L} {BOT_Y}L{TIP_L + HEAD} {BOT_Y + HEAD}"
)


def svg(px: int, colour: str, busy: bool) -> str:
    if busy:
        defs, mask, letter = "", "", ""
    else:
        defs = (
            '<defs><mask id="v" maskUnits="userSpaceOnUse">'
            f'<rect width="{GRID}" height="{GRID}" fill="#fff"/>'
            f'<polygon points="{V}" stroke="#000" stroke-width="{2 * GAP}" stroke-linejoin="round"/>'
            "</mask></defs>"
        )
        mask = ' mask="url(#v)"'
        letter = f'<polygon points="{V}" fill="{colour}"/>'
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{px}" height="{px}" viewBox="0 0 {GRID} {GRID}">{defs}'
        f'<path{mask} d="{S_PATH}" fill="none" stroke="{colour}" stroke-width="{S_W}" '
        f'stroke-linecap="round" stroke-linejoin="round"/>{letter}</svg>'
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
    for nominal, px2x in SIZES.items():
        for busy in (False, True):
            for colour, suffix in (("#000000", ""), ("#ffffff", "-light")):
                for scale, px in ((1, px2x // 2), (2, px2x)):
                    name = f"tray-{nominal}{'-busy' if busy else ''}{suffix}{'@2x' if scale == 2 else ''}.png"
                    render(svg(px, colour, busy), OUT / name)
    print(f"wrote tray icons to {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
