import assert from "node:assert/strict";
import test from "node:test";
import {
  localDateTimeToRfc3339,
  rfc3339ToLocalDateTime,
} from "../src/services/executionApi";

test("execution datetime helper returns RFC3339 UTC for valid local input", () => {
  const value = localDateTimeToRfc3339("2026-08-09T10:30");
  assert.ok(value);
  assert.match(value, /^2026-08-09T\d{2}:30:00\.000Z$/);
});

test("execution datetime helper rejects blank or invalid input", () => {
  assert.equal(localDateTimeToRfc3339(""), null);
  assert.equal(localDateTimeToRfc3339("not-a-date"), null);
});

test("execution datetime formatter is reversible to a datetime-local string", () => {
  const local = rfc3339ToLocalDateTime("2026-08-09T02:30:00.000Z");
  assert.match(local, /^2026-08-09T\d{2}:30$/);
  assert.equal(rfc3339ToLocalDateTime(null), "");
});
