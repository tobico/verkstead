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
//! So the box the text is really in here is a `textarea`. It is not what Monaco
//! draws and is not meant to be: it is somewhere for a test to read a value off
//! and fire an input at, standing where the real editor's own view would be.
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
};

/// A buffer, which is the path it was made at and the text that was in it.
///
/// The path is the whole of how a language is chosen — see `Editor.tsx` — so it
/// is what a test about colouring asks after.
export type Model = {
  path: string;
  text: string;
  /// Whether the tab it was made for has gone. Monaco registers a buffer
  /// against the file's own address and refuses a second one there, so a tab
  /// that did not dispose its own would be a file that could never be reopened.
  disposed: boolean;
  dispose(): void;
};

/// One editor the pane has opened.
export type Editor = {
  /// The buffer under it, and so the file it is showing.
  model: Model;
  /// What it was opened with.
  opening: Opening;
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
      const model: Model = {
        path: at.path,
        text,
        disposed: false,
        dispose: () => {
          model.disposed = true;
        },
      };

      return model;
    },

    create(at: HTMLElement, opening: Opening): Standalone {
      const model =
        opening.model ?? monaco.editor.createModel("", undefined, { path: "" });

      const typing = at.ownerDocument.createElement("textarea");

      typing.value = model.text;
      typing.readOnly = opening.readOnly === true;

      if (opening.ariaLabel !== undefined) {
        typing.setAttribute("aria-label", opening.ariaLabel);
      }

      at.append(typing);

      const made: Editor = { model, opening, typing, disposed: false };

      opened.push(made);

      return {
        getValue: () => typing.value,
        setValue: (text: string) => {
          typing.value = text;
        },
        onDidChangeModelContent: (said: () => void) => {
          typing.addEventListener("input", said);
        },
        updateOptions: (options: { readOnly?: boolean }) => {
          if (options.readOnly !== undefined) {
            typing.readOnly = options.readOnly;
          }
        },
        dispose: () => {
          made.disposed = true;
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

/// And one editor, as much of it as the pane calls.
type Standalone = {
  getValue(): string;
  setValue(text: string): void;
  onDidChangeModelContent(said: () => void): void;
  updateOptions(options: { readOnly?: boolean }): void;
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
