// Keys of the quick panel (spec 009, contracts/popup-and-panel.md § Quick panel).
//
// Pure so the focus model can be tested without a window: the page only applies the decision.

export type QuickFocus = "input" | "result";

export type QuickAction =
  | { type: "none" }
  | { type: "swap" }
  | { type: "close" }
  | { type: "translate" }
  | { type: "focus-result" }
  | { type: "copy" }
  | { type: "speak" }
  | { type: "text-size"; delta: number }
  | { type: "reset-size" }
  /** Cmd+, (Ctrl+, on Windows): open the Settings window. */
  | { type: "settings" }
  /** Return focus to the input and insert this text (a printable key pressed on the result). */
  | { type: "type"; text: string };

/** Whether a key inserts text (letters, digits, punctuation) rather than acting on the panel. */
function isPrintable(event: {
  key: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
}): boolean {
  if (event.metaKey || event.ctrlKey || event.altKey) return false;
  return event.key.length === 1;
}

export function quickKey(
  focus: QuickFocus,
  event: {
    key: string;
    shiftKey?: boolean;
    metaKey?: boolean;
    ctrlKey?: boolean;
    altKey?: boolean;
    /** An input method is composing (marked text): the key belongs to it. */
    isComposing?: boolean;
    keyCode?: number;
  },
  hasResult: boolean,
): QuickAction {
  // Spec 004 FR-030: while an input method composes (Vietnamese Telex/VNI, CJK…), Enter, Tab, Esc
  // and the letter keys commit or edit the composition; the panel must not act on them. WebKit
  // reports such keys with `isComposing` or the IME key code 229.
  if (event.isComposing || event.keyCode === 229) return { type: "none" };
  const key = event.key;

  if (key === "Tab") return { type: "swap" };
  if (key === "," && (event.metaKey || event.ctrlKey) && !event.altKey) {
    return { type: "settings" };
  }

  if (focus === "input") {
    if (key === "Escape") return hasResult ? { type: "focus-result" } : { type: "close" };
    if (key === "Enter" && !event.shiftKey) return { type: "translate" };
    return { type: "none" };
  }

  if (key === "Escape") return { type: "close" };
  if (key === "c" && !event.metaKey && !event.ctrlKey) return { type: "copy" };
  if (key === "s" && !event.metaKey && !event.ctrlKey) return { type: "speak" };
  if (key === "-") return { type: "text-size", delta: -1 };
  if (key === "=" || key === "+") return { type: "text-size", delta: 1 };
  if (key === "0") return { type: "reset-size" };
  if (key === "Tab") return { type: "swap" };
  if (isPrintable(event)) return { type: "type", text: key };
  return { type: "none" };
}
