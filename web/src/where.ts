//! What the open Conversation can go to, told to the app.
//!
//! **The Mac's Go menu is drawn from this** (Set 1283 Q7): Timeline, Code,
//! Backlog, Roadmap and Share, each enabled where the open Conversation has one.
//! Which Conversation is open, which device it is on and what its record holds
//! are all the page's to know, so the paths are built here, by the page's own
//! path builders, and the app is told a list it can draw a menu from. A push,
//! like the head's colours: nothing comes back.
//!
//! **Pushed only when it changes.** The workbench re-reads a Conversation every
//! time anything about it moves, and nearly every re-read leaves the list as it
//! was — so what was last said is kept, per bridge, and a list equal to it is
//! not said again. The backlog landing mid-session is the change that is.
//!
//! **And none of it reaches a browser.** A browser on this machine and a phone on
//! the tailnet have no bridge, so [`tell`] finds nobody to tell and says nothing,
//! exactly as [`dress`](./head.ts) does.

import type { ConversationView } from "./api/types";
import type { Device } from "./reaching";
import { bridge, type Bridge, type Where } from "./settings/bridge";
import { opensRoadmap, pathOf, pathTo } from "./workbench/openings";

/// What a page with no Conversation open can go to: nothing. The settings page,
/// the compose page, the bare workbench and a page that is no such page.
export const NOWHERE: Where = { panes: [] };

/// What the Conversation can go to, in the Go menu's order, each at the path the
/// page routes to for it.
///
/// **Timeline** is the Conversation's own path, which closes whatever details
/// pane is open; **Code** and **Share** are always there. **Backlog** is there
/// where the record has a task list still readable — the rule the backlog card
/// follows — and **Roadmap** where it landed one, naming the last roadmap of the
/// last stage list: one item, however many roadmaps the Worktree holds.
///
/// Steer is an act rather than a place, and is not among them.
export function whereIn(
  conversation: ConversationView,
  id: string | number,
  device: Device,
): Where {
  const timeline = conversation.timeline;

  const backlog = timeline.some(
    (event) => "TaskList" in event && event.TaskList.list !== null,
  );

  const roadmap = timeline
    .flatMap((event) => ("StageList" in event ? [event.StageList] : []))
    .at(-1)
    ?.roadmaps.at(-1)?.name;

  return {
    panes: [
      { pane: "timeline", path: pathOf(id, device) },
      { pane: "code", path: pathTo(id, "code", device) },
      ...(backlog
        ? [{ pane: "backlog" as const, path: pathTo(id, "backlog", device) }]
        : []),
      ...(roadmap === undefined
        ? []
        : [
            {
              pane: "roadmap" as const,
              path: pathTo(id, opensRoadmap(roadmap), device),
            },
          ]),
      { pane: "share", path: pathTo(id, "share", device) },
    ],
  };
}

/// What each bridge was last told, so that a list it already has is not said
/// again. Keyed by the bridge rather than held once, because the bridge is what
/// was told: a window whose page came up again has a bridge of its own.
const told = new WeakMap<Bridge, string>();

/// Tell the app what the open Conversation can go to, where that is not what it
/// was last told. Nothing at all outside the app.
export function tell(where: Where): void {
  const app = bridge();

  if (app === null) {
    return;
  }

  const said = JSON.stringify(where);

  if (told.get(app) === said) {
    return;
  }

  told.set(app, said);
  app.where(where);
}
