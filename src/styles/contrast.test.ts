import { describe, expect, it } from "vitest";
import { contrastRatio, MATERIAL_COLOURS } from "../lib/theme-tokens";

describe("popup contrast (popup-ui-contract § Rules)", () => {
  it("body text reaches 4.5:1 against every material fallback, both themes", () => {
    for (const colours of MATERIAL_COLOURS) {
      const ratio = contrastRatio(colours.text, colours.fallback);
      expect(
        ratio,
        `${colours.os}/${colours.material}/${colours.theme} body text`,
      ).toBeGreaterThanOrEqual(4.5);
      const secondary = contrastRatio(colours.secondaryText, colours.fallback);
      expect(
        secondary,
        `${colours.os}/${colours.material}/${colours.theme} secondary text`,
      ).toBeGreaterThanOrEqual(3);
      const tertiary = contrastRatio(colours.tertiaryText, colours.fallback);
      expect(
        tertiary,
        `${colours.os}/${colours.material}/${colours.theme} tertiary text`,
      ).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("there is a row for every material the app can report", () => {
    for (const material of ["glass", "vibrancy", "acrylic", "opaque"] as const) {
      for (const theme of ["light", "dark"] as const) {
        expect(
          MATERIAL_COLOURS.some(
            (colours) => colours.material === material && colours.theme === theme,
          ),
          `${material}/${theme}`,
        ).toBe(true);
      }
    }
  });
});
