//! How wide the three panes stand, and where that is remembered.
//!
//! Widths are shares of the frame rather than lengths: a column fixed in `rem`
//! is the same column on a laptop and on a thirty-inch screen, and the whole
//! reason for dragging one is that those are not the same window. So what a
//! human settles on here is a percentage, and the pane that is not named — the
//! details — takes what the named two leave.
//!
//! The floors under those widths are the other way about. What makes a pane too
//! narrow is what stands in it, and a card, a Brief and a Diff are the same
//! size on both of those screens: a floor written as a share is a floor that
//! means something different at every width, and is either room wasted on the
//! wide window or a pane too narrow to read on the small one. So [`MINIMUMS`] are
//! lengths in `rem`, and what they are worth as shares is arithmetic against
//! the frame they are shares of.
//!
//! Which is the one thing here that has to be told how wide the window actually
//! stands. It arrives as [`Frame`], along with the layout in front of the human
//! — the two queries below being the whole of what says which layout that is,
//! and the stylesheet's own breakpoints said again here so that the page and
//! the rules cannot come to disagree about which one is standing.
//!
//! Kept per device, beside the wrap setting and the answer sheets' drafts, and
//! never sent to the server: how wide a phone draws a list has nothing to say
//! about how wide a desktop should. One pair for the device rather than one per
//! page: the frame is the same frame wherever it stands, and a human who has
//! dragged the conversations narrower has said how wide they want it.
//!
//! None of this knows what a pane holds or how many of them are drawn. It is
//! arithmetic over two numbers and the frame they are measured against; the
//! measuring itself is the page's, in `Panes.tsx`, because a width is only ever
//! a width of something drawn.

import { forget, read, write } from "./device";

/// From here the sidebar stands beside the level being read, and the divider
/// between the two can be dragged. Below it the frame pages one pane at a time:
/// nothing stands beside anything, so there are no dividers to drag and
/// whatever this device remembers is left where it is.
export const BESIDE = "(min-width: 60rem)";

/// And from here all three panes stand together, which is the layout with two
/// dividers in it.
export const ALL_THREE = "(min-width: 80rem)";

/// Which divider a drag is of — named for the pane on its left, which is the
/// one whose width the drag decides.
export type Divider = "sidebar" | "middle";

/// Where each width lives. Namespaced like everything else this app leaves in a
/// browser.
///
/// The middle one is still under the name it was written under, when the only
/// frame there was was the workbench's and the pane between the two dividers
/// was the Timeline. A device that has dragged it has the width under that key
/// and nothing else; renaming it here would be a width quietly forgotten.
const KEYS: Record<Divider, string> = {
  sidebar: "verkstead.pane-sidebar",
  middle: "verkstead.pane-timeline",
};

/// The two widths together: the conversations pane and the middle pane, each as
/// a percentage of the frame.
export type Widths = Record<Divider, number>;

/// The frame those are shares *of*: how wide it stands in `rem`, and whether
/// all three panes are standing in it.
///
/// Both are the page's to answer — the first by measuring the frame, the second
/// by asking which breakpoint holds — and together they are the whole of what
/// the arithmetic here needs to know about the window in front of the human.
export type Frame = { rem: number; three: boolean };

/// What they are worth with nobody having said otherwise — the 15rem and 25rem
/// the columns used to be fixed at, as shares of the 80rem window the third
/// pane arrives at, rounded to numbers a person could have chosen.
export const DEFAULTS: Widths = { sidebar: 20, middle: 30 };

/// And the least each pane may be left with, in `rem`: the width below which
/// what the pane holds stops being presented rather than merely being narrow.
/// A card whose title has one word to a line, a Brief the size of a stamp, a
/// Diff with no room for a line of code.
///
/// Lengths rather than shares, so that they mean the same thing on every window
/// the frame is drawn on — which is the point of writing them here at all. They
/// have to fit beside each other in the narrowest window that stands all three
/// panes, which is the 80rem [`ALL_THREE`] names; what is left over after them
/// is how far the dividers can travel there.
export const MINIMUMS = { sidebar: 16, middle: 24, details: 24 } as const;

