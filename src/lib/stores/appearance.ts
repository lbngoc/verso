import { get, writable } from "svelte/store";
import type { Accent, Appearance, Material } from "../ipc";
import { commands, events } from "../ipc";
import { pickAccent } from "../theme-tokens";

/** Material fallback used before the native value arrives (and in a plain browser). */
const browserAppearance: Appearance = {
  os: navigator.userAgent.includes("Mac") ? "macos" : "windows",
  os_version: "",
  material: "opaque",
  theme: window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light",
  reduce_transparency: false,
  high_contrast: false,
  accent: null,
};

export const appearance = writable<Appearance>(browserAppearance);

/** `settings.accent`: the app teal (default) or the guarded system accent (FR-027). */
let accentPreference: Accent = "app";

function resolvedTheme(): "light" | "dark" {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

/**
 * Writes the accent token family. The system accent is only used when the user chose it *and* it
 * passes the contrast guard; each failing role falls back to the app values (theme-tokens.ts).
 */
export function applyAccent(): void {
  const root = document.documentElement;
  const system = accentPreference === "system" ? get(appearance).accent : null;
  const tokens = pickAccent(system, resolvedTheme());
  root.style.setProperty("--accent", tokens.accent);
  root.style.setProperty("--accent-contrast", tokens.contrast);
  root.style.setProperty("--accent-text", tokens.text);
}

export function setAccentPreference(value: Accent | undefined): void {
  accentPreference = value === "system" ? "system" : "app";
  applyAccent();
}

export function applyAppearance(value: Appearance): void {
  const root = document.documentElement;
  root.dataset.os = value.os;
  root.dataset.material = value.material;
  root.dataset.theme = value.theme;
  root.dataset.reduceTransparency = String(value.reduce_transparency);
  root.dataset.highContrast = String(value.high_contrast);
  appearance.set(value);
  // The system accent is kept in the store; whether it is used is decided by applyAccent().
  applyAccent();
}

export function materialIsOpaque(material: Material): boolean {
  return material === "opaque";
}

export async function initAppearance(): Promise<Appearance> {
  try {
    const value = await commands.getAppearance();
    applyAppearance(value);
    await events.onAppearanceChanged(applyAppearance);
    return value;
  } catch {
    applyAppearance(browserAppearance);
    return browserAppearance;
  }
}
