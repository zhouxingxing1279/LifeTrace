import react from "@vitejs/plugin-react";
import path from "node:path";
import { defineConfig } from "vite";

const projectRoot = path.resolve(import.meta.dirname);

export default defineConfig({
  root: path.join(projectRoot, "tauri-ui"),
  publicDir: path.join(projectRoot, "public"),
  plugins: [react()],
  resolve: {
    alias: {
      "@": projectRoot,
    },
    dedupe: ["react", "react-dom"],
  },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    fs: {
      allow: [projectRoot],
    },
  },
  build: {
    outDir: path.join(projectRoot, "dist-tauri"),
    emptyOutDir: true,
    sourcemap: false,
  },
});
