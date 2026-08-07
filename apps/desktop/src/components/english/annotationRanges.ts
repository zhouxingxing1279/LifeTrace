import type { EnglishHighlight, EnglishNote } from "@/src/types/english";

export type ArticleTextBlock = {
  id: string;
  text: string;
};

export type AnnotationLike = Pick<EnglishHighlight, "id" | "blockId" | "startOffset" | "endOffset" | "prefix" | "suffix">
  & Partial<Pick<EnglishHighlight, "selectedText" | "text">>
  & Partial<Pick<EnglishNote, "quote">>;

export type ResolvedAnnotation<T extends AnnotationLike> = {
  annotation: T;
  blockId: string;
  startOffset: number;
  endOffset: number;
};

export type AnnotationSegment = {
  text: string;
  startOffset: number;
  endOffset: number;
  highlightIds: string[];
  noteIds: string[];
};

const selectedTextOf = (annotation: AnnotationLike) =>
  annotation.selectedText?.trim() || annotation.text?.trim() || annotation.quote?.trim() || "";

const contextMatches = (text: string, start: number, end: number, annotation: AnnotationLike) => {
  const prefix = annotation.prefix?.trim();
  const suffix = annotation.suffix?.trim();
  return (!prefix || text.slice(Math.max(0, start - prefix.length), start).endsWith(prefix))
    && (!suffix || text.slice(end, end + suffix.length).startsWith(suffix));
};

const findWithContext = (text: string, selectedText: string, annotation: AnnotationLike) => {
  let cursor = 0;
  let fallback = -1;
  while (cursor <= text.length - selectedText.length) {
    const index = text.indexOf(selectedText, cursor);
    if (index < 0) break;
    if (fallback < 0) fallback = index;
    if (contextMatches(text, index, index + selectedText.length, annotation)) return index;
    cursor = index + Math.max(1, selectedText.length);
  }
  return fallback;
};

/**
 * Resolve both current anchors and legacy text-only annotations.
 * Exact block offsets win; context-assisted text lookup is the compatibility fallback.
 */
export function resolveAnnotation<T extends AnnotationLike>(
  annotation: T,
  blocks: ArticleTextBlock[],
): ResolvedAnnotation<T> | null {
  const selectedText = selectedTextOf(annotation);
  if (!selectedText) return null;

  const preferredBlock = annotation.blockId
    ? blocks.find((block) => block.id === annotation.blockId)
    : undefined;
  if (
    preferredBlock
    && Number.isInteger(annotation.startOffset)
    && Number.isInteger(annotation.endOffset)
    && (annotation.startOffset ?? -1) >= 0
    && (annotation.endOffset ?? 0) > (annotation.startOffset ?? 0)
    && (annotation.endOffset ?? 0) <= preferredBlock.text.length
  ) {
    const startOffset = annotation.startOffset!;
    const endOffset = annotation.endOffset!;
    if (preferredBlock.text.slice(startOffset, endOffset) === selectedText) {
      return { annotation, blockId: preferredBlock.id, startOffset, endOffset };
    }
  }

  const candidates = preferredBlock ? [preferredBlock] : blocks;
  for (const block of candidates) {
    const startOffset = findWithContext(block.text, selectedText, annotation);
    if (startOffset >= 0) {
      return {
        annotation,
        blockId: block.id,
        startOffset,
        endOffset: startOffset + selectedText.length,
      };
    }
  }
  return null;
}

export function resolveAnnotations<T extends AnnotationLike>(
  annotations: T[],
  blocks: ArticleTextBlock[],
) {
  return annotations
    .map((annotation) => resolveAnnotation(annotation, blocks))
    .filter((annotation): annotation is ResolvedAnnotation<T> => Boolean(annotation));
}

/**
 * Partition a paragraph at every annotation boundary. This avoids nested marks
 * when ranges touch or overlap and lets highlights and note anchors share a range.
 */
export function buildAnnotationSegments(
  text: string,
  highlights: Array<ResolvedAnnotation<EnglishHighlight>>,
  notes: Array<ResolvedAnnotation<EnglishNote>>,
): AnnotationSegment[] {
  const validHighlights = highlights.filter((item) =>
    item.startOffset >= 0 && item.endOffset > item.startOffset && item.endOffset <= text.length,
  );
  const validNotes = notes.filter((item) =>
    item.startOffset >= 0 && item.endOffset > item.startOffset && item.endOffset <= text.length,
  );
  const boundaries = new Set([0, text.length]);
  [...validHighlights, ...validNotes].forEach((item) => {
    boundaries.add(item.startOffset);
    boundaries.add(item.endOffset);
  });
  const offsets = [...boundaries].sort((left, right) => left - right);

  return offsets.slice(0, -1).map((startOffset, index) => {
    const endOffset = offsets[index + 1];
    return {
      text: text.slice(startOffset, endOffset),
      startOffset,
      endOffset,
      highlightIds: validHighlights
        .filter((item) => item.startOffset < endOffset && item.endOffset > startOffset)
        .map((item) => item.annotation.id),
      noteIds: validNotes
        .filter((item) => item.startOffset < endOffset && item.endOffset > startOffset)
        .map((item) => item.annotation.id),
    };
  }).filter((segment) => segment.text);
}
