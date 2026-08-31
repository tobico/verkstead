//! Picking one of the server's rows out of a dropdown: the two controls a
//! choice is offered through here, and the divergence neither of them lets
//! happen.
//!
//! What a `<select>` shows is a property of the element and not of its markup,
//! and the browser keeps it plausible on its own: take away the option that was
//! selected and the visible selection snaps to the first one left. Nothing is
//! said about it. The signal behind the control still holds the old choice, and
//! Solid never re-applies `value`, because the string it was given did not
//! change — so the human reads one repository off the screen and the form sends
//! another.
//!
//! The merge of ADR-0009 keeps the option elements alive whenever the list came
//! back the same, which is nearly every re-read. "Nearly" is where a bug class
//! lives, and a wrong repository is not a class to leave one in: the divergence
//! is closed here instead, for every re-read and not just the quiet ones.
//!
//! ## The guarantees, which are the module's whole point
//!
//! Both controls hold all four, and hold them off the one set of readings in
//! [`showing`] rather than each its own:
//!
//! - **What is shown and what would be sent are the same string**, always, and
//!   however the option list is rebuilt underneath the choice.
//! - **A chosen row that is no longer among the options falls to the
//!   placeholder**, and says so through the optional `gone` callback. Nothing
//!   unpicks itself, and no choice is quietly moved to the row that happens to
//!   be left.
//! - **A caller-supplied row that sends the empty string is a choice** rather
//!   than a placeholder: "Not chosen" is drawn only where nothing is chosen and
//!   the caller offers no such row.
//! - **The disabled state, and the label-by-id contract**: a `<label for=…>`
//!   reaches either control. Which is why the listbox's own is a `button` — that
//!   is the labelable element a `div` with a role on it is not.
//!
//! ## Which control, and why there are two
//!
//! [`Picker`] is the native `<select>`, and it is what an ordinary choice is
//! still offered through: a repository, a base branch, a merge strategy. Two
//! things give it the guarantees above. The displayed value is re-applied after
//! the options are rebuilt, every time they are rebuilt, so a browser's
//! fixing-up is undone before anybody sees it. And a choice that is no longer
//! among the options is not shown as one: the control falls to its placeholder,
//! which is the honest reading of *what you picked is gone*.
//!
//! [`Listbox`] is the same choice drawn out of ordinary elements, for the rows
//! that carry a harness mark beside their words: an `<option>` holds text and
//! nothing else, in every browser, which is the whole reason there is a second
//! control here at all. It is the four pairing pickers and the profile form's
//! harness type, and no other choice in the app — a control the app draws itself
//! has to be given back everything the native one arrived with, which is the
//! keyboard, the roles a screen reader reads it by and a row a finger can hit,
//! so it earns its keep only where a row has something of its own to draw.

import {
  For,
  Show,
  createEffect,
  createSignal,
  createUniqueId,
  type JSX,
} from "solid-js";

import { HarnessMark } from "./HarnessMark";
import type { AgentType } from "./agents";
import styles from "./picking.module.css";

/// What either control says where nothing is chosen.
///
/// The same words on both, because it is the same state on both: a choice has
/// not been made, or the one that was made is gone. A picker whose reader would
/// need to be told something else can take the words as a prop then.
const NOTHING = "Not chosen";

