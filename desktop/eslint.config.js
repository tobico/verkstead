//! The lint wall around Electron itself.
//!
//! One rule, and it exists for one reason: the whole of this stage's proof is
//! vitest over the main process's pure parts (ADR-0020), and a module that
//! imports `electron` cannot be run by vitest at all — the module resolves to a
//! binary's own bindings, so a test that reached one would fail about Electron
//! rather than about the code. So `electron` is the entry file's and nobody
//! else's: everything the app does is a function of values that can be handed
//! in, and `src/main.ts` is the one place those values are read off the running
//! application.
//!
//! The list below is a list rather than a directory so that adding to it is a
//! deliberate act. A later stage with a window to open and a bridge to expose
//! will have more than one file at the edge, and the file that goes on it says
//! why it is there.
//!
//! Nothing else is linted here. `tsc --noEmit` is the other static check and
//! the one that reads types; this is deliberately a wall and not a style
//! regime — the same split the viewer's `eslint.config.js` makes.

import parser from "@typescript-eslint/parser";

/// The files allowed to reach the running application. Everything else is a
/// function of what they hand it.
const EDGE = ["src/main.ts"];

export default [
  {
    files: ["src/**/*.ts", "tests/**/*.ts"],

    // `tsc` is what typechecks; this parser is here to read TypeScript into an
    // AST the core rule can see the imports in, and no more than that.
    languageOptions: { parser },

    rules: {
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "electron",
              message:
                "Electron is the entry file's — take what this needs as an " +
                "argument, so that vitest can run it. See the wall at the " +
                "top of eslint.config.js.",
            },
          ],
        },
      ],
    },
  },
  {
    files: EDGE,
    rules: { "no-restricted-imports": "off" },
  },
];
