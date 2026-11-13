import { describe, expect, it } from "vitest";
import { acceleratorFromEvent, displayAccelerator, keyFromCode } from "./keys";

const mac = { mac: true } as const;
const win = { mac: false } as const;

function event(code: string, mods: Partial<Record<"meta" | "ctrl" | "alt" | "shift", boolean>> = {}, key = "") {
  return {
    code,
    key,
    metaKey: mods.meta ?? false,
    ctrlKey: mods.ctrl ?? false,
    altKey: mods.alt ?? false,
    shiftKey: mods.shift ?? false,
  };
}

describe("acceleratorFromEvent", () => {
  it("maps letters, digits and punctuation by KeyboardEvent.code", () => {
    expect(acceleratorFromEvent(event("KeyT", { alt: true, shift: true }), mac)).toBe("Alt+Shift+T");
    expect(acceleratorFromEvent(event("Digit1", { alt: true }), mac)).toBe("Alt+1");
    expect(acceleratorFromEvent(event("Minus", { meta: true }), mac)).toBe("Cmd+-");
    expect(acceleratorFromEvent(event("Equal", { ctrl: true }), mac)).toBe("Ctrl+=");
  });

  it("maps function keys, space and arrows", () => {
    expect(acceleratorFromEvent(event("F5", { alt: true }), mac)).toBe("Alt+F5");
    expect(acceleratorFromEvent(event("Space", { ctrl: true, alt: true }), mac)).toBe("Ctrl+Alt+Space");
    expect(acceleratorFromEvent(event("ArrowUp", { alt: true, shift: true }), mac)).toBe("Alt+Shift+Up");
  });

  it("orders modifiers Cmd/Super, Ctrl, Alt, Shift", () => {
    expect(
      acceleratorFromEvent(event("KeyT", { shift: true, alt: true, ctrl: true, meta: true }), mac),
    ).toBe("Cmd+Ctrl+Alt+Shift+T");
    expect(acceleratorFromEvent(event("KeyT", { shift: true, alt: true, meta: true }), win)).toBe(
      "Super+Alt+Shift+T",
    );
  });

  it("uses the physical key on macOS, where Option changes the character", () => {
    // ⌥T types "†"; the binding must still be T.
    expect(acceleratorFromEvent(event("KeyT", { alt: true }, "†"), mac)).toBe("Alt+T");
  });

  it("returns null when no key was pressed", () => {
    expect(acceleratorFromEvent(event("", { alt: true }), mac)).toBeNull();
    expect(acceleratorFromEvent(event("ShiftLeft", { shift: true }), mac)).toBeNull();
    expect(acceleratorFromEvent(event("MetaLeft", { meta: true }), mac)).toBeNull();
  });
});

describe("displayAccelerator", () => {
  it("uses macOS glyphs in ⌘ ⌃ ⌥ ⇧ order", () => {
    expect(displayAccelerator("Alt+Shift+T", mac)).toBe("⌥⇧T");
    expect(displayAccelerator("Alt+Shift+Cmd+T", mac)).toBe("⌘⌥⇧T");
    expect(displayAccelerator("Alt+Shift+S", mac)).toBe("⌥⇧S");
  });

  it("keeps Windows key names", () => {
    expect(displayAccelerator("Alt+Shift+T", win)).toBe("Alt+Shift+T");
    expect(displayAccelerator("Ctrl+Alt+Shift+T", win)).toBe("Ctrl+Alt+Shift+T");
    expect(displayAccelerator("Super+Alt+KeyT", win)).toBe("Alt+Win+T");
  });

  it("renders an unbound action as an empty string", () => {
    expect(displayAccelerator("", mac)).toBe("");
  });
});

describe("keyFromCode", () => {
  it("normalises the names the accelerator format uses", () => {
    expect(keyFromCode("ArrowLeft")).toBe("Left");
    expect(keyFromCode("Space")).toBe("Space");
    expect(keyFromCode("NumpadEnter")).toBe("Enter");
    expect(keyFromCode("KeyA")).toBe("A");
    expect(keyFromCode("F12")).toBe("F12");
    expect(keyFromCode("MouseLeft")).toBeNull();
  });
});
