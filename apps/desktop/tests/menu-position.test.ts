
import assert from "node:assert/strict";
import test from "node:test";
import { clampMenuPosition } from "../src/ui/menu/menuPosition";

test("keeps a menu at the requested position when it fits", () => {
  assert.deepEqual(clampMenuPosition({
    x: 120,
    y: 80,
    width: 200,
    height: 300,
    viewportWidth: 1200,
    viewportHeight: 800,
  }), { x: 120, y: 80 });
});

test("moves a menu back inside the lower-right viewport edge", () => {
  assert.deepEqual(clampMenuPosition({
    x: 1180,
    y: 780,
    width: 220,
    height: 260,
    viewportWidth: 1200,
    viewportHeight: 800,
  }), { x: 972, y: 532 });
});

test("honors viewport padding at the upper-left edge", () => {
  assert.deepEqual(clampMenuPosition({
    x: -20,
    y: -40,
    width: 180,
    height: 200,
    viewportWidth: 900,
    viewportHeight: 600,
    padding: 12,
  }), { x: 12, y: 12 });
});
