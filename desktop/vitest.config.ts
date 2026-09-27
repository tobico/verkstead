import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The main process's pure parts (ADR-0020), which run under Node exactly as
    // they do under Electron: no window, no jsdom, and nothing here importing
    // `electron` — see `eslint.config.js` for the wall that keeps it that way.
    environment: "node",

    // `tests/` and nowhere else. Left to itself vitest would walk `dist/` as
    // well, and a test compiled into it would then run twice.
    include: ["tests/**/*.test.ts"],
  },
});
