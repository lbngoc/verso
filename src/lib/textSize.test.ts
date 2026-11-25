import { describe, expect, it } from "vitest";
import {
  canEnlarge,
  canShrink,
  clampStep,
  fontPx,
  label,
  panelFontPx,
  percentFor,
  STEPS,
} from "./textSize";

describe("textSize", () => {
  it("maps steps -2..+4 to the shown percentages", () => {
    expect([...STEPS]).toEqual([80, 90, 100, 110, 125, 140, 150]);
    expect([-2, -1, 0, 1, 2, 3, 4].map(percentFor)).toEqual([80, 90, 100, 110, 125, 140, 150]);
  });

  it("gives the pixel sizes per platform", () => {
    const steps = [-2, -1, 0, 1, 2, 3, 4];
    expect(steps.map((s) => fontPx(s, false))).toEqual([13, 14, 16, 18, 20, 22, 24]);
    expect(steps.map((s) => fontPx(s, true))).toEqual([14, 15, 17, 19, 21, 24, 26]);
  });

  it("gives the panel one size for source and result, the system body size at step 0", () => {
    expect(panelFontPx(0, true)).toBe(13);
    expect(panelFontPx(0, false)).toBe(14);
    expect(panelFontPx(2, true)).toBe(16);
    expect(panelFontPx(-2, false)).toBe(11);
  });

  it("clamps and labels", () => {
    expect(clampStep(-9)).toBe(-2);
    expect(clampStep(9)).toBe(4);
    expect(label(1)).toBe("110 %");
  });

  it("knows the bounds", () => {
    expect(canShrink(-2)).toBe(false);
    expect(canShrink(0)).toBe(true);
    expect(canEnlarge(4)).toBe(false);
    expect(canEnlarge(0)).toBe(true);
  });
});
