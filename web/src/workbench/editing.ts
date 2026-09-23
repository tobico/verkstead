//! What Code asks of Monaco, and how it is fetched.
//!
//! A module of types and one call, holding no part of the editor itself: it is
//! statically imported by the pane and by [`./Editor`], and everything it says
//! about Monaco is a *type*, which the bundler erases. So what lands in the
//! workbench's own chunk is the `import()` at the foot of this file and nothing
//! more, and the editor is [`./monaco`] on the far side of it (ADR 0019,
//! *Monaco, whole*).
//!
//! **The shape is narrowed rather than invented**: each of the types below is
//! the package's own, cut down to what this pane touches. Narrowed, because a
//! list of five methods is what a reader needs to know the pane does with an
//! editor and a stub in the suite needs to stand in for — a test that had to
//! mount the real thing to ask which file the pane opened would be mounting
//! twenty-three megabytes of editor in a jsdom with no layout, no canvas and no
//! fonts. The package's own, because a shape written out by hand is a shape
//! that goes quietly wrong at the next release: what is here is checked against
//! Monaco every time `tsc` runs, at the one line in [`./monaco`] where the two
//! meet.

import type * as Package from "monaco-editor";

/// Monaco's own handle on a path.
///
/// Made from a path and handed straight back: nothing on this side ever reads
/// one.
export type Uri = Package.Uri;

/// A buffer the editor is drawn onto — what Monaco calls a model.
///
/// The language a file is coloured in is the model's, and the model's is its
/// path's: a model made at `Uri.file("…/main.rs")` is a Rust one because the
/// package registered `.rs` when it registered the language, and the TypeScript
/// service reads the same path as the name of the file it is compiling. Which
/// is why there is no table of extensions anywhere here — the package has one,
/// and it is the same one VS Code has.
export type Model = Package.editor.ITextModel;

/// One editor, as much of it as this pane uses.
export type Standalone = Pick<
  Package.editor.IStandaloneCodeEditor,
  | "getValue"
  | "setValue"
  | "onDidChangeModelContent"
  | "updateOptions"
  | "dispose"
>;

/// What an editor is opened with — the buffer, what it is read aloud as, the
/// theme, whether it takes typing, and whether it follows the element it is in.
///
/// VS Code's defaults but for those: word wrap, the font size and the minimap
/// are stage 03 of the roadmap, and are its defaults until then.
export type Opening = Package.editor.IStandaloneEditorConstructionOptions;

/// And the package, as the three calls this pane makes of it.
export type Monaco = {
  editor: Pick<typeof Package.editor, "create" | "createModel" | "setTheme">;
  Uri: Pick<typeof Package.Uri, "file">;
};

/// The fetch, once.
///
/// Held rather than made again: the chunk is the browser's after the first
/// import, but a promise kept here is also what makes a second editor opening
/// while the first is still in flight wait on the one fetch rather than start
/// another.
let fetching: Promise<Monaco> | undefined;

/// Fetch Monaco, and answer with it.
///
/// Called when Code opens rather than when a file is pressed — the pane warms
/// it on mount, so the editor is usually there before anybody has picked a file
/// to put in it — and called again by every editor that opens, which is what
/// makes the warming an optimisation rather than a rule anything depends on.
///
/// A fetch that failed is not held: the chunk comes from the same server the
/// rest of the workbench is talking to, and a network that was briefly down is
/// no reason for this pane to refuse to open an editor for the rest of the
/// session.
export function load(): Promise<Monaco> {
  fetching ??= import("./monaco")
    .then((module) => module.default)
    .catch((error: unknown) => {
      fetching = undefined;
      throw error instanceof Error ? error : new Error(String(error));
    });

  return fetching;
}
