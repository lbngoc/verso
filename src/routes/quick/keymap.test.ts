import { describe, expect, it } from "vitest";
import { quickKey } from "./keymap";

const key = (
  value: string,
  mods: { shiftKey?: boolean; metaKey?: boolean; ctrlKey?: boolean; altKey?: boolean } = {},
) => ({
  key: value,
  shiftKey: mods.shiftKey ?? false,
  metaKey: mods.metaKey ?? false,
  ctrlKey: mods.ctrlKey ?? false,
  altKey: mods.altKey ?? false,
});

describe("quickKey", () => {
  it("does nothing while an input method is composing (spec 004 FR-030)", () => {
    for (const name of ["Enter", "Tab", "Escape", "c", "s", "-"]) {
      expect(quickKey("input", { ...key(name), isComposing: true }, true)).toEqual({
        type: "none",
      });
      expect(quickKey("result", { ...key(name), keyCode: 229 }, true)).toEqual({ type: "none" });
    }
  });

  it("Tab swaps in both places and never moves focus", () => {
    expect(quickKey("input", key("Tab"), false)).toEqual({ type: "swap" });
    expect(quickKey("input", key("Tab", { shiftKey: true }), true)).toEqual({ type: "swap" });
    expect(quickKey("result", key("Tab"), true)).toEqual({ type: "swap" });
  });

  it("Esc in the input focuses the result, or closes when there is none", () => {
    expect(quickKey("input", key("Escape"), true)).toEqual({ type: "focus-result" });
    expect(quickKey("input", key("Escape"), false)).toEqual({ type: "close" });
  });

  it("Cmd+, and Ctrl+, open Settings from either focus", () => {
    expect(quickKey("input", key(",", { metaKey: true }), false)).toEqual({ type: "settings" });
    expect(quickKey("result", key(",", { ctrlKey: true }), true)).toEqual({ type: "settings" });
    expect(quickKey("input", key(","), false)).toEqual({ type: "none" });
    expect(quickKey("result", key(","), true)).toEqual({ type: "type", text: "," });
  });

  it("letters in the input are text", () => {
    expect(quickKey("input", key("a"), true)).toEqual({ type: "none" });
    expect(quickKey("input", key("-"), true)).toEqual({ type: "none" });
  });

  it("Enter translates now", () => {
    expect(quickKey("input", key("Enter"), true)).toEqual({ type: "translate" });
    expect(quickKey("input", key("Enter", { shiftKey: true }), true)).toEqual({ type: "none" });
  });

  it("single keys act on the result", () => {
    expect(quickKey("result", key("c"), true)).toEqual({ type: "copy" });
    expect(quickKey("result", key("s"), true)).toEqual({ type: "speak" });
    expect(quickKey("result", key("-"), true)).toEqual({ type: "text-size", delta: -1 });
    expect(quickKey("result", key("="), true)).toEqual({ type: "text-size", delta: 1 });
    expect(quickKey("result", key("0"), true)).toEqual({ type: "reset-size" });
    expect(quickKey("result", key("Escape"), true)).toEqual({ type: "close" });
  });

  it("another printable key returns to the input and types it", () => {
    expect(quickKey("result", key("h"), true)).toEqual({ type: "type", text: "h" });
    expect(quickKey("result", key(" "), true)).toEqual({ type: "type", text: " " });
    // Shortcuts keep their browser/menu meaning.
    expect(quickKey("result", key("c", { metaKey: true }), true)).toEqual({ type: "none" });
  });
});