/// The widths this device last settled on, or the defaults where it has settled
/// on nothing.
///
/// Anything that is not a number strictly between nothing and the whole frame
/// reads as unset — a storage somebody has edited by hand, or one written by a
/// version of this page that meant something else by the key. Unclamped,
/// deliberately: what a width is allowed to be depends on the frame it is being
/// drawn in, which is [`clamped`]'s question rather than this one's.
export function widths(): Widths {
  return { sidebar: held("sidebar"), middle: held("middle") };
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
  write(KEYS.middle, String(settled.middle));
}

/// And take it all away again, which is what a double-click on a divider does:
/// both widths at once, because what it restores is *the defaults* rather than
/// one of them.
export function restore(): void {
  forget(KEYS.sidebar);
  forget(KEYS.middle);
}

/// What `length` rem is worth as a share of this frame.
///
/// A frame nobody has measured yet is nought across, and nothing can be a share
/// of nothing: the floors are worth nothing until the page has measured, which
/// is a repaint away and leaves the widths as they were settled until then.
function shareOf(length: number, frame: Frame): number {
  return frame.rem > 0 ? (length / frame.rem) * 100 : 0;
}

/// How far a divider may travel: the least the pane on its left may be left
/// with, and the most it may take.
///
/// With all three panes up, every minimum has to fit beside every other, so the
/// sidebar may not eat the room the middle and the details are owed and the
/// middle may not eat the details'. With two, there is no middle pane to leave
/// room for: the second column is whichever pane is being read, and it takes
/// whatever the sidebar does not.
///
/// Said out loud rather than buried in [`clamped`] because the handle carries
/// it too — a separator says where it stands between what and what.
export function range(
  divider: Divider,
  settled: Widths,
  frame: Frame,
): { least: number; most: number } {
  const least = shareOf(MINIMUMS[divider], frame);

  // What has to be left standing to the right of this divider. Beyond the
  // sidebar is everything the layout has after it, which is the details alone
  // where only two panes stand; beyond the middle pane is the details, the
  // sidebar being on the other side of it and so already spent.
  const beyond =
    divider === "sidebar"
      ? shareOf(
          frame.three ? MINIMUMS.middle + MINIMUMS.details : MINIMUMS.details,
          frame,
        )
      : settled.sidebar + shareOf(MINIMUMS.details, frame);

  // A frame too narrow to hold every floor at once is one where a floor has to
  // give, and the one that gives is the room left for the panes beyond: a
  // divider that cannot be dragged as far as its own least is a divider with
  // nowhere to go, and its pane is the one with a handle to find it by.
  return { least, most: Math.max(least, 100 - beyond) };
}

/// The widths as they may actually be drawn, given the frame they are drawn in.
///
/// With two panes, the middle pane's width decides nothing, so it is passed
/// through untouched rather than squeezed against a minimum it has no business
/// meeting yet. That is what keeps a wide sidebar dragged in the two-pane
/// layout from quietly rewriting the three-pane one.
export function clamped(settled: Widths, frame: Frame): Widths {
  const sidebar = between(settled.sidebar, range("sidebar", settled, frame));

  return frame.three
    ? {
        sidebar,
        middle: between(
          settled.middle,
          range("middle", { ...settled, sidebar }, frame),
        ),
      }
    : { ...settled, sidebar };
}

/// Where a divider dropped at `share` of the way across the frame leaves the two
/// widths.
///
/// The sidebar's divider says where the sidebar ends, so its share *is* the
/// width. The middle one says where the middle pane ends, which is a share of
/// the frame and not of what is left of it, so the sidebar comes off it first.
export function dragged(
  settled: Widths,
  divider: Divider,
  share: number,
  frame: Frame,
): Widths {
  const moved: Widths =
    divider === "sidebar"
      ? { ...settled, sidebar: share }
      : { ...settled, middle: share - settled.sidebar };

  return clamped(moved, frame);
}

/// A width nudged by `by` percentage points, which is what an arrow key on a
/// focused divider does — the same travel as a drag, for a pointer nobody has.
export function nudged(
  settled: Widths,
  divider: Divider,
  by: number,
  frame: Frame,
): Widths {
  return clamped({ ...settled, [divider]: settled[divider] + by }, frame);
}

function between(share: number, { least, most }: ReturnType<typeof range>): number {
  return Math.min(Math.max(share, least), most);
}
