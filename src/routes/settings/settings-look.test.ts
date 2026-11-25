// Settings look audit (spec 011 R9, contracts/settings-look.md § Audit, SC-002): Settings, Setup and
// the About update card use only the shared text levels and the shared Button, and no action button
// sits in a centred block.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { describe, expect, it } from "vitest";

const root = resolve(__dirname, "../../..");

function svelteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return svelteFiles(path);
    return name.endsWith(".svelte") ? [path] : [];
  });
}

const components = [
  "Card.svelte",
  "SettingRow.svelte",
  "PermissionRow.svelte",
  "PermissionGuide.svelte",
  "UpdateCard.svelte",
  "ScreenTranslateCard.svelte",
].map((name) => join(root, "src/lib/components", name));

// Paths use "/" in this file, also on Windows, where node:path joins with "\\".
const slash = (path: string) => path.split(sep).join("/");

const files = [
  ...svelteFiles(join(root, "src/routes/settings")),
  ...svelteFiles(join(root, "src/routes/onboarding")),
  ...components.filter((path) => {
    try {
      return statSync(path).isFile();
    } catch {
      return false;
    }
  }),
].map(slash);

/** Allowed deviations, each with its reason. Keep this list short. */
const EXCEPTIONS: Array<{
  file: string;
  rule: "font-size" | "centred" | "raw-button";
  why: string;
}> = [];

function excepted(file: string, rule: string): boolean {
  return EXCEPTIONS.some((entry) => entry.file === file && entry.rule === rule);
}

function parts(source: string): { markup: string; style: string } {
  const style = [...source.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)]
    .map((match) => match[1])
    .join("\n");
  const markup = source
    .replace(/<style[^>]*>[\s\S]*?<\/style>/g, "")
    .replace(/<script[^>]*>[\s\S]*?<\/script>/g, "");
  return { markup, style };
}

/** The inner markup of every element carrying `className` (approximate tag matching). */
function elementsWithClass(markup: string, className: string): string[] {
  const found: string[] = [];
  const opening = new RegExp(
    `<([A-Za-z][\\w.-]*)\\b[^>]*?(?:class="[^"]*\\b${className}\\b[^"]*"|class:${className}\\b)[^>]*>`,
    "g",
  );
  for (const match of markup.matchAll(opening)) {
    const tag = match[1];
    const start = (match.index ?? 0) + match[0].length;
    if (match[0].endsWith("/>")) continue;
    const tags = new RegExp(`<${tag}\\b[^>]*?(/?)>|</${tag}>`, "g");
    tags.lastIndex = start;
    let depth = 1;
    let end = markup.length;
    for (let next = tags.exec(markup); next; next = tags.exec(markup)) {
      if (next[0].startsWith("</")) depth -= 1;
      else if (next[1] !== "/") depth += 1;
      if (depth === 0) {
        end = next.index;
        break;
      }
    }
    found.push(markup.slice(start, end));
  }
  return found;
}

function offences(path: string) {
  const file = slash(relative(root, path));
  const { markup, style } = parts(readFileSync(path, "utf8"));
  const found: string[] = [];

  if (!excepted(file, "font-size")) {
    for (const match of style.matchAll(/font-size:\s*([^;}\n]+)/g)) {
      const value = match[1].trim();
      if (/^\d|^\.\d/.test(value)) found.push(`${file}: literal font-size ${value}`);
    }
  }

  if (!excepted(file, "centred")) {
    for (const rule of style.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const [, selector, body] = rule;
      if (!/text-align:\s*center|justify-content:\s*center/.test(body)) continue;
      const classes = [...selector.matchAll(/\.([\w-]+)/g)].map((match) => match[1]);
      const last = classes.at(-1);
      if (!last) continue;
      for (const inner of elementsWithClass(markup, last)) {
        if (/<button\b|<Button\b/.test(inner)) {
          found.push(`${file}: .${last} centres a block that holds a button`);
        }
      }
    }
  }

  if (!excepted(file, "raw-button")) {
    const raw = markup.match(/<button\b/g)?.length ?? 0;
    if (raw > 0) found.push(`${file}: ${raw} raw <button> (use Button.svelte)`);
  }
  return found;
}

describe("Settings look audit (spec 011 SC-002)", () => {
  it("scans Settings, Setup and the shared Settings components", () => {
    expect(files.some((path) => path.endsWith("routes/settings/sections/About.svelte"))).toBe(true);
    expect(files.some((path) => path.endsWith("routes/onboarding/+page.svelte"))).toBe(true);
    expect(files.some((path) => path.endsWith("components/SettingRow.svelte"))).toBe(true);
  });

  it("has no literal font size, no centred button block and no raw <button>", () => {
    const all = files.flatMap(offences);
    expect(all, all.join("\n")).toEqual([]);
  });

  it("every exception names its reason", () => {
    for (const entry of EXCEPTIONS) expect(entry.why.trim(), entry.file).not.toBe("");
  });

  it("catches each kind of offence", () => {
    const sample = `
<div class="hero"><button>Go</button></div>
<style>
  .hero { text-align: center; font-size: 12px; }
</style>`;
    const { markup, style } = parts(sample);
    expect(style).toContain("font-size: 12px");
    expect(elementsWithClass(markup, "hero")[0]).toContain("<button>");
  });
});
