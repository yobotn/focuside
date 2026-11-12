import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed port in dev.
export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    sourcemap: process.env.TAURI_ENV_DEBUG === "true",
    minify: process.env.TAURI_ENV_DEBUG !== "true",
    chunkSizeWarningLimit: 4000,
  },
});
