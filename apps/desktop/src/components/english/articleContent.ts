export type ArticleReadingSections = {
  bodyParagraphs: string[];
  vocabularyLines: string[];
};

const VOA_SECTION_DIVIDER = /^[\s_━─—–-]{8,}$/;

const nonEmptyLines = (value: string) => value
  .split(/\n+/)
  .map((line) => line.trim())
  .filter(Boolean);

/**
 * VOA places a long underline between the article and its vocabulary notes.
 * Keep the stored source intact, but expose the two semantic sections to the reader.
 */
export function splitArticleReadingSections(content: string): ArticleReadingSections {
  const normalized = content.replace(/\r\n?/g, "\n");
  const lines = normalized.split("\n");
  const dividerIndex = lines.findIndex((line) => VOA_SECTION_DIVIDER.test(line.trim()));

  if (dividerIndex < 0) {
    return {
      bodyParagraphs: nonEmptyLines(normalized),
      vocabularyLines: [],
    };
  }

  return {
    bodyParagraphs: nonEmptyLines(lines.slice(0, dividerIndex).join("\n")),
    vocabularyLines: nonEmptyLines(lines.slice(dividerIndex + 1).join("\n")),
  };
}
