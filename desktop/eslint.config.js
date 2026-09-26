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
///
/// `main.ts` is the app read off the process: `app`, the dialogs, and the one
/// read of the environment everything below resolves out of. `window.ts` holds a
/// `BrowserWindow`, which is a thing rather than a value and cannot be handed in
/// — what it *decides* is elsewhere, in modules vitest runs. `tray.ts` holds a
/// `Tray`, which is the same case: the object *is* the icon on somebody's panel,
/// and what the menu on it says and means is `chosen.ts`, which vitest runs.
/// `menu.ts` is the handing of the application menu to Electron, and that call
/// is the whole of it: what the menu *holds* is `roles.ts`, which vitest runs.
/// `preload.mts` is
/// the one file here that is not the main process at all: it runs inside the
/// window and `contextBridge` is how it puts the bridge on it, which is a call
/// nothing else can make. What the bridge *is* is `bridge.ts` and what a set
/// *means* is `settings.ts`, and vitest runs both.
const EDGE = [
  "src/main.ts",
  "src/menu.ts",
  "src/preload.mts",
  "src/tray.ts",
  "src/window.ts",
];

export default [
  {
    // `.mts` as well as `.ts`: the preload is one, an ESM preload having to be
    // an `.mjs` once it is compiled, and the wall is no wall with a way round
    // it by extension.
    files: ["src/**/*.ts", "src/**/*.mts", "tests/**/*.ts"],

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
