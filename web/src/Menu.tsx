//! The one dropdown menu in the UI: a trigger, and what it drops over the page.
//!
//! There were three of these, built three ways — a native `details`/`summary`
//! on the conversation header, and two hand-rolled ones that had each grown
//! their own backdrop, their own Escape handler and their own idea of how far a
//! menu should stand off the page. They drifted, as three copies of one thing
//! do: only two of them cast a shadow at all, and only one of them gave the
//! focus back to the button it came from.
//!
//! So the chrome is here and the contents stay with the caller. What a menu is
//! — where it hangs, what a press away from it lands on, what Escape does and
//! where the focus goes afterwards — is the same wherever it is dropped, and
//! nothing that opens one has to get it right again.
//!
//! Every row of it is the caller's, though, because no two of these menus hold
//! the same kind of thing: a list of repositories, one destructive button, one
//! way to close a Set. What the caller is handed is the way to shut the menu,
//! which is the one thing it cannot work out for itself — and which it needs
//! wherever a press has done its work, be that in the row itself or in the
//! answer that came back from it.
//!
//! There are two shapes of it and one set of chrome. [`Menu`] is the ordinary
//! one: a button, and what it drops under itself. [`ContextMenu`] is the same
//! card opened by a right-click and put where the pointer was — no trigger, no
//! anchor, and nowhere to give the focus back to. Every other thing a menu is,
//! the two of them share.
//!
//! A card can hold more than one level of rows: a [`Nested`] row opens another
//! level of the menu it is in, with a way back out at the top of it. One card,
//! one backdrop, one Escape and one focus given back however deep it is — which
//! is what makes it a level of this menu rather than a second menu, and what
//! the flat alternative would have cost on a page with a dozen repositories to
//! list.

import {
  Show,
  createContext,
  createSignal,
  createUniqueId,
  onCleanup,
  onMount,
  useContext,
} from "solid-js";
import type { JSX } from "solid-js";

import styles from "./Menu.module.css";

/// A level of a menu below its first: what the row that opens it reads as, and
/// the rows it holds.
type Level = {
  label: string;
  children: () => JSX.Element;
};

/// How a [`Nested`] row reaches the menu it is drawn inside of.
///
/// A context rather than a prop, because a nested row is written where the rows
/// are — inside the caller's own thunk — and threading the way to open a level
/// down through whatever the caller has built there would be the caller's
/// problem rather than this component's.
const Levels = createContext<{
  /// Show this level instead of the one the card is on.
  open: (level: Level) => void;

  /// Whether this row is the one the human has just stepped back out of, asked
  /// once: the row that opened a level is where the focus goes when the level
  /// is left, and it is a new element by then.
  returning: (label: string) => boolean;
}>();

/// Take the focus to a row that has just been built.
///
/// In a microtask because focusing an element the document does not hold yet
/// does nothing at all: a row is built here and put in the page after.
function focusing(row: HTMLElement): void {
  queueMicrotask(() => {
    if (row.isConnected) row.focus();
  });
}

