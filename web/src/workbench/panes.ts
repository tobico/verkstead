//! How wide the workbench's panes stand, and where that is remembered.
//!
//! Widths are shares of the workbench rather than lengths: a column fixed in
//! `rem` is the same column on a laptop and on a thirty-inch screen, and the
//! whole reason for dragging one is that those are not the same window. So
//! everything here is a percentage, and the pane that is not named — the
//! details — takes what the named two leave.
//!
//! Kept per device, beside the wrap setting and the answer sheets' drafts, and
//! never sent to the server: how wide a phone draws a list has nothing to say
//! about how wide a desktop should.
//!
//! None of this knows what a pane holds or how many of them are drawn. It is
//! arithmetic over two numbers, and the layout it is arithmetic *for* arrives
//! as the one flag [`clamped`] takes — the two queries below being the whole of
//! what answers that flag, and the stylesheet's own breakpoints said again here
//! so that the page and the rules cannot come to disagree about which layout is
//! standing.

import { forget, read, write } from "../device";

/// From here the sidebar stands beside the level being read, and the divider
/// between the two can be dragged. Below it the workbench pages one pane at a
/// time: nothing stands beside anything, so there are no dividers to drag and
/// whatever this device remembers is left where it is.
export const BESIDE = "(min-width: 60rem)";

/// And from here all three panes stand together, which is the layout with two
/// dividers in it.
export const ALL_THREE = "(min-width: 80rem)";

/// Which divider a drag is of — named for the pane on its left, which is the
/// one whose width the drag decides.
export type Divider = "sidebar" | "timeline";

/// Where each width lives. Namespaced like everything else this app leaves in a
/// browser.
const KEYS: Record<Divider, string> = {
  sidebar: "verkstead.pane-sidebar",
  timeline: "verkstead.pane-timeline",
};

/// The two widths together: the conversations pane and the timeline pane, each
/// as a percentage of the workbench.
export type Widths = Record<Divider, number>;

/// What they are worth with nobody having said otherwise — the 15rem and 25rem
/// the columns used to be fixed at, as shares of the 80rem window the third
/// pane arrives at, rounded to numbers a person could have chosen.
export const DEFAULTS: Widths = { sidebar: 20, timeline: 30 };

/// And the least each pane may be left with. A divider dragged to the end of
/// its travel leaves the pane beyond it narrow rather than gone: a pane with no
/// width is a pane the human cannot find the divider of again.
export const MINIMUMS = { sidebar: 10, timeline: 15, details: 20 } as const;

/// The widths this device last settled on, or the defaults where it has settled
/// on nothing.
///
/// Anything that is not a number strictly between nothing and the whole
/// workbench reads as unset — a storage somebody has edited by hand, or one
/// written by a version of this page that meant something else by the key.
/// Unclamped, deliberately: what a width is allowed to be depends on how many
/// panes are standing, which is [`clamped`]'s question rather than this one's.
export function widths(): Widths {
  return { sidebar: held("sidebar"), timeline: held("timeline") };
}

function held(divider: Divider): number {
  const stored = read(KEYS[divider]);
  const share = stored === null ? Number.NaN : Number(stored);
  return Number.isFinite(share) && share > 0 && share < 100
    ? share
    : DEFAULTS[divider];
}

/// Remember what a drag settled on.
export function remember(settled: Widths): void {
  write(KEYS.sidebar, String(settled.sidebar));
  write(KEYS.timeline, String(settled.timeline));
}

/// And take it all away again, which is what a double-click on a divider does:
/// both widths at once, because what it restores is *the defaults* rather than
/// one of them.
export function restore(): void {
  forget(KEYS.sidebar);
  forget(KEYS.timeline);
}

/// How far a divider may travel: the least the pane on its left may be left
/// with, and the most it may take.
///
/// With all three panes up, every minimum has to fit beside every other, so the
/// sidebar may not eat the room the timeline and the details are owed and the
/// timeline may not eat the details'. With two, there is no timeline to leave
/// room for: the second column is whichever pane is being read, and it takes
/// whatever the sidebar does not.
///
/// Said out loud rather than buried in [`clamped`] because the handle carries
/// it too — a separator says where it stands between what and what.
export function range(
  divider: Divider,
  settled: Widths,
  three: boolean,
): { least: number; most: number } {
  if (divider === "sidebar") {
    return {
      least: MINIMUMS.sidebar,
      most: three
        ? 100 - MINIMUMS.timeline - MINIMUMS.details
        : 100 - MINIMUMS.details,
    };
  }

  return {
    least: MINIMUMS.timeline,
    most: 100 - settled.sidebar - MINIMUMS.details,
  };
}

/// The widths as they may actually be drawn, given how many panes are standing.
///
/// With two, the timeline's width decides nothing, so it is passed through
/// untouched rather than squeezed against a minimum it has no business meeting
/// yet. That is what keeps a wide sidebar dragged in the two-pane layout from
/// quietly rewriting the three-pane one.
export function clamped(settled: Widths, three: boolean): Widths {
  const sidebar = between(settled.sidebar, range("sidebar", settled, three));

  return three
    ? {
        sidebar,
        timeline: between(
          settled.timeline,
          range("timeline", { ...settled, sidebar }, true),
        ),
      }
    : { ...settled, sidebar };
}

/// Where a divider dropped at `share` of the way across the workbench leaves the
/// two widths.
///
/// The sidebar's divider says where the sidebar ends, so its share *is* the
/// width. The timeline's says where the timeline ends, which is a share of the
/// workbench and not of what is left of it, so the sidebar comes off it first.
export function dragged(
  settled: Widths,
  divider: Divider,
  share: number,
  three: boolean,
): Widths {
  const moved: Widths =
    divider === "sidebar"
      ? { ...settled, sidebar: share }
      : { ...settled, timeline: share - settled.sidebar };

  return clamped(moved, three);
}

/// A width nudged by `by` percentage points, which is what an arrow key on a
/// focused divider does — the same travel as a drag, for a pointer nobody has.
export function nudged(
  settled: Widths,
  divider: Divider,
  by: number,
  three: boolean,
): Widths {
  return clamped(
    { ...settled, [divider]: settled[divider] + by },
    three,
  );
}

function between(share: number, { least, most }: ReturnType<typeof range>): number {
  return Math.min(Math.max(share, least), most);
}
