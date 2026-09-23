//! The tree Code's pane is drawn from: splits with groups of tabs at the leaves.
//!
//! One group was the whole of the pane in stage 01 — a bar of tabs and the one
//! of them showing. A group splits to the right of itself or below itself, and
//! either half splits again, so what holds them is a tree rather than a list
//! (ADR 0019, *Tabs and groups*). It is the one thing the pane reads: what is
//! drawn, where the borders between them are and which group is **active** are
//! all read off it.
//!
//! **A split is two.** A third group beside two is a split inside one of them,
//! which is what makes this a tree rather than a row of columns — and what
//! makes a group leaving simple: the split it stood in collapses, and the other
//! half is hoisted into the room the pair had. That collapsing is the whole of
//! unsplitting, there being no command for it (ADR 0019).
//!
//! **A share is a percentage of the parent**, for the reason the frame's pane
//! widths are percentages of the window — see `widths.ts`. A border settled on
//! a laptop means the same thing on a wider screen, where a column fixed in
//! `rem` would not, and a window that changes shape moves no share at all: the
//! groups are drawn at the percentages they were left at, met afresh against
//! the floors at the new size.
//!
//! **And the floors under those shares are lengths**, the other way about and
//! for the reason the frame's minimums are: what makes a group too narrow is
//! what stands in it, and a bar of tabs over an editor is the same size on
//! every window. So [`FLOORS`] are `rem`, and what one is worth as a share is
//! arithmetic against the split it is measured in — which is the one thing here
//! that has to be told how large the layer actually stands, and it arrives as
//! [`Layer`].
//!
//! **A group is a stable object with signals in it**, rather than a value in
//! the tree that is replaced whenever a tab opens. Which is what keeps a
//! terminal's socket and an editor's caret through everything but a reshape of
//! the tree itself: the pane draws a group per [`Group`], `For` reconciles
//! those by identity, and a tab opened, turned to or closed writes a signal
//! inside one rather than making a new tree. The tree moves when the layout
//! does, and a layout that moves is a layout somebody asked to move.
//!
//! **And the pane places the groups rather than nesting them.** Every group is
//! drawn once, side by side in the one layer, and [`placed`] says where each of
//! them stands as a percentage of the layer — which is the same arithmetic the
//! nesting would have done in the browser's own box model, done here instead
//! so that a split, a collapse or a tab dragged between groups moves boxes
//! rather than rebuilding them. Nested boxes would mean a group's whole subtree
//! taken down and made again the moment the tree above it changed shape, which
//! is a terminal's socket closed and an editor's caret lost for a press about a
//! group next to it.

import { createSignal, type Accessor, type Setter } from "solid-js";

import type { Tab } from "./keeping";

/// Which way a split divides the room it was given.
///
/// Named for where the group that was split puts its other half: *beside* it,
/// which is a border down the middle, or *below* it, which is one across.
export type Way = "beside" | "below";

/// One group: a bar of tabs, and the one of them showing.
///
/// Its own signals rather than fields of the tree, because a tab opened is not
/// a layout that moved — see the note at the top of this file.
export interface Group {
  /// What names this group wherever one has to be named — the active one, the
  /// one a split was made from, the one a tab was closed in. Counted up by the
  /// keeping and never reused, so an id is one group for as long as the page
  /// stands.
  id: number;

  /// Every tab in this group, in the order they were opened — files and
  /// terminals alike, in the one order, because they are the one bar.
  tabs: Accessor<Tab[]>;
  setTabs: Setter<Tab[]>;

  /// Which of them the human turned to, where they have turned to one, by its
  /// key. The first tab shows while they have not.
  chosen: Accessor<string | undefined>;
  setChosen: Setter<string | undefined>;
}

/// A node of the tree: a group, or a split of two nodes.
export type Layout = { group: Group } | { split: Way; parts: [Part, Part] };

/// One half of a split: how much of the parent it takes, and what is in it.
export interface Part {
  /// Its share of the parent, as a percentage. The two of a split sum to a
  /// hundred.
  share: number;
  node: Layout;
}

/// Where a group stands, as percentages of the whole layer — see [`placed`].
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/// A group with nothing open in it.
export function group(id: number): Group {
  const [tabs, setTabs] = createSignal<Tab[]>([]);
  const [chosen, setChosen] = createSignal<string | undefined>();

  return { id, tabs, setTabs, chosen, setChosen };
}

/// Every group of a tree, in the order they are read: the near half of a split
/// before the far one, so the list is the order the pane draws them in.
export function groups(node: Layout): Group[] {
  return "group" in node
    ? [node.group]
    : node.parts.flatMap((part) => groups(part.node));
}

/// The one with this id, where the tree has one.
export function found(node: Layout, id: number): Group | undefined {
  return groups(node).find((one) => one.id === id);
}

