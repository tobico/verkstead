//! What Code has open, kept above the pane that draws it.
//!
//! The details pane draws one thing at a time and takes down whatever it is not
//! showing, so a pane swapped for an Event and back is a fresh mount: its
//! signals are made again, and whatever was in them is gone. Which is right for
//! every other pane — what those hold is a reading of the server, and a fresh
//! read is a truer one — and wrong for this one, because most of what Code
//! holds is the human's rather than the server's: the tabs they opened, the
//! text they have typed into one and not saved yet, and how far down the tree
//! they walked to find it.
//!
//! So it is kept here, above the frame's switch, and the pane is handed
//! whatever belongs to the Conversation it is being drawn for. Per
//! Conversation, because every one of these is: a file is a path in a Worktree,
//! and a terminal is a number in one Conversation's register.
//!
//! **The layout is here too**, which is where the tabs are: the tree of splits
//! with a group of tabs at every leaf, and which of those groups was last
//! pressed into — see [`./layout`]. How somebody divided the pane is as much
//! theirs as what they opened in it, and a swap to an Event and back that came
//! back undivided would be that work thrown away with the rest.
//!
//! **The buffers are here too**, which is the one thing in this list that is
//! not simply the human's doing (ADR 0019, *Tabs and groups*). A Monaco model
//! is registered at the file's own address and the package refuses a second one
//! there, so a buffer made by the editor drawing it could never be the buffer a
//! second editor of the same file was drawn over — and one text under two views
//! is what a file open in two groups has to be. Made when a file is first
//! opened and disposed when its last view closes; everything that reads or
//! writes the text — the dot, Ctrl+S, **Reload** — goes through [`Buffer`]
//! rather than through a string beside it.
//!
//! **And on the device as well**, which is what carries it through a reload —
//! see [`./remembering`]. What is here is what survives a swap; what is written
//! down there is the layout, each group's tabs and the one it is showing, and
//! the text of every buffer the disk has not got, per Conversation in the
//! browser's own storage (ADR 0019, *Tabs and groups*). So a keeping is made
//! out of whatever this device was last holding rather than empty, and goes on
//! writing itself down as it moves.
//!
//! The readings are not written down with it. What a file says is the disk's to
//! answer and a page coming back asks it afresh, the human's restored text
//! going over the top of what comes back — see `Code.tsx`. A page holding text
//! nobody has saved still warns before it goes, the browser's own way, because
//! storage is a convenience the whole way down and a browser that refuses it
//! would otherwise lose that text without a word — see [`unsaved`].
//!
//! The terminals are the exception inside the exception. A tab is kept, and the
//! shell under it is not this page's to keep: the register is the server's, and
//! a shell that ended while the pane was down is gone whatever this remembers.
//! So the pane reads the register on every opening and settles its tabs against
//! it — see `Code.tsx`.

import {
  createSignal,
  getOwner,
  runWithOwner,
  type Accessor,
  type Setter,
} from "solid-js";

import type { FileReading, FolderListing } from "../api/types";
import { load, type Model } from "./editing";
import { group as made, type Group, type Layout } from "./layout";
import { recalled, remembered, remembering, revived } from "./remembering";

/// One tab of a group: a terminal by the number the server issued it, or a
/// file by its path.
///
/// Two shapes rather than one with a kind beside it, because the two are named
/// by different things and nothing ever has to ask a tab what it is without
/// then using the answer.
///
/// **And one of these objects is one view.** It stands in exactly one group,
/// the pane draws exactly one box for it, and that box travels with it when it
/// is dragged into another group — which is what keeps a terminal's socket and
/// an editor's caret through the move. So the same file open in two groups is
/// two of these rather than one in both, and what says they are two views of
/// the one thing is what is *in* them: a path, or a number. See `copied` in
/// `Code.tsx`, which is what a split makes the second one with.
export type Tab = { terminal: number } | { file: string };

/// What a file's last save came to, where it came to anything the tab draws.
///
/// The file moved, which is a question to put to the human, or a sentence
/// saying the save was refused outright. Two shapes rather than one with a kind
/// beside it, for the reason [`Tab`] is two: they are drawn by different things
/// and nothing ever asks which one it is holding without then using the answer.
export type Bar = "moved" | { why: string };

