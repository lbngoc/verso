import { describe, expect, it } from "vitest";
import en from "./en.json";
import vi from "./vi.json";

type Catalog = Record<string, string>;

function placeholders(template: string): string[] {
  return [...template.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();
}

describe("i18n parity (FR-019)", () => {
  const enKeys = Object.keys(en as Catalog).sort();
  const viKeys = Object.keys(vi as Catalog).sort();

  it("both catalogs have the same keys", () => {
    expect(viKeys).toEqual(enKeys);
  });

  it("each key keeps the same placeholders in both languages", () => {
    for (const key of enKeys) {
      const english = (en as Catalog)[key];
      const vietnamese = (vi as Catalog)[key];
      if (vietnamese === undefined) continue; // reported by the key-set test
      expect(placeholders(vietnamese), key).toEqual(placeholders(english));
    }
  });

  it("no value is left empty", () => {
    for (const [key, value] of Object.entries(en as Catalog)) {
      expect(value.trim(), `en.${key}`).not.toBe("");
    }
    for (const [key, value] of Object.entries(vi as Catalog)) {
      expect(value.trim(), `vi.${key}`).not.toBe("");
    }
  });
});
