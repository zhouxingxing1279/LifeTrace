import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export const BROWSER_HOST = "0.0.0.0";
export const BROWSER_PORT = 4173;
export const DEFAULT_LIFETRACE_CLOUD_URL = "http://127.0.0.1:8787";

const cloudProxy = {
  "/api": {
    target: DEFAULT_LIFETRACE_CLOUD_URL,
    changeOrigin: true,
  },
  "/health": {
    target: DEFAULT_LIFETRACE_CLOUD_URL,
    changeOrigin: true,
  },
};

export default defineConfig({
  root: "web-client",
  publicDir: "../public",
  plugins: [react()],
  build: {
    outDir: "../dist-browser",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
    reportCompressedSize: true,
  },
  server: {
    host: BROWSER_HOST,
    port: BROWSER_PORT,
    strictPort: true,
    proxy: cloudProxy,
  },
  preview: {
    host: BROWSER_HOST,
    port: BROWSER_PORT,
    strictPort: true,
    proxy: cloudProxy,
  },
});
