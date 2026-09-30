//! Putting files on what is being written: the paperclip, the row of pills and
//! the box that takes a drop.
//!
//! One piece rather than three, and one piece rather than one per page. There
//! are three places a file is handed over beside some text — the composer of a
//! draft, where a choice is a request and the record comes back; the compose
//! page, where nothing exists on the server yet and the files are held in the
//! page; and an answer sheet, where every Question carries one of these and a
//! chosen file goes onto the Set under that Question's label. What is different
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
//!
//! **And the paperclip grows into a menu where the caller has more to offer at
//! it.** The draft's composer offers the MCP servers declared in the settings,
//! so its control is a menu whose first row is *Attach file* — the press this
//! button always was — and whose others are the caller's. Where a caller offers
//! nothing, which is the compose page and every answer sheet, it is the plain
//! button it has always been. What the rows *are* is the caller's whole
//! business: this knows there is a menu and not what is in it.
//!
//! **What a menu's row attaches is drawn in this same row, as a chip.** An MCP
//! server is not a file — it is a reference by name to something declared
//! elsewhere, rather than bytes handed over — so it is drawn beside the pills
//! and told apart from one at a glance, with the same × on it while there is
//! still something to press. See [`Chip`]. A chip whose declaration has gone is
//! drawn saying so rather than vanishing: the human attached it on purpose, and
//! it is still theirs to take off.
//!
//! And the same row once nothing can be put on it any more — [`Attachments`],
//! which is what a frozen Brief is read under. The pills are these pills,
//! because they are the same files; what is different is that there is no ×
//! on one and that each of them says how large it is, a record being read
//! rather than a list being arranged.

import { For, Show, createSignal, type JSX } from "solid-js";

import { faPaperclip, faPlug } from "@fortawesome/free-solid-svg-icons";

import styles from "./Attaching.module.css";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";
import { Menu } from "./Menu";
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

  /// How large the file is, in words — see [`sized`].
  ///
  /// Said on a row that is a record and left off one that is a list being
  /// arranged: while the human is still choosing, what they want to know is
  /// which files are on the Brief, and a file they have this minute picked off
  /// their own disk is not a file they need told the size of. Once it is
  /// settled the row is the whole account of what was handed over, and how
  /// large a thing is is part of that account — it is what the session's own
  /// prompt says about each of them.
  size?: string;

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

/// One thing in the row that is not a file: an MCP server the Conversation has
/// attached, as the composer and the frozen Brief pane both draw it.
///
/// A chip rather than a pill, and told apart from one at a glance, because it is
/// a different kind of thing: a file is bytes handed over and sitting in the
/// Conversation's own directory, and this is a *reference by name* to something
/// declared on the settings page. What the row is, taken together, is everything
/// the human put on the work at this one control.
///
/// [`Self::gone`] is the reference with nothing on the other end of it. It is
/// drawn saying so rather than left out: the human attached it on purpose, the
/// declaration is theirs to put back, and a chip that vanished on its own would
/// be the one change to the Conversation nobody was told about.
export type Chip = {
  /// What it is called, which is the whole of what the Conversation holds.
  name: string;

  /// Whether nothing is declared by that name any more, which is what the chip
  /// says under it.
  gone?: boolean;

  /// Taking it off, where that is something that can be done at all — nothing
  /// past a freeze, exactly as a pill's.
  remove?: () => void;

  /// And whether that press is already in flight.
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
  ///
  /// The class is where it stands, which is the caller's — see [`Row`].
  Pills: (props: { class?: string }) => JSX.Element;

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

  /// What these files are being put on, where the page draws more than one of
  /// these: the Question's own name on an answer sheet, which is what tells its
  /// paperclip and its row from the four beside them for a reader who cannot
  /// see which question they are under. A composer draws one of each and says
  /// nothing, so both are named for what they are.
  onto?: string;

  /// What else is attached here, drawn as chips after the pills — see [`Chip`].
  ///
  /// Absent on every caller that has none, which is all of them but the draft's
  /// composer. A caller that offers a menu passes this as well: what a row of
  /// that menu attaches has to be drawn somewhere, and where it is drawn is
  /// here.
  chips?: () => Array<Chip>;

  /// The rows the menu holds under *Attach file*, where this control is a menu
  /// at all.
  ///
  /// Absent is the plain paperclip, which is what it has always been and what
  /// every caller but the draft's composer still wants. A function rather than
  /// a node, because the menu builds its rows when it opens and throws them
  /// away when it closes — see `Menu.tsx`.
  ///
  /// Handed the way to shut the menu, which a row that has done its work calls:
  /// which of them has is the caller's to know, and a card left hanging under
  /// the trigger after a press is a list the human has finished with.
  offering?: (shut: () => void) => JSX.Element;
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

  const Pills = (props: { class?: string }) => (
    <Row
      files={what.shown()}
      chips={what.chips?.() ?? []}
      class={props.class}
      label={rowNamed(what.onto, what.chips !== undefined)}
    />
  );

  const Clip = (props: { class?: string }) =>
    what.offering === undefined ? (
      <Attach
        add={what.add}
        class={props.class}
        label={what.onto ? `Attach a file to ${what.onto}` : undefined}
      />
    ) : (
      <AttachMenu
        add={what.add}
        class={props.class}
        offering={what.offering}
      />
    );

  return {
    Pills,
    Clip,
    over: () => offered() && depth() > 0,
    dropping,
  };
}

