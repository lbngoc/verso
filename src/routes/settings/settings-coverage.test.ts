import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// FR-019 / SC-004: every setting that 0.1.3's one-page Settings wrote must still be reachable, and
// only one section may own it. The scan is static and deliberately simple: it reads the section
// components and looks for the field key.

const SECTION_DIR = join(process.cwd(), "src/routes/settings/sections");
const files = readdirSync(SECTION_DIR).filter((name) => name.endsWith(".svelte"));

function read(name: string): string {
  return readFileSync(join(SECTION_DIR, name), "utf8");
}

const contents = new Map(files.map((name) => [name, read(name)]));

// Some fields are written through a dedicated command instead of a patch object.
const NEEDLES: Record<string, string> = {
  text_size_step: "setTextSizeStep(",
  "ocr.enabled": "ocr: { enabled",
  "ocr.language": "ocr: { language",
};

const OWNERS: Record<string, string> = {
  target_lang: "Languages.svelte",
  secondary_lang: "Languages.svelte",
  source_lang: "Languages.svelte",
  engine_order: "Engines.svelte",
  engines_disabled: "Engines.svelte",
  "shortcuts.translate": "Shortcuts.svelte",
  "shortcuts.speak": "Shortcuts.svelte",
  "shortcuts.quick": "Shortcuts.svelte",
  quick_paste_clipboard: "Shortcuts.svelte",
  theme: "Appearance.svelte",
  accent: "Appearance.svelte",
  launch_at_login: "General.svelte",
  ui_language: "General.svelte",
  text_size_step: "PopupPanel.svelte",
  check_updates: "About.svelte",
  "ocr.enabled": "ScreenTranslate.svelte",
  "ocr.language": "ScreenTranslate.svelte",
  "shortcuts.screen": "ScreenTranslate.svelte",
};

describe("settings coverage (FR-019, SC-004)", () => {
  it("每个 0.1.3 写过的设置都有一个 section 负责", () => {
    for (const [field, owner] of Object.entries(OWNERS)) {
      // shortcuts.* are written through the recorder's action ids.
      const needle = field.startsWith("shortcuts.")
        ? `action="${field.split(".")[1]}"`
        : (NEEDLES[field] ?? `${field}:`);
      const source = contents.get(owner);
      expect(source, `${owner} exists`).toBeDefined();
      expect(source!.includes(needle), `${owner} writes ${field}`).toBe(true);
    }
  });

  it("a field is written by exactly one section", () => {
    for (const [field, owner] of Object.entries(OWNERS)) {
      if (field.startsWith("shortcuts.")) continue;
      const writers = [...contents.entries()]
        .filter(([, source]) => source.includes(NEEDLES[field] ?? `${field}:`))
        .map(([name]) => name);
      expect(writers, field).toEqual([owner]);
    }
  });

  it("the retired Expand setting is not written anywhere", () => {
    for (const [name, source] of contents) {
      expect(source.includes("popup_expanded"), name).toBe(false);
    }
  });
});
