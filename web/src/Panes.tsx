//! The three-pane frame: the grid, the lines between the panes, and how wide
//! each of them stands.
//!
//! A hierarchy the human walks into and back out of — something is picked, then
//! read, then looked into — so a window wide enough stands the three levels
//! side by side and a phone shows one of them at a time. The same page answers
//! both.
//!
//! Where there is a list to pick from it stands at every width, in one of three
//! bands. Under 60rem it is collapsed beside the one other level a phone shows;
//! from 60rem it is collapsed beside the timeline and the details; from 100rem
//! it is the full sidebar. Collapsed it is a strip of `COLLAPSED` rem, drawn at
//! the width it would stand at full and clipped down to the strip, so every
//! height in it is the height it has expanded; a mouse settling on it, a
//! keyboard focus inside it or a finger's tap on it expands it over the panes
//! beside it, which do not move — a finger's tap doing nothing else, and the
//! next one on a card or a link, or one anywhere outside, collapsing it again.
//!
//! So the list is never a level of its own: it is beside whatever the window
//! shows, and a narrow window walks the two levels after it, one at a time. The
//! root, where nothing is open, is neither of them — `pane` saying `nothing` —
//! and stands the list expanded over the empty paper until a card is picked.
//!
//! Which level is showing is `data-pane` on the frame, and the stylesheet is
//! what makes it mean anything: from 60rem it is ignored and every pane drawn.
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
//! Either of the outer two may be left out, and what is then drawn is two panes
//! with one border between them. A share has no list to pick from, so the first
//! goes; a Conversation whose record is the one Event has no Timeline worth
//! standing beside it, so the second does and the details take the room. The
//! hierarchy is the caller's to say, and a level with nothing in it is a level
//! the frame does not draw.
//!
//! Both at once is the third frame: the details pane by itself, across the whole
//! window. The compose page stands in it while there is nothing to list — no
//! list to pick from, and no record beside a Conversation that does not exist
//! yet — and it is the one frame with no border in it, so there is no divider on
//! it and no width of it to remember.
//!
//! How wide they stand is the frame's, because a width is a property of the
//! frame rather than of anything drawn in it. They are percentages kept per
//! device (`widths.ts`) — one set for the device rather than one per page, with
//! a width of its own for the frame that has no list in it — and the dividers
//! that set them exist only where they part two panes standing side by side:
//! below 60rem the page is walked through one pane at a time beside the
//! collapsed list, so there is no border to drag, and the only width read is
//! the sidebar's, for how wide the list expands to.
//!
//! The floors under those percentages are lengths rather than shares, because
//! what makes a pane too narrow is what stands in it. So this is the one part
//! of the frame that measures itself: how wide it is in rem is what turns a
//! pane's minimum into a share the widths can be held against, and it is
//! measured again whenever the window changes shape under the panes.
//!
//! And the last thing here that is the frame's rather than any pane's: keeping
//! its outermost heads clear of the window's own controls. The app's window has
//! no title bar, so the platform draws its controls over a corner of the page
//! (ADR-0020) — and which head is at that corner is a fact about the layout
//! rather than about a pane. The sidebar is leftmost where there is a list to
//! pick from — and its head drops below a left inset by the head's band rather
//! than being padded past it, a collapsed strip being narrower than a Mac's
//! traffic lights — the record is where there is none, the details pane is
//! rightmost always, and the one pane a narrow window shows beside the
//! collapsed list is rightmost too — and both at once where there is no list
//! beside it. Those are
//! the distinctions this file already makes for its columns and its dividers,
//! so the two insets arrive on the frame as variables the way the column widths
//! do and `Panes.module.css` hands each of them to the head that stands at that
//! edge. How much room the controls take at all is `controls.ts`'s, and outside
//! the app it is none.

import {
  Show,
  children,
  createSignal,
  onCleanup,
  onMount,
  type Accessor,
  type JSX,
} from "solid-js";

import { controls, reserved } from "./controls";
import styles from "./Panes.module.css";
import {
  BESIDE,
  FULL,
  clamped,
  dragged,
  expanded,
  nudged,
  range,
  remember,
  restore,
  widths as remembered,
  type Divider,
  type Frame,
  type Widths,
} from "./widths";

/// Which level of the hierarchy a narrow window is showing, or `nothing` where
/// nothing is open: the list is beside every level rather than one of them.
export type Pane = "nothing" | "middle" | "details";

