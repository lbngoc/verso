import { derived, writable } from "svelte/store";
import en from "./en.json";
import vi from "./vi.json";

export type Language = "en" | "vi";
type Params = Record<string, string | number>;

const catalogs: Record<Language, Record<string, string>> = { en, vi };

/** UI language: the OS locale picks the default, Settings can override it. */
export const uiLanguage = writable<Language>("en");

export function setUiLanguage(language: string): void {
  uiLanguage.set(language === "vi" ? "vi" : "en");
}

export function translate(language: Language, key: string, params: Params = {}): string {
  const template = catalogs[language][key] ?? catalogs.en[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (_, name: string) =>
    params[name] !== undefined ? String(params[name]) : `{${name}}`,
  );
}

export const t = derived(uiLanguage, (language) => {
  return (key: string, params: Params = {}) => translate(language, key, params);
});

interface NamedLanguage {
  code: string;
  name_en: string;
}

/**
 * A language's name in the UI language (FR-019): English keeps the catalogue's English name, other UI
 * languages ask the system's ICU data and fall back to English for codes it does not know.
 */
export function languageName(ui: Language, language: NamedLanguage): string {
  if (ui === "en") return language.name_en;
  try {
    const name = new Intl.DisplayNames([ui], { type: "language", fallback: "none" }).of(
      language.code,
    );
    if (name) return name.charAt(0).toLocaleUpperCase(ui) + name.slice(1);
  } catch {
    /* unknown code or no ICU data */
  }
  return language.name_en;
}

export const langName = derived(
  uiLanguage,
  (ui) => (language: NamedLanguage) => languageName(ui, language),
);
