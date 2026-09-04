//! Putting files on what is being written: the paperclip, the row of pills and
//! the box that takes a drop.
//!
//! One piece rather than three, and one piece rather than one per page. There
//! are two places a file is handed over beside some text — the composer of a
//! draft, where a choice is a request and the record comes back, and the
//! compose page, where nothing exists on the server yet and the files are held
//! in the page — and an Answer sheet will be the third. What is different
//! between them is what *becomes* of a chosen file, which is the one thing this
//! does not do: it is handed the list to draw and what to do when a file is put
//! on or taken off, and everything the human touches is the same on all of
//! them.
//!
//! **Three parts, taken together.** The pill row goes inside the box under the
//! text, the paperclip goes in the row of presses under it, and the drop
//! handling goes on the box itself — three places in the page, and no element
//! that contains all of them. So they are made in one call and drawn where each
//! of them belongs, which is what keeps them agreeing about the same list:
//!
//!     const files = attaching({ shown: ..., add: ... });
//!
//!     <div class={box} classList={{ [over]: files.over() }} {...files.dropping}>
//!       …the text…
//!       <files.Pills />
//!       …the setup row…
//!     </div>
//!     <files.Clip class={near} />
//!
//! **The whole box is the drop target**, text, pills and setup row alike, rather
//! than a strip somewhere in it: what the human is dropping onto is the thing
//! they are writing, and a target smaller than it would be a target to find.
//! While a drag carrying files is over it the box is highlighted — the caller's
//! own class, this saying only when — and the highlight goes when the drag
//! leaves or drops. A drag carrying anything else, a selection of text or a
//! link, is not a drop this takes and nothing is drawn for it.
//!
//! **Folders are skipped without a word.** A drop is whatever the human had
//! hold of, and one of them being a directory is not a mistake to report: the
//! files in it are attached, and it is not. A picker cannot offer one at all.

import { For, Show, createSignal, type JSX } from "solid-js";

import { faPaperclip } from "@fortawesome/free-solid-svg-icons";

import styles from "./Attaching.module.css";
import { IconButton } from "./IconButton";
import { Truncated } from "./Truncated";

/// One file to draw in the row.
///
/// A file waiting to be sent and a file that has landed look alike because they
/// are the same thing to the human — what is different about them is only what
/// the × does, and whether there is one to press yet.
export type Shown = {
  /// What the pill reads. Two files chosen together may share one, which is why
  /// nothing here is keyed on it: whoever hands the list over keeps its own
  /// hold of which file is which.
  name: string;

  /// Drawn dimmed where the file is on its way up and the record is not back:
  /// the press has been made and there is nothing to press on it yet.
  landing?: boolean;

  /// Taking it away, where that is something that can be done at all — nothing
  /// on one still landing, and nothing past a freeze.
  remove?: () => void;

  /// And whether the removal is already in flight, which is the one thing a
  /// press on the × can be truly disabled for.
  removing?: boolean;
};

/// What a box spreads onto itself to take a drop.
export type Dropping = {
  onDragEnter: (ev: DragEvent) => void;
  onDragOver: (ev: DragEvent) => void;
  onDragLeave: (ev: DragEvent) => void;
  onDrop: (ev: DragEvent) => void;
};

/// The three parts, made together and drawn apart.
export type Attaching = {
  /// The row of pills, inside the box under the text. Nothing at all where
  /// there are none, rather than an empty row with a heading over it.
  Pills: () => JSX.Element;

  /// The paperclip, wherever the caller's row of presses wants it — the class
  /// is what says where, this being no business of the button's.
  Clip: (props: { class?: string }) => JSX.Element;

  /// Whether a drag carrying files is over the box, which is what the
  /// highlight is drawn from.
  over: () => boolean;

  /// And what makes it the drop target, spread onto the element.
  dropping: Dropping;
};

