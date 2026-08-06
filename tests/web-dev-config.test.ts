import assert from "node:assert/strict";
import test from "node:test";
import { DEFAULT_LIFETRACE_CLOUD_URL } from "../vite.web.config";

test("Web dev proxy targets the default LifeTrace Cloud listener", () => {
  assert.equal(DEFAULT_LIFETRACE_CLOUD_URL, "http://127.0.0.1:8787");
});
