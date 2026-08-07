import assert from "node:assert/strict";
import test from "node:test";

import {
  BUILTIN_SKINS,
  contrastRatio,
  createSkinId,
  CUSTOM_SKINS_STORAGE_KEY,
  deleteCustomSkin,
  exportSkinDocument,
  getSkinLibrary,
  importSkinDocument,
  normalizeSkin,
  readActiveSkinId,
  readCustomSkins,
  resolveSkin,
  saveCustomSkin,
  skinToCss,
  writeActiveSkinId,
} from "../src/services/skinEngine";

class MemoryStorage {
  private readonly values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }
}

test("skin normalization clamps visual values and rejects CSS injection through colors", () => {
  const normalized = normalizeSkin({
    id: "../../../Bad Skin",
    name: "  Custom  ",
    light: {
      primary: "red;display:none",
      background: "#abcdef",
    },
    visual: {
      roundness: 999,
      shadowStrength: -10,
      surfaceOpacity: 20,
    },
    background: {
      type: "gradient",
      angle: 800,
      opacity: 95,
    },
  });

  assert.equal(normalized.id, "bad-skin");
  assert.equal(normalized.name, "Custom");
  assert.equal(normalized.light.background, "#ABCDEF");
  assert.equal(normalized.light.primary, BUILTIN_SKINS[0].light.primary);
  assert.equal(normalized.visual.roundness, 100);
  assert.equal(normalized.visual.shadowStrength, 0);
  assert.equal(normalized.visual.surfaceOpacity, 72);
  assert.equal(normalized.background.angle, 360);
  assert.equal(normalized.background.opacity, 70);
});

test("custom skin persistence, resolution and active fallback are deterministic", () => {
  const storage = new MemoryStorage();
  const custom = {
    ...structuredClone(BUILTIN_SKINS[1]),
    id: "my-forest",
    name: "My Forest",
    builtIn: false,
  };

  const saved = saveCustomSkin(custom, storage);
  assert.equal(readCustomSkins(storage).length, 1);
  assert.equal(resolveSkin("my-forest", storage).name, "My Forest");
  assert.equal(getSkinLibrary(storage).length, BUILTIN_SKINS.length + 1);
  assert.ok(storage.getItem(CUSTOM_SKINS_STORAGE_KEY)?.includes("My Forest"));

  writeActiveSkinId(saved.id, storage);
  assert.equal(readActiveSkinId(storage), "my-forest");
  deleteCustomSkin(saved.id, storage);
  assert.equal(readCustomSkins(storage).length, 0);
  assert.equal(readActiveSkinId(storage), "lifetrace");
  assert.equal(resolveSkin("missing", storage).id, "lifetrace");
});

test("skin export and import use a versioned document and de-duplicate ids", () => {
  const storage = new MemoryStorage();
  const source = {
    ...structuredClone(BUILTIN_SKINS[2]),
    id: "night-work",
    name: "Night Work",
    builtIn: false,
  };
  saveCustomSkin(source, storage);

  const exported = exportSkinDocument(source);
  const parsed = JSON.parse(exported) as Record<string, unknown>;
  assert.equal(parsed.format, "lifetrace-skin");
  assert.equal(parsed.formatVersion, 1);

  const imported = importSkinDocument(exported, storage);
  assert.equal(imported.name, "Night Work");
  assert.notEqual(imported.id, "night-work");
  assert.equal(imported.builtIn, false);

  assert.throws(
    () => importSkinDocument('{"format":"other","formatVersion":1,"skin":{}}', storage),
    /受支持/,
  );
});

test("skin CSS only emits controlled semantic tokens for both display modes", () => {
  const skin = normalizeSkin({
    ...structuredClone(BUILTIN_SKINS[1]),
    id: "controlled",
    background: {
      type: "gradient",
      from: "#112233",
      to: "#445566",
      angle: 120,
      opacity: 30,
    },
  });
  const css = skinToCss(skin);

  assert.match(css, /:root\[data-theme="light"\]/);
  assert.match(css, /:root\[data-theme="dark"\]/);
  assert.match(css, /--ui-primary:/);
  assert.match(css, /--skin-wallpaper-image:linear-gradient\(120deg/);
  assert.doesNotMatch(css, /display:none/);
});

test("contrast ratio and skin id generation provide editor safety helpers", () => {
  assert.ok(Math.abs(contrastRatio("#000000", "#FFFFFF") - 21) < 0.001);
  assert.equal(createSkinId("My Skin", ["my-skin"]), "my-skin-2");
  assert.equal(createSkinId("Fresh Skin", []), "fresh-skin");
});
