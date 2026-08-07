import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import {
  ENTITY_TYPES, REQUESTED_SCOPES, createBrowserFetch, createDailyReview,
  createHabitActivity, createHabitLog, createTrainingNote, createWorkout,
} from "../web-client/src/core";
import { NAV_GROUPS, ROUTES, SECONDARY_NAV } from "../web-client/src/navigation";

function jsonResponse(payload: unknown) {
  return new Response(JSON.stringify(payload), { status: 200, headers: { "content-type": "application/json" } });
}

test("browser fetch preserves the global receiver before entering Chromium networking", async () => {
  const original = globalThis.fetch;
  let receiver: unknown;
  globalThis.fetch = function (this: typeof globalThis) {
    receiver = this;
    return Promise.resolve(jsonResponse({ ok: true }));
  } as typeof fetch;
  try {
    await createBrowserFetch()("http://127.0.0.1/test");
    assert.equal(receiver, globalThis);
  } finally {
    globalThis.fetch = original;
  }
});

test("browser navigation keeps every application module except photos", () => {
  const routes = [...NAV_GROUPS.flatMap((group) => group.items), ...SECONDARY_NAV].map((item) => item.route);
  for (const required of ["/", "/assistant", "/habits", "/english/articles", "/fitness", "/notes", "/calendar", "/review", "/finance", "/finance/transactions", "/finance/accounts", "/finance/import", "/devices", "/settings"]) {
    assert.ok(routes.includes(required as never), required);
    assert.ok(ROUTES.has(required as never), required);
  }
  assert.equal(routes.some((route) => route.includes("photo") || route.includes("vault")), false);
});

test("browser sync registry includes all non-photo product domains", () => {
  for (const entityType of ["habit.activity", "habit.log", "review.daily", "workout.workout", "workout.exercise", "workout.set", "workout.training_note", "finance.transaction", "note.note", "english.learning_record", "user.preference"]) {
    assert.ok(ENTITY_TYPES.includes(entityType as never), entityType);
  }
  assert.equal(ENTITY_TYPES.some((value) => value.includes("photo") || value.includes("vault")), false);
  for (const scope of ["habits:read", "habits:write", "reviews:read", "reviews:write", "workouts:read", "workouts:write"]) assert.ok(REQUESTED_SCOPES.includes(scope as never), scope);
});

test("browser build remains a normal web application rather than a PWA", () => {
  const packageJson = JSON.parse(readFileSync("package.json", "utf8")) as { scripts?: Record<string, string> };
  const scripts = packageJson.scripts ?? {};
  assert.equal(Object.keys(scripts).some((name) => name.startsWith("pwa:")), false);
  assert.equal(existsSync("vite.web.config.ts"), false);
  assert.equal(existsSync("public/manifest.webmanifest"), false);
  assert.equal(existsSync("public/sw.js"), false);
});

test("local browser authentication uses a cookie name accepted without HTTPS", () => {
  const compose = readFileSync("../../deploy/cloud/docker-compose.local.yml", "utf8");
  assert.match(compose, /AUTH_COOKIE_NAME:\s*\$\{AUTH_COOKIE_NAME:-lifetrace_session\}/);
  assert.doesNotMatch(compose, /AUTH_COOKIE_NAME:\s*\$\{AUTH_COOKIE_NAME:-__Host-/);
});

test("habit review and workout factories produce syncable payloads", () => {
  const habit = createHabitActivity("u", "d", { name: "练钢琴", unit: "分钟", normalTarget: 30 });
  const log = createHabitLog("u", "d", habit.meta.id, 35, "完成音阶练习", "2026-08-07");
  const review = createDailyReview("u", "d", { reviewDate: "2026-08-07", energy: 4, mood: 4, bestThing: "完成训练", tomorrowPriority: "阅读" });
  const workout = createWorkout("u", "d", { name: "胸肩训练", durationMinutes: 60, exerciseCount: 5, setCount: 20, volumeKg: 5200 });
  const unnamedWorkout = createWorkout("u", "d", { durationMinutes: 45 });
  const note = createTrainingNote("u", "d", "卧推复盘", "下一次保持肩胛稳定", workout.meta.id);
  assert.equal(habit.name, "练钢琴");
  assert.equal(log.activityId, habit.meta.id);
  assert.equal(log.logDate, "2026-08-07");
  assert.equal(review.reviewDate, "2026-08-07");
  assert.equal(workout.durationSeconds, 3600);
  assert.equal(unnamedWorkout.name, "训练");
  assert.equal(unnamedWorkout.durationSeconds, 2700);
  assert.equal(note.workoutId, workout.meta.id);
});
