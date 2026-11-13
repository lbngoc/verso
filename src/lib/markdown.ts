// Markdown subset for release notes (spec 009, contracts/update-dialog.md).
//
// Headings, paragraphs, one list level, **bold**, *italic*, `code` and [text](http(s)://…) links.
// Images and raw HTML never become markup; DOM is built with createElement/textContent only.

export type InlineNode =
  | { type: "text"; value: string }
  | { type: "strong"; value: string }
  | { type: "em"; value: string }
  | { type: "code"; value: string }
  | { type: "link"; value: string; href: string };

export type ListItem = { spans: InlineNode[]; nested: boolean };

export type Block =
  | { type: "heading"; level: 3 | 4; spans: InlineNode[] }
  | { type: "paragraph"; spans: InlineNode[] }
  | { type: "list"; ordered: boolean; items: ListItem[] };

const SAFE_SCHEMES = ["http:", "https:"];

function safeHref(raw: string): string | null {
  try {
    const url = new URL(raw, "https://example.invalid");
    return SAFE_SCHEMES.includes(url.protocol) ? raw : null;
  } catch {
    return null;
  }
}

function parseInline(source: string): InlineNode[] {
  const nodes: InlineNode[] = [];
  let index = 0;

  const pushText = (value: string): void => {
    if (!value) return;
    const last = nodes[nodes.length - 1];
    if (last && last.type === "text") {
      last.value += value;
      return;
    }
    nodes.push({ type: "text", value });
  };

  while (index < source.length) {
    const rest = source.slice(index);

    const image = /^!\[([^\]]*)\]\(([^)]*)\)/.exec(rest);
    if (image) {
      // Images are dropped; their alt text stays as plain text.
      pushText(image[1]);
      index += image[0].length;
      continue;
    }

    const link = /^\[([^\]]+)\]\(([^)]+)\)/.exec(rest);
    if (link) {
      const href = safeHref(link[2].trim());
      if (href) nodes.push({ type: "link", value: link[1], href });
      else pushText(link[1]);
      index += link[0].length;
      continue;
    }

    const code = /^`([^`]+)`/.exec(rest);
    if (code) {
      nodes.push({ type: "code", value: code[1] });
      index += code[0].length;
      continue;
    }

    const strong = /^\*\*([^*]+)\*\*/.exec(rest);
    if (strong) {
      nodes.push({ type: "strong", value: strong[1] });
      index += strong[0].length;
      continue;
    }

    const em = /^\*([^*]+)\*/.exec(rest);
    if (em) {
      nodes.push({ type: "em", value: em[1] });
      index += em[0].length;
      continue;
    }

    if (rest.startsWith("<")) {
      // Raw HTML is kept as text, never parsed.
      const tag = /^<\/?[A-Za-z][^>]*>/.exec(rest);
      if (tag) {
        pushText(tag[0]);
        index += tag[0].length;
        continue;
      }
    }

    // A single character that starts none of the patterns above is plain text.
    pushText(source[index]);
    index += 1;
  }

  return nodes;
}

const HEADING = /^(#{1,3})\s+(.*)$/;
const BULLET = /^(\s*)[-*]\s+(.*)$/;
const NUMBERED = /^(\s*)\d+\.\s+(.*)$/;

export function parseMarkdown(source: string): Block[] {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  let list: { ordered: boolean; items: ListItem[] } | null = null;

  const flushParagraph = (): void => {
    if (!paragraph.length) return;
    blocks.push({ type: "paragraph", spans: parseInline(paragraph.join(" ").trim()) });
    paragraph = [];
  };
  const flushList = (): void => {
    if (!list) return;
    blocks.push({ type: "list", ordered: list.ordered, items: list.items });
    list = null;
  };

  for (const line of lines) {
    if (!line.trim()) {
      flushParagraph();
      flushList();
      continue;
    }

    const heading = HEADING.exec(line);
    if (heading) {
      flushParagraph();
      flushList();
      blocks.push({
        type: "heading",
        level: heading[1].length >= 3 ? 4 : 3,
        spans: parseInline(heading[2].trim()),
      });
      continue;
    }

    const bullet = BULLET.exec(line);
    const numbered = NUMBERED.exec(line);
    const item = bullet ?? numbered;
    if (item) {
      flushParagraph();
      const ordered = Boolean(numbered);
      if (!list || list.ordered !== ordered) {
        flushList();
        list = { ordered, items: [] };
      }
      list.items.push({
        spans: parseInline(item[2].trim()),
        nested: item[1].length >= 2,
      });
      continue;
    }

    flushList();
    paragraph.push(line.trim());
  }

  flushParagraph();
  flushList();
  return blocks;
}

/** Builds DOM nodes for the parsed blocks; links go through `open` (the Rust `open_url` command). */
export function renderMarkdown(
  source: string,
  document: Document,
  open: (href: string) => void,
): DocumentFragment {
  const fragment = document.createDocumentFragment();

  const inline = (parent: Node, spans: InlineNode[]): void => {
    for (const span of spans) {
      switch (span.type) {
        case "text":
          parent.appendChild(document.createTextNode(span.value));
          break;
        case "strong": {
          const strong = document.createElement("strong");
          strong.textContent = span.value;
          parent.appendChild(strong);
          break;
        }
        case "em": {
          const em = document.createElement("em");
          em.textContent = span.value;
          parent.appendChild(em);
          break;
        }
        case "code": {
          const code = document.createElement("code");
          code.textContent = span.value;
          parent.appendChild(code);
          break;
        }
        case "link": {
          const link = document.createElement("a");
          link.textContent = span.value;
          link.href = "#";
          link.addEventListener("click", (event) => {
            event.preventDefault();
            open(span.href);
          });
          parent.appendChild(link);
          break;
        }
      }
    }
  };

  for (const block of parseMarkdown(source)) {
    if (block.type === "heading") {
      const heading = document.createElement(block.level === 4 ? "h4" : "h3");
      inline(heading, block.spans);
      fragment.appendChild(heading);
    } else if (block.type === "paragraph") {
      const paragraph = document.createElement("p");
      inline(paragraph, block.spans);
      fragment.appendChild(paragraph);
    } else {
      const list = document.createElement(block.ordered ? "ol" : "ul");
      for (const item of block.items) {
        const li = document.createElement("li");
        if (item.nested) li.classList.add("nested");
        inline(li, item.spans);
        list.appendChild(li);
      }
      fragment.appendChild(list);
    }
  }

  return fragment;
}