/// How slow a mouse over the collapsed list has to be going, in CSS pixels a
/// millisecond, before it is taken as resting there and the list expands —
/// 250 a second. A hand reaching for the list slows to well under it before it
/// arrives; one flung off the window's edge crosses the strip at many times it.
/// Meant to be tuned, as the two after it are.
const SETTLED = 0.25;

/// And how long a mouse over it may send no move before it is taken as
/// stopped, in milliseconds: a pointer that stops dead says nothing more to be
/// timed, and a flung one is across the 3rem strip in well under this.
const STOPPED = 60;

/// How long the list stays expanded after a mouse has left it, in milliseconds,
/// for a hand that overshot its edge to come back inside.
const LINGER = 150;

/// Whether a media query holds, as something the page can be built out of.
///
/// The stylesheet answers the same question for itself; this is for the parts
/// of the layout that are the page's rather than the rules' — which dividers
/// exist, whether this device's remembered widths are read at all, and whether
/// the list is collapsed. A browser with no `matchMedia` to ask answers no to
/// everything, which is the narrow layout: one pane beside the collapsed list,
/// and no dividers.
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

  /// The three panes: the list, and the two levels walked through beside it.
  ///
  /// The first of them is absent in the one frame that has nothing to pick
  /// from: a share, which is one Conversation and no list of them. Then the
  /// frame is two panes — the record and what it opens — standing side by side
  /// from the width the sidebar used to arrive at, with one divider between
  /// them where the sidebar's and the middle pane's were. Nothing else changes:
  /// the panes are the same panes, drawn by the same components, walked through
  /// one at a time on a phone.
  ///
  /// The middle one is absent where the level it stands for has nothing to
  /// show: a Conversation whose record is the one Event has no Timeline worth
  /// a column, so the workbench hands none in and the details take the room
  /// the record would have had. Two panes again, and the divider between them
  /// is the sidebar's — the same border, moving the same width, so the frame
  /// this device settled its columns in is the frame it comes back to.
  conversations?: JSX.Element;
  middle?: JSX.Element;
  details: JSX.Element;

  /// What the middle pane holds, which is what it is called and what the
  /// divider beside it says it moves: the workbench's Timeline, the settings.
  /// The other two are the same thing on every page and name themselves.
  middleLabel: string;
}): JSX.Element {
  /// The list itself, resolved once — because a prop holding JSX is a getter,
  /// and asking it twice builds what it holds twice. The frame asks whether
  /// there is a list whenever it works out its columns and again from inside
  /// every drag, and a conversations pane built afresh each time is a pane's
  /// worth of queries thrown away — or, from a pointer's own handler, a query
  /// with no page around it to belong to. Resolved, it is one pane, asked after
  /// as often as the frame likes and drawn where the frame draws it.
  const list = children(() => props.conversations);

  /// Whether there is a list to pick from at all — see `conversations` above.
  /// Where there is not, the frame is two panes with one border between them,
  /// and the width that border decides is the pair's own: what a reader settles
  /// a share's panes at says nothing about how wide this device stands the
  /// workbench's columns, and the other way about.
  const picking = () => list() !== undefined;

  /// And the middle pane, resolved once for the same reason the list is: the
  /// frame asks whether there is one every time it works out its columns, and
  /// a Timeline built afresh at each asking is a Timeline's worth of work
  /// thrown away.
  const record = children(() => props.middle);

  /// Whether there is a level between the list and the details at all — see
  /// `middle` above. Where there is not, the frame is the list and the details,
  /// with the sidebar's own divider between them wherever the list is full.
  const reading = () => record() !== undefined;

  /// And the frame with neither of them: the details pane by itself, across the
  /// whole window. The compose page while there is nothing to list stands in it
  /// — no list to pick from, and no record to read beside a Conversation that
  /// does not exist yet — and it is the one frame with no border in it at all,
  /// so there is no divider and no width to remember. See `workbench/zero.ts`.
  const alone = () => !picking() && !reading();

  /// Which layout is standing, which decides how many dividers there are and
  /// how much room each pane is allowed to leave the others.
  const beside = matching(BESIDE);
  const full = matching(FULL);

  /// Whether the list is collapsed, which is every width under [`FULL`]: no
  /// divider of its own, and a column of `COLLAPSED` rem whatever share this
  /// device settled it at.
  const collapsed = () => picking() && !full();

  /// And whether all three of them are actually up, which is the width *and*
  /// there being a middle pane to fill the column it brings: a frame handed no
  /// Timeline stands two panes however wide the window is, so the middle
  /// divider never arrives and the width behind it is left exactly as this
  /// device settled it.
  const three = () => beside() && reading();

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

  /// How much room the platform's own window controls take at each edge of the
  /// window, which the heads standing at those edges have to be padded clear of.
  ///
  /// Followed rather than read once: the rectangle the page is left at load is
  /// not always the true one, and a maximise, an unmaximise and a resize each
  /// change it. Nothing at all outside the app, where there is no overlay to
  /// ask — see `controls.ts`.
  const taken = controls();

  /// The frame as the arithmetic asks about it: how much room there is, how
  /// many panes are sharing it, and whether the list among them is collapsed.
  const frame = (): Frame => ({
    rem: across(),
    three: three(),
    picking: picking(),
    collapsed: collapsed(),
  });

  /// And the widths as they may actually be drawn: a sidebar dragged wide while
  /// two panes stand is not allowed to squeeze the middle pane out of the
  /// three-pane layout, so the minimums are met against the frame in front of
  /// the human rather than against the one the width was settled in.
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
      remember(settled(), frame());
    };

    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", dropped);
    window.addEventListener("pointercancel", dropped);
  };

  /// Moving one with the keyboard, which settles at once: there is no letting
  /// go of an arrow key.
  const nudge = (divider: Divider, by: number) => {
    setSettled((was) => nudged(was, divider, by, frame()));
    remember(settled(), frame());
  };

  /// And putting them back, which is what a double-click on any divider does:
  /// every width the frame has, because what it restores is the defaults rather
  /// than one of them. What another frame's dividers settled is left where it
  /// is — see `restore` in `widths.ts`.
  const defaults = () => setSettled((was) => restore(was, frame()));

  /// How wide the collapsed list expands to, in rem: the share it would stand at
  /// full, met against this frame as the full band meets it — with the middle
  /// pane owed its room wherever there is one, whether or not it is beside the
  /// details at this width. A length rather than a share, because the pane is
  /// drawn over a column `COLLAPSED` wide and a percentage would be one of that.
  ///
  /// Nothing until the frame has been measured: a share of nought rem is no
  /// width, and the stylesheet has the sidebar's floor behind the name.
  const expandsTo = () =>
    across() > 0
      ? (expanded(settled(), { ...frame(), three: reading() }) * across()) / 100
      : undefined;

  /// The collapsed list's own section, which a finger is asked about.
  let strip: HTMLElement | undefined;

  /// Whether a finger has expanded the collapsed list, which is how a screen
  /// with nothing to hover opens it: a tap anywhere on the strip expands it and
  /// does nothing else, a tap on a card or a link in it then goes where it was
  /// put and collapses it behind, and a tap anywhere outside it collapses it
  /// and lands where it was put too — nothing is washed behind the pane to
  /// catch it. Drawn as `data-expanded`, the same expansion nothing being open
  /// stands it at.
  ///
  /// Read off the pointer each gesture began with rather than off the device,
  /// as the sidebar's drag reads it: a laptop with a touchscreen hovers with
  /// its mouse and taps with a finger. A mouse expands it by resting on it —
  /// see `hovered` — and a press of one leaves it to the hover, so it is a
  /// mouse's press as much as a finger's that puts down an expansion a finger
  /// made, the pointer resting there holding it open until it leaves.
  const [touched, setTouched] = createSignal(false);

  /// Whether a mouse is resting on the collapsed list, which is how a mouse
  /// opens it: drawn as `data-hovered`, and put there by the page rather than
  /// left to `:hover`, for two things `:hover` cannot do.
  ///
  /// It waits for the pointer to settle. A pointer flung off the window's left
  /// edge crosses the strip on its way, and a pane that opened under it and
  /// shut again behind it is a flash of the whole list for nothing — so the
  /// pane expands once the pointer over it has slowed below `SETTLED`, or has
  /// stopped for `STOPPED`, and a pointer that leaves before either never
  /// opened it. Slowing rather than a fixed delay, which would make every
  /// deliberate reach for the list that much slower too.
  ///
  /// And it lets go a moment late: a pointer leaving the pane collapses it
  /// `LINGER` later, and one coming back inside that keeps it open, so a hand
  /// overshooting the edge does not snap it shut.
  ///
  /// Only a mouse arms it. A browser leaves `:hover` standing where a finger
  /// tapped, and a pane held open by that would be a finger's tap on a card
  /// that never collapsed it — which a pointer's type, read here, never is.
  const [hovered, setHovered] = createSignal(false);

  /// Where and when the pointer over the strip was last seen, while it is
  /// being watched for settling; nothing when it is not.
  let last: { x: number; y: number; at: number } | undefined;

  /// The wait for a pointer that has stopped, which sends no more moves to be
  /// timed, and the wait after one that has left.
  let stopping: ReturnType<typeof setTimeout> | undefined;
  let leaving: ReturnType<typeof setTimeout> | undefined;

  const settle = () => {
    clearTimeout(stopping);
    stopping = undefined;
    last = undefined;
    setHovered(true);
  };

  /// A mouse coming onto the list: back inside the wait after it left, which
  /// keeps the pane open, or onto the strip, which starts watching it.
  const entered = (event: PointerEvent) => {
    if (event.pointerType !== "mouse") {
      return;
    }

    clearTimeout(leaving);
    leaving = undefined;
    if (hovered() || !collapsed()) {
      return;
    }

    last = { x: event.clientX, y: event.clientY, at: performance.now() };
    clearTimeout(stopping);
    stopping = setTimeout(settle, STOPPED);
  };

  /// A mouse moving over the strip, timed against where it was last.
  /// `performance.now()` rather than the event's own stamp, which is a clock
  /// the tests' timers do not drive.
  const moved = (event: PointerEvent) => {
    if (event.pointerType !== "mouse" || last === undefined) {
      return;
    }

    const at = performance.now();
    const travelled = Math.hypot(event.clientX - last.x, event.clientY - last.y);
    const elapsed = at - last.at;

    if (elapsed > 0 && travelled / elapsed < SETTLED) {
      settle();
      return;
    }

    last = { x: event.clientX, y: event.clientY, at };
    clearTimeout(stopping);
    stopping = setTimeout(settle, STOPPED);
  };

  /// And a mouse leaving it: a crossing that never settled is forgotten, and a
  /// pane it had opened collapses after the wait.
  const left = (event: PointerEvent) => {
    if (event.pointerType !== "mouse") {
      return;
    }

    clearTimeout(stopping);
    stopping = undefined;
    last = undefined;
    if (hovered()) {
      clearTimeout(leaving);
      leaving = setTimeout(() => {
        leaving = undefined;
        setHovered(false);
      }, LINGER);
    }
  };

  onCleanup(() => {
    clearTimeout(stopping);
    clearTimeout(leaving);
  });

  /// Whether the tap now landing is the one that expanded the list, and so is
  /// to be swallowed before anything inside it hears it. Set by each press
  /// and spent by the click that follows it. A press that brings no click — a
  /// finger that scrolled the list, which the browser cancels, or one that held
  /// a card until it lifted — is forgotten by the next press, the cancel, or a
  /// key, so a keyboard's click is never the one swallowed.
  let swallowing = false;
  const forget = () => {
    swallowing = false;
  };

  /// Whether the list is already standing expanded, by any of the ways it
  /// does: nothing open, a finger, or a keyboard's focus. Hover is not asked
  /// after, a finger having nothing to hover with.
  const standing = (pane: HTMLElement): boolean => {
    if (!collapsed() || pane.hasAttribute("data-expanded")) {
      return true;
    }

    try {
      return pane.matches(":focus-visible, :has(:focus-visible)");
    } catch {
      return false;
    }
  };

  /// A press on the list: a finger's on the collapsed strip expands it.
  /// Heard on the way down, ahead of the card that would otherwise take it.
  const pressed = (event: PointerEvent) => {
    swallowing =
      event.pointerType !== "mouse" &&
      !standing(event.currentTarget as HTMLElement);

    if (swallowing) {
      setTouched(true);
    }
  };

  /// And the click it ends in: the expanding tap's goes no further, and a
  /// card's or a link's goes where it was put and collapses the list behind it.
  const clicked = (event: MouseEvent) => {
    if (swallowing) {
      swallowing = false;
      event.preventDefault();
      event.stopPropagation();
      return;
    }

    const target = event.target as Element | null;
    if (target?.closest("a[href], [data-row]")) {
      setTouched(false);
    }
  };

  /// A press anywhere outside the list puts a finger's expansion down, and is
  /// left to land where it was put.
  onMount(() => {
    const elsewhere = (event: PointerEvent) => {
      if (touched() && !strip?.contains(event.target as Node)) {
        setTouched(false);
      }
    };

    document.addEventListener("pointerdown", elsewhere, true);
    onCleanup(() =>
      document.removeEventListener("pointerdown", elsewhere, true),
    );
  });

  /// What the frame carries the widths as. Only the standing layout's own: the
  /// stylesheet has a default behind each of them, so a name the frame does not
  /// carry is the untouched frame rather than a broken one. The pair's from
  /// where its two panes stand side by side; the middle pane's from where it
  /// stands beside the details; the sidebar's where the list is full; and how
  /// wide a collapsed list expands to wherever it is collapsed, phones included,
  /// that being the one width a list standing as a strip still has.
  ///
  /// Which is why the frame with no middle pane names the sidebar's width and
  /// nothing else: there is no column for the middle one to be the width of,
  /// and a name written over a column that is not there would be this frame
  /// answering for the one the Timeline comes back to.
  const columns = (): Record<string, string> | undefined => {
    if (alone()) {
      return undefined;
    }

    if (!picking()) {
      return beside() ? { "--pane-pair": `${shown().pair}%` } : undefined;
    }

    const middle: Record<string, string> = three()
      ? { "--pane-middle": `${shown().middle}%` }
      : {};

    if (collapsed()) {
      const to = expandsTo();

      return to === undefined
        ? middle
        : { "--pane-expanded": `${to}rem`, ...middle };
    }

    return { "--pane-sidebar": `${shown().sidebar}%`, ...middle };
  };

  /// And everything the frame carries on itself, which is those two kinds of
  /// variable and nothing else: how wide its columns stand, and how much of each
  /// edge the window's controls have taken. Nothing where it comes to neither —
  /// a browser below the first breakpoint with no list on it — so the element
  /// is left exactly as bare as it was before either of them existed.
  const carried = () => {
    const named = { ...columns(), ...reserved(taken()) };

    return Object.keys(named).length > 0 ? named : undefined;
  };

  return (
    <div
      class={[
        styles.panes,
        // The frame with a list in it, which is the one whose bands are the
        // collapsed list's: the other two keep the one line they always had.
        picking() ? styles.listing : undefined,
        // The one pane by itself, which is neither of the two frames below and
        // not both of them either: what those say is where the column that is
        // missing used to be, and here there is only the one left.
        alone() ? styles.alone : undefined,
        alone() || picking() ? undefined : styles.two,
        alone() || reading() ? undefined : styles.widened,
      ]
        .filter(Boolean)
        .join(" ")}
      data-pane={props.pane}
      ref={element}
      style={carried()}
    >
      <Show when={picking()}>
        {/* Standing expanded while it is collapsed and nothing is open:
            there is no level to show beside it, and what is there is empty
            paper until a card is picked. And while a finger has
            expanded it — see `touched`. The press and the click are heard on
            the way down, before anything in the list hears them. */}
        <section
          class={`${styles.pane} ${styles.conversationsPane}`}
          aria-label="Conversations"
          ref={(section) => {
            strip = section;
            section.addEventListener("pointerdown", pressed, true);
            section.addEventListener("click", clicked, true);
            section.addEventListener("pointerenter", entered);
            section.addEventListener("pointermove", moved, true);
            section.addEventListener("pointerleave", left);
            section.addEventListener("pointercancel", forget, true);
            section.addEventListener("keydown", forget, true);
          }}
          data-expanded={
            collapsed() && (props.pane === "nothing" || touched())
              ? ""
              : undefined
          }
          data-hovered={collapsed() && hovered() ? "" : undefined}
        >
          {list()}
        </section>
      </Show>

      {/* One divider per border there is: the sidebar's wherever the list
          stands full, the middle pane's wherever the middle pane stands beside
          the details, and the pair's where there is no list and so no sidebar.
          A collapsed list has none: its column is the same strip whatever the
          share says. Each sits between the two panes it parts, so the grid
          places it without being told where.

          The sidebar's is the border of whatever stands to its right, which is
          the Timeline where there is one and the details where there is not:
          the same handle moving the same width either way, so the frame with
          no middle pane keeps the columns this device settled and hands them
          straight back when the Timeline returns. */}
      <Show when={picking() && !collapsed()}>
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

      {/* The middle pane, where the caller handed one in. Where it did not,
          there is no such section in the document at all — the level is not
          drawn rather than drawn empty, so nothing of what it would have
          carried is left standing in a strip above the pane beside it. */}
      <Show when={reading()}>
        <section
          class={`${styles.pane} ${styles.middlePane}`}
          aria-label={props.middleLabel}
        >
          {record()}
        </section>
      </Show>

      <Show when={three() && picking()}>
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

      {/* And the pair's, which stands in the same place for the same reason:
          the middle pane is what it is the far edge of. It arrives with the
          panes themselves rather than with a third one, there being no third
          one to wait for — the two are side by side from 60rem. */}
      <Show when={beside() && !picking() && !alone()}>
        <Handle
          divider="pair"
          label={`Resize the ${props.middleLabel.toLowerCase()} pane`}
          share={shown().pair}
          travel={range("pair", shown(), frame())}
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
