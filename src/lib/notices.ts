import type { Language, TranslationResult } from "./ipc";

export interface NoticeItem {
  key: string;
  params?: Record<string, string | number>;
}

/** Notices shown with a result (popup-ui-contract § States). */
export function buildNotices(result: TranslationResult, extra: NoticeItem[] = []): NoticeItem[] {
  const notices: NoticeItem[] = [];
  if (result.truncated && result.original_length) {
    const shown = result.text ? result.text.length : 1000;
    notices.push({
      key: "notice.truncated",
      params: { shown: shown.toLocaleString(), total: result.original_length.toLocaleString() },
    });
  }

  // Another engine answering after one failed is not the user's concern: the engine name already
  // says who answered, and the failure stays in the log (spec 004 FR-032).

  return [...notices, ...extra];
}

export function engineName(engineId: string): string {
  switch (engineId) {
    case "http-google":
      return "Google";
    case "http-bing":
      return "Microsoft";
    default:
      return engineId;
  }
}

/**
 * Which modifier the frontmost terminal needs for the bypass hint (matches
 * core::capture::terminal_bypass_modifier).
 */
export function terminalModifier(app?: string | null): "option" | "shift" | null {
  if (!app) return null;
  const name = app.toLowerCase();
  if (
    name.includes("iterm2") ||
    name.includes("com.apple.terminal") ||
    name.includes("terminal.app")
  ) {
    return "option";
  }
  if (["wezterm", "kitty", "ghostty", "kaku"].some((known) => name.includes(known))) {
    return "shift";
  }
  return null;
}

export function isRtl(code: string, languages: Language[]): boolean {
  return languages.find((item) => item.code.toLowerCase() === code.toLowerCase())?.rtl ?? false;
}
