//! The three-pane frame: the grid, the lines between the panes, and how wide
//! each of them stands.
//!
//! A hierarchy the human walks into and back out of — something is picked, then
//! read, then looked into — so a window wide enough stands the three levels
//! side by side and a phone shows one of them at a time. The same page answers
//! both.
//!
//! Which level is showing is `data-pane` on the frame, and the stylesheet is
//! what makes it mean anything: a wide window ignores it and draws all three.
//! The attribute rather than a rendered-or-not pane, because walking back out
//! should not throw away what the pane it came from had drawn. Which level it
//! is is the caller's — the workbench reads it off the URL and its selection,
//! and the frame has no way of knowing what either says.
//!
//! What stands *in* the panes is the caller's too. This draws the three
//! sections, names them, and puts a divider on every border there is; the
//! workbench hands it a Timeline where the settings page hands it its own root,
//! and neither is anything this file knows about. The middle pane is named for
//! its place for that reason.
//!
//! How wide they stand is the frame's, because a width is a property of the
//! frame rather than of anything drawn in it. They are percentages kept per
//! device (`widths.ts`) — one pair for the device rather than one per page —
//! and the dividers that set them exist only in the layouts that stand panes
//! side by side: below that breakpoint the page is walked through one pane at a
//! time, so there is no border to drag and nothing remembered is read.
//!
//! The floors under those percentages are lengths rather than shares, because
//! what makes a pane too narrow is what stands in it. So this is the one part
//! of the frame that measures itself: how wide it is in rem is what turns a
//! pane's minimum into a share the widths can be held against, and it is
//! measured again whenever the window changes shape under the panes.

import {
  Show,
  createSignal,
  onCleanup,
  onMount,
  type Accessor,
  type JSX,
} from "solid-js";

import styles from "./Panes.module.css";
import {
  ALL_THREE,
  BESIDE,
  DEFAULTS,
  clamped,
  dragged,
  nudged,
  range,
  remember,
  restore,
  widths as remembered,
  type Divider,
  type Frame,
  type Widths,
} from "./widths";

/// Which level of the hierarchy a narrow window is showing.
export type Pane = "conversations" | "middle" | "details";

/// Whether a media query holds, as something the page can be built out of.
///
/// The stylesheet answers the same question for itself; this is for the parts
/// of the layout that are the page's rather than the rules' — which dividers
/// exist, and whether this device's remembered widths are read at all. A
/// browser with no `matchMedia` to ask answers no to everything, which is the
/// narrow layout: one pane, no dividers, nothing read.
function matching(query: string): Accessor<boolean> {
  if (typeof window.matchMedia !== "function") {
    return () => false;
  }

  const asked = window.matchMedia(query);
  const [holds, setHolds] = createSignal(asked.matches);
  const changed = () => setHolds(asked.matches);

  asked.addEventListener?.("change", changed);
  onCleanup(() => asked.removeEventListener?.("change", changed));

  return holds;
}

/// The line between two panes, and the handle that moves it.
///
/// A separator rather than a button, because that is what it is: the thing it
/// does to the page is not an action but a value, and the value is the share of
/// the frame the pane on its left is worth. Which is what the arrow keys move
/// it by, for the pointer nobody dragging with a keyboard has.
function Handle(props: {
  divider: Divider;
  label: string;
  share: number;
  travel: { least: number; most: number };
  drag: (divider: Divider, event: PointerEvent) => void;
  nudge: (divider: Divider, by: number) => void;
  restore: () => void;
}): JSX.Element {
  return (
    <div
      class={styles.divider}
      role="separator"
      aria-orientation="vertical"
      aria-label={props.label}
      aria-valuenow={Math.round(props.share)}
      aria-valuemin={Math.round(props.travel.least)}
      aria-valuemax={Math.round(props.travel.most)}
      tabindex="0"
      onPointerDown={(event) => props.drag(props.divider, event)}
      onDblClick={() => props.restore()}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") {
          event.preventDefault();
          props.nudge(props.divider, -1);
        }
        if (event.key === "ArrowRight") {
          event.preventDefault();
          props.nudge(props.divider, 1);
        }
      }}
    />
  );
}

