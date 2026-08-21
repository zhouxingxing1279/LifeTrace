import react from "@vitejs/plugin-react";
import path from "node:path";
import { defineConfig } from "vite";

const projectRoot = path.resolve(import.meta.dirname);
const appsRoot = path.resolve(projectRoot, "..");

export default defineConfig({
  root: path.join(projectRoot, "tauri-ui"),
  publicDir: path.join(projectRoot, "public"),
  plugins: [react()],
  css: {
    postcss: path.join(appsRoot, "web", "postcss.config.cjs"),
  },
  resolve: {
    alias: {
      "@": projectRoot,
    },
  },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    fs: {
      allow: [appsRoot],
    },
  },
  build: {
    outDir: path.join(projectRoot, "dist-tauri"),
    emptyOutDir: true,
    sourcemap: false,
  },
});