/// The whole of that, made once by whoever is drawing it.
export function attaching(what: {
  /// The files to draw, in the order they were put on.
  shown: () => Array<Shown>;

  /// Files chosen or dropped, several at a time: a picker takes more than one
  /// and a drop is whatever was being carried.
  add: (files: Array<File>) => void;

  /// Whether files can be put on at all. Where they cannot — a Brief that has
  /// frozen, a compose page locked to a roadmap card — there is no paperclip
  /// and the box takes no drop, while the row goes on drawing what is already
  /// there. True unless said otherwise.
  offered?: () => boolean;
}): Attaching {
  const offered = () => what.offered?.() ?? true;

  // How many elements inside the box the drag is currently inside, rather than
  // whether it is over the box: `dragenter` and `dragleave` fire again for
  // every child it crosses, so a flag would go out the moment the drag moved
  // from the text onto a pill. Counting up and down leaves the box highlighted
  // for as long as the drag is anywhere in it.
  const [depth, setDepth] = createSignal(0);

  /// Whether this drag is one this box would take at all — a selection of text
  /// or a link being dragged over the page is not, and nothing is drawn for it.
  const carrying = (ev: DragEvent) =>
    offered() && (ev.dataTransfer?.types ?? []).includes("Files");

  const dropping: Dropping = {
    onDragEnter: (ev) => {
      if (!carrying(ev)) return;
      ev.preventDefault();
      setDepth((was) => was + 1);
    },

    // The one that has to be taken for a drop to happen at all: a page that
    // does not answer `dragover` is a page the browser opens the file in.
    onDragOver: (ev) => {
      if (!carrying(ev)) return;
      ev.preventDefault();
      if (ev.dataTransfer) ev.dataTransfer.dropEffect = "copy";
    },

    onDragLeave: (ev) => {
      if (!carrying(ev)) return;
      setDepth((was) => Math.max(0, was - 1));
    },

    onDrop: (ev) => {
      if (!carrying(ev)) return;
      ev.preventDefault();
      // Straight to nothing rather than down one: the drag is over however
      // many children of the box it was counted into.
      setDepth(0);

      const files = dropped(ev.dataTransfer);
      if (files.length) what.add(files);
    },
  };

  const Pills = () => (
    <Show when={what.shown().length}>
      <ul class={styles.attachments} aria-label="Attached files">
        <For each={what.shown()}>{(one) => <Pill file={one} />}</For>
      </ul>
    </Show>
  );

  const Clip = (props: { class?: string }) => (
    <Attach add={what.add} class={props.class} />
  );

  return {
    Pills,
    Clip,
    over: () => offered() && depth() > 0,
    dropping,
  };
}

/// The files in a drop, the folders in it left behind.
///
/// Through `items` where the browser has them, that being the only place it
/// will say which of the things dropped was a directory: a folder arrives as a
/// `File` like anything else, with a name and no way of telling from the file
/// beside it. Through `files` otherwise, which is every drop that carried no
/// folder anyway.
function dropped(transfer: DataTransfer | null): Array<File> {
  if (!transfer) return [];

  const items = [...(transfer.items ?? [])];

  if (items.length && items.every((item) => item.webkitGetAsEntry)) {
    return items
      .filter(
        (item) => item.kind === "file" && !item.webkitGetAsEntry()?.isDirectory,
      )
      .map((item) => item.getAsFile())
      .filter((file): file is File => file !== null);
  }

  return [...(transfer.files ?? [])];
}

/// The paperclip, and the browser's own picker behind it.
///
/// A button over the picker rather than the picker itself: an
/// `<input type="file">` draws a control of the platform's choosing with a word
/// beside it, and what belongs in a row of presses is an icon. So the input is
/// there and hidden, and the button is what reaches it.
///
/// Several files at once. What becomes of them is the caller's own business, so
/// what this hands over is the choice and nothing else.
function Attach(props: {
  add: (files: Array<File>) => void;
  class?: string;
}): JSX.Element {
  let picker!: HTMLInputElement;

  return (
    <>
      <IconButton
        of={faPaperclip}
        label="Attach a file"
        open={false}
        press={() => picker.click()}
        class={props.class}
      />
      <input
        ref={picker}
        class={styles.picker}
        type="file"
        multiple
        onChange={(ev) => {
          props.add(Array.from(ev.currentTarget.files ?? []));
          // Emptied on the way out, so that choosing the same file again is a
          // change: an input still holding what was chosen last fires nothing.
          ev.currentTarget.value = "";
        }}
      />
    </>
  );
}

/// One file drawn as a pill: its name cut to a line, and the × that takes it
/// away where there is one to draw.
///
/// Cut at the front, which is how every other name in the app is cut — see
/// [`Truncated`](./Truncated.tsx). On a file name that is the half worth
/// keeping too: the extension is what says what the thing is, and the whole
/// name is under the pointer either way.
function Pill(props: { file: Shown }): JSX.Element {
  return (
    <li
      class={styles.attachment}
      classList={{ [styles.landing!]: props.file.landing }}
    >
      <Truncated text={props.file.name} class={styles.attachmentName} />

      {/* A mark rather than a word, the way a companion row's is, and named for
          this file: the row is a line of names and the × on its own says
          nothing about which one it takes. */}
      <Show when={props.file.remove !== undefined}>
        <button
          type="button"
          class={styles.forget}
          aria-label={`Remove ${props.file.name}`}
          disabled={props.file.removing}
          onClick={() => props.file.remove?.()}
        >
          ×
        </button>
      </Show>
    </li>
  );
}
