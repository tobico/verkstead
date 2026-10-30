//! What the open Conversation can go to, as the page tells the app: the list the
//! Mac's Go menu is greyed and enabled by.
//!
//! **The stub bridge is the app being told**, and what it keeps is every list it
//! was handed, in order — so a push that should not have been made is one more
//! entry, and the last entry is what the menu would be drawn from. Navigating is
//! the app's other half, a clicked notification's path, which routes the page in
//! place the way a press on the menu will.
//!
//! **Mounted through the real gate**, so the routes the list changes across are
//! the table the app builds, and the pushes are made by the page's own shell and
//! workbench rather than by anything a test called.

import { render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ConversationView, TimelineEvent } from "../src/api/types";
import { Gate } from "../src/App";
import { NAME, WHERE, type Bridge, type Where } from "../src/settings/bridge";
import { NOWHERE, tell } from "../src/where";
import { OPEN, SET_UP, theWorkbench } from "./bench";
import { askedFor, json, whenever } from "./serving";
import tasks from "./fixtures/conversation-tasks.json" with { type: "json" };

afterEach(() => {
  vi.unstubAllGlobals();
});

/// The backlog the tasks fixture landed, as its row on the record.
const BACKLOG = (tasks as ConversationView).timeline.find(
  (event) => "TaskList" in event,
)!;

/// And two roadmaps landed on one row, of which the Go menu names the last.
const ROADMAPS: TimelineEvent = {
  StageList: {
    id: 40,
    at: "2026-08-03T09:08:00.000Z",
    roadmaps: [
      { name: "mvp", title: "MVP", stages: [] },
      { name: "after", title: "After", stages: [] },
    ],
  },
};

/// And a roadmap landing after them, which a Timeline that is advancing opens.
const LATER: TimelineEvent = {
  StageList: {
    id: 41,
    at: "2026-08-03T09:09:00.000Z",
    roadmaps: [{ name: "later", title: "Later", stages: [] }],
  },
};

/// A Conversation with both lists on its record, and one with neither — the
/// fixture's own, which is a Brief and nothing else.
const PLANNED: ConversationView = {
  ...OPEN,
  timeline: [...OPEN.timeline, BACKLOG, ROADMAPS],
};
const BARE = 7;

/// A member of this device's cluster, and where a call for it stands.
const MEMBER = "8f2a1c0b4d6e7f902b13c4d5e6f70819";
const on = (path: string) => `/api/ui/members/${MEMBER}${path}`;

/// The app's window, stood in for: every list the page told it, and the way to
/// take the page somewhere.
type Told = {
  readonly lists: Where[];
  go(path: string): void;
};

function standingIn(): Told {
  const lists: Where[] = [];
  let heard: ((path: string) => void) | undefined;

  vi.stubGlobal(NAME, {
    platform: "darwin",
    settings: () =>
      Promise.resolve({ whenClosed: "tray", trayIcon: true, notifications: true }),
    set: () =>
      Promise.resolve({ whenClosed: "tray", trayIcon: true, notifications: true }),
    logs: () => Promise.resolve(),
    startup: () => Promise.resolve({ possible: false, on: false }),
    register: () => Promise.resolve({ possible: false, on: false }),
    notifiable: () => Promise.resolve(true),
    placed: () => Promise.resolve({ outside: false }),
    install: () => Promise.resolve({}),
    head: () => {},
    where: (open) => {
      lists.push(open);
    },
    opening: (listener) => {
      heard = listener;
      return () => {
        heard = undefined;
      };
    },
  } satisfies Bridge);

  return {
    lists,
    go(path) {
      if (heard === undefined) {
        throw new Error("nothing on the page is listening");
      }
      heard(path);
    },
  };
}

/// The panes the last list named, which is what the menu would enable.
function last(told: Told): Array<[string, string]> {
  return (told.lists.at(-1)?.panes ?? []).map(({ pane, path }) => [pane, path]);
}

/// The app at `path`, set up, with the planned Conversation at the fixture's id
/// — what it holds read at each request, so a test can land a backlog on it —
/// the bare one beside it, and the planned one again on a member.
function at(path: string, planned: () => ConversationView = () => PLANNED) {
  window.history.pushState({}, "", path);
  const fetching = theWorkbench(
    whenever("/api/ui/onboarding", json(SET_UP)),
    whenever(`/api/ui/conversations/${OPEN.id}`, () => json(planned())()),
    whenever(`/api/ui/conversations/${BARE}`, json({ ...OPEN, id: BARE })),
    whenever(on(`/conversations/${OPEN.id}`), json(PLANNED)),
    // Anything else either device is asked for is a refusal, so a pane these
    // tests are not about draws its error and says nothing here.
    json({ error: "nothing serves that" }, 404),
  );

  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  render(() => (
    <QueryClientProvider client={client}>
      <Gate />
    </QueryClientProvider>
  ));

  return { client, fetching };
}

