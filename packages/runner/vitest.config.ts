import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
  },
  resolve: {
    alias: {
      "rscpp-wasm": new URL("../../crates/wasm/pkg/rscpp_wasm.js", import.meta.url)
        .pathname,
    },
  },
});