/// One open file's buffer: the text the human is typing into.
///
/// The model is Monaco's own, and is where the text, the undo stack and the
/// language all are. It is made at `Uri.file(<the path>)`, which is the whole
/// of how a file is coloured — the package registered every extension when it
/// registered every language, so a `.rs` file is Rust because Monaco says it is
/// — and it is that address that makes there be exactly one of these per file.
///
/// **The signal beside it is what this side reads.** A model is not reactive,
/// so a dot that asked one for its text would be drawn once and never again;
/// what keeps the two in step is the model's own change event, which is the
/// only thing that writes the signal. So the model is where the text *is* and
/// the signal is how anything here asks what it says.
export interface Buffer {
  /// The model, which is what an editor is opened over.
  model: Model;

  /// The text as it stands, followed: what the dot on the tab, Ctrl+S and the
  /// warning on the way out of the page all compare against the reading.
  text: Accessor<string>;

  /// And text put in from somewhere that is not an editor, which is **Reload**:
  /// the disk's text, into a buffer still holding the human's. Every view of
  /// the file shows it, there being one buffer under them.
  ///
  /// Only where the two have come apart. Writing a model what it already says
  /// would cost the caret and the undo stack of every view of it, and **Reload**
  /// on a file the agent moved back to what it was is a press that should come
  /// to nothing.
  put: (text: string) => void;
}

/// Everything one Conversation has open in Code.
///
/// The pane's own signals, made out here instead of in it. Handed over whole
/// rather than read through an accessor apiece: what the pane does with them is
/// what it did when they were its own.
export interface Kept {
  /// How the pane is divided: the tree of splits, with a group of tabs at
  /// every leaf — see [`./layout`]. The one thing the pane draws from, which
  /// is what the flat list of tabs and the single record of which one was
  /// showing became.
  layout: Accessor<Layout>;
  setLayout: Setter<Layout>;

  /// The group last pressed into, by its id: where a file pressed in the tree
  /// opens, where a new terminal lands, and what Ctrl+S is a save of.
  active: Accessor<number>;
  setActive: Setter<number>;

  /// A group with nothing in it, which is what a split puts beside or below
  /// the one it was made from. The id is this keeping's own count and is never
  /// reused, so it names one group for as long as the page stands.
  group: () => Group;

  /// What each open file came back as: the text and its version, the picture,
  /// or the line saying why there is nothing to draw.
  readings: Accessor<Record<string, FileReading>>;
  setReadings: Setter<Record<string, FileReading>>;

  /// And the buffer behind each: the text as it stands, which starts as what
  /// was read and is what the human types into. One per open file path, however
  /// many views of it there are — see [`Buffer`].
  buffers: Accessor<Record<string, Buffer>>;

  /// Open a file's buffer at what the disk said, or — where the file already
  /// has one — put that text into it, which is what **Reload** does.
  ///
  /// The one way a buffer is ever made. Monaco is fetched first, so nothing is
  /// here the moment this is called and a file let go of before the chunk lands
  /// gets no buffer at all; a chunk that never arrives is the line the editor
  /// draws in place of itself, and this has nothing to add to it.
  hold: (path: string, text: string) => void;

  /// And let one go, which disposes the model.
  ///
  /// Called when a file's last view closes, because the address is Monaco's own
  /// register: a model left in it is a file that could never be opened again.
  release: (path: string) => void;

  /// The text this device was holding for a file when the page last went, where
  /// it is still holding it.
  ///
  /// Asked by the read that happens *because* the tab came back, which is the
  /// one read that puts the human's text over what the disk says now (see
  /// `Code.tsx`). It goes when that text reaches a buffer, or when the file is
  /// let go of — so a **Reload** pressed afterwards is the disk, and a read that
  /// never landed leaves the text where the device can still be holding it.
  recall: (path: string) => string | undefined;

  /// Which folders of the tree are open, and what each of them last read.
  ///
  /// A path is in here or it is not, and that is the whole of what open means:
  /// collapsing one takes its entry away, so expanding it again is a fresh
  /// reading of the disk. Kept out here with the tabs because walking down to
  /// a file is work the human did — a tree that came back to its roots every
  /// time an Event was opened would be that walk made again, and the pane that
  /// keeps the text they typed should keep how they got to it.
  expanded: Accessor<Record<string, FolderListing>>;
  setExpanded: Setter<Record<string, FolderListing>>;

  /// And what each tab that is standing rather than running says: the shell
  /// ended at once, or the refusal the server answered the open with.
  over: Accessor<Record<number, string>>;
  setOver: Setter<Record<number, string>>;

  /// What each tab's shell has called itself, where it has called itself
  /// anything.
  titles: Accessor<Record<number, string>>;
  setTitles: Setter<Record<number, string>>;

