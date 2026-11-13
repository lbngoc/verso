import { get } from "svelte/store";
import type { SettingsProfile, Theme } from "../ipc";
import { commands, events } from "../ipc";
import { appearance, applyAccent, applyAppearance, setAccentPreference } from "./appearance";

/** Applies the theme setting on top of the OS appearance (light/dark/system) and the accent. */
export function applyTheme(theme: Theme): void {
  const current = get(appearance);
  const resolved =
    theme === "system" ? current.theme : theme === "dark" ? "dark" : "light";
  document.documentElement.dataset.theme = resolved;
  // The accent has a light and a dark value, so it follows the resolved theme.
  applyAccent();
}

export async function initTheme(): Promise<SettingsProfile | null> {
  try {
    const settings = await commands.getSettings();
    setAccentPreference(settings.accent);
    applyTheme(settings.theme);
    await events.onSettingsChanged((next) => {
      setAccentPreference(next.accent);
      applyTheme(next.theme);
    });
    await events.onAppearanceChanged((next) => {
      applyAppearance(next);
      commands
        .getSettings()
        .then((current) => {
          setAccentPreference(current.accent);
          applyTheme(current.theme);
        })
        .catch(() => undefined);
    });
    return settings;
  } catch {
    return null;
  }
}
