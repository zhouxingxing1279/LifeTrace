import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(scriptDir, "../../web");
const routerPackage = path.join(webRoot, "node_modules", "react-router-dom", "package.json");

if (!existsSync(routerPackage)) {
  const npm = process.platform === "win32" ? "npm.cmd" : "npm";
  execFileSync(npm, ["install", "--prefix", webRoot, "--no-audit", "--no-fund"], {
    stdio: "inherit",
  });
}