export function Panes(props: {
  /// Which level a narrow window is showing. The caller's, because what says
  /// which level the page stands at is the page's own URL and selection.
  pane: Pane;

  /// The three panes, in the order they are walked through.
  conversations: JSX.Element;
  middle: JSX.Element;
  details: JSX.Element;

  /// What the middle pane holds, which is what it is called and what the
  /// divider beside it says it moves: the workbench's Timeline, the settings.
  /// The other two are the same thing on every page and name themselves.
  middleLabel: string;
}): JSX.Element {
  /// Which layout is standing, which decides how many dividers there are and
  /// how much room each pane is allowed to leave the others.
  const beside = matching(BESIDE);
  const allThree = matching(ALL_THREE);

  /// How wide the panes stand. Read off this device once, and written back when
  /// a drag is let go of rather than on the way — a width settled on is worth
  /// remembering, and the hundred it passed through on the way there are not.
  const [settled, setSettled] = createSignal<Widths>(remembered());

  /// The frame the shares are shares *of* — a divider dropped at a point on the
  /// screen means nothing until it is measured against this.
  let element!: HTMLDivElement;

  /// How wide the frame stands, in rem.
  ///
  /// Which the arithmetic needs because the minimums under these widths are
  /// lengths: what a pane is owed is the same span of paper on every window,
  /// and turning that into a share of the frame takes the frame's own width.
  /// Nought until the page has been laid out, which `widths.ts` reads as no
  /// floors yet rather than as floors of nothing.
  ///
  /// In rem rather than in pixels, and converted here where the browser can be
  /// asked what a rem is: a human who has told their browser to draw text
  /// larger has said the panes should hold what they held, which is what makes
  /// this the right unit for a floor and the wrong one for a width.
  const [across, setAcross] = createSignal(0);

  const measure = (width = element.getBoundingClientRect().width) => {
    const root =
      parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

    if (width > 0) {
      setAcross(width / root);
    }
  };

  /// Measured when the frame is first drawn, and again whenever it changes
  /// shape under the panes — a window dragged narrower is exactly when a pane
  /// stops being wide enough for what it holds. A browser with no
  /// `ResizeObserver` to ask keeps the width it opened at, and re-measures at
  /// the start of every drag.
  onMount(() => {
    measure();

    if (typeof ResizeObserver !== "function") {
      return;
    }

    const watching = new ResizeObserver(() => measure());

    watching.observe(element);
    onCleanup(() => watching.disconnect());
  });

  /// The frame as the arithmetic asks about it: how much room there is, and how
  /// many panes are sharing it.
  const frame = (): Frame => ({ rem: across(), three: allThree() });

  /// And the widths as they may actually be drawn: a sidebar dragged wide in
  /// the two-pane layout is not allowed to squeeze the middle pane out of the
  /// three-pane one, so the minimums are met against the frame in front of the
  /// human rather than against the one the width was settled in.
  const shown = () => clamped(settled(), frame());

  /// Dragging one. The listeners go on the window rather than on the handle,
  /// because a pointer that has outrun the handle — which every drag's does —
  /// is still dragging it.
  const drag = (divider: Divider, event: PointerEvent) => {
    // Which stops the drag selecting the text of both panes on the way past.
    event.preventDefault();

    const frameRect = element.getBoundingClientRect();
    if (frameRect.width === 0) {
      return;
    }

    // Free, the rect being in hand: it keeps a browser with no observer to ask
    // from meeting the floors against a frame the window has been resized out
    // from under.
    measure(frameRect.width);

    const moved = (at: PointerEvent) => {
      const share = ((at.clientX - frameRect.left) / frameRect.width) * 100;
      setSettled((was) => dragged(was, divider, share, frame()));
    };

    const dropped = () => {
      window.removeEventListener("pointermove", moved);
      window.removeEventListener("pointerup", dropped);
      window.removeEventListener("pointercancel", dropped);
      remember(settled());
    };

    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", dropped);
    window.addEventListener("pointercancel", dropped);
  };

  /// Moving one with the keyboard, which settles at once: there is no letting
  /// go of an arrow key.
  const nudge = (divider: Divider, by: number) => {
    setSettled((was) => nudged(was, divider, by, frame()));
    remember(settled());
  };

  /// And putting them back, which is what a double-click on either divider
  /// does: both widths, because what it restores is the defaults rather than
  /// one of them.
  const defaults = () => {
    restore();
    setSettled(DEFAULTS);
  };

  /// What the frame carries the widths as. Only where a layout stands panes
  /// side by side: below that the page is walked through one pane at a time,
  /// and what this device remembers about a desktop's columns is nothing a
  /// phone should be reading. The stylesheet has a default of its own behind
  /// each name, so an absent pair is the untouched frame rather than a broken
  /// one.
  const columns = () =>
    beside()
      ? {
          "--pane-sidebar": `${shown().sidebar}%`,
          "--pane-middle": `${shown().middle}%`,
        }
      : undefined;

  return (
    <div
      class={styles.panes}
      data-pane={props.pane}
      ref={element}
      style={columns()}
    >
      <section
        class={`${styles.pane} ${styles.conversationsPane}`}
        aria-label="Conversations"
      >
        {props.conversations}
      </section>

      {/* One divider per border there is: the sidebar's wherever the sidebar
          stands beside something, the middle pane's only where all three panes
          are up. Each sits between the two panes it parts, so the grid places
          it without being told where. */}
      <Show when={beside()}>
        <Handle
          divider="sidebar"
          label="Resize the conversations pane"
          share={shown().sidebar}
          travel={range("sidebar", shown(), frame())}
          drag={drag}
          nudge={nudge}
          restore={defaults}
        />
      </Show>

      <section
        class={`${styles.pane} ${styles.middlePane}`}
        aria-label={props.middleLabel}
      >
        {props.middle}
      </section>

      <Show when={allThree()}>
        <Handle
          divider="middle"
          label={`Resize the ${props.middleLabel.toLowerCase()} pane`}
          share={shown().middle}
          travel={range("middle", shown(), frame())}
          drag={drag}
          nudge={nudge}
          restore={defaults}
        />
      </Show>

      <section
        class={`${styles.pane} ${styles.detailsPane}`}
        aria-label="Details"
      >
        {props.details}
      </section>
    </div>
  );
}

