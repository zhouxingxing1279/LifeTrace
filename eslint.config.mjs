import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    ".vinext/**",
    ".wrangler/**",
    ".venv-*/**",
    "backups/**",
    "dist/**",
    "out/**",
    "build/**",
    "desktop/**/*.cjs",
    "恒序个人管理平台整合Demo/**",
    "next-env.d.ts",
  ]),
]);

export default eslintConfig;
