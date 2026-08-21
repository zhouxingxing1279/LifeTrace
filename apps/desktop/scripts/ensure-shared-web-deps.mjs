import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(scriptDir, "../../web");
const routerPackage = path.join(webRoot, "node_modules", "react-router-dom", "package.json");

if (!existsSync(routerPackage)) {
  const npmCli = process.env.npm_execpath;
  if (!npmCli) {
    throw new Error("npm_execpath is unavailable; run this bootstrap through an npm lifecycle script.");
  }

  execFileSync(
    process.execPath,
    [npmCli, "install", "--prefix", webRoot, "--no-audit", "--no-fund"],
    { stdio: "inherit" },
  );
}
