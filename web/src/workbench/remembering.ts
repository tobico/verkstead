//! What the device remembers of Code: the splits, the tabs and the text nobody
//! has saved.
//!
//! [`./keeping`] holds all of that above the frame's switch, which carries it
//! through a pane swapped for an Event and back. This carries it through a
//! **reload**, which is the other thing a human does to a page — and the one
//! that would otherwise cost them the text they had typed and not saved (ADR
//! 0019, *Tabs and groups*).
//!
//! **Per Conversation, and per device.** A tab is a path in a Worktree and a
//! terminal is a number in one Conversation's register, so there is a key per
//! Conversation; and it is the browser's storage rather than the server's
//! record because a device's layout is a device's, exactly as the pane widths
//! beside it are — a phone that opened Code has nothing to say about a laptop's
//! splits. Under the same `verkstead.*` namespace as everything else this app
//! leaves in a browser, through `device.ts`.
//!
//! **The layout is written apart from the text**, which is the one arrangement
//! decision here. A dirty file may be two megabytes and a browser's storage is
//! a few, so a Conversation holding two large unsaved files can overrun it: two
//! keys and two writes mean a storage too full for the text still comes back to
//! the splits and the tabs, the text being what is lost and never the layout.
//! And a text write that did not land takes the key with it rather than leaving
//! what was under it — stale text restored over a file it no longer describes
//! would be worse than no text at all.
//!
//! **A stored body that will not parse, or is not the shape of one of these, is
//! dropped on the way past**, the way the compose page's own draft is (see
//! `composing.ts`): it will be no more use on the next visit than it is on this
//! one, and a layout half-read would be a pane drawn out of somebody else's
//! idea of what this key meant.
//!
//! **What is *not* here is the readings.** A file's text and the version it was
//! read at are the disk's to answer, and a page coming back reads every open
//! file afresh: the human's unsaved text goes over the top of what comes back,
//! so a file the agent rewrote overnight is dirty against what is really there
//! rather than against a version that is no longer on the disk. The terminals
//! are not here either, for the near reason — the register is the server's, and
//! the tabs are settled against it on every opening (see `Code.tsx`).

import { createEffect, type Accessor } from "solid-js";

import { forget, read, write } from "../device";
import type { Tab } from "./keeping";
import {
  group as made,
  type Group,
  type Layout,
  type Part,
  type Way,
} from "./layout";

/// Where one Conversation's layout lives on this device. Keyed by the
/// Conversation, the way a Set's own draft is keyed by the Set.
function layoutKey(conversation: number): string {
  return `verkstead.code.${conversation}`;
}

/// And where the text nobody has saved in it lives, which is a key of its own
/// for the reason at the top of this file.
function unsavedKey(conversation: number): string {
  return `verkstead.code-unsaved.${conversation}`;
}

/// One group as it is written down: what names it, what is in it, and which of
/// them it was showing.
///
/// The group's id travels with it because a layout is a tree of ids — the
/// active one is named by id, and a restored keeping goes on counting from the
/// highest of them so that a group opened after the reload is a group nothing
/// else answers to.
interface Held {
  id: number;
  tabs: Tab[];
  chosen?: string;
}

/// And the tree they stand in, which is [`Layout`] with the signals taken out:
/// what a group holds is written down here rather than read off it.
type Stood = { group: Held } | { split: Way; parts: [Share, Share] };

/// One half of a written-down split.
interface Share {
  share: number;
  node: Stood;
}

/// The whole of what one Conversation's Code looked like: how it was divided,
/// and which group was last pressed into.
export interface Remembered {
  layout: Stood;
  active: number;
}

/// What this device last had open in a Conversation, or nothing where it had
/// nothing.
///
/// A body that will not parse, or is not the shape of one of these, is dropped
/// on the way past — and taken away with it, the way the compose page's draft
/// is.
export function remembered(conversation: number): Remembered | null {
  const body = read(layoutKey(conversation));

  if (body === null) {
    return null;
  }

  const held = parsed(body);

  if (held === null) {
    forget(layoutKey(conversation));
    return null;
  }

  return held;
}

/// And the text it was holding for each file that had any, by path.
///
/// Empty where there is none, where what is there will not parse, or where it is
/// not a record of strings: there is nothing to restore either way, and what is
/// under the key is taken away.
export function recalled(conversation: number): Record<string, string> {
  const body = read(unsavedKey(conversation));

  if (body === null) {
    return {};
  }

  const held = texts(body);

  if (held === null) {
    forget(unsavedKey(conversation));
    return {};
  }

  return held;
}

/// The tree a stored layout comes back as: live groups, with the tabs and the
/// one showing put back into each.
///
/// And the two numbers that go with it — which group is active, and the highest
/// id in the tree, which is where the keeping's own counting takes up again.
/// An active group the tree does not have reads as the first of them: something
/// has to be active, and the first is where the eye starts.
export function revived(stored: Remembered): {
  layout: Layout;
  active: number;
  last: number;
} {
  let last = 0;
  const live: Group[] = [];

  const put = (node: Stood): Layout => {
    if ("group" in node) {
      const one = made(node.group.id);

      one.setTabs(node.group.tabs);

      if (node.group.chosen !== undefined) {
        one.setChosen(node.group.chosen);
      }

      last = Math.max(last, node.group.id);
      live.push(one);

      return { group: one };
    }

    return {
      split: node.split,
      parts: node.parts.map((part) => ({
        share: part.share,
        node: put(part.node),
      })) as [Part, Part],
    };
  };

  const layout = put(stored.layout);
  const active = live.some((one) => one.id === stored.active)
    ? stored.active
    : live[0]!.id;

  return { layout, active, last };
}

