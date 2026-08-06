import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  return {
    root: "web-client",
    publicDir: "../public",
    plugins: [react()],
    build: {
      outDir: "../dist-web",
      emptyOutDir: true,
      target: "es2022",
      sourcemap: true,
      reportCompressedSize: true,
    },
    server: {
      host: "0.0.0.0",
      port: 4173,
      proxy: {
        "/api": {
          target: env.LIFETRACE_CLOUD_URL || "http://127.0.0.1:8080",
          changeOrigin: true,
        },
      },
    },
  };
});