/// The same row where nothing can be put on it any more: what was handed over,
/// as the frozen Brief pane and a share read it.
///
/// The files rather than a [`Shown`] apiece, because there is nothing here for
/// the caller to decide — a record has no × on it and no file on its way up, and
/// the size is said off the record's own number. Which is the whole difference
/// between this and the row a draft draws: the same pills, with the controls
/// gone and the account of each file filled in.
///
/// Nothing at all where there are none, the way the other row draws none: a
/// Conversation nobody attached anything to is most of them, and a heading over
/// an empty row would read as something having gone missing.
export function Attachments(props: {
  /// What is on the record, in the order it was attached in.
  files: Array<{ name: string; bytes: number }>;

  /// Where the row stands, which is the caller's — see [`Row`].
  class?: string;

  /// What these files were put on, where the page draws more than one of these
  /// rows: the Question's own name on the record of an answered Set, which is
  /// what tells its row from the four beside it — the same naming the sheet
  /// gives them, because it is the same row read after the fact. A Brief's row
  /// is the one on its pane and says nothing.
  onto?: string;

  /// And what else was attached, as chips after the pills — see [`Chip`].
  ///
  /// Read off the record like the files beside them, so nothing here is
  /// pressable: what a frozen Brief draws is the account of what the sessions
  /// were given. Absent on the sheet's rows, an Answer having none.
  chips?: Array<Chip>;
}): JSX.Element {
  return (
    <Row
      files={props.files.map((file) => ({
        name: file.name,
        size: sized(file.bytes),
      }))}
      chips={props.chips ?? []}
      class={props.class}
      label={rowNamed(props.onto, props.chips !== undefined)}
    />
  );
}

/// What the row is called, for whoever cannot see what is in it.
///
/// Three answers, because the row holds three different things across the pages
/// that draw one: one of several rows on a page is named for what it is under,
/// a row that may hold chips as well as pills says so, and a row of files alone
/// is what it always was. Said once here rather than at each of the three places
/// a `<Row>` is built, so the two halves of the composer's row — the live one
/// and the frozen one — cannot come to be called different things.
function rowNamed(onto: string | undefined, chips: boolean): string {
  if (onto !== undefined) return `Files attached to ${onto}`;

  return chips ? "Attached files and MCP servers" : "Attached files";
}

/// The row itself, which both of them are.
///
/// Where it *stands* is the caller's and the class is how they say so: the
/// composer puts it inside its box at the inset the text is written at, the
/// compose page does the same, and the Brief pane puts it under the document at
/// the pane's own edge. What is the same wherever it is drawn is the row and the
/// pills in it, which is what this is.
function Row(props: {
  files: Array<Shown>;

  /// And what is attached here that is not a file, drawn after them — see
  /// [`Chip`]. Empty on every row but the composer's and the Brief pane's.
  chips: Array<Chip>;

  class?: string;

  /// What to call the row — see [`rowNamed`], which is where the three answers
  /// are decided.
  label?: string;
}): JSX.Element {
  return (
    <Show when={props.files.length + props.chips.length}>
      <ul
        class={
          props.class === undefined
            ? styles.attachments
            : `${styles.attachments} ${props.class}`
        }
        aria-label={props.label ?? "Attached files"}
      >
        <For each={props.files}>{(one) => <Pill file={one} />}</For>

        {/* And the chips after them, wherever there are any: the files are what
            the control was always for, and a reference to something declared
            elsewhere reads as the addition it is. */}
        <For each={props.chips}>{(one) => <ServerChip chip={one} />}</For>
      </ul>
    </Show>
  );
}

