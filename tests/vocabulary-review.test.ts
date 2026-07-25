import assert from "node:assert/strict";
import test from "node:test";
import { scheduleReview } from "../src/server/vocabulary/reviewScheduler";

const now = new Date("2026-07-25T00:00:00.000Z");
test("forgot resets stage and schedules tomorrow", () => {
  const result = scheduleReview(4, "FORGOT", now);
  assert.equal(result.stageAfter, 0);
  assert.equal(result.nextReviewAt, "2026-07-26T00:00:00.000Z");
  assert.equal(result.status, "LEARNING");
});
test("hard moves one stage back", () => assert.equal(scheduleReview(3, "HARD", now).stageAfter, 2));
test("good advances one stage", () => assert.equal(scheduleReview(2, "GOOD", now).stageAfter, 3));
test("easy advances two stages", () => assert.equal(scheduleReview(2, "EASY", now).stageAfter, 4));
test("stage intervals are transparent", () => {
  assert.equal(scheduleReview(0, "GOOD", now).nextReviewAt, "2026-07-26T00:00:00.000Z");
  assert.equal(scheduleReview(1, "GOOD", now).nextReviewAt, "2026-07-28T00:00:00.000Z");
  assert.equal(scheduleReview(2, "GOOD", now).nextReviewAt, "2026-08-01T00:00:00.000Z");
});
test("maximum stage becomes mastered", () => {
  const result = scheduleReview(5, "GOOD", now);
  assert.equal(result.stageAfter, 6); assert.equal(result.status, "MASTERED"); assert.equal(result.nextReviewAt, null);
});
test("stages are clamped", () => {
  assert.equal(scheduleReview(-10, "HARD", now).stageAfter, 0);
  assert.equal(scheduleReview(99, "EASY", now).stageAfter, 6);
});
