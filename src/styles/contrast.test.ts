import { readFileSync } from "node:fs";
import { resolve } from "node:path";
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

describe("status colours (spec 009 FR-029, FR-043)", () => {
  // Read from tokens.css so the test follows the shipped values.
  const css = readFileSync(resolve(__dirname, "tokens.css"), "utf8");
  function block(selector: string): Record<string, string> {
    const start = css.indexOf(`${selector} {`);
    const body = css.slice(start, css.indexOf("}", start));
    return Object.fromEntries(
      [...body.matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{6})/g)].map((match) => [match[1], match[2]]),
    );
  }
  const themes = { light: block('[data-theme="light"]'), dark: block('[data-theme="dark"]') };

  // `color-mix(in srgb, <token> N%, transparent)` painted over a surface.
  function tint(colour: string, surface: string, share: number): string {
    const channels = (hex: string) => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16));
    const mixed = channels(colour).map((value, index) =>
      Math.round(value * share + channels(surface)[index] * (1 - share)),
    );
    return `#${mixed.map((value) => value.toString(16).padStart(2, "0")).join("")}`;
  }

  it("warning and success text reach 4.5:1 on their badges and on cards, both themes", () => {
    for (const [theme, tokens] of Object.entries(themes)) {
      for (const surfaceName of ["surface-opaque", "surface-raised"]) {
        const surface = tokens[surfaceName];
        expect(surface, `${theme} ${surfaceName}`).toBeDefined();
        for (const [token, share] of [
          ["warning", 0.16],
          ["success", 0.14],
        ] as const) {
          const colour = tokens[token];
          expect(colour, `${theme} --${token}`).toBeDefined();
          expect(
            contrastRatio(colour, surface),
            `${theme} --${token} on ${surfaceName}`,
          ).toBeGreaterThanOrEqual(4.5);
          expect(
            contrastRatio(colour, tint(colour, surface, share)),
            `${theme} --${token} on its badge over ${surfaceName}`,
          ).toBeGreaterThanOrEqual(4.5);
        }
      }
    }
  });
});

describe("Settings colour levels on the card fill (spec 011 R2, data-model § Text level)", () => {
  const read = (file: string) => readFileSync(resolve(__dirname, file), "utf8");
  const tokens = read("tokens.css");
  function hexes(css: string, selector: string): Record<string, string> {
    const start = css.indexOf(`${selector} {`);
    expect(start, selector).toBeGreaterThanOrEqual(0);
    const body = css.slice(start, css.indexOf("}", start));
    return Object.fromEntries(
      [...body.matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{6})/g)].map((match) => [match[1], match[2]]),
    );
  }
  const light = hexes(tokens, '[data-theme="light"]');
  const dark = hexes(tokens, '[data-theme="dark"]');
  const windowsLight = hexes(tokens, '[data-os="windows"]');
  const windowsDark = hexes(tokens, '[data-os="windows"][data-theme="dark"]');

  // macOS Settings overrides the box fill (and the light secondary level) from the T003 reference;
  // Windows sets its own --card-fill.
  const macLight = hexes(tokens, '[data-os="macos"][data-theme="light"] .onboarding-body');
  const macDark = hexes(tokens, '[data-os="macos"][data-theme="dark"] .settings-body');
  const cases = [
    {
      name: "macOS light",
      text: { ...light, "text-secondary": macLight["text-secondary"] },
      card: macLight["card-fill"],
    },
    { name: "macOS dark", text: dark, card: macDark["card-fill"] },
    { name: "macOS dark, window", text: dark, card: macDark["settings-surface"] },
    { name: "Windows light", text: light, card: windowsLight["card-fill"] },
    { name: "Windows dark", text: dark, card: windowsDark["card-fill"] },
    {
      name: "Windows light, page",
      text: light,
      card: hexes(tokens, '[data-os="windows"] .settings-body')["settings-surface"],
    },
    {
      name: "Windows dark, page",
      text: dark,
      card: hexes(tokens, '[data-os="windows"][data-theme="dark"] .settings-body')[
        "settings-surface"
      ],
    },
  ];

  it("primary and secondary reach 4.5:1, tertiary 3:1, in every case", () => {
    for (const { name, text, card } of cases) {
      expect(card, `${name} card fill`).toBeDefined();
      expect(contrastRatio(text["text"], card), `${name} primary`).toBeGreaterThanOrEqual(4.5);
      expect(
        contrastRatio(text["text-secondary"], card),
        `${name} secondary`,
      ).toBeGreaterThanOrEqual(4.5);
      expect(contrastRatio(text["text-tertiary"], card), `${name} tertiary`).toBeGreaterThanOrEqual(
        3,
      );
    }
  });

  it("the levels keep their order: primary darker (light) or brighter (dark) than secondary", () => {
    for (const { name, text, card } of cases) {
      expect(contrastRatio(text["text"], card), `${name} primary vs secondary`).toBeGreaterThan(
        contrastRatio(text["text-secondary"], card),
      );
    }
  });
});