/// A menu, and the button that drops it.
export function Menu(props: {
  /// Which menu this is, put on the anchor so the caller can paint this one's
  /// trigger and size its drop. The shared chrome is `Menu.module.css` — the
  /// anchor, the trigger, the backdrop and the drop underneath it, and the ⋯ a
  /// `mark` menu is drawn as.
  class: string;
  /// What the trigger reads as. Whatever the caller would have put inside its
  /// own button — a word, a badge, a mark. A pane's ⋯ passes none: that trigger
  /// is drawn here, being the same button in both places there is one.
  trigger?: JSX.Element;
  /// Whether this is the ⋯ at the head of a workbench pane. The mark and the
  /// paint under it are this component's rather than the caller's, so the
  /// sidebar's and the Conversation's render as one button rather than as two
  /// rules that were written apart and drifted.
  mark?: boolean;
  /// What a screen reader calls the trigger, for a trigger whose contents are
  /// a mark rather than a word.
  label?: string;
  /// What a screen reader calls the drop, where the trigger's own name is not
  /// enough to tell one menu on the page from another.
  name?: string;
  /// Whether the trigger takes a press. A disabled trigger still says what it
  /// says — a badge with a locking in flight is the case this is for.
  disabled?: boolean;
  /// Said each time the menu is opened, for the caller that has something to
  /// reset before its rows are drawn again.
  opening?: () => void;
  /// Handed the way to shut this menu, once, as it is built.
  ///
  /// What a press that has done its work calls — and what a press that *failed*
  /// deliberately does not: a menu that shut on the way out would take the only
  /// place the failure had left to be said in.
  closer?: (close: () => void) => void;
  /// The rows, as a thunk: they are built when the menu opens and thrown away
  /// when it closes, so anything the caller wants standing while it is shut
  /// belongs outside it. A thunk rather than plain children because *built when
  /// it opens* is the whole of what a row that takes the focus depends on.
  ///
  /// A [`Nested`] among them is a row that opens another level of this same
  /// menu rather than doing something.
  children: () => JSX.Element;
}): JSX.Element {
  // `true` while the menu hangs open under the trigger.
  const [open, setOpen] = createSignal(false);

  props.closer?.(() => setOpen(false));

  // The drop's own id, for the `aria-controls` that ties it to the trigger.
  // Generated rather than named by the caller, because two of these can be on
  // one page and an id is the page's to keep unique. Only said while the menu is
  // open, because closed there is nothing of that id on the page to point at.
  const id = createUniqueId();

  // The trigger, so the keyboard's way out puts the focus back where it came
  // from rather than at the top of the page.
  let trigger!: HTMLButtonElement;

  // The way out that needs no aim: a menu drawn over the page has to be
  // dismissible from the keyboard. The other way — a press on the page — is the
  // backdrop's, so the press taking the menu back cannot also press something
  // underneath it. That one leaves the focus where the press put it, because a
  // hand that has moved on is not asking to be sent back.
  const escape = (ev: KeyboardEvent) => {
    if (ev.key === "Escape" && open()) {
      setOpen(false);
      trigger.focus();
    }
  };

  document.addEventListener("keydown", escape);
  onCleanup(() => document.removeEventListener("keydown", escape));

  return (
    <div class={`${styles.menu} ${props.class}`}>
      <button
        type="button"
        class={props.mark ? `${styles.trigger} ${styles.mark}` : styles.trigger}
        ref={trigger}
        aria-haspopup="menu"
        aria-expanded={open() ? "true" : "false"}
        aria-controls={open() ? id : undefined}
        aria-label={props.label}
        disabled={props.disabled}
        onClick={() => {
          if (!open()) props.opening?.();
          setOpen(!open());
        }}
      >
        {props.mark ? "⋯" : props.trigger}
      </button>

      <Show when={open()}>
        <div
          class={styles.backdrop}
          aria-hidden="true"
          onClick={() => setOpen(false)}
        />
        <div class={styles.drop} id={id} role="menu" aria-label={props.name}>
          <Rows rows={props.children} />
        </div>
      </Show>
    </div>
  );
}

/// What is inside the card: the caller's rows, or whichever level of them the
/// human has walked to.
///
/// The one part of a menu that is neither the chrome nor the caller's, and it
/// belongs to the drop rather than to either shape of menu — which is what
/// gives the right-click menu its levels without a line of its own. Built with
/// the card it is in and thrown away with it, so a menu that has been shut and
/// opened is at its first level again: a card that came back down where it was
/// left would be showing a level the human closed their way out of.
function Rows(props: { rows: () => JSX.Element }): JSX.Element {
  // Which level the card is showing, or `null` at the first of them. One card,
  // one backdrop and one way out however deep it is: what a level changes is
  // the rows inside, and nothing about what a menu is.
  const [level, setLevel] = createSignal<Level | null>(null);

  // The level just stepped back out of, held for exactly as long as it takes
  // the row that opened it to be built again — that row is where the focus
  // belongs, and it is a new element by the time it is there to take it.
  const [back, setBack] = createSignal<string | null>(null);

  return (
    <Levels.Provider
      value={{
        open: (opened) => setLevel(opened),
        returning: (label) => {
          if (back() !== label) return false;
          setBack(null);
          return true;
        },
      }}
    >
      {/* One level or the other, never both: a level is what the card is
          showing rather than something drawn beside what it was showing
          before. */}
      <Show when={level()} fallback={props.rows()}>
        {(inner) => (
          <>
            {/* The way back out, at the top of the level it leaves — and
                where the focus goes as the level opens, so a hand on the
                keyboard is inside the card rather than at the top of the
                page. */}
            <button
              type="button"
              role="menuitem"
              class={styles.back}
              ref={focusing}
              onClick={() => {
                setBack(inner().label);
                setLevel(null);
              }}
            >
              <span aria-hidden="true">←</span>
              {inner().label}
            </button>
            {inner().children()}
          </>
        )}
      </Show>
    </Levels.Provider>
  );
}

/// A row that opens another level of the menu it is in, rather than doing
/// something.
///
/// The one thing a nested level is *not* is another menu: it comes down in the
/// same card, over the same backdrop, and goes back the same two ways —
/// Escape, or a press away from it — which is the whole reason there is one
/// menu component at all. What it adds is a third way, which belongs to the
/// level alone: the row back out of it, at the top of the rows it holds.
///
/// The rows are a thunk for [`Menu`]'s reason, and it matters more here: they
/// are built when the level is opened rather than when the menu is, so a level
/// listing something the caller is still reading is a level that reads it when
/// the human asks for it.
///
/// Drawn inside a [`Menu`] or a [`ContextMenu`] and nowhere else — outside one
/// there is no card for a level to come down in, and the row does nothing.
export function Nested(props: {
  /// What the row reads as, and what the way back out of the level it opens
  /// says. One word for both, because they are the same place named twice.
  label: string;

  /// What the level holds.
  children: () => JSX.Element;
}): JSX.Element {
  const levels = useContext(Levels);

  return (
    <button
      type="button"
      role="menuitem"
      class={styles.nested}
      aria-haspopup="menu"
      ref={(row) => {
        if (levels?.returning(props.label)) focusing(row);
      }}
      onClick={() =>
        levels?.open({ label: props.label, children: props.children })
      }
    >
      {props.label}
      {/* Which way the row goes, and no part of what it says. */}
      <span aria-hidden="true">›</span>
    </button>
  );
}

