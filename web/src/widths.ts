//! How wide the frame's panes stand, and where that is remembered.
//!
//! Widths are shares of the frame rather than lengths: a column fixed in `rem`
//! is the same column on a laptop and on a thirty-inch screen, and the whole
//! reason for dragging one is that those are not the same window. So what a
//! human settles on here is a percentage, and the pane that is not named — the
//! details — takes what the named ones leave.
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
//! — the two queries below and whether there is a list to pick from being the
//! whole of what says which layout that is, and the stylesheet's own breakpoints
//! said again here so that the page and the rules cannot come to disagree about
//! which one is standing.
//!
//! Kept per device, beside the wrap setting and the answer sheets' drafts, and
//! never sent to the server: how wide a phone draws a list has nothing to say
//! about how wide a desktop should. One set for the device rather than one per
//! page: the frame is the same frame wherever it stands, and a human who has
//! dragged the conversations narrower has said how wide they want it. Each
//! layout keeps its own width all the same — see [`Divider`].
//!
//! None of this knows what a pane holds or how many of them are drawn. It is
//! arithmetic over a few numbers and the frame they are measured against; the
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
///
/// Two of them belong to the frame with a list in it, and the third to the one
/// without: `pair` is the only divider a share has, between the record and
/// whatever it has open. The pane on its left is the middle pane there as well,
/// and it is a width of its own all the same — that pane starts at the frame's
/// edge rather than after a sidebar, so one percentage would mean two different
/// columns, and a device that has dragged one frame has said nothing about how
/// wide the other should stand.
export type Divider = "sidebar" | "middle" | "pair";

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
  pair: "verkstead.pane-pair",
};

/// The widths together: the conversations pane, the middle pane beside it, and
/// the middle pane of the frame that has no conversations — each as a
/// percentage of the frame it is drawn in.
export type Widths = Record<Divider, number>;

/// The frame those are shares *of*: how wide it stands in `rem`, whether all
/// three panes are standing in it, and whether it has a list to pick from at
/// all.
///
/// All three are the page's to answer — the first by measuring the frame, the
/// second by asking which breakpoint holds, the third by whether it was handed
/// a conversations pane — and together they are the whole of what the
/// arithmetic here needs to know about the window in front of the human.
///
/// A frame with nothing to pick from is the share's pair of panes, and `three`
/// says nothing about one: there is no third pane for a breakpoint to bring in.
export type Frame = { rem: number; three: boolean; picking: boolean };

/// What they are worth with nobody having said otherwise — the 15rem and 25rem
/// the columns used to be fixed at, as shares of the 80rem window the third
/// pane arrives at, rounded to numbers a person could have chosen. And for the
/// pair, the 40% its record stood at while its border could not be moved.
export const DEFAULTS: Widths = { sidebar: 20, middle: 30, pair: 40 };

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
///
/// Three of them for three dividers, one of the three counted twice: what the
/// pair's divider has either side of it is a middle pane and the details, and
/// those are owed what they are owed wherever they stand.
export const MINIMUMS = { sidebar: 16, middle: 24, details: 24 } as const;

/// Which dividers a frame has, which is what says whose widths it reads back
/// and writes down.
///
/// The frame with no list in it has the one, and it is not either of the
/// others: what a reader settles a share's panes at must leave the workbench's
/// own columns exactly as this device left them, and the way to be sure of that
/// is to touch nothing else.
function moving(frame: Frame): Divider[] {
  if (!frame.picking) {
    return ["pair"];
  }

  return frame.three ? ["sidebar", "middle"] : ["sidebar"];
}

/// The widths this device last settled on, or the defaults where it has settled
/// on nothing.
///
/// Anything that is not a number strictly between nothing and the whole frame
/// reads as unset — a storage somebody has edited by hand, or one written by a
/// version of this page that meant something else by the key. Unclamped,
/// deliberately: what a width is allowed to be depends on the frame it is being
/// drawn in, which is [`clamped`]'s question rather than this one's.
export function widths(): Widths {
  return { sidebar: held("sidebar"), middle: held("middle"), pair: held("pair") };
}

