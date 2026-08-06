import assert from "node:assert/strict";
import test from "node:test";
import { BROWSER_HOST, BROWSER_PORT, DEFAULT_LIFETRACE_CLOUD_URL } from "../vite.browser.config";

test("browser application uses the documented local listeners", () => {
  assert.equal(BROWSER_HOST, "0.0.0.0");
  assert.equal(BROWSER_PORT, 4173);
  assert.equal(DEFAULT_LIFETRACE_CLOUD_URL, "http://127.0.0.1:8787");
});
