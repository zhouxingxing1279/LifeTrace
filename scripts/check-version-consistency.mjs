// Checks that the application version is identical across the three files
// that define it. Exits non-zero on mismatch so CI can fail before releasing.
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function readJson(relativePath) {
  return JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));
}

const packageJson = readJson("package.json");
const tauriConfig = readJson("src-tauri/tauri.conf.json");
const cargoToml = readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;

const versions = {
  "package.json": packageJson.version ?? null,
  "src-tauri/tauri.conf.json": tauriConfig.version ?? null,
  "src-tauri/Cargo.toml": cargoVersion,
};

for (const [file, version] of Object.entries(versions)) {
  console.log(`${file}: ${version ?? "（未找到 version 字段）"}`);
}

const uniqueVersions = new Set(Object.values(versions));
if (uniqueVersions.size !== 1 || uniqueVersions.has(null)) {
  console.error(
    "版本不一致：package.json、src-tauri/tauri.conf.json、src-tauri/Cargo.toml 的 version 必须完全一致。",
  );
  process.exit(1);
}

console.log(`版本一致：v${versions["package.json"]}`);
