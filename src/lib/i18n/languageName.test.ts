import { describe, expect, it } from "vitest";
import { languageName } from "./index";

describe("languageName (FR-019)", () => {
  it("keeps the English name for the English UI", () => {
    expect(languageName("en", { code: "vi", name_en: "Vietnamese" })).toBe("Vietnamese");
  });

  it("names languages in Vietnamese for the Vietnamese UI", () => {
    expect(languageName("vi", { code: "en", name_en: "English" })).toBe("Tiếng Anh");
    expect(languageName("vi", { code: "vi", name_en: "Vietnamese" })).toBe("Tiếng Việt");
  });

  it("falls back to English for codes the system cannot name", () => {
    expect(languageName("vi", { code: "not-a-code", name_en: "Something" })).toBe("Something");
  });
});
