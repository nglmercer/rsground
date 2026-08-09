import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import tsconfigPaths from "vite-tsconfig-paths";
import devtools from "solid-devtools/vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [
    wasm(),
    devtools({ autoname: true }),
    solidPlugin(),
    tsconfigPaths(),
  ],
  optimizeDeps: {
    include: ["@codemirror/state", "@codemirror/view"],
    // wasm-bindgen's bundler output initializes through top-level await.
    // Prebundling this linked package makes Vite treat it like the web target
    // and leaves the generated wrapper's internal wasm handle unset.
    exclude: ["frontend-wasm"],
  },
  server: {
    port: 3000,
  },
  build: {
    target: "esnext",
    // @codemirror/lsp-client is intentionally loaded as a separate lazy
    // chunk; its minified size is just above Vite's default threshold.
    chunkSizeWarningLimit: 650,
  },
  css: {
    preprocessorOptions: {
      sass: {
        api: "modern",
        silenceDeprecations: [
          "mixed-decls",
        ],
      },
    },
  },
});
