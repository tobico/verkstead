//! The field a path is written into: a text input, and the dropdown that
//! browses the filesystem into it.
//!
//! It *extends* a text field rather than standing in for one. What the human
//! types is still the value, the form around it still owns the label and the
//! press that submits it, and what the server makes of the submitted path is
//! still the only thing that decides what it does. What the dropdown adds is the
//! one thing a text field never had: a path is otherwise typed blind from a
//! phone, where a directory nobody has made, a directory the server cannot see
//! and a directory typed with a letter missing all look the same.
//!
//! ## How a browse goes
//!
//! One directory at a time, because that is how the endpoint behind it answers
//! — see `crates/server/src/browsing.rs`, where a browse costs one reading of
//! one directory however much is under it.
//!
//! **Typing steers it.** The rows are the entries of the deepest directory the
//! field's own text names: everything up to the last `/` is the directory to
//! list, and whatever follows it filters the rows. Which is what a path halfway
//! through being typed already means, so there is nothing here for the human to
//! learn.
//!
//! **A tap both writes and opens.** Tapping a row puts that directory in the
//! field *and* lists it, and the row at the top of the list goes back up a level
//! and writes that the same way. There is no separate pick affordance: the
//! field's contents are the choice, and browsing is a way of writing them.
//!
//! **The human closes it.** The backdrop or Escape takes the rows away and the
//! field keeps whatever it holds. Closing is how a browse ends rather than how
//! one is committed, so there is no way to leave the dropdown having chosen
//! something the field does not show.
//!
//! ## What it is made of
//!
//! The chrome is [`Listbox`]'s, imported rather than copied: the same combobox
//! roles, the same keyboard walk, the same rows hung off the same measurement of
//! what would clip them — see `picking.tsx` and `picking.module.css`. The two
//! dropdowns the app draws for itself are one thing to the eye, and two copies
//! of that would be two things to drift. What is this component's own is the
//! browsing: which directory the rows come out of, and what a tap does with one.
//!
//! Which scope an ask is made in is the caller's, because it is a fact about the
//! field rather than about the dropdown: a value the server would refuse outside
//! the Watched Paths browses inside them, and a value it says nothing about
//! browses anywhere. A browse bounded by them stops at those roots on the way
//! back out as well: above one is outside the boundary, and a row leading
//! somewhere the server would refuse is a row nobody should be offered.
//!
//! So is what the field is looking *for*. One of them is looking for a
//! repository — the Repos' form, which is the only place a `.git` means
//! anything — and there a repository draws marked and is where the browse
//! stops. Every other field says nothing about one and treats it as the
//! directory it also is.

import {
  For,
  Match,
  Show,
  Switch as Choose,
  createEffect,
  createSignal,
  createUniqueId,
  type JSX,
} from "solid-js";

import { listDirectory } from "./api/client";
import type {
  BrowseScope,
  DirectoryEntry,
  DirectoryListing,
} from "./api/types";
import { useReading } from "./freshness";
import { Empty, ErrorLine } from "./notices";
import styles from "./PathField.module.css";
import { clipping } from "./picking";
import chrome from "./picking.module.css";

/// One row of the dropdown: what it reads as, what a tap on it writes into the
/// field, whether it is the way back up rather than somewhere to go into, and
/// whether it is the thing this field is looking for — a repository, in the one
/// field that is looking for one, which draws marked and is where a browse
/// stops.
type Row = {
  label: string;
  path: string;
  back: boolean;
  found: boolean;
};

/// What a listing came back holding, or `null` where it came back as one of the
/// refusals instead.
///
/// Told apart by the shape rather than by a field, the way the wire writes them
/// — an outcome with nothing to carry is the word itself, and only the two that
/// carry something are objects.
function listed(
  listing: DirectoryListing | undefined,
): { path: string | null; entries: DirectoryEntry[] } | null {
  return listing !== undefined &&
    typeof listing !== "string" &&
    "Listed" in listing
    ? listing.Listed
    : null;
}

/// And what the server made of the path where it made nothing of it: the line
/// the dropdown draws where its rows would be.
///
/// Every one of these is a state a field is ordinarily in halfway through being
/// typed into rather than something that went wrong, so they are drawn as
/// quietly as an empty list is. The unreadable one is the server's own sentence,
/// being the only one of the five saying something a human could not work out
/// from what they typed.
function refusal(listing: DirectoryListing): string | null {
  if (typeof listing !== "string") {
    return "Unreadable" in listing ? listing.Unreadable.why : null;
  }

  switch (listing) {
    case "NotAbsolute":
      return "A path to browse starts with /.";
    case "Missing":
      return "There is nothing at that path.";
    case "NotADirectory":
      return "That is not a directory.";
    case "OutsideWatchedPaths":
      return "That path is outside the watched paths.";
  }
}

