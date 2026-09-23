//! The editor a text file opens in: Monaco, filling its tab.
//!
//! One of these per open file, made when the tab is drawn and disposed when it
//! goes. What it is handed is the text and what it hands back is the typing:
//! the buffer lives above the pane in `Code.tsx`, because a tab turned away
//! from and back to has to find its text where it was, and a save is a
//! comparison between that buffer and what the disk said.
//!
//! **Nothing here saves.** Ctrl+S is the pane's, on the document, and what it
//! writes is the buffer above rather than anything read back out of this — so
//! an editor knows only what it was given and what has been typed into it.
//!
//! **The language is the path's**, which is the whole of how a file is
//! coloured: the buffer is made at `Uri.file(<the path>)`, and the package
//! registered every extension when it registered every language. So a `.rs`
//! file is Rust because Monaco says it is, and the TypeScript service reads the
//! same path as the name of the file it is compiling — which is what makes a
//! completion in a `.ts` file a completion about that file rather than about an
//! anonymous buffer. Nothing here has a table of extensions, and nothing here
//! should ever grow one (ADR 0019, *Monaco, whole*).
//!
//! **It follows the workbench's light and dark**, which is the one thing about
//! it this stage sets: `vs` and `vs-dark`, VS Code's own two, picked by the
//! scheme the browser is in and followed live — the workbench has no theme
//! switch, so `prefers-color-scheme` is both the setting and the only warning
//! of a change, exactly as it is for the diagrams in `set/diagrams.ts`. Word
//! wrap, the font size and the minimap are stage 03's, on the pane's own menu;
//! until then they are VS Code's defaults like everything else.
//!
//! **The editor is fetched rather than bundled in** — see [`./editing`] and
//! [`./monaco`]. The pane warms that fetch when it opens, so by the time a file
//! is pressed the chunk is usually already there; this waits on it either way,
//! and says so where it never arrives.
//!
//! **And it is made once, for the tab it is in.** A tab turned away from is
//! unmounted with its editor, which costs the cursor and the undo stack and
//! costs the text nothing — the buffer above outlives both. Holding the buffers
//! themselves above the pane is stage 02's, where a file may be open in two
//! groups at once and one buffer under two editors is the thing that has to be
//! true.

import {
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import { ErrorLine } from "../notices";
import { load, type Model, type Monaco, type Standalone } from "./editing";
import styles from "./Editor.module.css";

/// VS Code's own two themes, by the names Monaco ships them under.
export const LIGHT = "vs";
export const DARK = "vs-dark";

/// How the page decides which of them it is in. The workbench has no switch and
/// keeps no preference, so this is the question and the only notice of the
/// answer changing.
const SCHEME = "(prefers-color-scheme: dark)";

/// What the tab says where the editor never arrived.
///
/// The one thing that can go wrong here that is not the file's: the chunk is
/// fetched from the same server the rest of the workbench is talking to, so a
/// tab that simply stayed blank would be the pane saying nothing at all about a
/// network that had gone.
export const NO_EDITOR =
  "The editor could not be loaded. Check the connection and open the file again.";

export function Editor(props: {
  /// The file, by the path it was read at — which is what it is coloured by.
  path: string;
  /// And what it is called, which is what the editor is read aloud as: a path
  /// in a checkout is a sentence, and the name is what the tab above says too.
  name: string;
  /// The text as it stands: the buffer, which starts as what the disk said.
  text: string;
  /// Whether the root it is in takes writes. A file in a read-only companion
  /// opens read-only and takes no typing — the root's own flag rather than the
  /// file's mode.
  writable: boolean;
  /// And what typing into it does, which is to put the text back in the buffer
  /// above.
  typed: (text: string) => void;
}): JSX.Element {
  /// Where the editor is drawn. Its own element rather than this component's
  /// root, because Monaco fills whatever it is given and the line below has to
  /// stand somewhere when there is no editor to fill it.
  let host: HTMLDivElement | undefined;

  /// The package, the editor made out of it, and the buffer under that — all
  /// three once the chunk has landed, and none of them before.
  let bundle: Monaco | undefined;
  let editing: Standalone | undefined;
  let buffer: Model | undefined;

  /// Whether the chunk failed to arrive, which is the one thing drawn in place
  /// of an editor.
  const [lost, setLost] = createSignal(false);

  /// Which of the two themes the browser is in, answered live: a
  /// `MediaQueryList` is asked rather than read, and its change is the only
  /// warning there is.
  const scheme = window.matchMedia(SCHEME);
  const [dark, setDark] = createSignal(scheme.matches);
  const follow = (event: MediaQueryListEvent) => setDark(event.matches);

  scheme.addEventListener("change", follow);
  onCleanup(() => scheme.removeEventListener("change", follow));

  onMount(() => {
    /// Set once this tab has gone, so a chunk that lands after it has nothing
    /// left to open an editor on.
    let gone = false;

    onCleanup(() => {
      gone = true;
      editing?.dispose();
      // The buffer is Monaco's rather than this element's — it is registered
      // against the file's own address, and a second one at that address would
      // be refused — so it goes when the tab does, which is what makes opening
      // the file again a fresh editor over a fresh reading.
      buffer?.dispose();
      bundle = undefined;
      editing = undefined;
      buffer = undefined;
    });

    void load()
      .then((monaco) => {
        if (gone || !host) {
          return;
        }

        bundle = monaco;

        // The language is left to Monaco, which reads it off the path — see the
        // note at the top.
        buffer = monaco.editor.createModel(
          props.text,
          undefined,
          monaco.Uri.file(props.path),
        );

        editing = monaco.editor.create(host, {
          model: buffer,
          ariaLabel: props.name,
          theme: dark() ? DARK : LIGHT,
          readOnly: !props.writable,
          automaticLayout: true,
        });

        // Every keystroke back into the buffer above, which is where the dot on
        // the tab and the save under Ctrl+S both read it from.
        editing.onDidChangeModelContent(() => {
          if (editing) {
            props.typed(editing.getValue());
          }
        });
      })
      .catch(() => setLost(true));
  });

  // The theme, followed for as long as the tab is open. Monaco's themes are the
  // package's rather than an editor's, so this sets it for every editor there
  // is — which is every editor this pane has open, and all of them are in the
  // one browser being asked the one question.
  createEffect(() => {
    const theme = dark() ? DARK : LIGHT;

    bundle?.editor.setTheme(theme);
  });

  // And the read-only flag, which the root a tab was opened from settles and
  // nothing changes under it. Said as an effect anyway, so that the one place
  // it is read is the one place it is written.
  createEffect(() => {
    editing?.updateOptions({ readOnly: !props.writable });
  });

  // The buffer above, put back into the editor where the two have come apart.
  //
  // Only where they have: writing what the editor already holds back into it
  // would cost the cursor and the undo stack on every keystroke, the typing
  // above being what feeds this. What moves them apart is the buffer being set
  // from somewhere other than this editor, which is **Reload** on the bar a
  // refused stale save puts up: the disk's text, into an editor still holding
  // the human's.
  createEffect(() => {
    const text = props.text;

    if (editing && editing.getValue() !== text) {
      editing.setValue(text);
    }
  });

  return (
    <div class={styles.editor}>
      <div class={styles.host} ref={host} />
      <Show when={lost()}>
        <ErrorLine>{NO_EDITOR}</ErrorLine>
      </Show>
    </div>
  );
}
