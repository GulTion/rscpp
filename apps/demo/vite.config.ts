import { defineConfig } from "vite";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

export default defineConfig({
  server: { port: 5173 },
  resolve: {
    alias: {
      "rscpp-wasm": path.join(root, "crates/wasm/pkg/rscpp_wasm.js"),
      "@rscpp/timeline": path.join(root, "packages/timeline/src/index.ts"),
      "@rscpp/runner": path.join(root, "packages/runner/src/index.ts"),
    },
  },
  assetsInclude: ["**/*.wasm"],
});
