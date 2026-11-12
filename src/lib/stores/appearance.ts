import { writable } from "svelte/store";
import type { Appearance, Material } from "../ipc";
import { commands, events } from "../ipc";

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

export function applyAppearance(value: Appearance): void {
  const root = document.documentElement;
  root.dataset.os = value.os;
  root.dataset.material = value.material;
  root.dataset.theme = value.theme;
  if (value.accent) {
    root.style.setProperty("--accent", value.accent);
  }
  appearance.set(value);
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
