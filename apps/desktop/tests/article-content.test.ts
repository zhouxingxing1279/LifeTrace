import assert from "node:assert/strict";
import test from "node:test";
import { splitArticleReadingSections } from "../src/components/english/articleContent";

test("VOA underline separates article body from vocabulary notes", () => {
  const result = splitArticleReadingSections([
    "The article ends here.",
    "Quiz - Example",
    "_____________________________________________",
    "function -- n. a computer subroutine",
    "feature -- n. a prominent part",
  ].join("\n"));

  assert.deepEqual(result.bodyParagraphs, [
    "The article ends here.",
    "Quiz - Example",
  ]);
  assert.deepEqual(result.vocabularyLines, [
    "function -- n. a computer subroutine",
    "feature -- n. a prominent part",
  ]);
});

test("regular article content is not split into a glossary", () => {
  const result = splitArticleReadingSections("First paragraph.\n\nSecond paragraph.");

  assert.deepEqual(result.bodyParagraphs, ["First paragraph.", "Second paragraph."]);
  assert.deepEqual(result.vocabularyLines, []);
});

test("dash-based source dividers are recognized without exposing the divider", () => {
  const result = splitArticleReadingSections("Body\n----------\nword -- definition");

  assert.deepEqual(result, {
    bodyParagraphs: ["Body"],
    vocabularyLines: ["word -- definition"],
  });
});
