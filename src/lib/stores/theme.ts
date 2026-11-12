import { get } from "svelte/store";
import type { SettingsProfile, Theme } from "../ipc";
import { commands, events } from "../ipc";
import { appearance, applyAppearance } from "./appearance";

/** Applies the theme setting on top of the OS appearance (light/dark/system). */
export function applyTheme(theme: Theme): void {
  const current = get(appearance);
  const resolved =
    theme === "system" ? current.theme : theme === "dark" ? "dark" : "light";
  document.documentElement.dataset.theme = resolved;
}

export async function initTheme(): Promise<SettingsProfile | null> {
  try {
    const settings = await commands.getSettings();
    applyTheme(settings.theme);
    await events.onSettingsChanged((next) => applyTheme(next.theme));
    await events.onAppearanceChanged((next) => {
      applyAppearance(next);
      commands.getSettings().then((current) => applyTheme(current.theme)).catch(() => undefined);
    });
    return settings;
  } catch {
    return null;
  }
}