/// The same menu, opened by a right-click rather than by a button of its own.
///
/// A context menu is the one dropdown with nothing to hang from: what it is
/// about is whatever the pointer is over, and where it belongs is where the
/// pointer was. So the trigger goes, the anchor it hung from goes, and so does
/// giving the focus back to a button there never was. What is left is
/// everything else a menu is — the wash over the page, the card it comes down
/// as, the way out that needs no aim — and none of it is written twice.
///
/// A pointer affordance and only that. A touch device has no right-click, and a
/// long press there already means something else, so nothing here is reachable
/// from one.
///
/// Whether it is open is the caller's, because *which* of its rows was
/// right-clicked is the caller's: one of these per row would be a component
/// held open by a page that only ever wants one of them.
export function ContextMenu(props: {
  /// Which menu this is, put on the anchor so the caller can size and paint
  /// what it drops. As [`Menu`]'s, and by the same rules.
  class: string;
  /// What a screen reader calls the drop.
  name?: string;
  /// Where on the window the pointer was, or `null` while nothing is open.
  at: { x: number; y: number } | null;
  /// Said whenever the menu should go: Escape, a press away from it, or a row
  /// of the caller's that has done its work.
  close: () => void;
  /// The rows, as a thunk — built when it opens and thrown away when it closes,
  /// exactly as [`Menu`]'s are.
  children: () => JSX.Element;
}): JSX.Element {
  // The keyboard's way out. The other one — a press on the page — is the
  // backdrop's, as it is for every menu.
  const escape = (ev: KeyboardEvent) => {
    if (ev.key === "Escape" && props.at) props.close();
  };

  document.addEventListener("keydown", escape);
  onCleanup(() => document.removeEventListener("keydown", escape));

  return (
    <div class={`${styles.menu} ${props.class}`}>
      {/* Keyed, so a menu opened somewhere else is a new menu: the rows are
          built on the way open like every other menu's, and where it hangs is
          settled once, as it is placed. */}
      <Show when={props.at} keyed>
        {(at) => (
          <>
            <div
              class={styles.backdrop}
              aria-hidden="true"
              onClick={() => props.close()}
              // A second right-click is a press away from this menu like any
              // other, and the browser's own menu is not what it is asking for:
              // the hand is aiming at another card, and what it gets is this
              // one taken back.
              onContextMenu={(event) => {
                event.preventDefault();
                props.close();
              }}
            />
            <Pointed at={at} name={props.name} rows={props.children} />
          </>
        )}
      </Show>
    </div>
  );
}

/// The drop, put where the pointer was and kept inside the window.
///
/// Fixed to the window rather than positioned against anything on the page,
/// because the coordinates a pointer event carries are the window's own.
///
/// Placed at the pointer on the first paint, there being nothing to measure
/// yet, and inside the window's edges the moment there is — and again whenever
/// what it is holding changes size, which is not a rare case: a menu whose rows
/// are still being read is a line of text tall, and the rows that land in its
/// place are not. A right-click low down the sidebar would otherwise drop a menu
/// mostly below the fold, on a page that does not scroll to reach it.
function Pointed(props: {
  at: { x: number; y: number };
  name?: string;
  rows: () => JSX.Element;
}): JSX.Element {
  let drop!: HTMLDivElement;

  const [put, setPut] = createSignal(props.at);

  onMount(() => {
    const place = () => {
      const box = drop.getBoundingClientRect();

      setPut({
        x: Math.max(
          EDGE,
          Math.min(props.at.x, window.innerWidth - box.width - EDGE),
        ),
        y: Math.max(
          EDGE,
          Math.min(props.at.y, window.innerHeight - box.height - EDGE),
        ),
      });
    };

    place();

    // And again each time the card changes size. Guarded rather than assumed:
    // the test environment lays nothing out and has no observer to give, and a
    // menu placed once is what it had before this line anyway.
    if (typeof ResizeObserver === "undefined") return;

    const watching = new ResizeObserver(place);
    watching.observe(drop);
    onCleanup(() => watching.disconnect());
  });

  return (
    <div
      ref={drop}
      class={`${styles.drop} ${styles.pointed}`}
      role="menu"
      aria-label={props.name}
      style={{ left: `${put().x}px`, top: `${put().y}px` }}
    >
      <Rows rows={props.rows} />
    </div>
  );
}

/// How close to the window's edge a context menu may be put, in pixels. Enough
/// that the card's shadow has somewhere to fall.
const EDGE = 8;
