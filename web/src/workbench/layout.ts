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
//! `rem` would not. Dragging one is stage 02's next task; what is here is where
//! they sit.
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
  within: Rect = { x: 0, y: 0, width: 100, height: 100 },
  into: Record<number, Rect> = {},
): Record<number, Rect> {
  if ("group" in node) {
    into[node.group.id] = within;
    return into;
  }

  const across = node.split === "beside";
  let along = across ? within.x : within.y;

  for (const part of node.parts) {
    const room = ((across ? within.width : within.height) * part.share) / 100;

    placed(
      part.node,
      across
        ? { ...within, x: along, width: room }
        : { ...within, y: along, height: room },
      into,
    );

    along += room;
  }

  return into;
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
