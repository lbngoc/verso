/**
 * The colours every material falls back to, mirrored in `src/styles/tokens.css`.
 *
 * Body text must reach 4.5:1 against each of these (popup-ui-contract § Rules), which the Vitest
 * check in `contrast.test.ts` enforces.
 */
export interface MaterialColours {
  os: "macos" | "windows";
  material: "glass" | "vibrancy" | "acrylic" | "opaque";
  theme: "light" | "dark";
  /** What the material looks like if the native layer cannot draw (opaque fallback). */
  fallback: string;
  text: string;
  secondaryText: string;
  /** Hints, placeholders and group labels; still normal text, so it needs 4.5:1 too. */
  tertiaryText: string;
}

export const MATERIAL_COLOURS: MaterialColours[] = [
  {
    os: "macos",
    material: "glass",
    theme: "light",
    fallback: "#f2f2f7",
    text: "#1c1c1e",
    secondaryText: "#5c5c60",
    tertiaryText: "#6e6e73",
  },
  {
    os: "macos",
    material: "glass",
    theme: "dark",
    fallback: "#1c1c1e",
    text: "#f2f2f7",
    secondaryText: "#aeaeb2",
    tertiaryText: "#8e8e93",
  },
  {
    os: "macos",
    material: "vibrancy",
    theme: "light",
    fallback: "#f2f2f7",
    text: "#1c1c1e",
    secondaryText: "#5c5c60",
    tertiaryText: "#6e6e73",
  },
  {
    os: "macos",
    material: "vibrancy",
    theme: "dark",
    fallback: "#26262a",
    text: "#f2f2f7",
    secondaryText: "#aeaeb2",
    tertiaryText: "#8e8e93",
  },
  {
    os: "macos",
    material: "opaque",
    theme: "light",
    fallback: "#ffffff",
    text: "#1c1c1e",
    secondaryText: "#5c5c60",
    tertiaryText: "#6e6e73",
  },
  {
    os: "macos",
    material: "opaque",
    theme: "dark",
    fallback: "#1e1e20",
    text: "#f2f2f7",
    secondaryText: "#aeaeb2",
    tertiaryText: "#8e8e93",
  },
  {
    os: "windows",
    material: "acrylic",
    theme: "light",
    fallback: "#f3f3f3",
    text: "#1b1b1b",
    secondaryText: "#5f5f5f",
    tertiaryText: "#6e6e73",
  },
  {
    os: "windows",
    material: "acrylic",
    theme: "dark",
    fallback: "#202020",
    text: "#ffffff",
    secondaryText: "#c5c5c5",
    tertiaryText: "#8e8e93",
  },
  {
    os: "windows",
    material: "opaque",
    theme: "light",
    fallback: "#fbfbfb",
    text: "#1b1b1b",
    secondaryText: "#5f5f5f",
    tertiaryText: "#6e6e73",
  },
  {
    os: "windows",
    material: "opaque",
    theme: "dark",
    fallback: "#202020",
    text: "#ffffff",
    secondaryText: "#c5c5c5",
    tertiaryText: "#8e8e93",
  },
];

export function relativeLuminance(hex: string): number {
  const value = hex.replace("#", "");
  const channels = [0, 2, 4].map((offset) => {
    const channel = parseInt(value.slice(offset, offset + 2), 16) / 255;
    return channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

export function contrastRatio(first: string, second: string): number {
  const a = relativeLuminance(first);
  const b = relativeLuminance(second);
  const lighter = Math.max(a, b);
  const darker = Math.min(a, b);
  return (lighter + 0.05) / (darker + 0.05);
}

/** The accent token family (spec 009, contracts/settings-window.md § Accent tokens). */
export interface AccentTokens {
  accent: string;
  contrast: string;
  text: string;
}

export const APP_ACCENT: Record<"light" | "dark", AccentTokens> = {
  light: { accent: "#1b6f62", contrast: "#ffffff", text: "#17695c" },
  dark: { accent: "#3fb8a3", contrast: "#06231e", text: "#5cc9b6" },
};

const SURFACE_OPAQUE: Record<"light" | "dark", string> = {
  light: "#ffffff",
  dark: "#1e1e20",
};

function readableOn(background: string): string {
  const onWhite = contrastRatio("#ffffff", background);
  const onDark = contrastRatio("#06231e", background);
  return onDark >= onWhite ? "#06231e" : "#ffffff";
}

/**
 * Chooses the accent per role: the system colour while it passes the contrast guard, the app's value
 * for each failing role (≥ 3:1 against the surface for rings/fills, ≥ 4.5:1 for text on accent and
 * accent-coloured text).
 */
export function pickAccent(systemAccent: string | null, theme: "light" | "dark"): AccentTokens {
  const app = APP_ACCENT[theme];
  if (!systemAccent) return app;
  const surface = SURFACE_OPAQUE[theme];
  const passesFilled = contrastRatio(systemAccent, surface) >= 3;
  if (!passesFilled) return app;
  const contrast = readableOn(systemAccent);
  return {
    accent: systemAccent,
    contrast: contrastRatio(contrast, systemAccent) >= 4.5 ? contrast : app.contrast,
    text: contrastRatio(systemAccent, surface) >= 4.5 ? systemAccent : app.text,
  };
}