/// What either control is given: the caller's rows, and the two functions that
/// read them.
///
/// Both functions because nothing here knows what a row is — the repos, the
/// profiles, the pairings and whatever comes next carry different fields and the
/// same problem.
type Choosing<T> = {
  /// The control's own id, for the `<label>` that names it.
  id: string;
  /// Every row that can be picked, as the last read of the list had them.
  options: T[];
  /// What one row sends when it is the choice.
  value: (option: T) => string;
  /// What one row reads as.
  label: (option: T) => string;
  /// What is chosen now, as `value` would have written it — the empty string
  /// for nothing chosen.
  chosen: string;
  /// What the human just picked. Never the empty string, unless the caller
  /// offered one: the placeholder is a state to be in and not a choice to make,
  /// but an option of the caller's own that sends nothing is a choice like any
  /// other — the base dropdown's first entry, which is the rule to go back to.
  pick: (value: string) => void;
  /// Said when what was chosen is no longer among the options, for the caller
  /// that holds the choice in a signal of its own to clear it.
  ///
  /// Optional because not every choice is the page's to reset: the profile
  /// pickers show what the *server* says the conversation chose, and a viewer
  /// that unpicked it on its own would be arguing with the record. Falling to
  /// the placeholder is the whole of the correction there, and the conversation
  /// read back afterwards is what settles it.
  gone?: () => void;
  disabled?: boolean;
};

/// What a control is showing, off the rows it was given and the choice it was
/// handed.
///
/// Shared by both controls rather than written twice, because the guarantees at
/// the head of this file are these four readings: two implementations of them
/// would be two things to be wrong, and the native one's bug class is exactly
/// what happens when a control decides for itself what it is showing.
function showing<T>(props: {
  options: T[];
  value: (option: T) => string;
  chosen: string;
}): {
  offered: () => string[];
  offersNothing: () => boolean;
  standing: () => boolean;
  shown: () => string;
} {
  /// What each option would send, in the order they are drawn.
  const offered = (): string[] => props.options.map(props.value);

  /// Whether the caller's own list holds an option that sends nothing.
  ///
  /// Where it does, the empty string is one of the choices rather than the
  /// absence of one: no placeholder is drawn over it, and picking it is a pick
  /// like any other. Read off the options rather than taken as a flag, because
  /// it is the same fact either way and two ways of saying it could disagree.
  const offersNothing = (): boolean => offered().includes("");

  /// Whether what is chosen is still something that can be picked.
  const standing = (): boolean => offered().includes(props.chosen);

  /// What the control shows: the choice for as long as it exists, and the
  /// placeholder the moment it does not.
  const shown = (): string => (standing() ? props.chosen : "");

  return { offered, offersNothing, standing, shown };
}

/// And the telling: that what was chosen is gone, said to whoever owns the
/// choice.
///
/// Its own effect so that nothing about what a control shows depends on anybody
/// acting on it — the control is honest either way, and this is the caller's
/// chance to be.
function telling(
  props: { chosen: string; gone?: () => void },
  standing: () => boolean,
): void {
  createEffect(() => {
    if (props.chosen !== "" && !standing()) {
      props.gone?.();
    }
  });
}

/// One of the server's rows, in a native dropdown.
export function Picker<T>(props: Choosing<T>): JSX.Element {
  const { offered, offersNothing, standing, shown } = showing(props);

  // The element is built first and the effect made after it, because the order
  // they are created in is the order they run in. The options are rebuilt by a
  // computation this expression makes, so an effect made before it would be
  // re-applying the value to the option set that is on its way out.
  const select = (
    <select
      id={props.id}
      disabled={props.disabled}
      onChange={(ev) => {
        const picked = ev.currentTarget.value;
        if (picked || offersNothing()) {
          props.pick(picked);
        }
      }}
    >
      {/* Drawn only while it is what the control is showing, so a settled
          choice is not sitting next to an invitation to unmake it — there is
          no unpicking here, on any of the pickers.

          The same words on all of them, because it is the same state on all of
          them: a choice has not been made, or the one that was made is gone.
          A picker whose reader would need to be told something else can take
          the words as a prop then.

          Nothing at all where the caller offers an option of its own that sends
          nothing: that option is what the empty string means there, and a
          placeholder over it would be two rows for one state. */}
      <Show when={shown() === "" && !offersNothing()}>
        <option value="">{NOTHING}</option>
      </Show>
      <For each={props.options}>
        {(option) => (
          <option value={props.value(option)}>{props.label(option)}</option>
        )}
      </For>
    </select>
  ) as HTMLSelectElement;

  // The re-apply. Reading the values is what makes this run again when the
  // option set moves, which is exactly when the browser will have chosen for
  // itself.
  createEffect(() => {
    offered();
    select.value = shown();
  });

  telling(props, standing);

  return select;
}

