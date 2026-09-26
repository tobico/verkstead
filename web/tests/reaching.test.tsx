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
  ConversationView,
  QuestionSetEvent,
  RepoEntry,
  Submitted,
} from "../src/api/types";
import { Reaching } from "../src/reaching";
import sheet from "../src/set/Sheet.module.css";
import { Asked } from "../src/workbench/Asked";
import dropdown from "../src/Menu.module.css";
import shell from "../src/Panes.module.css";
import actions from "../src/workbench/Actions.module.css";
import paneHead from "../src/workbench/PaneHead.module.css";
import setup from "../src/workbench/Setup.module.css";
import steerForm from "../src/workbench/Steer.module.css";
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

  /// And the sidebar beside it is still this device's: the merged list is a
  /// later stage, so what the rows say is the work being done here.
  it("goes on listing this device's own work beside it", async () => {
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
