import { defineConfig } from "vite";

/// What the built library is called, which the server looks it up by.
const MERMAID = "mermaid.js";

/// Mermaid on its own, built to the one script a share with a Diagram in it
/// carries.
///
/// Its own config rather than a second entry of the share build, because the
/// two want opposite things of the same output directory: the share build folds
/// everything into its document and refuses to leave a file beside it, and this
/// writes exactly that file. So this runs after it, into the same directory,
/// and the server picks up whichever of the two it needs — see
/// `crates/server/src/viewer.rs`.
///
/// One chunk and a plain script, for the reasons the share build has them: there
/// is nothing to fetch a second file with, and a `file://` document is the one
/// place module semantics cost something.
export default defineConfig({
  publicDir: false,

  build: {
    outDir: "dist-share",

    // After the share build, into what it left. Emptying here would take the
    // document with it.
    emptyOutDir: false,

    rollupOptions: {
      input: "src/share/mermaid-library.ts",
      output: {
        inlineDynamicImports: true,
        entryFileNames: MERMAID,
        format: "iife",
      },
    },

    // Nothing may be written beside it either: mermaid carries fonts and icon
    // data, and a share is one file.
    assetsInlineLimit: Number.MAX_SAFE_INTEGER,
    cssCodeSplit: false,
    modulePreload: false,

    // And no complaint about the size of it. Three megabytes in one chunk is
    // what this build is for: the advice the warning gives — split it, fetch
    // the parts — is the one thing a share cannot do.
    chunkSizeWarningLimit: Number.MAX_SAFE_INTEGER,
  },
});
