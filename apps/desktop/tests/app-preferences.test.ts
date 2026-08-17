import assert from "node:assert/strict";
import test from "node:test";

import {
  APP_PREFERENCES_STORAGE_KEY,
  applyAppPreferences,
  DEFAULT_APP_PREFERENCES,
  readAppPreferences,
  setAppThemePreference,
  writeAppPreferences,
} from "../src/services/appPreferences";

class MemoryStorage {
  private readonly values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

test("application preferences survive local persistence and reject invalid values", () => {
  const storage = new MemoryStorage();
  writeAppPreferences({ theme: "dark", density: "compact", fontScale: "large", reduceMotion: true }, storage);
  assert.deepEqual(readAppPreferences(storage), {
    theme: "dark",
    density: "compact",
    fontScale: "large",
    reduceMotion: true,
  });

  storage.setItem(APP_PREFERENCES_STORAGE_KEY, JSON.stringify({ theme: "unknown", density: 1 }));
  assert.deepEqual(readAppPreferences(storage), DEFAULT_APP_PREFERENCES);
});

test("application preferences are applied as root data attributes", () => {
  const root = { dataset: {}, style: {} } as unknown as HTMLElement;
  applyAppPreferences({ theme: "dark", density: "compact", fontScale: "small", reduceMotion: true }, root);
  assert.equal(root.dataset.theme, "dark");
  assert.equal(root.dataset.density, "compact");
  assert.equal(root.dataset.fontScale, "small");
  assert.equal(root.dataset.reduceMotion, "true");
  assert.equal(root.style.fontSize, "14px");
  assert.equal(root.style.colorScheme, "dark");
});

test("setting a desktop theme persists and applies through one preference path", () => {
  const storage = new MemoryStorage();
  const root = { dataset: {}, style: {} } as unknown as HTMLElement;

  setAppThemePreference("dark", storage, root);
  assert.equal(readAppPreferences(storage).theme, "dark");
  assert.equal(root.dataset.themePreference, "dark");
  assert.equal(root.dataset.theme, "dark");
  assert.equal(root.style.colorScheme, "dark");

  setAppThemePreference("light", storage, root);
  assert.equal(readAppPreferences(storage).theme, "light");
  assert.equal(root.dataset.theme, "light");
});