  /// And what each open file's save last came to, where it came to anything to
  /// draw.
  bars: Accessor<Record<string, Bar>>;
  setBars: Setter<Record<string, Bar>>;

  /// Which files a save is in flight for, so that a second Ctrl+S while the
  /// first is still being answered is not a second write of the same text.
  saving: Set<string>;

  /// When the pane asked for each terminal it opened, which is what the guard
  /// on a shell that could not start is measured from.
  askedAt: Map<number, number>;

  /// The key the next tab standing on a refusal gets. Below every number the
  /// server issues, counting the other way, because a refused open was never
  /// given one — there is no shell for it to name.
  refuse: () => number;
}

/// What a page keeps of Code, over every Conversation it has drawn it for.
export interface Keeping {
  /// One Conversation's, made the first time it is asked for. A Conversation
  /// nobody has opened Code on keeps nothing, and the first opening is where
  /// its tabs start out empty.
  of: (conversation: number) => Kept;

  /// Whether anything kept anywhere in here has text nobody has saved, which is
  /// the whole of what the warning on the way out of the page asks.
  unsaved: () => boolean;
}

/// A page's keeping, made where the page is — above the frame that swaps the
/// details pane, so that the swap does not reach it.
export function keeping(): Keeping {
  const kept = new Map<number, Kept>();

  /// Whose the effects that write a keeping down belong to: this page, rather
  /// than whatever happened to be drawing when the keeping was first asked for.
  ///
  /// A keeping is made the first time a Conversation's Code is drawn, which is
  /// inside the details pane — and that pane is taken down the moment somebody
  /// opens an Event. An effect owned by it would go with it, so a device would
  /// stop writing down anything that happened after the first swap. This owner
  /// is the page's own, and lasts as long as the keeping does.
  const page = getOwner();

  const of = (conversation: number): Kept => {
    const already = kept.get(conversation);

    if (already !== undefined) {
      return already;
    }

    const make = (): Kept => opened(conversation);
    const held = page === null ? make() : runWithOwner(page, make)!;

    kept.set(conversation, held);

    return held;
  };

  const unsaved = (): boolean =>
    [...kept.values()].some((one) =>
      Object.keys(one.buffers()).some((path) => dirty(one, path)),
    );

  return { of, unsaved };
}