/// Where each group of the tree stands, as a percentage of the layer they are
/// all drawn in.
///
/// The arithmetic a nest of boxes would have done in the browser: a split hands
/// each half its share of the room it was given, along the axis it divides, and
/// a group is the room it ends up with. Percentages the whole way down, so a
/// layer of any size draws the same layout.
export function placed(
  node: Layout,
  within: Rect = LAYER,
  into: Record<number, Rect> = {},
): Record<number, Rect> {
  if ("group" in node) {
    into[node.group.id] = within;
    return into;
  }

  const rooms = halves(within, node.split, node.parts[0].share);

  node.parts.forEach((part, half) => placed(part.node, rooms[half]!, into));

  return into;
}

/// The whole layer, which is the room the root of the tree is given.
const LAYER: Rect = { x: 0, y: 0, width: 100, height: 100 };

/// The two boxes a split makes of the one it was given, the first taking
/// `share` of it along the axis the split divides.
///
/// The whole of the arithmetic a nest of boxes would have done in the browser,
/// said once: where the groups stand, where the border between two of them is,
/// and how far that border may travel are all this sum with a different
/// question asked of it.
function halves(within: Rect, way: Way, share: number): [Rect, Rect] {
  const across = way === "beside";
  const room = ((across ? within.width : within.height) * share) / 100;

  return across
    ? [
        { ...within, width: room },
        { ...within, x: within.x + room, width: within.width - room },
      ]
    : [
        { ...within, height: room },
        { ...within, y: within.y + room, height: within.height - room },
      ];
}

/// How large the layer the groups are drawn in stands, in `rem`.
///
/// Which the arithmetic needs because the floors under the shares are lengths:
/// what a group is owed is the same span of paper on every window, and turning
/// that into a share of the split it stands in takes the layer's own size.
/// Nought until the page has been laid out, which reads as no floors yet rather
/// than as floors of nothing — exactly as an unmeasured frame does in
/// `widths.ts`.
///
/// In `rem` rather than in pixels, and converted where the browser can be asked
/// what a rem is: a human who has told their browser to draw text larger has
/// said the groups should hold what they held.
export interface Layer {
  width: number;
  height: number;
}

/// And the least a group may be left with along each axis, in `rem`: the size
/// below which what stands in it stops being a group and starts being a sliver.
///
/// Named for the split whose border meets them — a `beside` split divides
/// across, so what its border is held off is a width, and a `below` one's is a
/// height. A bar with a tab on it is about ten rems wide; a bar with anything
/// under it at all is about six tall.
export const FLOORS: Record<Way, number> = { beside: 10, below: 6 };

/// What a node is owed along an axis, in `rem`.
///
/// A group is owed the floor. A split divides the room it is given, so one
/// *along* this axis owes what both its halves owe, one after the other, and
/// one across it owes whatever the hungrier of its halves does — which is what
/// keeps a border from squeezing a group three levels down out of existence.
function needs(node: Layout, way: Way): number {
  if ("group" in node) {
    return FLOORS[way];
  }

  const [near, far] = node.parts.map((part) => needs(part.node, way)) as [
    number,
    number,
  ];

  return node.split === way ? near + far : Math.max(near, far);
}

/// Which split a border belongs to: the turnings taken from the root to reach
/// it, near half first.
///
/// A split has no id of its own — it is the shape of the tree rather than a
/// thing in it, and a group leaving takes one away without anything else
/// moving. So a border is addressed by where it is, which is what the tree
/// already says.
export type Path = number[];

/// One border of the layout: the line between the halves of a split, and
/// everything moving it takes.
export interface Border {
  /// Which split it divides.
  path: Path;

  /// Which way that split divides, which is the border's own orientation, the
  /// axis it travels along and the arrow keys that move it.
  way: Way;

  /// The share of that split the half before it is worth — the value this
  /// border carries, and the one thing a drag changes.
  share: number;

  /// How far it may go, as shares of that same split: what the floors either
  /// side of it come to, measured against the room it divides.
  least: number;
  most: number;

  /// And that room, as percentages of the layer — what turns a point on the
  /// layer into a share of this split, and the other way about.
  within: Rect;
}

/// How far a split's border may travel, as shares of the split itself.
///
/// The floors either side of it, measured against the room the split has along
/// the axis it divides. A layer nobody has measured yet is nought across, and
/// nothing is a share of nothing: the floors are worth nothing until the page
/// has laid out, which leaves the shares exactly as they were settled.
///
/// A split with less room than its two halves are owed is one where a floor has
/// to give, and the one that gives is the half beyond the border: it stops
/// where the near half's own floor is rather than travelling back past it.
function travel(
  way: Way,
  parts: readonly [Part, Part],
  within: Rect,
  layer: Layer,
): { least: number; most: number } {
  const across = way === "beside";
  const room =
    (across ? layer.width * within.width : layer.height * within.height) / 100;

  if (room <= 0) {
    return { least: 0, most: 100 };
  }

  const least = (needs(parts[0].node, way) / room) * 100;

  return {
    least,
    most: Math.max(least, 100 - (needs(parts[1].node, way) / room) * 100),
  };
}

/// A share held to what a border may be left at.
function between(
  share: number,
  { least, most }: { least: number; most: number },
): number {
  return Math.min(Math.max(share, least), most);
}

