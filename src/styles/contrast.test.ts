import { describe, expect, it } from "vitest";
import { contrastRatio, MATERIAL_COLOURS, APP_ACCENT, pickAccent } from "../lib/theme-tokens";

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

describe("accent tokens (spec 009 FR-027/FR-028)", () => {
  const surfaces = {
    light: { opaque: "#ffffff", raised: "#f7f7f9" },
    dark: { opaque: "#1e1e20", raised: "#2a2a2e" },
  } as const;

  it("the app teal passes every role in both themes", () => {
    for (const theme of ["light", "dark"] as const) {
      const tokens = APP_ACCENT[theme];
      const surface = surfaces[theme];
      expect(
        contrastRatio(tokens.accent, surface.opaque),
        `${theme} accent on surface`,
      ).toBeGreaterThanOrEqual(3);
      expect(
        contrastRatio(tokens.contrast, tokens.accent),
        `${theme} contrast on accent`,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        contrastRatio(tokens.text, surface.opaque),
        `${theme} accent text on surface`,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        contrastRatio(tokens.text, surface.raised),
        `${theme} accent text on raised`,
      ).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("keeps a system accent that passes and falls back per failing role", () => {
    // Yellow cannot carry white text or a focus ring on white: everything falls back.
    const yellow = pickAccent("#ffd60a", "light");
    expect(yellow.accent).toBe(APP_ACCENT.light.accent);
    expect(yellow.text).toBe(APP_ACCENT.light.text);

    // Graphite passes the 3:1 ring test but not 4.5:1 as text.
    const graphite = pickAccent("#8e8e93", "light");
    expect(graphite.accent).toBe("#8e8e93");
    expect(graphite.text).toBe(APP_ACCENT.light.text);
    expect(contrastRatio(graphite.contrast, graphite.accent)).toBeGreaterThanOrEqual(4.5);

    const pink = pickAccent("#ff375f", "light");
    expect(pink.accent).toBe("#ff375f");
    expect(contrastRatio(pink.contrast, pink.accent)).toBeGreaterThanOrEqual(4.5);

    const blue = pickAccent("#0a84ff", "light");
    expect(blue.accent).toBe("#0a84ff");
    expect(contrastRatio(blue.contrast, blue.accent)).toBeGreaterThanOrEqual(4.5);
  });

  it("falls back to the app values when there is no system accent", () => {
    expect(pickAccent(null, "light")).toEqual(APP_ACCENT.light);
    expect(pickAccent(null, "dark")).toEqual(APP_ACCENT.dark);
  });
});