/// One Conversation's, as this device last left it — and with nothing open in
/// it where this device has never opened anything.
///
/// The layout, the tabs and the text come off the browser's storage, which is
/// what makes a reload come back to the splits (see [`./remembering`]). What
/// does not is the readings: the pane reads every restored file afresh and puts
/// the human's text over the top of it, so a file that moved while the page was
/// away is dirty against what is really there.
function opened(conversation: number): Kept {
  /// What this device was last holding, where it was holding anything: the tree
  /// with its groups made again, and which of them was active.
  const restored = remembered(conversation);
  const came = restored === null ? null : revived(restored);

  /// And the text it had typed into each file and not saved, by path.
  ///
  /// Each entry stands until there is a buffer to answer for that file — which
  /// is what the read restoring the tab makes — or until the file is let go of,
  /// and is what the device goes on holding in the meantime.
  const recalling = recalled(conversation);

  /// What the next group is called. Counted up and never reused, so a group
  /// closed and another opened are two groups rather than one name meaning two
  /// things — and a restored layout takes up above the highest id in it, so a
  /// group made after the reload answers to nothing that came back.
  let groups = came?.last ?? 0;

  const group = (): Group => made((groups += 1));

  // Where the pane starts: what came back, or — where nothing did — undivided,
  // which is one group with nothing in it. A split is something the human asks
  // for, and the group it is asked of is this one.
  const began =
    came ??
    (() => {
      const first = group();

      return { layout: { group: first } as Layout, active: first.id };
    })();

  const [layout, setLayout] = createSignal<Layout>(began.layout);
  const [active, setActive] = createSignal(began.active);
  const [readings, setReadings] = createSignal<Record<string, FileReading>>({});
  const [buffers, setBuffers] = createSignal<Record<string, Buffer>>({});
  const [expanded, setExpanded] = createSignal<Record<string, FolderListing>>(
    {},
  );
  const [over, setOver] = createSignal<Record<number, string>>({});
  const [titles, setTitles] = createSignal<Record<number, string>>({});
  const [bars, setBars] = createSignal<Record<string, Bar>>({});

  let refusals = 0;

  /// Which paths a buffer is wanted for, which is what a chunk that lands after
  /// the tab has gone is checked against: the fetch is a promise, and a file
  /// closed while it was in flight would otherwise be a model registered at an
  /// address nothing is ever going to dispose.
  const wanted = new Set<string>();

  const hold = (path: string, text: string): void => {
    const already = buffers()[path];

    if (already !== undefined) {
      already.put(text);
      return;
    }

    wanted.add(path);

    void load()
      .then((monaco) => {
        if (!wanted.has(path) || buffers()[path] !== undefined) {
          return;
        }

        const model = monaco.editor.createModel(
          text,
          undefined,
          monaco.Uri.file(path),
        );
        const [held, setHeld] = createSignal(text);

        // The one thing that writes the signal: every edit, wherever it was
        // typed, arrives here — which is how two views of one file wear the one
        // dot. Disposed with the model, Monaco's listeners being the model's.
        model.onDidChangeContent(() => setHeld(model.getValue()));

        const put = (next: string): void => {
          if (model.getValue() !== next) {
            model.setValue(next);
          }
        };

        setBuffers((was) => ({ ...was, [path]: { model, text: held, put } }));
      })
      // A chunk that never arrived is a tab with no editor in it, which is what
      // `Editor.tsx` says in words. Nothing to add to it from here.
      .catch(() => {});
  };

  const release = (path: string): void => {
    wanted.delete(path);

    // And a file let go of is one nothing is holding text for: what came off
    // the device with it goes rather than waiting for a tab that is not coming
    // back.
    delete recalling[path];

    const buffer = buffers()[path];

    if (buffer === undefined) {
      return;
    }

    buffer.model.dispose();
    setBuffers((was) => {
      const rest = { ...was };
      delete rest[path];
      return rest;
    });
  };

  /// What this device was holding for this file when the page went, where it is
  /// still holding it: the read that restores a tab is the one read that wants
  /// it, and every read after that is somebody asking for the disk.
  const recall = (path: string): string | undefined => recalling[path];

  const kept: Kept = {
    layout,
    setLayout,
    active,
    setActive,
    group,
    readings,
    setReadings,
    buffers,
    hold,
    release,
    recall,
    expanded,
    setExpanded,
    over,
    setOver,
    titles,
    setTitles,
    bars,
    setBars,
    saving: new Set<string>(),
    askedAt: new Map<number, number>(),
    refuse: () => (refusals -= 1),
  };

  // And from here it writes itself down as it moves, which is what a reload
  // comes back to — the layout as the tree says it, and the text of every
  // buffer the disk has not got. Asked of [`dirty`] rather than compared here,
  // so that what the device keeps and what the dot on a tab means are the one
  // idea.
  //
  // **Over whatever has no buffer to answer for it yet**, which is a page that
  // has only just loaded: the files are being read, Monaco is on its way, and
  // there is not a buffer among them. What came off the device is written back
  // as it came until one of them arrives, so a second reload before any of that
  // lands — or a read that never lands at all — comes back to the same text
  // rather than to a key this page emptied while it was getting ready.
  remembering(conversation, {
    layout,
    active,
    unsaved: () => {
      const live = kept.buffers();

      return {
        ...Object.fromEntries(
          Object.entries(recalling).filter(
            ([path]) => live[path] === undefined,
          ),
        ),
        ...Object.fromEntries(
          Object.keys(live)
            .filter((path) => dirty(kept, path))
            .map((path) => [path, live[path]!.text()]),
        ),
      };
    },
  });

  return kept;
}

/// What the disk said about one open file, where what it said was text.
///
/// The reading rather than the buffer: this is what was read and the version it
/// was read at, which is what a save is a write over — and what the human's
/// text is compared against to know whether there is anything to save at all.
export function disk(
  kept: Kept,
  path: string,
): Extract<FileReading, { Text: unknown }>["Text"] | undefined {
  const read = kept.readings()[path];

  return read !== undefined && typeof read !== "string" && "Text" in read
    ? read.Text
    : undefined;
}

/// Whether a file has text in it that is not on the disk.
///
/// The buffer against the reading, which is the whole of what dirty means here:
/// the reading is what the disk said at the version it said it at, and the
/// buffer is what would be written over it. So a file typed into and typed back
/// is clean again, which is what VS Code's own dot says too.
///
/// A file with no buffer yet is not dirty: the read is in flight, or what came
/// back was not text at all, and neither is a tab with something in it to lose.
export function dirty(kept: Kept, path: string): boolean {
  const read = disk(kept, path);
  const held = kept.buffers()[path];

  return read !== undefined && held !== undefined && held.text() !== read.text;
}
