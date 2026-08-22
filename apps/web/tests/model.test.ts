import assert from "node:assert/strict";
import test from "node:test";
import { financeSummary, initialState, reviewMetrics, searchState } from "../src/v2/model";

test("finance summary keeps money in minor units", () => {
  const summary = financeSummary([
    { id: "1", date: "2026-08-21", title: "Salary", category: "income", account: "bank", amountCents: 100_00, direction: "income" },
    { id: "2", date: "2026-08-21", title: "Lunch", category: "food", account: "card", amountCents: 35_50, direction: "expense" }
  ]);
  assert.deepEqual(summary, { income: 100_00, expense: 35_50, balance: 64_50 });
});

test("review metrics handle empty state without NaN", () => {
  const metrics = reviewMetrics(initialState(), "2026-08-21");
  assert.equal(metrics.taskCompletion, 0);
  assert.equal(metrics.habitCompletion, 0);
});

test("global search crosses feature boundaries", () => {
  const state = initialState();
  state.tasks.push({ id: "task", title: "Prepare frontend v2", dueDate: "2026-08-21", project: "LifeTrace", priority: "high", completed: false });
  state.notes.push({ id: "note", title: "Apple UI", content: "token system", updatedAt: new Date(0).toISOString(), pinned: false });
  assert.equal(searchState(state, "frontend")[0]?.path, "/app/execution");
  assert.equal(searchState(state, "token")[0]?.path, "/app/notes");
});
