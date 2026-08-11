import react from "@vitejs/plugin-react";
import path from "node:path";
import { defineConfig } from "vite";

const projectRoot = path.resolve(import.meta.dirname);
const base = process.env.LIFETRACE_UI_BASE || "/";

export default defineConfig({
  root: path.join(projectRoot, "tauri-ui"),
  publicDir: path.join(projectRoot, "public"),
  base,
  plugins: [react()],
  resolve: {
    alias: {
      "@": projectRoot,
    },
  },
  define: {
    "import.meta.env.VITE_UI_PREVIEW": JSON.stringify("1"),
  },
  server: {
    host: "0.0.0.0",
    port: 1420,
    strictPort: true,
  },
  preview: {
    host: "0.0.0.0",
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: path.join(projectRoot, "dist-ui-preview"),
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
  },
});
