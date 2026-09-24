//! A stand-in for Monaco, shared by the tests that open a file in Code.
//!
//! The suite does not mount the real editor, and this is why: twenty-three
//! megabytes of it into a jsdom with no layout, no canvas and no fonts would be
//! a heap the run has tripped over before, and what it would buy is assertions
//! about Microsoft's code. What is worth asking on this side is what the *pane*
//! does around an editor — which file it opened, at which path, with which
//! theme, read-only or not, and where the typing went — and all of that is
//! askable of this.
//!
//! So what stands where the real editor's own view would be is a `textarea`:
//! somewhere for a test to read a value off and fire an input at. It is not
//! what Monaco draws and is not meant to be — and it is not where the text is
//! either, any more than it is in a real editor. The text is in the model above
//! the pane, and the box is a view of it: typing into the box writes the model,
//! and a model written from anywhere else shows in the box.
//!
//! Installed by a test file with
//!
//! ```ts
//! vi.mock("../src/workbench/editing", () => import("./editing"));
//! ```
//!
//! which puts this module in place of the seam `Code.tsx` and `Editor.tsx`
//! import — so the `import("./monaco")` behind it is never made, and the real
//! package is never in the room. Everything below is then readable from the
//! test file by importing this module the ordinary way: it is the same instance.

/// How the pane opened one, as much of it as a test asks about.
export type Opening = {
  ariaLabel?: string;
  theme?: string;
  readOnly?: boolean;
  automaticLayout?: boolean;
  model?: Model;
} & Drawing;

/// The three settings the pane's own menu carries, as Monaco takes them —
/// which is what a test about them reads back.
///
/// Apart from the rest of an opening because they are also what `updateOptions`
/// is handed: they are the pane's rather than a tab's, and a change to one is
/// told to every editor already open.
export type Drawing = {
  wordWrap?: string;
  fontSize?: number;
  minimap?: { enabled?: boolean };
};

/// A buffer, which is the path it was made at and the text that is in it.
///
/// The path is the whole of how a language is chosen — see `Editor.tsx` — so it
/// is what a test about colouring asks after.
///
/// Made above the pane rather than by an editor — see `keeping.ts` — and this
/// stands in for the real one closely enough to say so: the text really is
/// here, an editor drawn over it is a view of it, and every change to it is
/// told to whoever asked to hear.
export type Model = {
  path: string;
  /// The text as it stands, which is what an editor over it is showing.
  readonly text: string;
  /// Whether it has been disposed. Monaco registers a buffer against the file's
  /// own address and refuses a second one there, so a file whose last view
  /// closed without disposing its own would be one that could never be
  /// reopened.
  disposed: boolean;
  getValue(): string;
  setValue(text: string): void;
  onDidChangeContent(said: () => void): { dispose(): void };
  dispose(): void;
};

/// One editor the pane has opened.
export type Editor = {
  /// The buffer under it, and so the file it is showing.
  model: Model;
  /// What it was opened with.
  opening: Opening;
  /// And how it is drawn *now*: what it was opened with, and every change the
  /// pane has told it about since. The three settings on the pane's menu are
  /// the pane's rather than a tab's, so what a test about one asks is what the
  /// editor is set to at the moment rather than what it was made with.
  drawn: Drawing;
  /// Where the text really is, for a test to read and type into.
  typing: HTMLTextAreaElement;
  /// And whether the tab it was in has gone.
  disposed: boolean;
};

/// Every editor opened since the last [`reset`], oldest first.
export const opened: Editor[] = [];

/// How many times the pane has asked for the editor, which is the one thing
/// that says whether the chunk was fetched — and when.
export const loading = { times: 0 };

/// The theme every editor is in, which Monaco holds for the package rather than
/// for an editor. What `Editor.tsx` last set, or nothing where it has set none.
export const themed = { last: undefined as string | undefined };

