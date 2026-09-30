import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  resolve: {
    alias: {
      "@": path.resolve(dirname, "./src"),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Workspace-root Cargo output + Tauri glue must never enter Vite's
      // file watcher: while cargo writes/locks target/**/*.exe, a concurrent
      // chokidar watch handle on the same file crashes Windows dev launches
      // with EBUSY. Frontend HMR scope (src/) is unaffected by these globs.
      ignored: ["**/target/**", "**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    chunkSizeWarningLimit: 1024,
  },
});
