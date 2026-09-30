//! A Conversation that lives on another device, opened through this one.
//!
//! The whole of the viewer's device dimension is one value read off the URL —
//! see `src/reaching.ts` — and what it changes is three things: where a call
//! goes, which entry of the cache an answer lands in, and which URL a press
//! writes. So what is asked here is exactly those three, over the page the app
//! really builds: the Conversation on the far end draws, every press on it
//! lands there, and this device's own work is untouched beside it.
//!
//! **This device is asked for nothing of the member's.** The browser holds one
//! cookie and one origin; the device it opened puts the call to the member and
//! hands the answer back (ADR-0020, *The opened device relays*). So a member's
//! call is `/api/ui/members/{device}/…` and this device's is what it always
//! was, and a test that found the same path under both would have found a page
//! reading the wrong Verkstead.
//!
//! The golden fixtures are `workbench.test.tsx`'s: `cargo test` renders the
//! real endpoints and writes them, and a member answers the very same shapes —
//! the far end is this same viewer's namespace, served over the Peer Listener.

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  ConversationArchived,
  ConversationEntry,
  ConversationView,
  DevicesView,
  DroppedRow,
  QuestionSetEvent,
  RepoEntry,
  Submitted,
} from "../src/api/types";
import {
  RefusedError,
  loadConversation,
  retrying,
} from "../src/api/client";
import { osIcon } from "../src/devices";
import { Reaching, rowKey } from "../src/reaching";
import sheet from "../src/set/Sheet.module.css";
import { Asked } from "../src/workbench/Asked";
import dropdown from "../src/Menu.module.css";
import shell from "../src/Panes.module.css";
import actions from "../src/workbench/Actions.module.css";
import paneHead from "../src/workbench/PaneHead.module.css";
import setup from "../src/workbench/Setup.module.css";
import steerForm from "../src/workbench/Steer.module.css";
import sidebar from "../src/workbench/Conversations.module.css";
import timeline from "../src/workbench/Timeline.module.css";
import {
  BRANCHES,
  HIDING_ARCHIVED,
  OPEN,
  PROFILES,
  REPOS,
  SET_UP,
  SIDEBAR,
  drawn,
  mount,
} from "./bench";
import { offered, press } from "./pickers";
import {
  askedFor,
  json,
  readable,
  reads,
  serving,
  whenever,
  type Answer,
} from "./serving";
import { pending } from "./steering";
import grilling from "./fixtures/conversation-grilling.json" with {
  type: "json",
};
import waiting from "./fixtures/set-answering.json" with { type: "json" };

/// The member, by the Device Id the URL carries: sixteen random hex bytes, as
/// every Verkstead's is.
const MEMBER = "8f2a1c0b4d6e7f902b13c4d5e6f70819";

/// Where a call for that member stands. The prefix takes the place of
/// `/api/ui`, so the far end sees the path the browser would have written
/// locally — see `relaying.rs`, which is this device in the middle.
const at = (path: string) => `/api/ui/members/${MEMBER}${path}`;

/// And where the page it is drawn on stands.
const on = (path: string) => `/devices/${MEMBER}${path}`;

/// The Conversation this device holds, and the one the member holds — the same
/// id, which is the point of the exercise: every Verkstead issues a 3, so the
/// two collide by construction and nothing but the device tells them apart.
const HERE = grilling as ConversationView;
const THERE: ConversationView = { ...HERE, branch: "over-on-the-member" };

/// The workbench with a member on the other end of it: this device answers for
/// its own sidebar and nothing else, and everything the open Conversation is
/// drawn from comes back under the member's prefix.
function theMember(...answers: Answer[]) {
  return serving(
    // This device's own, and the whole of what it is asked for: the sidebar
    // lists the work being done here, whichever Conversation is open beside it.
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/onboarding", json(SET_UP)),
    // And the member's, every one of them under the prefix the relay stands at.
    whenever(at(`/conversations/${THERE.id}`), json(THERE)),
    whenever(at(`/conversations/${OPEN.id}`), json(OPEN)),
    whenever(at("/repos"), json(REPOS)),
    whenever(at("/profiles"), json(PROFILES)),
    ...REPOS.map((repo) =>
      whenever(at(`/repos/${repo.id}/branches`), json(BRANCHES)),
    ),
    ...answers,
    // Anything neither side was asked for is a refusal rather than a throw, so
    // a pane this test is not about draws its error and says nothing here.
    json({ error: "nothing serves that" }, 404),
  );
}

