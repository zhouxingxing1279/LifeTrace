export type MarkdownFormat =
  | "bold"
  | "italic"
  | "strike"
  | "inline-code"
  | "heading"
  | "quote"
  | "bullet"
  | "ordered"
  | "task"
  | "link"
  | "code-block"
  | "horizontal-rule";

export interface MarkdownEditResult {
  value: string;
  selectionStart: number;
  selectionEnd: number;
}

function wrap(value: string, start: number, end: number, before: string, after = before, placeholder = "text"): MarkdownEditResult {
  const selected = value.slice(start, end) || placeholder;
  const next = `${value.slice(0, start)}${before}${selected}${after}${value.slice(end)}`;
  const selectionStart = start + before.length;
  return { value: next, selectionStart, selectionEnd: selectionStart + selected.length };
}

function prefixLines(value: string, start: number, end: number, prefixer: (index: number) => string): MarkdownEditResult {
  const lineStart = value.lastIndexOf("\n", Math.max(0, start - 1)) + 1;
  const nextBreak = value.indexOf("\n", end);
  const lineEnd = nextBreak === -1 ? value.length : nextBreak;
  const block = value.slice(lineStart, lineEnd);
  const lines = block.split("\n");
  const prefixed = lines.map((line, index) => `${prefixer(index)}${line}`).join("\n");
  return {
    value: `${value.slice(0, lineStart)}${prefixed}${value.slice(lineEnd)}`,
    selectionStart: lineStart,
    selectionEnd: lineStart + prefixed.length,
  };
}

export function applyMarkdownFormat(value: string, start: number, end: number, format: MarkdownFormat): MarkdownEditResult {
  switch (format) {
    case "bold": return wrap(value, start, end, "**", "**", "bold text");
    case "italic": return wrap(value, start, end, "_", "_", "italic text");
    case "strike": return wrap(value, start, end, "~~", "~~", "strikethrough");
    case "inline-code": return wrap(value, start, end, "`", "`", "code");
    case "link": return wrap(value, start, end, "[", "](https://)", "link text");
    case "code-block": return wrap(value, start, end, "```\n", "\n```", "code");
    case "heading": return prefixLines(value, start, end, () => "## ");
    case "quote": return prefixLines(value, start, end, () => "> ");
    case "bullet": return prefixLines(value, start, end, () => "- ");
    case "ordered": return prefixLines(value, start, end, (index) => `${index + 1}. `);
    case "task": return prefixLines(value, start, end, () => "- [ ] ");
    case "horizontal-rule": {
      const insertion = `${start > 0 && value[start - 1] !== "\n" ? "\n" : ""}---\n`;
      const next = `${value.slice(0, start)}${insertion}${value.slice(end)}`;
      const cursor = start + insertion.length;
      return { value: next, selectionStart: cursor, selectionEnd: cursor };
    }
  }
}