/// Whether the fetch fails, which is the one thing that can go wrong opening a
/// file that is nothing to do with the file: the chunk comes over the same
/// network the rest of the workbench is on.
export const refusing = { chunk: false };

/// Forget all of it, between tests.
export function reset(): void {
  opened.length = 0;
  loading.times = 0;
  themed.last = undefined;
  refusing.chunk = false;
}

/// The one the pane has just opened, which is the last one made and not gone.
export function theEditor(): Editor {
  const made = opened.filter((one) => !one.disposed).at(-1);

  if (!made) {
    throw new Error("the pane should have opened an editor");
  }

  return made;
}

/// And the package, as the three calls the pane makes of it.
const monaco = {
  editor: {
    createModel(text: string, _language: string | undefined, at: Uri): Model {
      let held = text;
      const watching = new Set<() => void>();

      const model: Model = {
        path: at.path,
        get text() {
          return held;
        },
        disposed: false,
        getValue: () => held,
        setValue: (next: string) => {
          held = next;

          for (const said of [...watching]) {
            said();
          }
        },
        onDidChangeContent: (said: () => void) => {
          watching.add(said);

          return { dispose: () => watching.delete(said) };
        },
        dispose: () => {
          model.disposed = true;
          watching.clear();
        },
      };

      return model;
    },

    create(at: HTMLElement, opening: Opening): Standalone {
      const model =
        opening.model ?? monaco.editor.createModel("", undefined, { path: "" });

      const typing = at.ownerDocument.createElement("textarea");

      typing.value = model.getValue();
      typing.readOnly = opening.readOnly === true;

      if (opening.ariaLabel !== undefined) {
        typing.setAttribute("aria-label", opening.ariaLabel);
      }

      // Typing into the box is typing into the buffer, which is what it is in
      // the real editor: an editor is a view *of* a model, and everything that
      // follows the text — the dot, Ctrl+S, the second view of the same file —
      // follows it from there.
      typing.addEventListener("input", () => {
        if (model.getValue() !== typing.value) {
          model.setValue(typing.value);
        }
      });

      // And a buffer written from outside any editor — Reload — shows here, the
      // way it shows in an editor Monaco is drawing.
      const watching = model.onDidChangeContent(() => {
        if (typing.value !== model.getValue()) {
          typing.value = model.getValue();
        }
      });

      at.append(typing);

      const made: Editor = {
        model,
        opening,
        // Whatever it was opened with, to be moved by every change since — the
        // way Monaco merges what `updateOptions` is handed into what it already
        // had.
        drawn: {
          wordWrap: opening.wordWrap,
          fontSize: opening.fontSize,
          minimap: opening.minimap,
        },
        typing,
        disposed: false,
      };

      opened.push(made);

      return {
        updateOptions: (options: { readOnly?: boolean } & Drawing) => {
          if (options.readOnly !== undefined) {
            typing.readOnly = options.readOnly;
          }

          for (const [named, value] of Object.entries(options)) {
            if (named !== "readOnly" && value !== undefined) {
              Object.assign(made.drawn, { [named]: value });
            }
          }
        },
        dispose: () => {
          made.disposed = true;
          watching.dispose();
          typing.remove();
        },
      };
    },

    setTheme(name: string): void {
      themed.last = name;
    },
  },

  Uri: {
    file: (path: string): Uri => ({ path }),
  },
};

/// What a path becomes on its way to a model. Monaco's own is a URL; what
/// matters here is that the path arrives.
type Uri = { path: string };

/// And one editor, as much of it as the pane calls — which is two things, an
/// editor here being a view over a buffer rather than somewhere text is kept.
type Standalone = {
  updateOptions(options: { readOnly?: boolean } & Drawing): void;
  dispose(): void;
};

/// The fetch, stood in for: nothing is fetched, and the count is what says the
/// pane asked.
export function load(): Promise<typeof monaco> {
  loading.times += 1;

  return refusing.chunk
    ? Promise.reject(new Error("the editor never arrived"))
    : Promise.resolve(monaco);
}