/// Write one Conversation's Code down as it stands, and go on writing it down
/// as it moves.
///
/// Two effects rather than one, which is the two keys again: the layout is
/// written when the layout moves, and the text when somebody types. One effect
/// would re-serialise a megabyte of unsaved file every time a tab was turned
/// to, and a tree every time a key was pressed.
///
/// Made where the keeping is — above the frame that swaps the details pane — so
/// that what they follow outlives every pane that draws it.
export function remembering(
  conversation: number,
  held: {
    layout: Accessor<Layout>;
    active: Accessor<number>;
    /// The text of every buffer the disk has not got, by path.
    unsaved: () => Record<string, string>;
  },
): void {
  createEffect(() => keepLayout(conversation, held.layout(), held.active()));
  createEffect(() => keepText(conversation, held.unsaved()));
}

/// The layout, written down — or taken away, where there is nothing open.
///
/// A pane holding one empty group is a pane nobody has opened anything in, and
/// a stored layout of one would restore as exactly what an untouched browser
/// already gives: nothing to come back to, and a key left behind saying so.
function keepLayout(conversation: number, layout: Layout, active: number): void {
  const written = writing(layout);

  if ("group" in written && written.group.tabs.length === 0) {
    forget(layoutKey(conversation));
    return;
  }

  write(
    layoutKey(conversation),
    JSON.stringify({ layout: written, active } satisfies Remembered),
  );
}

/// And the text, written down beside it — or taken away, where there is none of
/// it and where it would not fit.
///
/// The refusal is the whole reason [`write`] answers at all. A storage that is
/// full leaves whatever was under the key, and what is under this one is the
/// text of a page that has moved on since: restored, it would be put over a file
/// it no longer describes and called the human's unsaved work. So it goes, and
/// what comes back is the file as the disk has it.
function keepText(conversation: number, unsaved: Record<string, string>): void {
  const key = unsavedKey(conversation);

  if (Object.keys(unsaved).length === 0) {
    forget(key);
    return;
  }

  if (!write(key, JSON.stringify(unsaved))) {
    forget(key);
  }
}

/// The tree as it is written down: the groups' signals read out into values.
function writing(node: Layout): Stood {
  if ("group" in node) {
    const chosen = node.group.chosen();

    return {
      group: {
        id: node.group.id,
        tabs: node.group.tabs(),
        ...(chosen === undefined ? {} : { chosen }),
      },
    };
  }

  return {
    split: node.split,
    parts: node.parts.map((part) => ({
      share: part.share,
      node: writing(part.node),
    })) as [Share, Share],
  };
}

/// And back the other way, with every field met: a body from another version of
/// this page, or one somebody has edited by hand, is no layout at all.
function parsed(body: string): Remembered | null {
  let payload: unknown;

  try {
    payload = JSON.parse(body);
  } catch {
    return null;
  }

  if (typeof payload !== "object" || payload === null) {
    return null;
  }

  const held = payload as Partial<Remembered>;

  if (typeof held.active !== "number" || !Number.isFinite(held.active)) {
    return null;
  }

  const layout = stood(held.layout);

  return layout === null ? null : { layout, active: held.active };
}

/// One node of a stored tree, or nothing where it is not one.
///
/// Every field of it, because the pane is drawn straight off what comes back: a
/// split missing a share is a group with no room, and a tab that is neither a
/// path nor a number is a box the pane has nothing to put in.
function stood(value: unknown): Stood | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }

  if ("group" in value) {
    const held = (value as { group: unknown }).group;

    if (typeof held !== "object" || held === null) {
      return null;
    }

    const { id, tabs, chosen } = held as Partial<Held>;

    if (
      typeof id !== "number" ||
      !Number.isFinite(id) ||
      !Array.isArray(tabs) ||
      !(chosen === undefined || typeof chosen === "string")
    ) {
      return null;
    }

    const open: Tab[] = [];

    for (const one of tabs as unknown[]) {
      const tab = opened(one);

      if (tab === null) {
        return null;
      }

      open.push(tab);
    }

    return { group: { id, tabs: open, ...(chosen === undefined ? {} : { chosen }) } };
  }

  const { split, parts } = value as { split?: unknown; parts?: unknown };

  if (
    (split !== "beside" && split !== "below") ||
    !Array.isArray(parts) ||
    parts.length !== 2
  ) {
    return null;
  }

  const halves: Share[] = [];

  for (const part of parts as unknown[]) {
    if (typeof part !== "object" || part === null) {
      return null;
    }

    const { share, node } = part as { share?: unknown; node?: unknown };

    if (typeof share !== "number" || !Number.isFinite(share)) {
      return null;
    }

    const under = stood(node);

    if (under === null) {
      return null;
    }

    halves.push({ share, node: under });
  }

  return { split, parts: halves as [Share, Share] };
}

/// One stored tab: a terminal by its number, or a file by its path.
function opened(value: unknown): Tab | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }

  const tab = value as { terminal?: unknown; file?: unknown };

  if (typeof tab.terminal === "number" && Number.isFinite(tab.terminal)) {
    return { terminal: tab.terminal };
  }

  return typeof tab.file === "string" ? { file: tab.file } : null;
}

/// And the text map, met the same way: a record of strings, or nothing.
function texts(body: string): Record<string, string> | null {
  let payload: unknown;

  try {
    payload = JSON.parse(body);
  } catch {
    return null;
  }

  if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
    return null;
  }

  const held: Record<string, string> = {};

  for (const [path, text] of Object.entries(payload)) {
    if (typeof text !== "string") {
      return null;
    }

    held[path] = text;
  }

  return held;
}
