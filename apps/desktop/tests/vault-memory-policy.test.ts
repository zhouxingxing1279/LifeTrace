import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync(new URL("../src/components/LocalVaultModule.tsx", import.meta.url), "utf8");

test("private album bounds the number of mounted thumbnails", () => {
  assert.match(source, /const VAULT_PAGE_SIZE = 48;/);
  assert.match(source, /pageAssets\.map\(/);
  assert.doesNotMatch(source, /nextAssets\.filter\(asset=>asset\.hasThumbnail\)/);
});

test("private album throttles thumbnail IPC instead of loading the whole album at once", () => {
  assert.match(source, /const THUMBNAIL_BATCH_SIZE = 4;/);
  assert.match(source, /candidates\.slice\(start,start\+THUMBNAIL_BATCH_SIZE\)/);
  assert.match(source, /revokeTrackedUrl/);
});

test("large original files are rejected before Base64 inline preview", () => {
  assert.match(source, /const MAX_INLINE_PREVIEW_BYTES = 32 \* 1024 \* 1024;/);
  assert.match(source, /if\(asset\.size>MAX_INLINE_PREVIEW_BYTES\)/);
  assert.match(source, /避免 WebView2 内存不足/);
});
