// Keyboard ⇄ accelerator conversion for the shortcut recorder (spec 009, R6).
//
// Accelerators stay in the format tauri-plugin-global-shortcut parses ("Alt+Shift+T"); the recorder
// reads KeyboardEvent.code so the result does not depend on the keyboard layout (on macOS ⌥T types
// "†", but the code is still KeyT).

export type Modifier = "Cmd" | "Super" | "Ctrl" | "Alt" | "Shift";

export interface KeyEventLike {
  code: string;
  key?: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export interface PlatformOptions {
  /** macOS uses Cmd for the meta key; Windows uses Super. */
  mac: boolean;
}

const CODE_NAMES: Record<string, string> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Space: "Space",
  Enter: "Enter",
  NumpadEnter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Escape: "Esc",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Insert: "Insert",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backquote: "`",
};

/** The accelerator name for a `KeyboardEvent.code`, or null for a bare modifier / unknown key. */
export function keyFromCode(code: string): string | null {
  if (!code) return null;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return code.slice(6);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (code === "ShiftLeft" || code === "ShiftRight") return null;
  if (code === "ControlLeft" || code === "ControlRight") return null;
  if (code === "AltLeft" || code === "AltRight") return null;
  if (code === "MetaLeft" || code === "MetaRight") return null;
  return CODE_NAMES[code] ?? null;
}

/** Modifiers in the normalised order the settings store and the OS parser expect. */
export function modifiersFor(event: KeyEventLike, options: PlatformOptions): Modifier[] {
  const modifiers: Modifier[] = [];
  if (event.metaKey) modifiers.push(options.mac ? "Cmd" : "Super");
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  return modifiers;
}

/** Builds the accelerator for a key event, or null when nothing usable was pressed. */
export function acceleratorFromEvent(event: KeyEventLike, options: PlatformOptions): string | null {
  const key = keyFromCode(event.code);
  if (!key) return null;
  return [...modifiersFor(event, options), key].join("+");
}

const MAC_GLYPHS: Record<Modifier, string> = {
  Cmd: "⌘",
  Super: "⌘",
  Ctrl: "⌃",
  Alt: "⌥",
  Shift: "⇧",
};

const WINDOWS_NAMES: Record<Modifier, string> = {
  Cmd: "Ctrl",
  Super: "Win",
  Ctrl: "Ctrl",
  Alt: "Alt",
  Shift: "Shift",
};

const DISPLAY_ORDER: Modifier[] = ["Cmd", "Super", "Ctrl", "Alt", "Shift"];

function splitAccelerator(accelerator: string): { modifiers: string[]; key: string } {
  const parts = accelerator
    .split("+")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
  if (parts.length === 0) return { modifiers: [], key: "" };
  // A trailing "+" key ("Cmd++") arrives as ["Cmd", "+"]; anything but the last part is a modifier.
  const key = parts[parts.length - 1];
  return { modifiers: parts.slice(0, -1), key };
}

function normaliseModifier(name: string, mac: boolean): Modifier | null {
  switch (name) {
    case "Cmd":
    case "Command":
    case "Meta":
      return mac ? "Cmd" : "Super";
    case "Super":
    case "Win":
    case "Windows":
      return "Super";
    case "Ctrl":
    case "Control":
      return "Ctrl";
    case "Alt":
    case "Option":
      return "Alt";
    case "Shift":
      return "Shift";
    default:
      return null;
  }
}

function displayKey(key: string): string {
  if (/^Key[A-Z]$/.test(key)) return key.slice(3);
  if (/^Digit[0-9]$/.test(key)) return key.slice(5);
  if (key === "ArrowUp") return "↑";
  if (key === "ArrowDown") return "↓";
  if (key === "ArrowLeft") return "←";
  if (key === "ArrowRight") return "→";
  return key;
}

/** Platform notation: "⌥⇧T" on macOS, "Ctrl+Alt+Shift+T" on Windows. Empty stays empty. */
export function displayAccelerator(accelerator: string, options: PlatformOptions): string {
  if (!accelerator.trim()) return "";
  const { modifiers, key } = splitAccelerator(accelerator);
  const normalised = modifiers
    .map((modifier) => normaliseModifier(modifier, options.mac))
    .filter((modifier): modifier is Modifier => modifier !== null);
  const ordered = DISPLAY_ORDER.filter((modifier) => normalised.includes(modifier));
  const shownKey = displayKey(key);
  if (options.mac) {
    return ordered.map((modifier) => MAC_GLYPHS[modifier]).join("") + shownKey;
  }
  // Windows convention lists Win last: Ctrl, Alt, Shift, Win.
  const windowsOrder: Modifier[] = ["Ctrl", "Alt", "Shift", "Super"];
  const parts = windowsOrder
    .filter((modifier) => ordered.includes(modifier))
    .map((modifier) => WINDOWS_NAMES[modifier]);
  return [...parts, shownKey].filter((part) => part.length > 0).join("+");
}

/** The default accelerators shown in the recorder, per platform. */
export function defaultAccelerators(options: PlatformOptions): {
  translate: string;
  speak: string;
  quick: string;
} {
  return {
    translate: "Alt+Shift+T",
    speak: "Alt+Shift+S",
    quick: options.mac ? "Alt+Shift+Cmd+T" : "Ctrl+Alt+Shift+T",
  };
}
