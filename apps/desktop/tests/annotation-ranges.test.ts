import assert from "node:assert/strict";
import test from "node:test";
import {
  buildAnnotationSegments,
  resolveAnnotation,
  resolveAnnotations,
} from "../src/components/english/annotationRanges";
import type { EnglishHighlight, EnglishNote } from "../src/types/english";

const stamp = "2026-07-27T00:00:00.000Z";
const blocks = [
  { id: "body-0", text: "A repeated phrase appears here." },
  { id: "body-1", text: "The repeated phrase appears again." },
];

const highlight = (overrides: Partial<EnglishHighlight> = {}): EnglishHighlight => ({
  id: "highlight-1",
  userId: "local-user",
  articleId: "article-1",
  text: "repeated phrase",
  color: "yellow",
  createdAt: stamp,
  updatedAt: stamp,
  ...overrides,
});

const note = (overrides: Partial<EnglishNote> = {}): EnglishNote => ({
  id: "note-1",
  userId: "local-user",
  articleId: "article-1",
  content: "Useful collocation.",
  createdAt: stamp,
  updatedAt: stamp,
  ...overrides,
});

test("stable block offsets select the intended repeated phrase", () => {
  const resolved = resolveAnnotation(highlight({
    blockId: "body-1",
    startOffset: 4,
    endOffset: 19,
    selectedText: "repeated phrase",
  }), blocks);

  assert.deepEqual(resolved && {
    blockId: resolved.blockId,
    startOffset: resolved.startOffset,
    endOffset: resolved.endOffset,
  }, { blockId: "body-1", startOffset: 4, endOffset: 19 });
});

test("legacy text-only annotations recover without crashing", () => {
  const resolved = resolveAnnotation(highlight(), blocks);

  assert.equal(resolved?.blockId, "body-0");
  assert.equal(resolved?.startOffset, 2);
});

test("prefix and suffix recover a shifted annotation in its original block", () => {
  const changedBlocks = [{ id: "body-0", text: "New intro. A repeated phrase appears here." }];
  const resolved = resolveAnnotation(highlight({
    blockId: "body-0",
    startOffset: 2,
    endOffset: 17,
    prefix: "A ",
    suffix: " appears",
  }), changedBlocks);

  assert.equal(resolved?.startOffset, 13);
  assert.equal(resolved?.endOffset, 28);
});

test("segments partition overlapping highlight and note anchors without nested markup", () => {
  const activeHighlight = resolveAnnotations([highlight({
    blockId: "body-0",
    startOffset: 2,
    endOffset: 17,
  })], blocks);
  const activeNote = resolveAnnotations([note({
    blockId: "body-0",
    startOffset: 11,
    endOffset: 24,
    selectedText: "phrase appears",
  })], blocks);
  const segments = buildAnnotationSegments(blocks[0].text, activeHighlight, activeNote);

  assert.deepEqual(segments.map((segment) => ({
    text: segment.text,
    highlights: segment.highlightIds,
    notes: segment.noteIds,
  })), [
    { text: "A ", highlights: [], notes: [] },
    { text: "repeated ", highlights: ["highlight-1"], notes: [] },
    { text: "phrase", highlights: ["highlight-1"], notes: ["note-1"] },
    { text: " appears", highlights: [], notes: ["note-1"] },
    { text: " here.", highlights: [], notes: [] },
  ]);
});

test("unresolvable old annotations are ignored safely", () => {
  assert.equal(resolveAnnotation(highlight({ text: "missing phrase" }), blocks), null);
});
