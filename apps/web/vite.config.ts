import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    port: 4173,
    proxy: {
      "/api": { target: process.env.LIFETRACE_CLOUD_URL ?? "http://127.0.0.1:8787", changeOrigin: true },
      "/health": { target: process.env.LIFETRACE_CLOUD_URL ?? "http://127.0.0.1:8787", changeOrigin: true }
    }
  },
  preview: { host: "127.0.0.1", port: 4183 },
  build: { target: "es2022", sourcemap: false }
});
