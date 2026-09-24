//! The editor a text file opens in: Monaco, filling its tab.
//!
//! One of these per view of a file — a view rather than a file, because the
//! same file open in two groups is two of these over one buffer. What it is
//! handed is that buffer and nothing else about the text: the model is Monaco's
//! own and is held above the pane, in `keeping.ts`, because a model is
//! registered at the file's own address and a second one there is refused.
//!
//! So this file has no text in it at all. Typing goes into the model, which is
//! where the dot on the tab and the save under Ctrl+S both read it from, and
//! **Reload** writes the model and this shows it — the way an editor shows
//! whatever its buffer says, which is what an editor is.
//!
//! **Nothing here saves.** Ctrl+S is the pane's, on the document, and what it
//! writes is the buffer above.
//!
//! **The language is the buffer's**, which is the path's: the model is made at
//! `Uri.file(<the path>)`, and the package registered every extension when it
//! registered every language. So a `.rs` file is Rust because Monaco says it
//! is, and the TypeScript service reads the same path as the name of the file
//! it is compiling. Nothing here has a table of extensions, and nothing here
//! should ever grow one (ADR 0019, *Monaco, whole*).
//!
//! **It follows the workbench's light and dark**, which is the one thing about
//! it this stage sets: `vs` and `vs-dark`, VS Code's own two, picked by the
//! scheme the browser is in and followed live — the workbench has no theme
//! switch, so `prefers-color-scheme` is both the setting and the only warning
//! of a change, exactly as it is for the diagrams in `set/diagrams.ts`.
//!
//! **And the three settings on the pane's own menu**, which are followed the
//! same way and for the same reason: word wrap, the font size and the minimap
//! (ADR 0019, *Monaco, whole*). They are handed in rather than read here,
//! because they are one answer for the whole pane — a change on the menu
//! reaches every editor open in every group at once, with no tab reopened —
//! and where that answer is kept is `device.ts`, per device and never sent to
//! the server. Everything else about an editor is VS Code's default.
//!
//! **The editor is fetched rather than bundled in** — see [`./editing`] and
//! [`./monaco`]. The pane warms that fetch when it opens, so by the time a file
//! is pressed the chunk is usually already there; this waits on it either way,
//! and says so where it never arrives. It waits on the buffer too, the two
//! landing together: the buffer above is made out of the same chunk.
//!
//! **And it is made once, for as long as its tab is open.** A tab turned away
//! from is hidden rather than taken down — see `Code.tsx` — so the caret and
//! the undo stack are where they were left when somebody turns back to it. What
//! does take an editor down is the tab closing and the pane being swapped away
//! for an Event; the second of those costs the caret and costs the text
//! nothing, the buffer above outliving every editor drawn over it.

import {
  Show,
  createEffect,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import type { Drawn } from "../device";
import { ErrorLine } from "../notices";
import {
  load,
  type Drawing,
  type Model,
  type Monaco,
  type Standalone,
} from "./editing";
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

/// The pane's three settings, in the words Monaco takes them in.
///
/// One function for both the opening and the change, so that an editor is never
/// made one way and redrawn another — see [`Drawing`]. Said in full each time
/// rather than as a difference: Monaco merges what it is handed into the options
/// it already has, and three fields is cheaper to read than a diff would be.
function drawing(settings: Drawn): Drawing {
  return {
    wordWrap: settings.wrap ? "on" : "off",
    fontSize: settings.size,
    minimap: { enabled: settings.minimap },
  };
}

export function Editor(props: {
  /// What the file is called, which is what the editor is read aloud as: a path
  /// in a checkout is a sentence, and the name is what the tab above says too.
  name: string;
  /// The buffer to draw, or nothing at all while it is still being made — the
  /// model is Monaco's and Monaco is fetched. What the file is, what is in it
  /// and what it is coloured as are all this.
  model: Model | undefined;
  /// Whether the root it is in takes writes. A file in a read-only companion
  /// opens read-only and takes no typing — the root's own flag rather than the
  /// file's mode.
  writable: boolean;
  /// How the pane wants its editors drawn: word wrap, the font size and the
  /// minimap. One answer for every editor the pane has open, so a change on the
  /// menu reaches all of them at once rather than the one being looked at.
  drawn: Drawn;
}): JSX.Element {
  /// Where the editor is drawn. Its own element rather than this component's
  /// root, because Monaco fills whatever it is given and the line below has to
  /// stand somewhere when there is no editor to fill it.
  let host: HTMLDivElement | undefined;

  /// The editor, once there is a chunk and a buffer to make one out of.
  let editing: Standalone | undefined;

  /// The package, where the chunk has landed — a signal rather than a variable
  /// because the editor below waits on it and on the buffer together, and
  /// either may be the one to arrive last.
  const [bundle, setBundle] = createSignal<Monaco | undefined>();

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

  void load()
    .then((monaco) => setBundle(() => monaco))
    .catch(() => setLost(true));

  // The editor itself, made the moment both halves are here and never again:
  // what would remake it is a buffer swapped under a standing tab, and a tab is
  // one path for as long as it is open.
  //
  // The buffer is *not* disposed with it. The model is the keeping's — one file
  // is one of them however many views there are — and an editor is a view over
  // it that comes and goes.
  createEffect(() => {
    const monaco = bundle();
    const model = props.model;

    if (
      editing !== undefined ||
      monaco === undefined ||
      model === undefined ||
      host === undefined
    ) {
      return;
    }

    editing = monaco.editor.create(host, {
      model,
      ariaLabel: props.name,
      theme: dark() ? DARK : LIGHT,
      readOnly: !props.writable,
      automaticLayout: true,
      // Opened at whatever the pane's menu says rather than at Monaco's own and
      // corrected afterwards, so a tab opened while the text is set large is
      // never drawn small for a frame first.
      ...drawing(props.drawn),
    });
  });

  onCleanup(() => {
    editing?.dispose();
    editing = undefined;
  });

  // The theme, followed for as long as the tab is open. Monaco's themes are the
  // package's rather than an editor's, so this sets it for every editor there
  // is — which is every editor this pane has open, and all of them are in the
  // one browser being asked the one question.
  createEffect(() => {
    const theme = dark() ? DARK : LIGHT;

    bundle()?.editor.setTheme(theme);
  });

  // And the read-only flag, which the root a tab was opened from settles and
  // nothing changes under it. Said as an effect anyway, so that the one place
  // it is read is the one place it is written.
  createEffect(() => {
    editing?.updateOptions({ readOnly: !props.writable });
  });

  // And the three the pane's menu carries, which do change under an editor that
  // is already open: a press on the menu moves the setting the whole pane reads,
  // and every editor drawing it is redrawn where it stands. Unlike the theme,
  // these are the editor's rather than the package's, so each one is told —
  // which is what a hidden tab needs as much as the one showing, its editor
  // being alive behind the `hidden` and read the moment somebody turns back.
  //
  // Read before the editor is reached for rather than inside the call, because
  // `?.` on an editor that is not there yet would short-circuit the argument
  // with it — and an effect that never read the settings is one that is never
  // told they moved.
  createEffect(() => {
    const settings = drawing(props.drawn);

    editing?.updateOptions(settings);
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