function held(divider: Divider): number {
  const stored = read(KEYS[divider]);
  const share = stored === null ? Number.NaN : Number(stored);
  return Number.isFinite(share) && share > 0 && share < 100
    ? share
    : DEFAULTS[divider];
}

/// Remember what a drag settled on — the widths the frame in front of the human
/// has dividers for, and no others. A width nothing on this page can move
/// belongs to a layout that is not standing, and writing it back would be one
/// frame answering for another.
export function remember(settled: Widths, frame: Frame): void {
  for (const divider of moving(frame)) {
    write(KEYS[divider], String(settled[divider]));
  }
}

/// And give them up again, which is what a double-click on a divider does: every
/// width this frame has, because what it restores is *the defaults* rather than
/// one of them — and nothing belonging to a frame that is not on the screen.
///
/// What comes back is the widths as they stand afterwards: this frame's put
/// back, and the rest as they were.
export function restore(settled: Widths, frame: Frame): Widths {
  const restored = { ...settled };

  for (const divider of moving(frame)) {
    forget(KEYS[divider]);
    restored[divider] = DEFAULTS[divider];
  }

  return restored;
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
/// middle may not eat the details'. With the sidebar and one other, there is no
/// middle pane to leave room for: the second column is whichever pane is being
/// read, and it takes whatever the sidebar does not. And the pair has only the
/// two, so its divider owes the details their width and nothing else.
///
/// Said out loud rather than buried in [`clamped`] because the handle carries
/// it too — a separator says where it stands between what and what.
export function range(
  divider: Divider,
  settled: Widths,
  frame: Frame,
): { least: number; most: number } {
  // What the pane on the divider's left is owed. The pair's has a middle pane
  // on its left as the three-pane frame's middle divider does, so it is owed
  // what a middle pane is owed.
  const least = shareOf(
    divider === "sidebar" ? MINIMUMS.sidebar : MINIMUMS.middle,
    frame,
  );

  // What has to be left standing to the right of this divider. Beyond the
  // sidebar is everything the layout has after it, which is the details alone
  // where only two panes stand; beyond either middle pane is the details, and
  // beyond the three-pane frame's the sidebar as well, that being on the other
  // side of it and so already spent.
  const beyond =
    divider === "sidebar"
      ? shareOf(
          frame.three ? MINIMUMS.middle + MINIMUMS.details : MINIMUMS.details,
          frame,
        )
      : shareOf(MINIMUMS.details, frame) +
        (divider === "middle" ? settled.sidebar : 0);

  // A frame too narrow to hold every floor at once is one where a floor has to
  // give, and the one that gives is the room left for the panes beyond: a
  // divider that cannot be dragged as far as its own least is a divider with
  // nowhere to go, and its pane is the one with a handle to find it by.
  return { least, most: Math.max(least, 100 - beyond) };
}

/// The widths as they may actually be drawn, given the frame they are drawn in.
///
/// Only the widths this frame has dividers for are met against it, and the rest
/// are passed through untouched rather than squeezed against a minimum they
/// have no business meeting yet. That is what keeps a sidebar dragged wide while
/// two panes stand from quietly rewriting the three-pane layout, and what keeps
/// a share — which has neither — from touching either.
export function clamped(settled: Widths, frame: Frame): Widths {
  if (!frame.picking) {
    return {
      ...settled,
      pair: between(settled.pair, range("pair", settled, frame)),
    };
  }

  const sidebar = between(settled.sidebar, range("sidebar", settled, frame));

  return frame.three
    ? {
        ...settled,
        sidebar,
        middle: between(
          settled.middle,
          range("middle", { ...settled, sidebar }, frame),
        ),
      }
    : { ...settled, sidebar };
}

/// Where a divider dropped at `share` of the way across the frame leaves the
/// widths.
///
/// The sidebar's divider says where the sidebar ends, so its share *is* the
/// width — and so is the pair's, whose pane starts at the frame's own edge. The
/// middle one says where the middle pane ends, which is a share of the frame and
/// not of what is left of it, so the sidebar comes off it first.
export function dragged(
  settled: Widths,
  divider: Divider,
  share: number,
  frame: Frame,
): Widths {
  const moved: Widths =
    divider === "middle"
      ? { ...settled, middle: share - settled.sidebar }
      : { ...settled, [divider]: share };

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
