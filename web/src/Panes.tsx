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

import {
  Show,
  createSignal,
  onCleanup,
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

  /// And as they may actually be drawn: a sidebar dragged wide in the two-pane
  /// layout is not allowed to squeeze the middle pane out of the three-pane
  /// one, so the minimums are met against the layout in front of the human
  /// rather than against the one the width was settled in.
  const shown = () => clamped(settled(), allThree());

  /// The frame the shares are shares *of* — a divider dropped at a point on the
  /// screen means nothing until it is measured against this.
  let frame!: HTMLDivElement;

  /// Dragging one. The listeners go on the window rather than on the handle,
  /// because a pointer that has outrun the handle — which every drag's does —
  /// is still dragging it.
  const drag = (divider: Divider, event: PointerEvent) => {
    // Which stops the drag selecting the text of both panes on the way past.
    event.preventDefault();

    const frameRect = frame.getBoundingClientRect();
    if (frameRect.width === 0) {
      return;
    }

    const moved = (at: PointerEvent) => {
      const share = ((at.clientX - frameRect.left) / frameRect.width) * 100;
      setSettled((was) => dragged(was, divider, share, allThree()));
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
    setSettled((was) => nudged(was, divider, by, allThree()));
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
      ref={frame}
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
          travel={range("sidebar", shown(), allThree())}
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
          travel={range("middle", shown(), allThree())}
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