/// The tree as it may actually be drawn in a layer this size: every share met
/// against the floors under the groups either side of it.
///
/// Which is what keeps a window changing shape from moving a share. What is
/// held is where the human left the border; what is drawn is that held against
/// the room there is now — so a layer that grows again hands the share back
/// rather than having quietly rewritten it.
export function clamped(
  node: Layout,
  layer: Layer,
  within: Rect = LAYER,
): Layout {
  if ("group" in node) {
    return node;
  }

  const share = between(
    node.parts[0].share,
    travel(node.split, node.parts, within, layer),
  );
  const rooms = halves(within, node.split, share);

  return {
    split: node.split,
    parts: node.parts.map((part, half) => ({
      share: half === 0 ? share : 100 - share,
      node: clamped(part.node, layer, rooms[half]!),
    })) as [Part, Part],
  };
}

/// Every border of the tree, in the order they are read — the near half of a
/// split before the far one, which is the order the groups themselves come in.
///
/// Asked of the tree as it is *drawn* rather than as it is held: what a border
/// carries is the share the eye can see, so it is read off [`clamped`]'s answer
/// and the two cannot come apart.
export function borders(
  node: Layout,
  layer: Layer,
  within: Rect = LAYER,
  path: Path = [],
  into: Border[] = [],
): Border[] {
  if ("group" in node) {
    return into;
  }

  const share = node.parts[0].share;

  into.push({
    path,
    way: node.split,
    share,
    ...travel(node.split, node.parts, within, layer),
    within,
  });

  const rooms = halves(within, node.split, share);

  node.parts.forEach((part, half) =>
    borders(part.node, layer, rooms[half]!, [...path, half], into),
  );

  return into;
}

/// Where a border dropped at `share` of the split it divides leaves the tree.
///
/// What the half before it takes, the half after it gives: the two of a split
/// sum to a hundred, so nothing outside that split moves at all. Held to the
/// travel the border was picked up with — a drag past a group's floor stops
/// there rather than running on.
export function moved(node: Layout, border: Border, share: number): Layout {
  return divided(node, border.path, between(share, border));
}

/// A border nudged by `by` percentage points, which is what an arrow key on a
/// focused one does — the same travel as a drag, for a pointer nobody has.
export function nudged(node: Layout, border: Border, by: number): Layout {
  return moved(node, border, border.share + by);
}

/// The tree with the split at this path divided at `share`, and everything else
/// the object it was.
function divided(node: Layout, path: Path, share: number): Layout {
  if ("group" in node) {
    return node;
  }

  const [step, ...rest] = path;

  if (step === undefined) {
    return {
      split: node.split,
      parts: node.parts.map((part, half) => ({
        ...part,
        share: half === 0 ? share : 100 - share,
      })) as [Part, Part],
    };
  }

  return {
    split: node.split,
    parts: node.parts.map((part, half) =>
      half === step ? { ...part, node: divided(part.node, rest, share) } : part,
    ) as [Part, Part],
  };
}

/// Split the group with this id, putting the new one beside or below it.
///
/// Halves, which is where every split starts: the border is dragged from there
/// if it is not where the human wants it. The tree back, with the one node
/// replaced — everything else in it is the object it was, which is what keeps
/// the groups the split was not about drawn exactly as they were.
export function split(
  node: Layout,
  id: number,
  way: Way,
  made: Group,
): Layout {
  if ("group" in node) {
    return node.group.id === id
      ? {
          split: way,
          parts: [
            { share: 50, node },
            { share: 50, node: { group: made } },
          ],
        }
      : node;
  }

  return {
    split: node.split,
    parts: node.parts.map((part) => ({
      ...part,
      node: split(part.node, id, way, made),
    })) as [Part, Part],
  };
}

/// Take the group with this id out of the tree, which is what its last tab
/// leaving does.
///
/// The split it stood in goes with it and the other half takes the room — the
/// whole of unsplitting (ADR 0019, *Tabs and groups*). A tree that is nothing
/// but that one group is left where it is: there is always a group, and a pane
/// with nothing open is one empty group saying so.
export function without(node: Layout, id: number): Layout {
  if ("group" in node) {
    return node;
  }

  const [near, far] = node.parts;

  for (const [gone, kept] of [
    [near, far],
    [far, near],
  ] as const) {
    if ("group" in gone.node && gone.node.group.id === id) {
      return kept.node;
    }
  }

  return {
    split: node.split,
    parts: node.parts.map((part) => ({
      ...part,
      node: without(part.node, id),
    })) as [Part, Part],
  };
}

/// Which group takes the room when this one goes: the first of whatever stood
/// on the other side of the split it was in.
///
/// What the active group becomes when the active one is closed — the room its
/// tabs were in is where the eye already is. Nothing where the tree is that one
/// group, there being nowhere else to go.
export function neighbour(node: Layout, id: number): Group | undefined {
  if ("group" in node) {
    return undefined;
  }

  const [near, far] = node.parts;

  for (const [gone, kept] of [
    [near, far],
    [far, near],
  ] as const) {
    if ("group" in gone.node && gone.node.group.id === id) {
      return groups(kept.node)[0];
    }
  }

  return node.parts.reduce<Group | undefined>(
    (found, part) => found ?? neighbour(part.node, id),
    undefined,
  );
}
