import path from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export const BROWSER_HOST = "0.0.0.0";
export const BROWSER_PORT = 4173;
export const DEFAULT_LIFETRACE_CLOUD_URL = "http://127.0.0.1:8787";

const projectRoot = path.resolve(import.meta.dirname);
const sharedWebRoot = path.resolve(projectRoot, "../web");
const cloudProxy = {
  "/api": { target: DEFAULT_LIFETRACE_CLOUD_URL, changeOrigin: true },
  "/health": { target: DEFAULT_LIFETRACE_CLOUD_URL, changeOrigin: true }
};

export default defineConfig({
  root: sharedWebRoot,
  plugins: [react()],
  resolve: { dedupe: ["react", "react-dom"] },
  build: {
    outDir: path.join(projectRoot, "dist-browser"),
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
    reportCompressedSize: true
  },
  server: { host: BROWSER_HOST, port: BROWSER_PORT, strictPort: true, proxy: cloudProxy },
  preview: { host: BROWSER_HOST, port: BROWSER_PORT, strictPort: true, proxy: cloudProxy }
});
