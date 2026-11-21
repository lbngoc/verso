import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { parseMarkdown } from "./markdown";

const inlineText = (spans: { type: string; value: string }[]): string =>
  spans.map((span) => span.value).join("");

describe("parseMarkdown", () => {
  it("turns headings and paragraphs into blocks", () => {
    const blocks = parseMarkdown("### Added\n\n- one\n- two\n\nA paragraph.");
    expect(blocks[0]).toMatchObject({ type: "heading", level: 4 });
    expect(blocks[1]).toMatchObject({ type: "list", ordered: false });
    expect(blocks[2]).toMatchObject({ type: "paragraph" });
  });

  it("keeps one level of nested list items", () => {
    const blocks = parseMarkdown("- top\n  - child\n1. first\n2. second");
    const lists = blocks.filter((block) => block.type === "list");
    expect(lists).toHaveLength(2);
    expect(lists[0]).toMatchObject({ ordered: false });
    if (lists[0].type === "list") {
      expect(lists[0].items[0].nested).toBe(false);
      expect(lists[0].items[1].nested).toBe(true);
    }
    expect(lists[1]).toMatchObject({ ordered: true });
  });

  it("joins wrapped lines to their list item (T141)", () => {
    const blocks = parseMarkdown(
      "- Key hints in the footer: `↵` Translate and `⌘,`\n  Settings. The keys that work are highlighted.\n- Second item",
    );
    expect(blocks).toHaveLength(1);
    expect(blocks[0].type).toBe("list");
    if (blocks[0].type !== "list") return;
    expect(blocks[0].items).toHaveLength(2);
    const text = blocks[0].items[0].spans.map((span) => span.value).join("");
    expect(text).toBe(
      "Key hints in the footer: ↵ Translate and ⌘, Settings. The keys that work are highlighted.",
    );
  });

  it("ends a list at a blank line before a paragraph", () => {
    const blocks = parseMarkdown("- item\n\nParagraph after.");
    expect(blocks.map((block) => block.type)).toEqual(["list", "paragraph"]);
  });

  it("parses bold, italic and code", () => {
    const [paragraph] = parseMarkdown("**bold** *italic* `code`");
    expect(paragraph.type).toBe("paragraph");
    if (paragraph.type !== "paragraph") return;
    expect(paragraph.spans.map((span) => span.type)).toEqual([
      "strong",
      "text",
      "em",
      "text",
      "code",
    ]);
  });

  it("keeps http(s) links and drops other schemes to text", () => {
    const [paragraph] = parseMarkdown(
      "[site](https://example.com) [bad](javascript:alert(1)) [file](file:///etc/passwd)",
    );
    if (paragraph.type !== "paragraph") throw new Error("expected a paragraph");
    const link = paragraph.spans.find((span) => span.type === "link");
    expect(link).toMatchObject({ type: "link", href: "https://example.com" });
    const text = inlineText(paragraph.spans);
    expect(text).toContain("bad");
    expect(text).toContain("file");
    expect(paragraph.spans.filter((span) => span.type === "link")).toHaveLength(1);
  });

  it("drops images but keeps their alt text", () => {
    const [paragraph] = parseMarkdown("before ![shot](https://example.com/a.png) after");
    if (paragraph.type !== "paragraph") throw new Error("expected a paragraph");
    const text = inlineText(paragraph.spans);
    expect(text).toBe("before shot after");
  });

  it("keeps raw HTML as text", () => {
    const [paragraph] = parseMarkdown("<script>alert(1)</script>");
    if (paragraph.type !== "paragraph") throw new Error("expected a paragraph");
    expect(inlineText(paragraph.spans)).toBe("<script>alert(1)</script>");
  });

  it("handles the real release notes without throwing", () => {
    const notes = [
      "Stable signing identity for the preview.",
      "",
      "### Changed",
      "",
      "- macOS builds are signed with the same code-signing identity on every release, so the",
      "  Accessibility permission now survives updates.",
      "- The source is public at <https://github.com/lbngoc/verso> under the MIT licence.",
      "",
      "Signed (macOS): self-signed",
    ].join("\n");
    const blocks = parseMarkdown(notes);
    expect(blocks.length).toBeGreaterThan(2);
    expect(blocks[0]).toMatchObject({ type: "paragraph" });
    expect(blocks.some((block) => block.type === "heading")).toBe(true);
    expect(blocks.some((block) => block.type === "list")).toBe(true);
  });

  it("never builds markup with innerHTML", () => {
    const source = readFileSync(join(process.cwd(), "src/lib/markdown.ts"), "utf8");
    expect(source.includes("innerHTML")).toBe(false);
  });
});
