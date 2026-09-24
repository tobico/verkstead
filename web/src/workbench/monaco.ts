//! Monaco itself: the chunk Code fetches the first time it opens, and the
//! workers beside it.
//!
//! **Whole** (ADR 0019, *Monaco, whole*): the package's own root entry, which
//! is the one import that registers every built-in language and the four
//! language services — TypeScript and JavaScript, JSON, CSS, HTML — rather than
//! a list of the languages somebody guessed would be opened. A pane that
//! coloured whatever it had been told about in advance would be a worse pane in
//! exactly the moment it was wanted.
//!
//! **And nothing else in the viewer imports it**, which is the whole of why
//! this file exists rather than the editor importing the package where it is
//! used. Everything here is static, so this module *is* the chunk: it is
//! reached by the `import()` in [`./editing`] and by nothing else, and a bundle
//! that never opens Code never fetches a byte of it. What the server serves it
//! under is the hashed `/assets/` path it keeps for a year — see `HASHED` in
//! `crates/server/src/viewer.rs` — so the fetch happens once per release.
//!
//! **The workers are per-file entries**, which is what the package ships and
//! what Vite's `?worker` is for: one bundle each, emitted beside this chunk
//! under the same hashed path, and constructed on demand. There is no bundled
//! loader to point at a directory, and nothing here names a URL — the
//! constructor Vite hands back is the whole of the address.
//!
//! The TypeScript one carries the TypeScript compiler and is the largest file
//! of the lot by some way. It stays: ADR 0019 named it as what a diet would take
//! first and decided against taking it, colouring being most of the pane's value
//! and almost none of its weight.

import * as monaco from "monaco-editor";

import editorWorker from "monaco-editor/editor/editor.worker?worker";
import cssWorker from "monaco-editor/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/language/html/html.worker?worker";
import jsonWorker from "monaco-editor/language/json/json.worker?worker";
import tsWorker from "monaco-editor/language/typescript/ts.worker?worker";

import type { Monaco } from "./editing";

/// Which worker answers for which language, by the label the editor asks with.
///
/// The four services the package bundles, under every language each of them
/// serves — the labels are the language ids, so a `.scss` file asks for `scss`
/// and it is the CSS service that knows what to say about it. Anything not here
/// is a language with colouring and no service, and gets the plain editor
/// worker, which is what does the work every language needs: the diffs, the
/// links, and the word-based completions that stand in where there is no
/// service to ask.
const SERVICES: Record<string, new () => Worker> = {
  css: cssWorker,
  scss: cssWorker,
  less: cssWorker,
  html: htmlWorker,
  handlebars: htmlWorker,
  razor: htmlWorker,
  json: jsonWorker,
  typescript: tsWorker,
  javascript: tsWorker,
};

/// How the editor is told to make one. Read when a worker is wanted rather than
/// at any point before it, so setting it here — as this module is evaluated,
/// which is before anything has been drawn with it — is in time for every
/// editor this chunk will ever open.
globalThis.MonacoEnvironment = {
  getWorker: (_id: string, label: string) =>
    new (SERVICES[label] ?? editorWorker)(),
};

/// The package, as the little of it [`./editing`] describes.
///
/// Named as that narrower thing here rather than cast at the far end, so that
/// the one place the two meet is the one place the compiler checks them against
/// each other.
const whole: Monaco = monaco;

export default whole;