/// The same two Conversations with this device answering for its own as well,
/// which is what a page walking between them needs.
function bothDevices(...answers: Answer[]) {
  return theMember(
    whenever(`/api/ui/conversations/${HERE.id}`, json(HERE)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    ...answers,
  );
}

/// What the middle pane's header is calling the Conversation, which is its
/// branch: the one thing on the page that says *which* of the two records is
/// drawn.
async function titled(container: ParentNode): Promise<string> {
  const title = await drawn(
    container,
    `.${shell.middlePane} .${paneHead.head} h1 .${timeline.paneTitle}`,
  );

  return title.textContent ?? "";
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("a conversation that lives on another device", () => {
  it("draws the member's record, read through this device", async () => {
    const fetching = theMember();
    const { container } = mount(on(`/conversations/${THERE.id}`));

    await waitFor(async () => expect(await titled(container)).toBe(THERE.branch));

    expect(askedFor(fetching, at(`/conversations/${THERE.id}`))).toBeGreaterThan(
      0,
    );
  });

  /// Which is the half that makes it the *member's*: the same path unprefixed
  /// is this device's own Conversation 3, and nothing here asks for it.
  it("asks this device for nothing of the member's", async () => {
    const fetching = theMember();
    const { container } = mount(on(`/conversations/${THERE.id}`));

    await waitFor(async () => expect(await titled(container)).toBe(THERE.branch));

    expect(askedFor(fetching, `/api/ui/conversations/${THERE.id}`)).toBe(0);
  });

  /// And the sidebar beside it is read off this device as it always was: the
  /// list is merged from the whole cluster now, but the merging is the hub's —
  /// the browser asks the one endpoint it has always asked and never a member's
  /// own (ADR-0020, *The opened device relays*).
  it("reads the sidebar off this device alone, merged or not", async () => {
    const fetching = theMember();
    const { container } = mount(on(`/conversations/${THERE.id}`));

    await drawn(container, `.${shell.conversationsPane}`);

    await waitFor(() =>
      expect(askedFor(fetching, "/api/ui/conversations")).toBeGreaterThan(0),
    );
    expect(askedFor(fetching, at("/conversations"))).toBe(0);
  });

  /// The leaves are the leaves they always were — the device says which
  /// Verkstead and the leaf says which pane — so a link to a pane of a member's
  /// Conversation opens that pane on a cold load, and what the pane points at
  /// is on the member.
  it("opens the pane a nested path names, pointed at the member", async () => {
    theMember();
    const { container } = mount(on(`/conversations/${THERE.id}/share`));

    const download = await drawn<HTMLAnchorElement>(
      container,
      `.${shell.detailsPane} a[download]`,
    );

    // The share is a file the browser fetches for itself rather than a request
    // this page makes, so the device has to be in the link: a bare path here
    // would download this device's Conversation 3.
    expect(download.getAttribute("href")).toBe(
      at(`/conversations/${THERE.id}/share`),
    );
  });

  /// And looking at it is recorded where the record is: the mark is the
  /// member's to take off its own row.
  it("marks it looked at on the member", async () => {
    const seen = at(`/conversations/${THERE.id}/seen`);
    const fetching = theMember(whenever(seen, json({}), "POST"));
    mount(on(`/conversations/${THERE.id}`));

    await waitFor(() =>
      expect(
        fetching.mock.calls.filter(
          ([asked, init]) => String(asked) === seen && init?.method === "POST",
        ),
      ).toHaveLength(1),
    );
  });
});

describe("a press on a conversation that lives on another device", () => {
  /// The actions menu, which is the press every state of a Conversation has:
  /// it goes to the member, and the address it leaves the page at is the
  /// member's too.
  it("puts a steer to the member and stays on its address", async () => {
    const steering = at(`/conversations/${THERE.id}/steer`);

    // The member as the press leaves it: the drive stopped and a pending steer
    // standing, which is what a read after the press comes back with.
    let taken = false;
    const fetching = theMember(
      whenever(at(`/conversations/${THERE.id}`), () =>
        json(taken ? { ...THERE, pending_steer: pending() } : THERE)(),
      ),
      whenever(
        steering,
        () => {
          taken = true;
          return json("Opened")();
        },
        "POST",
      ),
      whenever(at(`/conversations/${THERE.id}/seen`), json({}), "POST"),
    );
    const { container, history } = mount(on(`/conversations/${THERE.id}`));

    fireEvent.click(
      await drawn(container, `.${actions.conversationActions} > .${dropdown.trigger}`),
    );
    const menu = await drawn(
      container,
      `.${actions.conversationActions} > .${dropdown.drop}`,
    );
    fireEvent.click(await drawn(menu, `.${actions.steer}`));

    await waitFor(() =>
      expect(
        fetching.mock.calls.filter(
          ([asked, init]) =>
            String(asked) === steering && init?.method === "POST",
        ),
      ).toHaveLength(1),
    );
    await waitFor(() =>
      expect(history.get()).toBe(on(`/conversations/${THERE.id}/steer`)),
    );
    await drawn(container, `.${steerForm.steerConversation}`);
  });

  /// And the Repo dropdown's **Open repo**, which is the press that reaches
  /// furthest into the machine the work is on: the path typed into it is a path
  /// on the member, and the registry it lands on is the member's.
  it("registers a repo on the member, and moves the draft onto it", async () => {
    const opened: RepoEntry = {
      id: 4242,
      name: "widgets",
      path: "/srv/repos/widgets",
      default_branch: "main",
    };
    const fetching = theMember(
      whenever(at("/repos"), json({ Added: opened }), "POST"),
      whenever(at(`/conversations/${OPEN.id}/repo`), json("Switched"), "POST"),
      whenever(at(`/conversations/${OPEN.id}/seen`), json({}), "POST"),
      ...OPEN.companions.map((companion) =>
        whenever(at(`/repos/${companion.repo.id}/branches`), json(BRANCHES)),
      ),
    );
    const { container } = mount(on(`/conversations/${OPEN.id}`));

    fireEvent.click(
      await drawn<HTMLButtonElement>(container, `.${setup.repoOption} > button`),
    );
    await waitFor(() => expect(offered("Repo").length).toBe(REPOS.length));
    press("Repo", "Open repo");

    fireEvent.input(
      await waitFor(() => screen.getByLabelText(/absolute path/i)),
      { target: { value: opened.path } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() =>
      expect(
        fetching.mock.calls.filter(
          ([asked, init]) =>
            String(asked) === at("/repos") && init?.method === "POST",
        ),
      ).toHaveLength(1),
    );
    // And the move behind it, which is the registration landing on the draft:
    // both halves are the member's, because both are about work being done
    // there.
    await waitFor(() =>
      expect(
        fetching.mock.calls.filter(
          ([asked, init]) =>
            String(asked) === at(`/conversations/${OPEN.id}/repo`) &&
            init?.method === "POST",
        ),
      ).toHaveLength(1),
    );
    expect(askedFor(fetching, "/api/ui/repos")).toBe(0);
  });
});

/// The sidebar as the cluster's one list: rows of this device's and rows of a
/// member's, side by side under one set of ranks.
///
/// **What is asked here is that a row is a Conversation *and* a device.** Every
/// Verkstead issues a Conversation 1, so a list merged from two of them holds
/// two rows nothing but the device tells apart — and a press addressed by the
/// number alone would act on whichever of the two the page found first, on the
/// very list the press was not about.
describe("the merged sidebar", () => {
  /// Both devices' rows, as the hub answers them: this device's own carrying a
  /// device block with no id, and the member's carrying its id, name and OS
  /// word.
  const MERGED: ConversationEntry[] = [
    {
      ...SIDEBAR[0]!,
      id: SIDEBAR[0]!.id,
      branch: "over-on-the-member",
      rank: `Zz-${MEMBER}`,
      device: {
        id: MEMBER,
        name: "the-laptop",
        os: "macOS 15.1",
        reachable: true,
      },
    },
    ...SIDEBAR.map((row) => ({
      ...row,
      device: { id: null, name: "the-desk", os: "Linux", reachable: true },
    })),
  ];

  /// And the two devices as the membership says them, which is the other
  /// reading the same two facts arrive in: the rows carry the name and the
  /// mark on them, and the pane header looks one up by the id in its URL.
  const LINKED: DevicesView = {
    this: {
      device: "aa00bb11cc22dd33ee44ff5566778899",
      fingerprint: "AA:BB",
      name: "the-desk",
      os: "Linux",
      addresses: [],
    },
    members: [
      {
        identity: {
          device: MEMBER,
          fingerprint: "CC:DD",
          name: "the-laptop",
          os: "macOS 15.1",
          addresses: [],
        },
        reachable: true,
      },
    ],
    pending: [],
  };

  /// The member's Conversation under the id it shares with this device's first
  /// row, which is what the menu below reads.
  ///
  /// Closed and not put away, which is the state whose menu offers **Archive** —
  /// the one row here that is a press with nothing to confirm and nowhere to
  /// navigate, so what it proves is where the press went and nothing else.
  const SHARED: ConversationView = {
    ...HERE,
    id: SIDEBAR[0]!.id,
    branch: "over-on-the-member",
    state: "Closed",
    archived: false,
  };

  function theCluster(...answers: Answer[]) {
    return serving(
      whenever("/api/ui/conversations", json(MERGED)),
      // The membership, which is what the header of an open Conversation
      // reads a device's name and mark out of — the rows carry their own.
      whenever("/api/ui/devices", json(LINKED)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/onboarding", json(SET_UP)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/conversations/${SIDEBAR[0]!.id}`, json(HERE)),
      whenever(at(`/conversations/${SHARED.id}`), json(SHARED)),
      ...answers,
      json({ error: "nothing serves that" }, 404),
    );
  }

  /// Where a move on the merged list is saved: the list's own path on the
  /// device the browser opened, whoever owns the row that moved.
  const RANK = "/api/ui/conversations/rank";

  /// Every move the sidebar saved, whole — both rows as they were named.
  function dropped(fetching: ReturnType<typeof serving>): DroppedRow[] {
    return fetching.mock.calls
      .filter(([asked, init]) => init?.method === "PUT" && String(asked) === RANK)
      .map(([, init]) => JSON.parse(String(init?.body)) as DroppedRow);
  }

  /// Every row the sidebar drew, in the order it drew them.
  async function rows(container: ParentNode): Promise<HTMLElement[]> {
    await drawn(container, `.${sidebar.conversationRow}`);

    return [
      ...container.querySelectorAll<HTMLElement>(`.${sidebar.conversationRow}`),
    ];
  }

  it("draws two rows for one number on two devices", async () => {
    theCluster();
    const { container } = mount("/");

    const drawnRows = await rows(container);

    expect(drawnRows).toHaveLength(MERGED.length);
    expect(drawnRows.map((row) => row.dataset.row)).toEqual(
      MERGED.map((row) => rowKey(row.device?.id ?? null, row.id)),
    );

    // Which is the point: the two rows of the shared number are two keys.
    const shared = drawnRows.filter(
      (row) => row.dataset.row?.endsWith(`/${SIDEBAR[0]!.id}`) ?? false,
    );
    expect(shared).toHaveLength(2);
    expect(shared[0]!.dataset.row).not.toBe(shared[1]!.dataset.row);
  });

  it("opens a member's row at that device's own address", async () => {
    theCluster();
    const { container, history } = mount("/");

    const drawnRows = await rows(container);
    fireEvent.click(
      drawnRows[0]!.querySelector<HTMLElement>(`.${sidebar.open}`)!,
    );

    await waitFor(() =>
      expect(history.get()).toBe(on(`/conversations/${SHARED.id}`)),
    );
  });

  /// And this device's own row of the same number opens where it always did,
  /// which is the other half of the same claim.
  it("opens this device's own row at the address it always had", async () => {
    theCluster();
    const { container, history } = mount("/");

    const drawnRows = await rows(container);
    fireEvent.click(
      drawnRows[1]!.querySelector<HTMLElement>(`.${sidebar.open}`)!,
    );

    await waitFor(() =>
      expect(history.get()).toBe(`/conversations/${SIDEBAR[0]!.id}`),
    );
  });

  /// The card's own menu is the bug this addressing is about: a close pressed on
  /// a member's row must be about the member's Conversation, so what the menu
  /// reads is the member's record and never this device's of the same number.
  it("reads a member's record for a member's card menu", async () => {
    const fetching = theCluster();
    const { container } = mount("/");

    const drawnRows = await rows(container);
    fireEvent.contextMenu(
      drawnRows[0]!.querySelector<HTMLElement>(`.${sidebar.open}`)!,
      { clientX: 20, clientY: 20 },
    );

    await drawn(container, `.${actions.conversationActions} [role="menu"]`);

    await waitFor(() =>
      expect(
        askedFor(fetching, at(`/conversations/${SHARED.id}`)),
      ).toBeGreaterThan(0),
    );
    expect(askedFor(fetching, `/api/ui/conversations/${SHARED.id}`)).toBe(0);
  });

  /// And the press it makes goes to the member too, which is the half a menu
  /// that merely *read* the right record would still get wrong: the rows shut
  /// the menu and post afterwards, so the card's device has to outlive the menu.
  it("puts a press from a member's card menu to the member", async () => {
    const fetching = theCluster(
      whenever(
        at(`/conversations/${SHARED.id}/archive`),
        json("Archived" satisfies ConversationArchived),
      ),
    );
    const { container } = mount("/");

    const drawnRows = await rows(container);
    fireEvent.contextMenu(
      drawnRows[0]!.querySelector<HTMLElement>(`.${sidebar.open}`)!,
      { clientX: 20, clientY: 20 },
    );

    fireEvent.click(
      await drawn(
        container,
        `.${shell.conversationsPane} .${actions.conversationActions} .${actions.archive}`,
      ),
    );

    await waitFor(() =>
      expect(
        askedFor(fetching, at(`/conversations/${SHARED.id}/archive`)),
      ).toBeGreaterThan(0),
    );
    expect(
      askedFor(fetching, `/api/ui/conversations/${SHARED.id}/archive`),
    ).toBe(0);
  });

  /// And a card moved on the merged list says both rows by device and id, to
  /// **this** device rather than to the one that owns the row.
  ///
  /// Which is the whole of the cross-device drag from this end: an id alone
  /// names a row on no particular machine, and the device the browser opened is
  /// the one holding every rank in the cluster — so it is the one that can mint
  /// a key between two neighbours belonging to two machines. Writing it to the
  /// owner is its business rather than the page's, and nothing here knows what a
  /// rank looks like. See `ranking.rs`.
  it("says a moved row and its neighbour by device, to this device", async () => {
    const fetching = theCluster(
      whenever(
        RANK,
        () => Promise.resolve(new Response(null, { status: 204 })),
        "PUT",
      ),
    );
    const { container } = mount("/");

    const drawnRows = await rows(container);

    // The member's row, one step down — which puts it under this device's own
    // row of the very same number.
    fireEvent.keyDown(
      drawnRows[0]!.querySelector<HTMLElement>(`.${sidebar.open}`)!,
      { key: "ArrowDown" },
    );

    await waitFor(() => expect(dropped(fetching)).toHaveLength(1));

    expect(dropped(fetching)[0]).toEqual({
      row: { device: MEMBER, id: SHARED.id },
      below: { device: null, id: SIDEBAR[0]!.id },
    });
    expect(
      askedFor(fetching, at("/conversations/rank")),
      "the mint is the opened device's, not the member's",
    ).toBe(0);
  });

  /// And the row says which machine, which is the other half of one list: a
  /// merged sidebar whose rows did not say whose they were would be this
  /// device's work with somebody else's mixed into it.
  ///
  /// The second line, under the branch: the mark for that device's OS, its
  /// name, and the Repo after them. On every row there is a device on, which
  /// in a cluster is every row — this device's own the same as a member's.
  it("says the device and the repo under the name, on every row", async () => {
    theCluster();
    const { container } = mount("/");

    const drawnRows = await rows(container);

    expect(
      drawnRows.map(
        (row) => row.querySelector(`.${sidebar.meta}`)?.textContent ?? "",
      ),
    ).toEqual(MERGED.map((row) => `${row.device!.name}${row.repo}`));
  });

  /// And the mark beside the name is the one that device wears wherever it is
  /// drawn — so the member's macOS row draws the Apple mark and this device's
  /// Linux row the Linux one, off the OS word the row itself carries.
  it("draws each device's own mark beside its name", async () => {
    theCluster();
    const { container } = mount("/");

    const drawnRows = await rows(container);

    expect(
      drawnRows.map((row) =>
        row.querySelector(`.${sidebar.device} svg path`)?.getAttribute("d"),
      ),
    ).toEqual(MERGED.map((row) => [osIcon(row.device!.os).icon[4]].flat()[0]));
  });

  /// A mark is nothing to a screen reader, so what the card says in one is in
  /// the sentence the row is read aloud by: the device, where the row draws
  /// it, in the place the eye finds it.
  it("names the device in the row read aloud", async () => {
    theCluster();
    const { container } = mount("/");

    const drawnRows = await rows(container);
    const label = drawnRows[0]!
      .querySelector(`.${sidebar.open}`)!
      .getAttribute("aria-label");

    expect(label).toBe(
      "over-on-the-member, the-laptop, verkstead, Implementing",
    );
  });

  /// A member that has stopped answering keeps its rows, from the last list
  /// this device held of it: everything they had, dimmed, and saying so when
  /// they are read aloud. The fade is the one a finished Conversation wears,
  /// which is why the word is in the label — the two are told apart by ear
  /// rather than by eye.
  describe("a member that has stopped answering", () => {
    const AWAY: ConversationEntry[] = MERGED.map((row) => ({
      ...row,
      device: { ...row.device!, reachable: row.device!.id === null },
    }));

    function unreached() {
      return serving(
        whenever("/api/ui/conversations", json(AWAY)),
        whenever("/api/ui/devices", json(LINKED)),
        whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
        whenever("/api/ui/onboarding", json(SET_UP)),
        json({ error: "nothing serves that" }, 404),
      );
    }

    it("dims its rows and leaves this device's own alone", async () => {
      unreached();
      const { container } = mount("/");

      const drawnRows = await rows(container);

      expect(
        drawnRows.map((row) => row.classList.contains(sidebar.unreachable!)),
      ).toEqual(AWAY.map((row) => !row.device!.reachable));
    });

    it("says unreachable beside the device it names", async () => {
      unreached();
      const { container } = mount("/");

      const drawnRows = await rows(container);
      const label = drawnRows[0]!
        .querySelector(`.${sidebar.open}`)!
        .getAttribute("aria-label");

      expect(label).toBe(
        "over-on-the-member, the-laptop, unreachable, verkstead, Implementing",
      );
    });
  });

  /// And the pane a row opens says it too, beside the branch where the Repo
  /// already stands: the card and the header are the one name said twice, and
  /// a header that did not say which machine would leave the one pane the work
  /// is actually read in as the only place in the app that could not.
  ///
  /// Looked up rather than carried: the URL says which device, and the Devices
  /// reading says its name and its mark.
  it("carries the device on the header of the pane a row opens", async () => {
    theCluster();
    const { container } = mount(on(`/conversations/${SHARED.id}`));

    const named = await drawn(
      container,
      `.${shell.middlePane} .${paneHead.head} h1 .${timeline.paneDevice}`,
    );

    // Trimmed: the space between the mark and the name is written out, the
    // heading being read out of its own contents run together.
    expect(named.textContent?.trim()).toBe("the-laptop");
    expect(named.querySelector("svg path")?.getAttribute("d")).toBe(
      [osIcon("macOS 15.1").icon[4]].flat()[0],
    );
  });

  /// And this device's own Conversation says this device, which is the same
  /// rule the rows are drawn under: in a cluster the header names the machine
  /// whatever machine it is, so that one header does not read differently from
  /// the next.
  it("carries this device on this device's own header", async () => {
    theCluster();
    const { container } = mount(`/conversations/${SIDEBAR[0]!.id}`);

    const named = await drawn(
      container,
      `.${shell.middlePane} .${paneHead.head} h1 .${timeline.paneDevice}`,
    );

    expect(named.textContent?.trim()).toBe("the-desk");
  });
});

/// And the whole of it taken back off a Verkstead that is in no cluster, which
/// is what nearly every one of them is: nothing on the page says a device,
/// because there is no second machine for a name to be telling it from.
///
/// The server's call rather than the page's — the rows come back carrying no
/// device at all — and the header's own, which reads the membership and finds
/// nobody in it.
describe("a device with no cluster", () => {
  const ALONE: DevicesView = {
    this: {
      device: "aa00bb11cc22dd33ee44ff5566778899",
      fingerprint: "AA:BB",
      name: "the-desk",
      os: "Linux",
      addresses: [],
    },
    members: [],
    pending: [],
  };

  function alone() {
    return serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/devices", json(ALONE)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/onboarding", json(SET_UP)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/conversations/${HERE.id}`, json(HERE)),
      json({ error: "nothing serves that" }, 404),
    );
  }

  it("draws no device and no mark on any row", async () => {
    alone();
    const { container } = mount("/");

    await drawn(container, `.${sidebar.conversationRow}`);

    expect(container.querySelectorAll(`.${sidebar.device}`)).toHaveLength(0);
    expect(
      [...container.querySelectorAll(`.${sidebar.meta}`)].map(
        (line) => line.textContent,
      ),
    ).toEqual(SIDEBAR.map((row) => row.repo));
  });

  it("draws none on the pane header either", async () => {
    alone();
    const { container } = mount(`/conversations/${HERE.id}`);

    await waitFor(async () => expect(await titled(container)).toBe(HERE.branch));

    expect(
      container.querySelectorAll(`.${timeline.paneDevice}`),
    ).toHaveLength(0);
  });
});

describe("walking between this device's work and a member's", () => {
  /// The cache hazard the whole stage is about: both Conversations are a 3, so
  /// a cache keyed by the id alone would hold one entry and draw it twice.
  it("holds both conversations at once, each under its own device", async () => {
    bothDevices();
    const { container, history, client } = mount(`/conversations/${HERE.id}`);

    await waitFor(async () => expect(await titled(container)).toBe(HERE.branch));

    history.set({ value: on(`/conversations/${THERE.id}`) });
    await waitFor(async () => expect(await titled(container)).toBe(THERE.branch));

    expect(
      client.getQueryData<ConversationView>(["conversation", String(HERE.id)])
        ?.branch,
    ).toBe(HERE.branch);
    expect(
      client.getQueryData<ConversationView>([
        MEMBER,
        "conversation",
        String(THERE.id),
      ])?.branch,
    ).toBe(THERE.branch);
  });

  /// And walking back is the record this device holds again, rather than the
  /// member's left standing under an id they share.
  it("draws this device's own again on the way back", async () => {
    bothDevices();
    const { container, history } = mount(on(`/conversations/${THERE.id}`));

    await waitFor(async () => expect(await titled(container)).toBe(THERE.branch));

    history.set({ value: `/conversations/${HERE.id}` });
    await waitFor(async () => expect(await titled(container)).toBe(HERE.branch));
  });

  /// The promise the local half of this makes: a page with no device in its URL
  /// is the page it has always been, and every call it makes is the call it
  /// always made.
  it("leaves a local page asking exactly what it always asked", async () => {
    const fetching = bothDevices();
    const { container } = mount(`/conversations/${HERE.id}`);

    await waitFor(async () => expect(await titled(container)).toBe(HERE.branch));

    expect(
      askedFor(fetching, `/api/ui/conversations/${HERE.id}`),
    ).toBeGreaterThan(0);
    expect(
      fetching.mock.calls.filter(([asked]) =>
        String(asked).startsWith("/api/ui/members/"),
      ),
    ).toHaveLength(0);
  });
});

/// Answering a Set is the press the whole workbench exists for, and a Set on a
/// member is answered exactly as one here is — the sheet, the file on an
/// Answer, and the Response — so the pane is mounted on its own with a device
/// around it, which is what the details pane hands it (see `Workbench.tsx`).
describe("a set on another device", () => {
  /// The button reading `text`, which is how the sheet is sent.
  function pressing(page: ParentNode, text: string): void {
    const button = [...page.querySelectorAll("button")].find(
      (found) => found.textContent === text,
    );
    expect(button, `expected a button reading "${text}"`).toBeTruthy();
    fireEvent.click(button!);
  }

  /// The Set the sheet is filled in on, out of the golden fixture the sheet's
  /// own tests use.
  const WAITING = readable(waiting);

  /// The Timeline row that opens it, as the details pane holds one.
  const ASKED: QuestionSetEvent = {
    id: 1,
    at: "2025-02-01T09:00:00Z",
    set_id: WAITING.id,
    title: "A question set",
    rows: [],
    standing: { Waiting: "waiting" },
  };

  /// The pane over that Set, drawn for the member the way the details pane
  /// draws it: a provider around it and nothing else different.
  async function asking(...answers: Answer[]) {
    const fetching = serving(
      whenever(at(`/sets/${WAITING.id}`), json(reads(WAITING))),
      ...answers,
      json({ error: "nothing serves that" }, 404),
    );

    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });

    const { container } = render(() => (
      <QueryClientProvider client={client}>
        <Reaching.Provider value={() => MEMBER}>
          <Asked asked={ASKED} back={() => {}} />
        </Reaching.Provider>
      </QueryClientProvider>
    ));

    await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());

    return { page: container, fetching };
  }

  it("reads the set off the member and asks this device for none of it", async () => {
    const { fetching } = await asking();

    expect(askedFor(fetching, at(`/sets/${WAITING.id}`))).toBeGreaterThan(0);
    expect(askedFor(fetching, `/api/ui/sets/${WAITING.id}`)).toBe(0);
  });

  /// The file on an Answer, which is the one press here that is a body rather
  /// than a form: the bytes are streamed through to the member and what comes
  /// back is its own answer about them.
  it("puts a file chosen on an answer up to the member", async () => {
    const upload = at(`/sets/${WAITING.id}/answers/Q1/attachments/counter.png`);
    const { page, fetching } = await asking(
      whenever(
        upload,
        json({
          Attached: {
            attachment: {
              id: 9,
              name: "counter.png",
              bytes: 4,
              origin: "Answer",
              label: "Q1",
            },
          },
        }),
        "POST",
      ),
    );

    const field = page.querySelector('textarea[name="Q1-free-text"]')!;
    const question = field.closest(`.${sheet.ask}`)!;
    const picker = question.querySelector<HTMLInputElement>(
      'input[type="file"]',
    )!;

    Object.defineProperty(picker, "files", {
      configurable: true,
      value: [new File(["png"], "counter.png")],
    });
    fireEvent.change(picker);

    await waitFor(() =>
      expect(
        fetching.mock.calls.filter(
          ([asked, init]) => String(asked) === upload && init?.method === "POST",
        ),
      ).toHaveLength(1),
    );
  });

  /// And the Response itself, which is what ends the wait the agent over there
  /// is holding.
  it("sends the response to the member", async () => {
    const response = at(`/sets/${WAITING.id}/response`);
    const { page, fetching } = await asking(
      whenever(response, json("Accepted" satisfies Submitted), "POST"),
    );

    // The first Option of the one question that has any, and then the sheet
    // sent with the warning the rest of it is unanswered accepted: what is on
    // the wire is the sheet's own business — `answering.test.tsx` is where that
    // is held true — and what is asked here is only where it went.
    fireEvent.click(
      page.querySelector<HTMLInputElement>(
        `input[name="Q1-option"][value="1"]`,
      )!,
    );
    pressing(page, "Submit");
    pressing(page, "Send anyway");

    await waitFor(() =>
      expect(
        fetching.mock.calls.filter(
          ([asked, init]) =>
            String(asked) === response && init?.method === "POST",
        ),
      ).toHaveLength(1),
    );
  });
});

describe("a member that is not answering", () => {
  /// The hop's own refusal is a verdict rather than a bad moment, and the app
  /// must not go back for three more of them.
  ///
  /// What a second attempt costs is the whole of the reason: a call put to a
  /// member walks every address that device advertised, at two seconds apiece,
  /// and writes its row unreachable when none of them answers — so the ordinary
  /// three retries are half a minute of blank page and four passes down the
  /// same dead list, to arrive at the sentence the first refusal already
  /// carried. A local refusal is still worth the ordinary attempts: it costs a
  /// round trip on the loopback.
  it("is not asked again three more times", async () => {
    serving(
      json(
        {
          error: `device ${MEMBER} answered at none of the addresses it advertised`,
        },
        502,
      ),
    );

    const refused = await loadConversation(MEMBER, String(THERE.id)).catch(
      (error: unknown) => error,
    );

    expect(refused).toBeInstanceOf(RefusedError);
    expect((refused as RefusedError).status).toBe(502);
    expect((refused as RefusedError).message).toContain("none of the addresses");
    expect(retrying(0, refused)).toBe(false);
  });

  /// And the two a Device Id earns before anything is dialled, which are the
  /// far end's own refusals besides — a Conversation it has no record of among
  /// them. None of the three is a machine that might answer next time.
  it("is not asked again for an id that was never going to be dialled", async () => {
    for (const [status, said] of [
      [400, "is this device"],
      [404, "knows no device"],
    ] as const) {
      serving(json({ error: said }, status));

      const refused = await loadConversation(MEMBER, String(THERE.id)).catch(
        (error: unknown) => error,
      );

      expect((refused as RefusedError).status).toBe(status);
      expect(retrying(0, refused)).toBe(false);
    }
  });

  /// While this device refusing the same way is still worth the ordinary three
  /// attempts: what the rule is about is the walk down a member's addresses,
  /// and a local refusal has none to walk.
  it("leaves a refusal of this device's own retried as it always was", async () => {
    serving(json({ error: "there isn't one" }, 404));

    const refused = await loadConversation(null, String(HERE.id)).catch(
      (error: unknown) => error,
    );

    expect((refused as RefusedError).status).toBe(404);
    expect(retrying(0, refused)).toBe(true);
    expect(retrying(3, refused)).toBe(false);
  });
});
