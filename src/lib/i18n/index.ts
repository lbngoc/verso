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

export function translate(
  language: Language,
  key: string,
  params: Params = {},
): string {
  const template = catalogs[language][key] ?? catalogs.en[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (_, name: string) =>
    params[name] !== undefined ? String(params[name]) : `{${name}}`,
  );
}

export const t = derived(uiLanguage, (language) => {
  return (key: string, params: Params = {}) => translate(language, key, params);
});
