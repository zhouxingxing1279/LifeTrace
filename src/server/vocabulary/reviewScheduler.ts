export type ReviewResult = "FORGOT" | "HARD" | "GOOD" | "EASY";
export type VocabularyStatus = "LEARNING" | "REVIEWING" | "MASTERED" | "ARCHIVED";

const STAGE_DAYS = [1, 1, 3, 7, 14, 30] as const;

export function scheduleReview(stage: number, result: ReviewResult, now = new Date()) {
  const before = Math.max(0, Math.min(6, Math.trunc(stage)));
  const after = result === "FORGOT" ? 0
    : result === "HARD" ? Math.max(0, before - 1)
      : Math.min(6, before + (result === "EASY" ? 2 : 1));
  const status: VocabularyStatus = after >= 6 ? "MASTERED" : after > 0 ? "REVIEWING" : "LEARNING";
  const nextReviewAt = status === "MASTERED" ? null : new Date(now.getTime() + STAGE_DAYS[Math.min(after, 5)] * 86400000).toISOString();
  return { stageBefore: before, stageAfter: after, status, nextReviewAt };
}
