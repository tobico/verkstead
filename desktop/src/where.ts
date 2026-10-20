//! What the open Conversation can go to, as the page says it — the list the
//! Mac's Go menu is greyed and enabled by.
//!
//! **The page tells the app, not the other way round** (Set 1283 Q7). Which
//! Conversation is open, which device it is on and what its record holds are the
//! page's to know, so the paths are built there, by the page's own path builders,
//! and what arrives here is a list a menu can be drawn from: each pane in the Go
//! menu's order, with the path the page routes to for it, and nothing where no
//! Conversation is open. See [`WHERE`](./bridge.js), and `where.ts` in the viewer
//! for the side that builds it.
//!
//! **And what arrives is checked before it is held**, the way a set from the page
//! is checked before it reaches the settings file: a bridge that trusted its
//! renderer would only have moved the question. A push of the wrong shape is
//! [`where`] answering `undefined`, and the edge drops it and says so.

/// One of the places the Go menu goes to, in the order it draws them.
export const PANES = ["timeline", "code", "backlog", "roadmap", "share"] as const;

/// One of the five.
export type Pane = (typeof PANES)[number];

/// What the open Conversation can go to: the panes it has, in the Go menu's
/// order, each with the path the page routes to for it — or none at all where no
/// Conversation is open.
export interface Where {
  readonly panes: ReadonlyArray<{ readonly pane: Pane; readonly path: string }>;
}

/// What the app holds before the page has said anything: nowhere, which is what
/// greys every pane item.
export const NOWHERE: Where = { panes: [] };

/// What the page pushed, where it is the shape: an object with a list of panes,
/// each one of the five at most once and each at a path of the workbench's own.
///
/// **A path of the workbench's own** is the one a clicked notice's is held to in
/// [`opens`](./notices.js): one slash in front and no backslash anywhere, a path
/// with two in front being another host to the router as it is to a browser. A
/// press on the menu is a navigation of the app's own window, as a click is.
export function where(pushed: unknown): Where | undefined {
  if (typeof pushed !== "object" || pushed === null) {
    return undefined;
  }

  const { panes } = pushed as Record<string, unknown>;

  if (!Array.isArray(panes)) {
    return undefined;
  }

  const read: Array<{ pane: Pane; path: string }> = [];

  for (const each of panes as unknown[]) {
    if (typeof each !== "object" || each === null) {
      return undefined;
    }

    const { pane, path } = each as Record<string, unknown>;

    if (!PANES.includes(pane as Pane) || read.some((had) => had.pane === pane)) {
      return undefined;
    }

    if (
      typeof path !== "string" ||
      !path.startsWith("/") ||
      path.startsWith("//") ||
      path.includes("\\")
    ) {
      return undefined;
    }

    read.push({ pane: pane as Pane, path });
  }

  return { panes: read };
}
