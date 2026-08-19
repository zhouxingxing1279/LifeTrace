export type MarkdownFormat =
  | "heading"
  | "bold"
  | "italic"
  | "strike"
  | "inline-code"
  | "quote"
  | "unordered"
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

function wrap(value: string, start: number, end: number, prefix: string, suffix = prefix, placeholder = "文本"): MarkdownEditResult {
  const selected = value.slice(start, end) || placeholder;
  const inserted = `${prefix}${selected}${suffix}`;
  return {
    value: `${value.slice(0, start)}${inserted}${value.slice(end)}`,
    selectionStart: start + prefix.length,
    selectionEnd: start + prefix.length + selected.length,
  };
}

function prefixLines(value: string, start: number, end: number, prefixer: (index: number) => string): MarkdownEditResult {
  const lineStart = value.lastIndexOf("\n", Math.max(0, start - 1)) + 1;
  const lineEndIndex = value.indexOf("\n", end);
  const lineEnd = lineEndIndex === -1 ? value.length : lineEndIndex;
  const block = value.slice(lineStart, lineEnd);
  const lines = block.split("\n");
  const transformed = lines.map((line, index) => `${prefixer(index)}${line}`).join("\n");
  return {
    value: `${value.slice(0, lineStart)}${transformed}${value.slice(lineEnd)}`,
    selectionStart: lineStart,
    selectionEnd: lineStart + transformed.length,
  };
}

export function applyMarkdownFormat(value: string, start: number, end: number, format: MarkdownFormat): MarkdownEditResult {
  switch (format) {
    case "heading":
      return prefixLines(value, start, end, () => "## ");
    case "bold":
      return wrap(value, start, end, "**", "**", "粗体文本");
    case "italic":
      return wrap(value, start, end, "*", "*", "斜体文本");
    case "strike":
      return wrap(value, start, end, "~~", "~~", "删除线文本");
    case "inline-code":
      return wrap(value, start, end, "`", "`", "code");
    case "quote":
      return prefixLines(value, start, end, () => "> ");
    case "unordered":
      return prefixLines(value, start, end, () => "- ");
    case "ordered":
      return prefixLines(value, start, end, (index) => `${index + 1}. `);
    case "task":
      return prefixLines(value, start, end, () => "- [ ] ");
    case "link":
      return wrap(value, start, end, "[", "](https://)", "链接文字");
    case "code-block": {
      const selected = value.slice(start, end) || "code";
      const inserted = `\`\`\`\n${selected}\n\`\`\``;
      return {
        value: `${value.slice(0, start)}${inserted}${value.slice(end)}`,
        selectionStart: start + 4,
        selectionEnd: start + 4 + selected.length,
      };
    }
    case "horizontal-rule": {
      const prefix = start > 0 && value[start - 1] !== "\n" ? "\n\n" : "";
      const inserted = `${prefix}---\n`;
      return {
        value: `${value.slice(0, start)}${inserted}${value.slice(end)}`,
        selectionStart: start + inserted.length,
        selectionEnd: start + inserted.length,
      };
    }
  }
}

export function markdownSummary(markdown: string, limit = 160): string {
  return markdown
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/!\[[^\]]*\]\([^)]*\)/g, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/^\s*[-*>+]\s+/gm, "")
    .replace(/^\s*\d+\.\s+/gm, "")
    .replace(/\*\*|__|~~|`|\*/g, "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, limit);
}
