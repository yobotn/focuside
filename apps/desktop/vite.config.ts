import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

// PDF.js data files, served at /pdfjs/<dir>/ so no PDF ever needs the network: font data for
// the standard fonts, CMaps for CJK text, ICC profiles, and the image decoders (JPEG 2000, JBIG2) as
// WebAssembly with JavaScript fallbacks.
const pdfjsRoot = fileURLToPath(new URL("node_modules/pdfjs-dist", import.meta.url));
const pdfjsDirs = ["cmaps", "standard_fonts", "wasm", "iccs"];
// The license files ship next to the data. quickjs runs PDF scripts, which FocusIDE does not.
const pdfjsSkip = /quickjs/i;

function pdfjsData(): Plugin {
  return {
    name: "pdfjs-data",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const m = /^\/pdfjs\/([a-z_]+)\/([\w.-]+)$/.exec(req.url?.split("?")[0] ?? "");
        const file = m && pdfjsDirs.includes(m[1]) && !pdfjsSkip.test(m[2]) ? join(pdfjsRoot, m[1], m[2]) : null;
        if (!file || !existsSync(file)) return next();
        res.setHeader("Content-Type", file.endsWith(".js") ? "text/javascript" : "application/octet-stream");
        res.end(readFileSync(file));
      });
    },
    generateBundle() {
      for (const dir of pdfjsDirs) {
        for (const name of readdirSync(join(pdfjsRoot, dir))) {
          if (pdfjsSkip.test(name)) continue;
          this.emitFile({ type: "asset", fileName: `pdfjs/${dir}/${name}`, source: readFileSync(join(pdfjsRoot, dir, name)) });
        }
      }
    },
  };
}

// Tauri expects a fixed port in dev.
export default defineConfig({
  plugins: [svelte(), tailwindcss(), pdfjsData()],
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