/// How large a file is said to be on a row that says so.
///
/// The words the session's own prompt uses, said the same way here — see
/// `sized` in `crates/server/src/skills.rs`, which this is the viewer's half
/// of. Rounded, and in whichever unit keeps it to a few digits: what the number
/// is for is knowing what was handed over rather than accounting. Decimal
/// units, the way a file manager says it.
export function sized(bytes: number): string {
  const size = Math.max(0, bytes);

  for (const [unit, scale] of [
    ["GB", 1e9],
    ["MB", 1e6],
    ["kB", 1e3],
  ] as const) {
    if (size >= scale) return `${(size / scale).toFixed(1)} ${unit}`;
  }

  return size === 1 ? "1 byte" : `${size} bytes`;
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

  /// What to call it, where one name for every paperclip on the page would not
  /// tell them apart — see `onto` in [`attaching`]. *Attach a file* otherwise,
  /// which is what one paperclip on a page is.
  label?: string;
}): JSX.Element {
  let picker!: HTMLInputElement;

  return (
    <>
      <IconButton
        of={faPaperclip}
        label={props.label ?? "Attach a file"}
        open={false}
        press={() => picker.click()}
        class={props.class}
      />
      <Picker ref={(input) => (picker = input)} add={props.add} />
    </>
  );
}

/// The same control where the caller has more to offer at it: a menu whose
/// first row is *Attach file* and whose others are the caller's — see
/// `offering` in [`attaching`].
///
/// The picker is the same hidden input, reached by the same click: what changes
/// is that the press that reaches it is a row of a menu rather than the button
/// itself. So a caller with nothing else to offer keeps the plain button, and
/// nothing about attaching a file is written twice.
///
/// The trigger is the same paperclip and is named *Attach* rather than *Attach a
/// file*: what it opens is a list of things to attach, and one of them is a
/// file.
///
/// The shut goes to the file row and to the caller's rows alike: every row here
/// is a press that has done its work by the time it returns, and which of the
/// caller's has is the caller's to know.
function AttachMenu(props: {
  add: (files: Array<File>) => void;
  offering: (shut: () => void) => JSX.Element;
  class?: string;
}): JSX.Element {
  let picker!: HTMLInputElement;

  // Handed over as the menu is built, so a row that has done its work can take
  // the card back and give the focus to the trigger it came from.
  let shut = (): void => {};

  return (
    <>
      <Menu
        class={[styles.clip, props.class].filter(Boolean).join(" ")}
        label="Attach"
        name="Attach"
        closer={(close) => (shut = close)}
        trigger={<Icon of={faPaperclip} />}
      >
        {() => (
          <>
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                // The menu first: the picker is the platform's own window and
                // what opens it is a press that has done its work, so the card
                // it was pressed in has no business still hanging under the
                // trigger behind it.
                shut();
                picker.click();
              }}
            >
              Attach file
            </button>

            {props.offering(shut)}
          </>
        )}
      </Menu>

      <Picker ref={(input) => (picker = input)} add={props.add} />
    </>
  );
}

/// One chip: the name of what is attached, whether anything still answers to
/// it, and the × that takes it off where there is one to draw.
///
/// The pill's shape in the pill's row, painted apart from one — see
/// [`Chip`], and `.server` in this module's stylesheet. The name is not cut the
/// way a file's is: a declaration's name is a handful of lowercase letters by
/// the rules the settings page enforces, and there is nothing in it to lose.
function ServerChip(props: { chip: Chip }): JSX.Element {
  return (
    <li
      class={`${styles.attachment} ${styles.server}`}
      classList={{ [styles.missing!]: props.chip.gone }}
    >
      {/* What kind of thing it is, said as a shape rather than as a word: the
          row is read across, and a chip that spelled out what it was would be
          longer than the name it is about. */}
      <Icon of={faPlug} class={styles.plug} label="MCP server" />

      <span class={styles.serverName}>{props.chip.name}</span>

      {/* And the reference with nothing on the end of it, said in words: a
          chip drawn differently and left to be noticed would be a difference
          nobody could look up. */}
      <Show when={props.chip.gone}>
        <span class={styles.attachmentSize}>no longer declared</span>
      </Show>

      <Show when={props.chip.remove !== undefined}>
        <button
          type="button"
          class={styles.forget}
          aria-label={`Remove ${props.chip.name}`}
          disabled={props.chip.removing}
          onClick={() => props.chip.remove?.()}
        >
          ×
        </button>
      </Show>
    </li>
  );
}

/// The browser's own picker, hidden, which both shapes of the control reach.
///
/// One of these rather than one per shape, because it is the same input with
/// the same handling behind either: what differs is only what is pressed to
/// open it.
function Picker(props: {
  ref: (input: HTMLInputElement) => void;
  add: (files: Array<File>) => void;
}): JSX.Element {
  return (
    <input
      ref={props.ref}
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
  );
}

/// One file drawn as a pill: its name cut to a line, how large it is where the
/// row says so, and the × that takes it away where there is one to draw.
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

      {/* And how large it is, where the row is a record. After the name and
          understated, because it is what the name is qualified by rather than a
          second thing on the pill. */}
      <Show when={props.file.size}>
        {(size) => <span class={styles.attachmentSize}>{size()}</span>}
      </Show>

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