/// The same choice, drawn out of ordinary elements so that every row can carry
/// its harness's mark.
///
/// A `button` with a listbox under it, which is the ARIA combobox pattern: the
/// focus stays on the button and `aria-activedescendant` says which row the
/// keyboard is on, so a screen reader announces the rows and the choice without
/// the focus ever leaving the control the label names.
///
/// Everything a native dropdown arrived with is given back here, because a
/// control that lost any of it would be a worse control drawn better: the whole
/// keyboard (Enter, Space or either arrow opens it; the arrows and Home/End walk
/// it; Enter picks; Escape closes), a press away from it to dismiss it, and rows
/// a finger can hit on the phone the workbench is answered from.
export function Listbox<T>(
  props: Choosing<T> & {
    /// Which harness's mark goes in front of a row's words, and `null` for a
    /// row that has none — the two rows that are not accounts at all.
    ///
    /// Optional so that the reading is the whole of a row by default, and drawn
    /// here rather than by each caller because the space between a mark and the
    /// words it belongs to is the same space in all five pickers.
    mark?: (option: T) => AgentType | null;

    /// The anchor's own class, for the caller with the field around it to lay
    /// out. What the control *looks* like is this module's, all five being one
    /// control in five places.
    class?: string;
  },
): JSX.Element {
  const { offered, standing, shown } = showing(props);

  /// Whether the rows are down.
  const [open, setOpen] = createSignal(false);

  /// And which of them the keyboard is on while they are: an index into the
  /// options, and the one thing a native dropdown kept for itself that this has
  /// to keep for it.
  const [walked, setWalked] = createSignal(0);

  /// Which way they come down: under the control where there is room for them
  /// there, and over it where there is not.
  ///
  /// The other thing a native dropdown kept for itself. Its popup is the
  /// browser's own and is put wherever it fits on the screen; these rows are an
  /// element of the page, inside whatever clips the page — a pane is
  /// `overflow-y: auto` from the second breakpoint up, and the steer modal's
  /// card is capped at `80vh` — so a control standing low in one of those would
  /// drop its rows past its edge and out of sight, behind a backdrop that draws
  /// nothing to say where they went.
  const [above, setAbove] = createSignal(false);

  /// The same index, held inside the list as it stands now.
  ///
  /// A Nudge can take a row away while the rows are down — a Profile deleted
  /// from another window is exactly that — and a walk left past the end would
  /// name a row the page no longer holds, which is a screen reader sent nowhere.
  /// Clamped on the way out rather than corrected on the way in, so there is one
  /// answer to *which row is the keyboard on* however the list moved.
  const walking = (): number => Math.min(walked(), Math.max(0, last()));

  /// Which row the choice is, or `-1` where nothing is chosen — read off
  /// [`showing`]'s own reading, so the row the control draws is the row it would
  /// send.
  const at = (): number => offered().indexOf(shown());

  /// And the row itself, for the closed control to draw.
  const picked = (): T | undefined => props.options[at()];

  // The rows' own ids, for the `aria-activedescendant` that says which one the
  // keyboard is on. Generated rather than made out of the control's id, because
  // one page can hold four of these and an id is the page's to keep unique.
  const list = createUniqueId();
  const rowId = (index: number): string => `${list}-${index}`;

  // The control, so that a press on a row — which is a press on something no
  // browser will focus — hands the keyboard back where it was.
  let control!: HTMLButtonElement;

  // And the rows, for the one measure that says which way they hang. Held as
  // `undefined` as well, unlike the control: they are on the page only while
  // they are down.
  let dropped: HTMLDivElement | undefined;

  /// Drop the rows, with the keyboard on the row that is the choice: a walk
  /// starts where the human already is rather than at the top of a list they
  /// have answered once.
  const drop = (): void => {
    setWalked(Math.max(0, at()));
    setOpen(true);
  };

  /// Take the rows back, and the keyboard with them.
  const shut = (): void => {
    setOpen(false);
    control.focus();
  };

  /// Take one row.
  ///
  /// The pick goes up exactly as the `<select>`'s does, and with nothing to
  /// guard: a row that sends the empty string is one of the caller's own and so
  /// is a choice like any other, and there is no placeholder row here to be
  /// mistaken for one — the placeholder is something the closed control *says*
  /// rather than a row of the list.
  const take = (index: number): void => {
    const option = props.options[index];
    shut();
    if (option) props.pick(props.value(option));
  };

  /// The last row, which is where End goes and where the walk stops.
  const last = (): number => props.options.length - 1;

  /// The whole keyboard, on the one element that holds the focus.
  const key = (ev: KeyboardEvent): void => {
    if (!open()) {
      // Every way into a native dropdown, and nothing else: a bare Enter or
      // Space on a closed one opens it rather than submitting whatever form it
      // is standing in.
      if (["Enter", " ", "ArrowDown", "ArrowUp"].includes(ev.key)) {
        ev.preventDefault();
        drop();
      }
      return;
    }

    switch (ev.key) {
      case "Escape":
        // Closed and nothing picked, which is what Escape means over a list.
        //
        // Prevented as well as handled, because one of these stands inside the
        // steer modal: a `dialog` closes itself on Escape, and a hand taking the
        // rows back is not asking for the modal under them to go too.
        ev.preventDefault();
        shut();
        break;
      case "Enter":
      case " ":
        ev.preventDefault();
        take(walking());
        break;
      case "ArrowDown":
        ev.preventDefault();
        setWalked(Math.min(walking() + 1, last()));
        break;
      case "ArrowUp":
        ev.preventDefault();
        setWalked(Math.max(walking() - 1, 0));
        break;
      case "Home":
        ev.preventDefault();
        setWalked(0);
        break;
      case "End":
        ev.preventDefault();
        setWalked(last());
        break;
      case "Tab":
        // Not ours to swallow: the hand is leaving the control, and the rows go
        // with it. Left to the browser rather than prevented, so the focus lands
        // wherever it was going.
        setOpen(false);
        break;
    }
  };

  // Which way the rows hang, measured each time they come down: what they need
  // against what is left under the control, inside whatever would clip them.
  //
  // Here rather than in [`drop`] because what is measured is the rows' own
  // height, which nothing knows until they are on the page — and an effect runs
  // after they are and before the browser paints, so the choice is made before
  // anybody has seen them anywhere. Back under the control as they go, so the
  // next measure starts from the ordinary way round rather than from the last
  // answer.
  createEffect(() => {
    if (!open()) {
      setAbove(false);
      return;
    }

    if (!dropped) return;

    const anchor = control.getBoundingClientRect();
    const wanted = dropped.getBoundingClientRect().height;
    const clip = clipping(control);

    // Over it only where they do not fit under it, and then only where there is
    // more room over it: the ordinary way round is the one to be in wherever
    // being in it costs nothing, and where neither side fits the rows go to
    // whichever side shows more of them. Which side that is changes nothing
    // about *which* rows — the list is capped and scrolls from its top either
    // way.
    setAbove(
      anchor.bottom + wanted > clip.bottom &&
        anchor.top - clip.top > clip.bottom - anchor.bottom,
    );
  });

  // The row the keyboard has walked to, kept in view: the drop is capped in
  // height, so a list longer than it can be walked past its own edge.
  createEffect(() => {
    if (!open()) return;

    // Reached by id rather than by a ref, the rows being a `For` that rebuilds
    // — and asked for rather than called, jsdom having no scrolling at all.
    document.getElementById(rowId(walking()))?.scrollIntoView?.({
      block: "nearest",
    });
  });

  telling(props, standing);

  return (
    <div class={[styles.listbox, props.class].filter(Boolean).join(" ")}>
      <button
        type="button"
        id={props.id}
        ref={control}
        class={styles.control}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open() ? "true" : "false"}
        aria-controls={open() ? list : undefined}
        aria-activedescendant={open() ? rowId(walking()) : undefined}
        disabled={props.disabled}
        onClick={() => (open() ? shut() : drop())}
        onKeyDown={key}
      >
        {/* What is chosen, drawn the way the row it came off is — a mark and
            words — or the placeholder, which is what this control says rather
            than a row it offers. */}
        <Show
          when={standing()}
          fallback={<span class={styles.words}>{NOTHING}</span>}
        >
          {/* A row for certain: `standing` is what says the choice is one of
              the options, which is what makes it one of them to draw. */}
          <Reading of={picked()!} mark={props.mark} label={props.label} />
        </Show>
        {/* Which way the rows come down, and no part of what the control
            says. */}
        <span class={styles.arrow} aria-hidden="true">
          ▾
        </span>
      </button>

      <Show when={open()}>
        {/* What a press away from the rows lands on, so that it lands on
            nothing else: a stray press that picked another pairing is not a
            small thing on this card. No wash over the page, unlike a menu's —
            this is a field being filled in rather than something opened over
            what the human was reading. */}
        <div
          class={styles.backdrop}
          aria-hidden="true"
          onClick={() => shut()}
        />
        <div
          ref={dropped}
          class={[styles.drop, above() ? styles.above : undefined]
            .filter(Boolean)
            .join(" ")}
          id={list}
          role="listbox"
        >
          <For each={props.options}>
            {(option, index) => (
              <div
                id={rowId(index())}
                class={[
                  styles.row,
                  index() === walking() ? styles.walked : undefined,
                ]
                  .filter(Boolean)
                  .join(" ")}
                role="option"
                aria-selected={
                  props.value(option) === shown() ? "true" : "false"
                }
                onClick={() => take(index())}
              >
                <Reading of={option} mark={props.mark} label={props.label} />
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}

/// The box the rows have to fit inside: the nearest thing above the control that
/// would clip them, and the window where nothing does.
///
/// Which is not the window as often as it looks. A pane scrolls its own content
/// from the second breakpoint up and the steer modal's card is capped at `80vh`,
/// so the edge the rows would disappear past is that box's rather than the
/// screen's — and it is met with the window all the same, a pane being able to
/// stand taller than the window it is scrolled inside.
///
/// `hidden` counts with `auto` and `scroll`: what matters here is that the box
/// clips, and one that clips without scrolling is the worse of the two to drop
/// rows into.
function clipping(from: Element): { top: number; bottom: number } {
  for (let at = from.parentElement; at; at = at.parentElement) {
    const { overflowY } = getComputedStyle(at);

    if (["auto", "scroll", "hidden"].includes(overflowY)) {
      const box = at.getBoundingClientRect();

      return {
        top: Math.max(box.top, 0),
        bottom: Math.min(box.bottom, window.innerHeight),
      };
    }
  }

  return { top: 0, bottom: window.innerHeight };
}

/// One row's reading: the harness's mark, and the words beside it.
///
/// Drawn by the list and again by the closed control, out of one component,
/// because a control showing one thing and offering the same thing drawn
/// differently is a control the eye has to check.
function Reading<T>(props: {
  of: T;
  mark?: (option: T) => AgentType | null;
  label: (option: T) => string;
}): JSX.Element {
  return (
    <>
      <HarnessMark of={props.mark?.(props.of) ?? null} />
      {/* The words in a span of their own, so the closed control can cut a
          reading too long for it and the rows under it never have to. */}
      <span class={styles.words}>{props.label(props.of)}</span>
    </>
  );
}
