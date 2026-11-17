#!/usr/bin/env python3
"""Generate the Verso tray marks with rsvg-convert: "Vs" between two opposite arrows (logo "E").

The glyph is drawn once on a 36-unit grid (the @2x pixel grid of the 18 pt macOS status item) and the same
geometry is rendered at every size, so the artwork is identical on both platforms. Only the @2x files are
embedded (src-tauri/src/tray.rs); the 1x files are the same drawing at half size.

Outputs, in src-tauri/icons/tray/:
  tray-{S}.png, tray-{S}@2x.png              black glyph (macOS template image, light Windows taskbar)
  tray-{S}-light.png, tray-{S}-light@2x.png  white glyph for dark Windows taskbars
  tray-{S}-busy*.png                         the translating state: the arrows alone, shortened and passing each other
with S = 18 (macOS, 36 px @2x) and 16 (Windows, 32 px @2x).

The glyph is about 28 units wide and tall, leaving about 4 units of padding inside the 36 canvas, so it
sits with the same breathing room as the Input Source and Control Center items beside it.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src-tauri" / "icons" / "tray"

GRID = 36
ARROW_W = 2.4
LETTER_W = 2.4
S_W = 2.0
# Nominal size -> pixels at @2x.
SIZES = {18: 36, 16: 32}

HEAD = 2.4
TOP_Y, BOT_Y = 7.5, 28.5
X0, X1 = 5, 31


def arrows(busy: bool) -> str:
    if busy:
        span = (X1 - X0) * 0.55
        top = f"M{X1 - span} {TOP_Y}H{X1}"
        bot = f"M{X0 + span} {BOT_Y}H{X0}"
    else:
        top = f"M{X0} {TOP_Y}H{X1}"
        bot = f"M{X1} {BOT_Y}H{X0}"
    top_head = f"M{X1 - HEAD} {TOP_Y - HEAD}L{X1} {TOP_Y}L{X1 - HEAD} {TOP_Y + HEAD}"
    bot_head = f"M{X0 + HEAD} {BOT_Y - HEAD}L{X0} {BOT_Y}L{X0 + HEAD} {BOT_Y + HEAD}"
    return (
        f'<path d="{top}{top_head}{bot}{bot_head}" fill="none" stroke-width="{ARROW_W}" '
        f'stroke-linecap="round" stroke-linejoin="round"/>'
    )


# "Vs", drawn as strokes: a capital V and a lower-case s, centred on the canvas.
LETTERS = (
    '<g transform="translate(-0.85 0)" fill="none" stroke-linecap="round" stroke-linejoin="round">'
    f'<path d="M9.2 13.3L14.8 22.7L20.4 13.3" stroke-width="{LETTER_W}"/>'
    f'<path d="M28.2 17C27.8 15.8 26.6 15.2 25.8 15.2C24.2 15.2 23.2 16.1 23.2 17C23.2 18.3 24.6 18.7 26 19'
    f'C27.4 19.3 28.8 19.8 28.8 21C28.8 22 27.6 22.6 26 22.6C24.6 22.6 23.4 22 23.1 20.9" stroke-width="{S_W}"/>'
    "</g>"
)


def svg(px: int, colour: str, busy: bool) -> str:
    body = arrows(busy) + ("" if busy else LETTERS)
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{px}" height="{px}" viewBox="0 0 {GRID} {GRID}">'
        f'<g stroke="{colour}">{body}</g></svg>'
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