describe("what the open Conversation can go to", () => {
  it("follows the page as it navigates", async () => {
    const told = standingIn();
    at("/settings");

    await waitFor(() => expect(told.lists).toHaveLength(1));
    expect(last(told)).toEqual([]);

    const planned = `/conversations/${OPEN.id}`;
    told.go(planned);
    await waitFor(() =>
      expect(last(told)).toEqual([
        ["timeline", `${planned}/timeline`],
        ["code", `${planned}/code`],
        ["backlog", `${planned}/backlog`],
        ["roadmap", `${planned}/roadmaps/after`],
        ["share", `${planned}/share`],
      ]),
    );

    const bare = `/conversations/${BARE}`;
    told.go(bare);
    await waitFor(() =>
      expect(last(told)).toEqual([
        ["timeline", `${bare}/timeline`],
        ["code", `${bare}/code`],
        ["share", `${bare}/share`],
      ]),
    );

    const member = `/devices/${MEMBER}/conversations/${OPEN.id}`;
    told.go(member);
    await waitFor(() =>
      expect(last(told)).toEqual([
        ["timeline", `${member}/timeline`],
        ["code", `${member}/code`],
        ["backlog", `${member}/backlog`],
        ["roadmap", `${member}/roadmaps/after`],
        ["share", `${member}/share`],
      ]),
    );

    told.go("/compose");
    await waitFor(() => expect(last(told)).toEqual([]));
    const emptied = told.lists.length;

    // And nothing again on the next two pages with nothing open, the bare
    // workbench and a page that is no such page: the list has not changed.
    told.go("/");
    await waitFor(() => expect(window.location.pathname).toBe("/"));
    told.go("/no/such/page");
    await waitFor(() => expect(window.location.pathname).toBe("/no/such/page"));
    expect(told.lists).toHaveLength(emptied);
  });

  it("says a list once, and again only when the record changes it", async () => {
    let planned: ConversationView = OPEN;
    const told = standingIn();
    const { client, fetching } = at(`/conversations/${OPEN.id}`, () => planned);
    const read = `/api/ui/conversations/${OPEN.id}`;

    await waitFor(() => expect(last(told)).toHaveLength(3));
    const before = told.lists.length;

    // The same record read again, which is what nearly every re-read is.
    const reads = askedFor(fetching, read);
    await client.invalidateQueries();
    await client.invalidateQueries();
    expect(askedFor(fetching, read)).toBe(reads + 2);
    expect(told.lists).toHaveLength(before);

    // And the backlog landing mid-session.
    planned = { ...OPEN, timeline: [...OPEN.timeline, BACKLOG] };
    await client.invalidateQueries();
    await waitFor(() => expect(told.lists).toHaveLength(before + 1));
    expect(last(told).map(([pane]) => pane)).toEqual([
      "timeline",
      "code",
      "backlog",
      "share",
    ]);

    await client.invalidateQueries();
    expect(told.lists).toHaveLength(before + 1);
  });
});

describe("the Timeline the menu goes to", () => {
  /// The Conversation's own path lands on the end of its record; the
  /// Timeline's path is the details pane closed, and kept closed.
  it("opens nothing beside it, however the record moves", async () => {
    let planned: ConversationView = PLANNED;
    const told = standingIn();
    const conversation = `/conversations/${OPEN.id}`;
    const { client } = at(`${conversation}/share`, () => planned);

    await waitFor(() => expect(last(told)).toHaveLength(5));

    // Where it would have landed, as a control: the bare path is rewritten.
    told.go(conversation);
    await waitFor(() =>
      expect(window.location.pathname).not.toBe(conversation),
    );
    const landed = window.location.pathname;
    expect(landed.startsWith(`${conversation}/`)).toBe(true);

    told.go(`${conversation}/timeline`);
    await waitFor(() =>
      expect(window.location.pathname).toBe(`${conversation}/timeline`),
    );

    // And a re-read, and a new Event landing on a Timeline that was
    // advancing, both leave it where it was asked to be.
    await client.invalidateQueries();
    planned = { ...PLANNED, timeline: [...PLANNED.timeline, LATER] };
    await client.invalidateQueries();
    await new Promise((settled) => setTimeout(settled, 50));
    expect(window.location.pathname).toBe(`${conversation}/timeline`);

    // Back is the landed pane it came from.
    window.history.back();
    await waitFor(() => expect(window.location.pathname).toBe(landed));
  });
});

describe("outside the app", () => {
  /// A browser and a phone have no bridge, and nobody to tell.
  it("tells nobody", () => {
    expect(() => tell(NOWHERE)).not.toThrow();
  });
});

describe("the channel it is told on", () => {
  /// The literal the preload sends on — pinned on this side as
  /// `desktop/tests/bridge.test.ts` pins it on that one.
  it("is the app's own word for where", () => {
    expect(WHERE).toBe("verkstead:where");
  });
});
