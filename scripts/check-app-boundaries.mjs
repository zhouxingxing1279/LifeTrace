import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const sourceExtensions = new Set([".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".json"]);
const ignoredDirectories = new Set(["node_modules", "dist", "dist-tauri", "dist-browser", "target", ".git"]);

const desktopToWebAllowlist = new Set([]);
const webToDesktopAllowlist = new Set([]);

const desktopToWebPatterns = [
  /(?:\.\.\/)+web\//,
  /apps\/web\//,
  /path\.join\([^\n]*["']web["']/,
  /path\.resolve\([^\n]*["']\.\.\/\.\.\/web["']/,
];
const webToDesktopPatterns = [
  /(?:\.\.\/)+desktop\//,
  /apps\/desktop\//,
  /path\.join\([^\n]*["']desktop["']/,
  /path\.resolve\([^\n]*["']\.\.\/\.\.\/desktop["']/,
];

async function collectFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (ignoredDirectories.has(entry.name)) continue;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await collectFiles(absolute));
    else if (entry.isFile() && sourceExtensions.has(path.extname(entry.name))) files.push(absolute);
  }
  return files;
}

function relative(absolute) {
  return path.relative(root, absolute).split(path.sep).join("/");
}

async function findViolations(directory, patterns, allowlist, direction) {
  const files = await collectFiles(path.join(root, directory));
  const violations = [];
  const staleAllowlist = new Set(allowlist);

  for (const absolute of files) {
    const file = relative(absolute);
    const content = await readFile(absolute, "utf8");
    if (!patterns.some((pattern) => pattern.test(content))) continue;

    if (allowlist.has(file)) {
      staleAllowlist.delete(file);
      continue;
    }
    violations.push(`${direction}: ${file}`);
  }

  return { violations, staleAllowlist: [...staleAllowlist] };
}

const desktop = await findViolations(
  "apps/desktop",
  desktopToWebPatterns,
  desktopToWebAllowlist,
  "desktop -> web",
);
const web = await findViolations(
  "apps/web",
  webToDesktopPatterns,
  webToDesktopAllowlist,
  "web -> desktop",
);

const violations = [...desktop.violations, ...web.violations];
if (violations.length > 0) {
  console.error("Cross-app dependency boundary violated:\n" + violations.map((item) => `  - ${item}`).join("\n"));
  console.error("Move shared code into an explicit shared package/contract instead of importing another app internals.");
  process.exit(1);
}

const stale = [
  ...desktop.staleAllowlist.map((file) => `desktop -> web allowlist is stale: ${file}`),
  ...web.staleAllowlist.map((file) => `web -> desktop allowlist is stale: ${file}`),
];
if (stale.length > 0) {
  console.error("Cross-app allowlist contains entries that no longer need an exception:\n" + stale.map((item) => `  - ${item}`).join("\n"));
  console.error("Remove stale exceptions in the same change that removes the dependency.");
  process.exit(1);
}

console.log("Cross-app boundary check passed. Desktop/Web application internals are fully isolated.");