/// The directory above one, or `null` where there is none to go to.
function above(path: string): string | null {
  if (path === "/") return null;

  const cut = path.lastIndexOf("/");

  return cut < 0 ? null : cut === 0 ? "/" : path.slice(0, cut);
}

export function PathField(props: {
  /// The field's own id, for the `<label for=…>` the caller writes.
  id: string;
  /// Where this field's value may be, which is what says where it may browse.
  scope: BrowseScope;
  /// Whether a repository is what this field is looking for.
  ///
  /// The one git-aware behaviour there is here, and it is the Repos' form's: a
  /// repository draws marked, because it is what that form is being filled in
  /// with, and it is a leaf — a tap writes it into the field like any other row
  /// and does not open it, there being nothing under it this field is after.
  /// Every other field leaves this off and treats a repository as the directory
  /// it also is.
  repositories?: boolean;
  /// What the field holds, and how it comes to hold something else — typed into
  /// or tapped together, which the caller cannot tell apart and has no reason
  /// to.
  value: string;
  write: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}): JSX.Element {
  /// Whether the rows are down. Nothing opens them but an ask to browse: a
  /// dropdown that fell open on the first keystroke would be in the way of
  /// somebody who knows the path and is typing it.
  const [open, setOpen] = createSignal(false);

  /// Which row the keyboard is on while they are, as [`Listbox`] keeps it.
  const [walked, setWalked] = createSignal(0);

  /// And which way they hang: under the field where there is room, over it where
  /// there is not.
  const [over, setOver] = createSignal(false);

  /// The directory a tap put in the field, or `null` while nothing did.
  ///
  /// The one thing the text alone cannot say. Tapping `/home/ada/src` writes
  /// that path, and the same text typed by hand means *the entries of
  /// `/home/ada` beginning with `src`* — where the tap means the entries of
  /// `/home/ada/src`, which is the drilling in. Held only for as long as the
  /// field still holds what the tap wrote, so a field cleared or rewritten from
  /// anywhere else is text again and the text steers.
  const [drilled, setDrilled] = createSignal<string | null>(null);

  const drilling = (): string | null =>
    drilled() === props.value ? drilled() : null;

  /// Which directory the rows come out of: the one a tap drilled into, or the
  /// one the text names — everything up to its last separator, with `null` for
  /// text with no separator in it at all, which is the empty field and whatever
  /// the two scopes make of one.
  const inside = (): string | null => {
    const at = drilling();
    if (at !== null) return at;

    const cut = props.value.lastIndexOf("/");

    return cut < 0 ? null : cut === 0 ? "/" : props.value.slice(0, cut);
  };

  /// And what filters them: the segment after that separator, which is the part
  /// of a path typed but not finished. Nothing filters the rows of a directory
  /// that was tapped into — the whole of it is what was asked for.
  const partial = (): string =>
    drilling() !== null
      ? ""
      : props.value.slice(props.value.lastIndexOf("/") + 1);

  /// What that directory holds, asked for only while the rows are down.
  ///
  /// Keyed by the directory rather than by the field, so typing inside one
  /// segment costs nothing: the filter moves over rows already read, and only a
  /// separator sends a request. Merged by path, a re-read being the same
  /// directory read again.
  const listing = useReading(() => ({
    queryKey: ["directories", props.scope, inside()],
    queryFn: () => listDirectory(props.scope, inside()),
    enabled: open(),
    freshness: { reconcile: "path" },
  }));

  /// The roots a browse bounded by the Watched Paths begins at, read only where
  /// it is bounded by them.
  ///
  /// What the way back out stops at: above a root is outside the boundary, and
  /// the server would refuse it. The same read the empty field makes and under
  /// the same key, so a browse that started at the roots has already paid for
  /// this one.
  const boundary = useReading(() => ({
    queryKey: ["directories", "watched", null],
    queryFn: () => listDirectory("watched", null),
    enabled: open() && props.scope === "watched",
    freshness: { reconcile: "path" },
  }));

  /// Whether a directory is as far out as this field's browse goes.
  ///
  /// Only the bounded scope has a ceiling: the other one's is `/`, which has
  /// nothing above it to offer anyway. Roots not yet read count as one — a way
  /// out missing for the moment the read takes is better than one offered and
  /// then refused.
  const ceiling = (path: string): boolean => {
    if (props.scope !== "watched") return false;

    const roots = listed(boundary.data);

    return roots === null || roots.entries.some((root) => root.path === path);
  };

  /// The entries as this field shows them: directories, which is what these
  /// fields name, and none of the dotfiles the endpoint always lists — showing
  /// those is the field's decision rather than the server's, and a browse is not
  /// how somebody reaches one.
  ///
  /// A repository is one of the directories. It is a directory that holds a
  /// `.git`, and a field with nothing to say about that treats it as the
  /// directory it also is.
  const shown = (): DirectoryEntry[] => {
    const answer = listed(listing.data);
    if (answer === null) return [];

    const looking = partial().toLowerCase();

    return answer.entries.filter(
      (entry) =>
        entry.kind !== "File" &&
        !entry.name.startsWith(".") &&
        entry.name.toLowerCase().startsWith(looking),
    );
  };

  /// The rows: those, and the way back out at the top of them.
  ///
  /// The way back is a row rather than a control beside the list, for the reason
  /// a nested menu's is — see [`Nested`] in `Menu.tsx`. It is one of the places
  /// this browse can go, and a hand walking the list should reach it the way it
  /// reaches everywhere else.
  const rows = (): Row[] => {
    const answer = listed(listing.data);
    const at = answer?.path ?? null;
    const up = at === null || ceiling(at) ? null : above(at);

    return [
      ...(up === null
        ? []
        : [{ label: `Up to ${up}`, path: up, back: true, found: false }]),
      ...shown().map((entry) => ({
        label: entry.name,
        path: entry.path,
        back: false,
        found: props.repositories === true && entry.kind === "Repository",
      })),
    ];
  };

  /// What the dropdown says instead of rows, or `null` while it has rows to say.
  const said = (): string | null =>
    listing.data === undefined ? null : refusal(listing.data);

  /// Which row the keyboard is on, held inside the list as it stands now: the
  /// rows move under a walk whenever the filter does.
  const walking = (): number =>
    Math.min(walked(), Math.max(0, rows().length - 1));

  // The rows' own ids, for the `aria-activedescendant` that says which one the
  // keyboard is on — one page can hold several of these fields.
  const list = createUniqueId();
  const rowId = (index: number): string => `${list}-${index}`;

  // The input, so that a press on a row — which is a press on something no
  // browser will focus — hands the keyboard back to it.
  let field!: HTMLInputElement;

  // And the rows, for the measure that says which way they hang.
  let dropped: HTMLDivElement | undefined;

  const drop = (): void => {
    setWalked(0);
    setOpen(true);
  };

  const shut = (): void => {
    setOpen(false);
    field.focus();
  };

  /// Take one row: write where it goes into the field, and go there.
  ///
  /// Both, which is the whole interaction — there is nothing else a row does.
  /// The rows stay down afterwards, because a browse ends when the human says it
  /// does rather than when they have gone somewhere.
  ///
  /// Except the row that is what the field was looking for, which is written and
  /// not opened: a repository is where the Repos' form's browse was going, and
  /// there is nothing under it that form is after. Nothing is drilled, so the
  /// text steers again — the field now names that repository, which is the level
  /// above it filtered to its own name, and the row the human took is the row
  /// left standing. Closing is still theirs, as it is everywhere else here.
  const take = (row: Row): void => {
    setDrilled(row.found ? null : row.path);
    setWalked(0);
    props.write(row.path);
  };

  /// The keyboard, on the field itself — everything [`Listbox`] answers to,
  /// minus the keys that are the input's own. A closed dropdown hears only the
  /// way in, so Enter still submits the form the field stands in.
  const key = (ev: KeyboardEvent): void => {
    if (!open()) {
      if (ev.key === "ArrowDown") {
        ev.preventDefault();
        drop();
      }
      return;
    }

    switch (ev.key) {
      case "Escape":
        // The rows go and the field keeps what it holds, which is how a browse
        // ends. Prevented as well as handled, so that Escape over the rows is
        // not also Escape over whatever they were dropped inside.
        ev.preventDefault();
        shut();
        break;
      case "Enter": {
        // The walked row, where there is one. Where there is none — a filter
        // matching nothing, a directory that would not list — the press is the
        // form's, and the field submits what it holds.
        const row = rows()[walking()];
        if (row) {
          ev.preventDefault();
          take(row);
        }
        break;
      }
      case "ArrowDown":
        ev.preventDefault();
        setWalked(Math.min(walking() + 1, rows().length - 1));
        break;
      case "ArrowUp":
        ev.preventDefault();
        setWalked(Math.max(walking() - 1, 0));
        break;
      case "Home":
        // Prevented, unlike in a plain text field: the walk is what these two
        // move while the rows are down, and the caret can be put back once they
        // are gone.
        ev.preventDefault();
        setWalked(0);
        break;
      case "End":
        ev.preventDefault();
        setWalked(rows().length - 1);
        break;
      case "Tab":
        // The hand is leaving the field and the rows go with it. Left to the
        // browser, so the focus lands wherever it was going.
        setOpen(false);
        break;
    }
  };

  // Which way the rows hang, measured each time they come down and each time
  // they are rebuilt underneath — a filter that cuts a long list to two rows is
  // a drop that fits under a field it did not fit under before. Against whatever
  // would clip them rather than against the window: these stand in a pane that
  // scrolls its own content.
  createEffect(() => {
    if (!open()) {
      setOver(false);
      return;
    }

    // Read, so that the measure is made again whenever the rows move.
    rows();

    if (!dropped) return;

    const anchor = field.getBoundingClientRect();
    const wanted = dropped.getBoundingClientRect().height;
    const clip = clipping(field);

    setOver(
      anchor.bottom + wanted > clip.bottom &&
        anchor.top - clip.top > clip.bottom - anchor.bottom,
    );
  });

  // The row the keyboard has walked to, kept in view: the drop is capped in
  // height, so a directory holding more than fits can be walked past its own
  // edge. Asked for rather than called, jsdom having no scrolling at all.
  createEffect(() => {
    if (!open()) return;

    document.getElementById(rowId(walking()))?.scrollIntoView?.({
      block: "nearest",
    });
  });

  return (
    <div class={styles.field}>
      {/* The keyboard is the Repos' form's, which asked for a URL's before this
          component existed and now every path field has one: a path is typed
          with slashes in it, and nothing about it wants a capital. */}
      <input
        id={props.id}
        ref={field}
        type="text"
        inputmode="url"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck={false}
        placeholder={props.placeholder}
        disabled={props.disabled}
        value={props.value}
        role="combobox"
        aria-expanded={open() ? "true" : "false"}
        aria-controls={open() ? list : undefined}
        aria-activedescendant={
          open() && rows().length > 0 ? rowId(walking()) : undefined
        }
        aria-autocomplete="list"
        onInput={(ev) => {
          // Typed text steers the rows from here on, whatever a tap put in the
          // field before it.
          setDrilled(null);
          setWalked(0);
          props.write(ev.currentTarget.value);
        }}
        onKeyDown={key}
      />

      {/* The way in, for the hand with no arrow key on it. Its own control
          rather than something the field does when it is focused, so that
          focusing a path field to correct one character does not drop a list
          over whatever is under it. */}
      <button
        type="button"
        class={styles.browse}
        aria-label="Browse"
        aria-expanded={open() ? "true" : "false"}
        disabled={props.disabled}
        onClick={() => (open() ? shut() : drop())}
      >
        <span aria-hidden="true">▾</span>
      </button>

      <Show when={open()}>
        {/* What a press away from the rows lands on, so that it lands on
            nothing else — the listbox's own, and for its reason: this is a
            field being filled in rather than a card opened over what the human
            was reading. */}
        <div class={chrome.backdrop} aria-hidden="true" onClick={() => shut()} />

        <div
          ref={dropped}
          class={[chrome.drop, over() ? chrome.above : undefined]
            .filter(Boolean)
            .join(" ")}
          id={list}
          role="listbox"
        >
          <Choose
            fallback={
              <For each={rows()}>
                {(row, index) => (
                  <div
                    id={rowId(index())}
                    class={[
                      chrome.row,
                      index() === walking() ? chrome.walked : undefined,
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    role="option"
                    aria-selected={
                      !row.back && row.path === props.value ? "true" : "false"
                    }
                    onClick={() => take(row)}
                  >
                    {/* Which way the row goes, on the side it goes: the way
                        back in front of its words, and the way in at the far
                        end of them. */}
                    <Show when={row.back}>
                      <span class={styles.back} aria-hidden="true">
                        ‹
                      </span>
                    </Show>
                    <span class={chrome.words}>{row.label}</span>
                    {/* And the row that is what this field is looking for,
                        which says so in a word rather than a mark: it is the
                        one row here that goes nowhere, and a screen reader
                        reading the list should hear which that was. */}
                    <Show when={row.found}>
                      <span class={styles.repository}>repository</span>
                    </Show>
                    <Show when={!row.back && !row.found}>
                      <span class={chrome.arrow} aria-hidden="true">
                        ›
                      </span>
                    </Show>
                  </div>
                )}
              </For>
            }
          >
            {/* The read itself failing is the one thing here that is not the
                server's answer about a path, so it is the one line drawn as a
                failure. */}
            <Match when={listing.isError}>
              <ErrorLine class={styles.line}>
                Could not read that directory: {listing.error?.message}
              </ErrorLine>
            </Match>
            <Match when={listing.data === undefined}>
              <Empty class={styles.line}>Loading…</Empty>
            </Match>
            <Match when={said()}>
              {(why) => <Empty class={styles.line}>{why()}</Empty>}
            </Match>
            <Match when={rows().length === 0}>
              <Empty class={styles.line}>Nothing here.</Empty>
            </Match>
          </Choose>
        </div>
      </Show>
    </div>
  );
}