/// The block a pane keeps against its top edge while the pane scrolls under
/// it: the pane's header, and whatever else is meant to stay with it — the
/// timeline hands in its pinned items as well.
///
/// The `paneChrome` name is the frame's, and what it means to a pane is said
/// once in `Panes.module.css`: stuck to the top, paper out past the pane's own
/// padding, and a rem of fade under it that the record thins out into. This is
/// the component that wears the name, so that a pane hands its chrome in rather
/// than knowing how a pane sticks one.
///
/// And it says how tall it stands, as `--pane-chrome` on the pane it is in.
/// Anything else in the pane that pins itself to the top edge has to pin below
/// this — the table of contents in a Set does — and how tall a header stands is
/// a question of what is in it and how wide the human left the pane, which is
/// nothing a stylesheet can be told in advance. Written on the pane rather than
/// here so that it reaches the whole of what the pane holds; measured again
/// whenever the block changes shape, a title that has wrapped included.
///
/// And whether it is stuck yet, as `data-stuck`, which is what the fade under
/// it is drawn by: there is no gap below the block for a fade to hang in, so a
/// pane at rest would be wearing a rem of paper over its first line.
///
/// Watched rather than listened for. A pane scrolls two ways — the page below
/// the first breakpoint, the pane itself above it — and a scroll listener would
/// have to be told which, and be moved from one to the other as the human
/// drags the window across the breakpoint. What both come to is the same thing
/// seen from the rem of pane above the block: the page carries it off the top of
/// the window, or the pane clips it away, and an observer of that rem reads
/// either as the one answer it is.
export function PaneSticky(props: { children?: JSX.Element }): JSX.Element {
  let element!: HTMLDivElement;
  let edge!: HTMLDivElement;

  /// Whether the record is passing under the block. Not until something says
  /// so: a browser with no observer to ask draws no fade, the fade being about
  /// a passing it cannot see.
  const [stuck, setStuck] = createSignal(false);

  onMount(() => {
    const pane = element.closest<HTMLElement>(`.${styles.pane}`);

    if (pane === null) {
      return;
    }

    const measure = () =>
      pane.style.setProperty("--pane-chrome", `${element.offsetHeight}px`);

    measure();
    onCleanup(() => pane.style.removeProperty("--pane-chrome"));

    // A browser with no observer to be had keeps the height the block opened
    // at, which is the right one until something in it wraps.
    if (typeof ResizeObserver === "function") {
      const watching = new ResizeObserver(() => measure());

      watching.observe(element);
      onCleanup(() => watching.disconnect());
    }

    if (typeof IntersectionObserver !== "function") {
      return;
    }

    const watching = new IntersectionObserver(([seen]) => {
      setStuck(seen !== undefined && !seen.isIntersecting);
    });

    watching.observe(edge);
    onCleanup(() => watching.disconnect());
  });

  return (
    <>
      <div class={styles.paneEdge} ref={edge} />
      <div
        class={styles.paneChrome}
        data-stuck={stuck() ? "" : undefined}
        ref={element}
      >
        {props.children}
      </div>
    </>
  );
}
