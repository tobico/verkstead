//! The workbench: the three panes, the Conversation they are about, and
//! everything the human settles about it before anything runs.
//!
//! `tests/fixtures/conversations.json` and `conversation.json` are golden
//! fixtures like the two Set lists': `cargo test` renders the real endpoints and
//! writes the files, so what these assertions read is what the server actually
//! said.
//!
//! What is worth proving here is the shape of the hierarchy and that each pane
//! draws what it was handed. Whether a branch name is one git would take, and
//! whether a base commit is in the repository, are the server's to decide — the
//! tests over there are what say so — and this side's job is to send what was
//! typed and say in words what came back.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  AbandonedRepo,
  Adopted,
  AgentOutputEvent,
  BriefEvent,
  Capture,
  CommitPane,
  ConversationAborted,
  ConversationEntry,
  ConversationReopened,
  ConversationStopped,
  ConversationView,
  GrillingStarted,
  ManualTaskStarted,
  PauseResumed,
  ProfileEntry,
  PullRequestDetails,
  Resumed,
  Screen,
  Shown,

  Submitted,
  TimelineEvent,
  TranscriptView,
  Turn,
} from "../src/api/types";
import stylesheet from "../src/main.css?raw";
import { ADOPT_REFUSAL } from "../src/workbench/Adoption";
import {
  CLAMPED_LINES,
  MANUAL_TASK_REFUSAL,
  RESUME_REFUSAL,
  STOP_REFUSAL,
  SWIPE,
} from "../src/workbench/Timeline";
import {
  BRANCHES,
  OPEN,
  PROFILES,
  REPOS,
  SIDEBAR,
  drawn,
  mount,
  nudged,
  theWorkbench,
} from "./bench";
import {
  askedFor,
  json,
  readable,
  reads,
  serving,
  unreadable,
  whenever,
} from "./serving";
import abandoned from "./fixtures/abandoned-roadmaps.json" with { type: "json" };
import adopting from "./fixtures/conversation-adopting.json" with { type: "json" };
import building from "./fixtures/conversation-building.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import halted from "./fixtures/conversation-halted.json" with { type: "json" };
import paused from "./fixtures/conversation-paused.json" with { type: "json" };
import reopened from "./fixtures/conversation-reopened.json" with { type: "json" };
import answeredSet from "./fixtures/set-answered.json" with { type: "json" };
import answeringSet from "./fixtures/set-answering.json" with { type: "json" };
import unreadableSet from "./fixtures/set-unreadable.json" with { type: "json" };
import roadmap from "./fixtures/conversation-roadmap.json" with { type: "json" };
import tasks from "./fixtures/conversation-tasks.json" with { type: "json" };
import capture from "./fixtures/capture.json" with { type: "json" };
import transcript from "./fixtures/transcript.json" with { type: "json" };
import more from "./fixtures/transcript-more.json" with { type: "json" };
import screenOfIt from "./fixtures/screen.json" with { type: "json" };
import wrapping from "./fixtures/conversation-wrapping.json" with { type: "json" };

/// The renderer, which is each pane's own doing rather than this file's: what is
/// asked here is whether a commit's pane reached for it at all, and never what it
/// drew — that is `diagrams.test.ts`. Mocked either way, so that nothing here
/// loads megabytes of mermaid.
const drawing = vi.hoisted(() =>
  vi.fn((_how?: { root?: ParentNode }) => vi.fn()),
);
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: drawing }));

/// How many columns fit in the pane, which is what the Screen of a live session
/// sends up its socket.
///
/// Mocked because it is arithmetic over a layout, and jsdom has none: the real
/// addon measures a rendered character against the width of the element the
/// terminal was opened in, and both are zero here. What is worth asserting on
/// this side of the wire is that the pane measures itself and says what it
/// found, which is what this makes askable.
const FITS = { cols: 132, rows: 43 };

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    activate() {}
    dispose() {}
    fit() {}
    proposeDimensions() {
      return FITS;
    }
  },
}));

const ABANDONED = abandoned as AbandonedRepo[];

/// The conversation that clicking one of those roadmaps made: a draft adopting
/// `mvp`, on the page shaped for adopting.
const ADOPTING = adopting as ConversationView;

/// The one the fixture opens, which is the second row of the sidebar.
const DRAFTING = SIDEBAR.find((entry) => entry.id === OPEN.id)!;

/// The Brief on a Conversation's Timeline, which is the first thing on every
/// one of them.
function briefOf(conversation: ConversationView): BriefEvent {
  const first = conversation.timeline[0]!;
  if (!("Brief" in first)) {
    throw new Error("the fixture's first Event should be the Brief");
  }
  return first.Brief;
}

/// The Brief on the opened Conversation's Timeline.
const BRIEF = briefOf(OPEN);

afterEach(() => {
  vi.unstubAllGlobals();
  // Counted per test: whether a pane reached for the renderer is a question
  // about the one pane the test opened.
  drawing.mockClear();
  // Two tests drive a self-saving field's typing pause on a clock of their own.
  vi.useRealTimers();
  // The state is the instance's own, over the one every other test reads off
  // the prototype — so it goes when the test does.
  delete (document as { visibilityState?: DocumentVisibilityState })
    .visibilityState;
});

/// The read of the open Conversation, which is the one worth counting: the page
/// fetches three other things around it.
const READING = `/api/ui/conversations/${OPEN.id}`;

/// The page reading everything it is showing again, provoked the way coming back
/// to the app provokes it — the cheapest of the two ways it happens, the other
/// being a Nudge. What is asked here is what a read does to the page, and both
/// do the same one.
function readAgain(): void {
  for (const state of ["hidden", "visible"] as const) {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => state,
    });
    document.dispatchEvent(new Event("visibilitychange", { bubbles: true }));
  }
}

/// The frame, which is what says which level a narrow window is showing.
function frame(container: ParentNode): HTMLElement {
  return container.querySelector(".workbench")!;
}

/// Open the conversation's action menu: press the trigger, and wait for what it
/// drops.
async function openActions(container: ParentNode): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, ".conversation-actions > .menu-trigger"));
  return drawn(container, ".conversation-actions > .menu-drop");
}

/// Open the sidebar's ⋯, which is what the rest of Verkstead is behind: press
/// the trigger, and wait for what it drops.
async function openWorkbenchActions(
  container: ParentNode,
): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, ".workbench-actions > .menu-trigger"));
  return drawn(container, ".workbench-actions > .menu-drop");
}

/// Drop the new-conversation menu, which is where both ways of starting one
/// live: press the button, and wait for what it drops.
async function openNewConversation(
  container: ParentNode,
): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, ".new-conversation > .menu-trigger"));
  return drawn(container, ".new-conversation > .menu-drop");
}

/// The repos in that menu, in the order they are offered — waited for, because
/// the menu opens whether or not the list has arrived.
async function repoRows(container: ParentNode): Promise<HTMLButtonElement[]> {
  await drawn(container, ".in-repo");
  return [...container.querySelectorAll<HTMLButtonElement>(".in-repo")];
}

/// The body the page put on the wire when it wrote to `path`.
///
/// By the request rather than by being the last thing sent: writing anything
/// here is followed by reading the Conversation back, so the last call is
/// ordinarily the read. `which` picks between them where a field saved more
/// than once and the test is about the later save.
function sent(
  fetching: ReturnType<typeof serving>,
  path: string,
  which = 0,
): unknown {
  const written = fetching.mock.calls.filter(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  );
  expect(
    written[which],
    `expected the page to have written to ${path} ${which + 1} time(s)`,
  ).toBeTruthy();
  return JSON.parse(String(written[which]![1]?.body));
}

/// How many times the page wrote to `path`, for the tests about *when* a save
/// goes out rather than what was in it.
function writes(fetching: ReturnType<typeof serving>, path: string): number {
  return fetching.mock.calls.filter(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  ).length;
}

/// An answer the test lands itself, rather than one the fetch resolves with
/// straight away.
///
/// For the fields that keep themselves: what one of them does with a keystroke
/// arriving while a save is in the air is only askable while a save is actually
/// in the air, and one that answered at once never is.
function holding(answer: () => Promise<Response>): {
  held: () => Promise<Response>;
  land: () => void;
} {
  let land!: () => void;
  const waited = new Promise<void>((resolve) => {
    land = resolve;
  });
  return { held: () => waited.then(answer), land: () => land() };
}

describe("the workbench", () => {
  it("draws all three panes", async () => {
    theWorkbench();
    mount();

    await waitFor(() => screen.getByText(DRAFTING.branch));

    // Every pane is in the document whatever the window is doing: which of them
    // a narrow one shows is the stylesheet's business, and a pane rendered away
    // would have to be rebuilt every time the human walked back into it.
    expect(screen.getByLabelText("Conversations")).toBeTruthy();
    expect(screen.getByLabelText("Timeline")).toBeTruthy();
    expect(screen.getByLabelText("Details")).toBeTruthy();
  });

  /// The sidebar is where Verkstead is entered, so what stands over the list is
  /// the mark: the icon and the name, in the line the word *Conversations* used
  /// to hold. Still a heading, so the pane has one for a screen reader to find,
  /// and the pane's own label is untouched — the test above reads it.
  it("leads with the mark rather than a title of its own", async () => {
    theWorkbench();
    const { container } = mount();

    const heading = await drawn(container, ".conversations-pane h1");

    expect(heading.textContent).toContain("Verkstead");
    expect(heading.textContent).not.toContain("Conversations");

    // The one icon source, served from `assets/` at the site root — the file the
    // favicon is, rather than a copy of it under `web/`.
    const icon = heading.querySelector("img")!;
    expect(icon.getAttribute("src")).toBe("/icons/verkstead.svg");

    // Nothing for a screen reader to read: the word beside it is the name, and
    // an alt that repeated it would have the heading say it twice.
    expect(icon.getAttribute("alt")).toBe("");
  });

  it("lists the conversations the server gave it, in that order", async () => {
    const fetching = theWorkbench();
    const { container } = mount();

    await waitFor(() => screen.getByText(DRAFTING.branch));

    expect(fetching).toHaveBeenCalledWith(
      "/api/ui/conversations",
      expect.anything(),
    );
    expect(
      [...container.querySelectorAll(".conversation-row .title")].map(
        (row) => row.textContent,
      ),
    ).toEqual(SIDEBAR.map((entry) => entry.branch));
  });

  it("says of each conversation which repo it is in", async () => {
    theWorkbench();
    mount();

    const row = (await waitFor(() => screen.getByText(DRAFTING.branch))).closest(
      "li",
    )!;

    expect(row.querySelector(".repo")!.textContent).toBe(DRAFTING.repo);

    // And nothing about where it has got to, in words: that is drawn now — see
    // *how a card says where its conversation has got to*.
    expect(row.querySelector(".state")).toBeNull();
  });

  it("says so plainly when nothing is being worked on", async () => {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/repos", json(REPOS)),
    );
    mount();

    await waitFor(() => screen.getByText("Nothing is being worked on yet."));
  });

  /// The sidebar is where the rest of Verkstead is reached from, now that the
  /// workbench has the root — and the rest of Verkstead is one page, since the
  /// Repos and the Agent Profiles were folded onto the settings page. Behind the
  /// ⋯ at the head of the pane, where the Conversation's own ⋯ is, rather than
  /// under a list with no end to it.
  it("reaches the rest of Verkstead from the sidebar's menu", async () => {
    theWorkbench();
    const { container, history } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    const drop = await openWorkbenchActions(container);
    expect(
      [...drop.querySelectorAll("a")].map((to) => to.getAttribute("href")),
    ).toEqual(["/settings"]);

    // A row that goes somewhere rather than one that does something, so pressing
    // it takes the whole sidebar with it and nothing here has to shut the menu.
    const settings = screen.getByText("Settings");
    expect(settings.getAttribute("role")).toBe("menuitem");
    fireEvent.click(settings);
    await waitFor(() => expect(history.get()).toBe("/settings"));
  });

  /// Nothing of it until it is pressed, which is the point of putting it there:
  /// the head of the pane gives up a mark's worth of room and no more.
  it("keeps the way out of the workbench behind that menu", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    expect(
      container.querySelector(".workbench-actions > .menu-drop"),
    ).toBeNull();
    expect(screen.queryByText("Settings")).toBeNull();
    // And the link that used to sit under the conversations is gone with it.
    expect(container.querySelector(".elsewhere")).toBeNull();
  });

  /// The menu still opens with nothing to start a conversation in — and what is
  /// in it is the page that fixes that, because a menu that opened on nothing
  /// would say only that the button was broken.
  it("says where to go when there is no repo to start one against", async () => {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/repos", json([])),
    );
    const { container } = mount();
    await openNewConversation(container);

    await waitFor(() => screen.getByText(/No repos are registered yet/));
    expect(container.querySelector(".in-repo")).toBeNull();
    expect(screen.getByText("register one").getAttribute("href")).toBe(
      "/settings",
    );
  });
});

/// The sidebar drawn over a sidebar of this test's own making, so that a row can
/// be given facts no fixture happens to carry.
///
/// The rows are the fixture's, altered: what is being asked here is what a card
/// does with `state`, `working` and `waiting`, and everything else about a row
/// should stay whatever the server really said.
function theSidebar(...rows: Array<Partial<ConversationEntry>>) {
  return theWorkbench(
    whenever(
      "/api/ui/conversations",
      json(
        rows.map((row, n) => ({
          ...SIDEBAR[0]!,
          id: n + 1,
          branch: `b${n}`,
          ...row,
        })),
      ),
    ),
  );
}

/// The cards of a sidebar drawn that way, in the order they were given.
async function cards(container: ParentNode): Promise<HTMLElement[]> {
  await drawn(container, ".conversation-row");
  return [...container.querySelectorAll<HTMLElement>(".conversation-row")];
}

/// How a card says where its Conversation has got to, now that it no longer says
/// it in words: a mark at the right edge for what is happening to it now, and
/// the card's own treatment for a draft and for work that has stopped.
///
/// Which specific active state it is in — grilling, implementing, wrapping — is
/// deliberately not drawn. What the sidebar is for is finding the Conversation to
/// look at, and all three of those are *this one is under way*.
describe("how a card says where its conversation has got to", () => {
  it("turns a spinner on a conversation whose session is running", async () => {
    theSidebar({ state: "Implementing", working: true, waiting: false });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(".mark.working")).toBeTruthy();
    expect(card!.querySelector(".mark.waiting")).toBeNull();
  });

  /// And the same ring empty once that session has stopped talking, which is the
  /// mark the Timeline row and the details pane draw for it too. The case it
  /// exists for is a grilling that has sat on a blocking ask for an hour: a
  /// spinner turning the whole time says something is happening when nothing is.
  it("empties the ring on a conversation whose session has gone quiet", async () => {
    theSidebar({
      state: "Grilling",
      working: true,
      idle: true,
      waiting: false,
    });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(".mark.idle")).toBeTruthy();
    expect(card!.querySelector(".mark.working")).toBeNull();
  });

  /// An icon and a border round the whole card, rather than the dot this used
  /// to be: what it has to survive is a glance down a list on a phone.
  it("marks a conversation waiting on the human, card and all", async () => {
    theSidebar({ state: "Grilling", working: false, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(".mark.waiting")?.textContent).toBe("!");
    expect(card!.querySelector(".mark.working")).toBeNull();

    // The border is the card's own, so the row carries it and the stylesheet
    // says what it looks like — jsdom lays nothing out.
    expect(card!.classList.contains("waiting")).toBe(true);
    expect(stylesheet).toContain(
      ".conversation-row.waiting .open {\n" +
        "  border-color: var(--accent);\n" +
        "  box-shadow: inset 0 0 0 1px var(--accent);\n" +
        "}",
    );
  });

  /// The mark is a character, and a character is something a screen reader
  /// would otherwise read out beside the label that already said it.
  it("keeps the mark out of what is read aloud", async () => {
    theSidebar({ state: "Grilling", working: false, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(
      card!.querySelector(".mark.waiting")!.getAttribute("aria-hidden"),
    ).toBe("true");
  });

  /// A Blocking Ask is exactly this: the session that asked is still alive and
  /// idling on the answer. Of the two things true of it, the one the human can do
  /// something about is the ask.
  it("shows the dot and not the spinner when it is both", async () => {
    theSidebar({ state: "Grilling", working: true, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(".mark.waiting")).toBeTruthy();
    expect(card!.querySelector(".mark.working")).toBeNull();
  });

  /// And over the empty one, which is the same case a step further on: the
  /// session that asked has been quiet since it asked. The mark that outranks
  /// both is the one the human can do something about.
  it("shows the dot and not the empty ring when it is both", async () => {
    theSidebar({ state: "Grilling", working: true, idle: true, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(".mark.waiting")).toBeTruthy();
    expect(card!.querySelector(".mark.idle")).toBeNull();
  });

  /// Both crossings reach the card on the news alone: a session going quiet is
  /// announced on the Conversation's own Nudge kind, and so is its coming back
  /// out of the silence — because what carries a session speaking is the Screen's
  /// kind, which is about the Conversation being watched rather than the list of
  /// them. What a page does with either is read this list again.
  it("moves between the two rings without the page being reloaded", async () => {
    let rows: ConversationEntry[] = [
      { ...SIDEBAR[0]!, working: true, idle: false },
    ];
    theWorkbench(whenever("/api/ui/conversations", () => json(rows)()));
    const { container, client } = mount();
    await drawn(container, ".conversation-row .mark.working");

    rows = [{ ...rows[0]!, idle: true }];
    await nudged(client);

    expect(container.querySelector(".conversation-row .mark.idle")).toBeTruthy();
    expect(container.querySelector(".conversation-row .mark.working")).toBeNull();

    // And back, on the session speaking again.
    rows = [{ ...rows[0]!, idle: false }];
    await nudged(client);

    expect(
      container.querySelector(".conversation-row .mark.working"),
    ).toBeTruthy();
    expect(container.querySelector(".conversation-row .mark.idle")).toBeNull();
  });

  it("marks nothing on a conversation that is neither", async () => {
    theSidebar({ state: "Implementing", working: false, waiting: false });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(".mark")).toBeNull();

    // Neither half of the waiting mark: no icon above, and no border here.
    expect(card!.classList.contains("waiting")).toBe(false);
  });

  it("draws a draft as a draft, and marks nothing on it", async () => {
    theSidebar({ state: "Draft", working: false, waiting: false });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.classList.contains("draft")).toBe(true);
    expect(card!.querySelector(".mark")).toBeNull();

    // What "draft" means is the stylesheet's, and jsdom lays nothing out.
    expect(stylesheet).toContain(
      ".conversation-row.draft .open {\n  border-style: dotted;\n}",
    );
  });

  /// Which of the two it was is the details pane's to say. The sidebar's business
  /// is that there is nothing here to do.
  it("dims finished and aborted work identically", async () => {
    theSidebar({ state: "Done" }, { state: "Aborted" }, { state: "Wrapping" });
    const { container } = mount();

    expect((await cards(container)).map((card) => card.className)).toEqual([
      "conversation-row ended",
      "conversation-row ended",
      "conversation-row",
    ]);
  });

  /// How far down is the stylesheet's, and jsdom lays nothing out: what these
  /// two say is that a closed card recedes far enough to read as closed, and
  /// that being the open one is the accent border and nothing beside it.
  it("takes a closed card well down, and marks the open one with a border", () => {
    expect(stylesheet).toContain(
      ".conversation-row.ended .open {\n  opacity: 0.45;\n}",
    );
    expect(stylesheet).toContain(
      ".conversation-row.selected .open {\n  border-color: var(--accent);\n}",
    );
    expect(
      stylesheet,
      "the inset stripe is retired everywhere it was drawn",
    ).not.toContain("box-shadow: inset 0.2rem");
  });

  /// Dimmed and still a row to press: a Done Conversation can be reopened.
  it("opens a dimmed conversation like any other", async () => {
    theSidebar({ state: "Done" });
    const { container, history } = mount();

    const [card] = await cards(container);
    fireEvent.click(card!.querySelector("button")!);

    await waitFor(() => expect(history.get()).toBe("/conversations/1"));
  });

  /// The marks are nothing to a screen reader, so the whole of what the card says
  /// goes on the button's label — including the state that used to be written
  /// under the name.
  it("keeps every one of them readable aloud", async () => {
    theSidebar(
      {
        branch: "spinning",
        repo: "verkstead",
        state: "Implementing",
        working: true,
        waiting: false,
      },
      {
        branch: "asking",
        repo: "askance",
        state: "Grilling",
        working: true,
        waiting: true,
      },
      {
        branch: "sitting",
        repo: "verkstead",
        state: "Grilling",
        working: true,
        idle: true,
        waiting: false,
      },
      {
        branch: "over",
        repo: "verkstead",
        state: "Done",
        working: false,
        waiting: false,
      },
    );
    const { container } = mount();

    expect(
      (await cards(container)).map((card) =>
        card.querySelector("button")!.getAttribute("aria-label"),
      ),
    ).toEqual([
      "spinning, verkstead, Implementing, a session is running",
      "asking, askance, Grilling, waiting on you",
      "sitting, verkstead, Grilling, a session is running and has gone quiet",
      "over, verkstead, Done",
    ]);
  });

  /// The spinner is motion, and motion is something to be able to turn off —
  /// everywhere it is drawn, which is every mark on the page rather than the
  /// sidebar's alone.
  it("holds the spinner still where motion is unwelcome", () => {
    expect(stylesheet).toContain(
      "@media (prefers-reduced-motion: reduce) {\n" +
        "  .mark.working {\n" +
        "    animation: none;\n" +
        "  }\n" +
        "}",
    );
  });
});

/// The order the sidebar is in, which is the human's own: they drag a row's grip
/// and the whole list goes to the server, so it survives a reload, a restart and
/// a second device without any of the three being a case this page knows about.
///
/// What the server does with the order is asked over there — the tests in
/// `crates/server/tests/conversations.rs` say where an unplaced Conversation
/// lands. What is asked here is that the list moves under the hand and that what
/// is sent is what is on the screen.
describe("the order the human puts the sidebar in", () => {
  /// A sidebar of three named rows, over an endpoint that takes an order and
  /// answers with nothing — which is what the real one does. A test wanting it
  /// to answer otherwise says so, and what it says wins.
  function three(...answers: Parameters<typeof serving>) {
    return theWorkbench(
      whenever(
        "/api/ui/conversations",
        json(
          ["first", "second", "third"].map((branch, n) => ({
            ...SIDEBAR[0]!,
            id: n + 1,
            branch,
          })),
        ),
      ),
      whenever(
        "/api/ui/conversations/order",
        () => Promise.resolve(new Response(null, { status: 204 })),
        "POST",
      ),
      ...answers,
    );
  }

  /// The branches the sidebar is showing, top first.
  async function order(container: ParentNode): Promise<(string | null)[]> {
    return (await cards(container)).map(
      (card) => card.querySelector(".title")!.textContent,
    );
  }

  /// The rows laid out down the pane, one under the last.
  ///
  /// jsdom has no layout — every rect is zeros — and a drag asks the rows where
  /// they are to say which one the pointer is over. The rect is worked out at
  /// the moment it is asked for rather than fixed here, because the list moves
  /// under the hand: a row that was second and is now first has to answer for
  /// where it is now.
  function laidOut(rows: HTMLElement[], height = 60) {
    const list = rows[0]!.parentElement!;
    const drawn = () => [...list.querySelectorAll<HTMLElement>(".conversation-row")];

    for (const row of rows) {
      row.getBoundingClientRect = () => {
        const at = drawn().indexOf(row) * height;
        return {
          top: at,
          bottom: at + height,
          height,
          left: 0,
          right: 240,
          width: 240,
          x: 0,
          y: at,
          toJSON: () => ({}),
        } as DOMRect;
      };
    }
  }

  /// Drag one row's grip to a height on the pane, and let go.
  function dragTo(card: HTMLElement, y: number) {
    const grip = card.querySelector<HTMLElement>(".grip")!;

    fireEvent.pointerDown(grip, { button: 0, pointerId: 1, clientY: 0 });
    fireEvent.pointerMove(grip, { pointerId: 1, clientY: y });
    fireEvent.pointerUp(grip, { pointerId: 1 });
  }

  it("moves the row under the hand and sends the whole list", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    // The third row, dragged to the top of the pane.
    dragTo(rows[2]!, 10);

    expect(await order(container)).toEqual(["third", "first", "second"]);

    // The list as it now stands, by id, rather than the row that moved: what is
    // on the screen is what they meant.
    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations/order")).toEqual({
        order: [3, 1, 2],
      }),
    );
  });

  it("drops a row at the end when the hand goes past the last one", async () => {
    three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    dragTo(rows[0]!, 500);

    expect(await order(container)).toEqual(["second", "third", "first"]);
  });

  /// A grip that could only be dragged would be a control half the people using
  /// it could not reach.
  it("moves a row a step at a time from the keyboard", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    fireEvent.keyDown(rows[2]!.querySelector(".grip")!, { key: "ArrowUp" });

    expect(await order(container)).toEqual(["first", "third", "second"]);
    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations/order")).toEqual({
        order: [1, 3, 2],
      }),
    );
  });

  it("leaves the top row where it is when it is asked to go higher", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    fireEvent.keyDown(rows[0]!.querySelector(".grip")!, { key: "ArrowUp" });

    expect(await order(container)).toEqual(["first", "second", "third"]);
    expect(
      askedFor(fetching, "/api/ui/conversations/order"),
      "there is nowhere to move it to, so there is nothing to save",
    ).toBe(0);
  });

  /// The grip is a second control on a row that already has one, and it does a
  /// different thing: it says so itself rather than leaving the card's label to
  /// cover both.
  it("names what each grip moves", async () => {
    three();
    const { container } = mount();

    expect(
      (await cards(container)).map((card) =>
        card.querySelector(".grip")!.getAttribute("aria-label"),
      ),
    ).toEqual(["Move first", "Move second", "Move third"]);
  });

  /// Most answering happens on a phone, and a touch that starts on the grip has
  /// to drag the row rather than scroll the list out from under it.
  it("takes the touch that starts on a grip", () => {
    expect(stylesheet).toContain("  cursor: grab;\n  touch-action: none;\n");
  });

  /// The order that was not saved is not the order to draw: what comes back is
  /// the server's, with the reason under it.
  it("says so and puts the list back when the order will not save", async () => {
    three(
      whenever(
        "/api/ui/conversations/order",
        json({ error: "the server is not taking orders" }, 503),
        "POST",
      ),
    );
    const { container } = mount();

    const rows = await cards(container);
    fireEvent.keyDown(rows[2]!.querySelector(".grip")!, { key: "ArrowUp" });

    await waitFor(() =>
      screen.getByText(/The order could not be saved/, { exact: false }),
    );
    expect(await order(container)).toEqual(["first", "second", "third"]);
  });
});

describe("the new conversation menu", () => {
  /// The whole of starting one: a press on the button, a press on a repo, and
  /// the conversation that came back is open.
  it("sends the repo that was pressed, and opens what came back", async () => {
    const fetching = theWorkbench(json({ Started: { id: OPEN.id } }));
    const { container, history } = mount();
    await openNewConversation(container);

    fireEvent.click((await repoRows(container))[1]!);

    // Straight into it: what the human does next is write the brief.
    await waitFor(() => expect(history.get()).toBe(`/conversations/${OPEN.id}`));
    expect(sent(fetching, "/api/ui/conversations")).toEqual({
      repo_id: REPOS[1]!.id,
    });
  });

  /// Every registered repo is a row, in the order the server sent them, and
  /// nothing is picked in advance: the row pressed *is* the choice.
  it("offers every registered repo, in the server's order", async () => {
    theWorkbench();
    const { container } = mount();
    await openNewConversation(container);

    expect((await repoRows(container)).map((row) => row.textContent)).toEqual(
      REPOS.map((repo) => repo.name),
    );
  });

  /// Nothing of it is on the page until it is asked for, which is the point of
  /// the menu: the sidebar is the conversations, and the way to add to them is
  /// one button.
  it("keeps the repos out of the sidebar until the button is pressed", async () => {
    theWorkbench();
    const { container } = mount();
    await drawn(container, ".new-conversation > .menu-trigger");

    expect(container.querySelector(".new-conversation > .menu-drop")).toBeNull();
  });

  it("closes once a repo has been chosen", async () => {
    theWorkbench(json({ Started: { id: OPEN.id } }));
    const { container } = mount();
    await openNewConversation(container);

    fireEvent.click((await repoRows(container))[0]!);

    await waitFor(() =>
      expect(container.querySelector(".new-conversation > .menu-drop")).toBeNull(),
    );
  });

  /// The way out that needs no aim, and the focus back on the button it came
  /// from rather than at the top of the page.
  it("closes on escape, and gives the button back the focus", async () => {
    theWorkbench();
    const { container } = mount();
    await openNewConversation(container);

    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() =>
      expect(container.querySelector(".new-conversation > .menu-drop")).toBeNull(),
    );
    expect(document.activeElement).toBe(
      container.querySelector(".new-conversation > .menu-trigger"),
    );
  });

  /// A press away from it lands on the backdrop rather than on the page, so the
  /// press that takes the menu back cannot also open a conversation.
  it("closes on a press outside it", async () => {
    theWorkbench();
    const { container } = mount();
    await openNewConversation(container);

    fireEvent.click(await drawn(container, ".new-conversation > .menu-backdrop"));

    await waitFor(() =>
      expect(container.querySelector(".new-conversation > .menu-drop")).toBeNull(),
    );
  });

  /// Opened from the keyboard, the first row is where the human is going: a
  /// menu whose first Tab landed past it is one they would have to walk
  /// backwards out of.
  it("puts the focus on the first row when it opens", async () => {
    theWorkbench();
    const { container } = mount();
    await openNewConversation(container);

    const rows = await repoRows(container);
    expect(document.activeElement).toBe(rows[0]);
  });

  /// A server that could not answer at all is the one thing here that is an
  /// error rather than an outcome — and the menu stays open to say it in,
  /// because a press that failed left nothing else on the screen to carry it.
  it("stays open to say a start failed", async () => {
    theWorkbench(
      whenever(
        "/api/ui/conversations",
        () => Promise.reject(new Error("down")),
        "POST",
      ),
    );
    const { container } = mount();
    await openNewConversation(container);

    fireEvent.click((await repoRows(container))[0]!);

    const said = await drawn(container, ".new-conversation > .menu-drop .error");
    expect(said.textContent).toContain("down");
  });
});

describe("the adopt-a-roadmap group", () => {
  /// Under the repos and behind a heading of its own, because pressing one
  /// starts a different kind of conversation.
  it("names each abandoned roadmap, its repo and the stage it is up to", async () => {
    theWorkbench(whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)));
    const { container } = mount();
    await openNewConversation(container);

    const group = await drawn(container, ".menu-group");
    expect(group.querySelector(".menu-heading")!.textContent).toBe(
      "Adopt a roadmap",
    );

    const rows = [
      ...group.querySelectorAll<HTMLButtonElement>(".adopt-roadmap"),
    ];
    const flat = ABANDONED.flatMap((repo) =>
      repo.roadmaps.map((roadmap) => ({ repo: repo.repo, roadmap })),
    );
    expect(rows.length).toBe(flat.length);

    for (const [n, held] of flat.entries()) {
      const said = rows[n]!.textContent!;
      expect(said).toContain(held.roadmap.name);
      expect(said).toContain(held.repo);
      expect(said).toContain(held.roadmap.stage);
      expect(said).toContain(held.roadmap.stage_title);
    }
  });

  /// The roadmaps are in the menu and nowhere else: nothing is waiting on the
  /// human here, so nothing of it is in view until the menu is opened.
  it("leaves no notice in the sidebar", async () => {
    theWorkbench(whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)));
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    expect(container.querySelector(".abandoned")).toBeNull();
    expect(container.querySelector(".adopt-roadmap")).toBeNull();
  });

  /// Beneath the repos, which is the order the two are decided in: starting
  /// work is the ordinary thing, and adopting is what is also there.
  it("is drawn beneath the repos", async () => {
    theWorkbench(whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)));
    const { container } = mount();
    await openNewConversation(container);

    const group = await drawn(container, ".menu-group");
    expect((await repoRows(container))[0]!.compareDocumentPosition(group)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  /// Nothing to adopt is nothing to say. A Repo whose roadmaps are all
  /// complete, mid-flight or broken contributes nothing at all, and a menu with
  /// none draws no heading over an empty group.
  it("says nothing when there is nothing to adopt", async () => {
    theWorkbench();
    const { container } = mount();
    await openNewConversation(container);

    expect(container.querySelector(".menu-group")).toBeNull();
  });

  /// Read again with everything else the page is showing, because the server
  /// reads it off the repositories every time it is asked: a roadmap somebody
  /// has since picked up stops being on the list, and the row goes with it.
  it("is read again when the page looks again", async () => {
    const fetching = theWorkbench(
      whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)),
    );
    const { container } = mount();
    await openNewConversation(container);
    await drawn(container, ".menu-group");

    const before = askedFor(fetching, "/api/ui/abandoned-roadmaps");
    readAgain();

    await waitFor(() =>
      expect(askedFor(fetching, "/api/ui/abandoned-roadmaps")).toBeGreaterThan(
        before,
      ),
    );
  });

  /// Pressing a roadmap starts a conversation to adopt it with, and goes
  /// straight into it — which is where both pairings and the base commit are
  /// fixed, and where adopting is pressed.
  ///
  /// What goes out is the repo and the roadmap and nothing else: which stage is
  /// next is the roadmap's own answer at the commit the conversation ends up
  /// branching from, and the page reads it back there.
  it("starts a conversation to adopt the roadmap that was pressed", async () => {
    const fetching = theWorkbench(
      whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)),
      whenever("/api/ui/adoptions", json({ Started: { id: OPEN.id } }), "POST"),
    );
    const { container, history } = mount();
    await openNewConversation(container);

    const rows = await drawn(container, ".menu-group");
    fireEvent.click(
      rows.querySelectorAll<HTMLButtonElement>(".adopt-roadmap")[1]!,
    );

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/adoptions")).toEqual({
        repo_id: ABANDONED[0]!.repo_id,
        roadmap: ABANDONED[0]!.roadmaps[1]!.name,
      }),
    );
    await waitFor(() => expect(history.get()).toBe(`/conversations/${OPEN.id}`));
  });
});

/// The page a conversation started from that notice opens on: the roadmap and
/// its stage named, the two profiles and the base commit to fix, and one press.
describe("the adoption page", () => {
  /// Everything the server said about the roadmap at this conversation's base
  /// commit — which is what the press would start, rather than what the notice
  /// happened to show when it was clicked.
  const ADOPTION = ADOPTING.adopting!;

  /// The workbench with the adopting conversation opened instead of the
  /// drafting one. Both fixtures carry the same id, which is what the sidebar
  /// row and the URL are shared through.
  function theAdoption(...answers: Parameters<typeof serving>) {
    return theWorkbench(
      whenever(`/api/ui/conversations/${ADOPTING.id}`, json(ADOPTING)),
      ...answers,
    );
  }

  it("names the roadmap and the stage adopting would start", async () => {
    theAdoption();
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    const panel = await drawn(container, ".adoption");
    expect(panel.textContent).toContain(ADOPTION.roadmap);
    expect(panel.textContent).toContain(ADOPTION.title);
    expect(panel.textContent).toContain(ADOPTION.stage!.label);
    expect(panel.textContent).toContain(ADOPTION.stage!.title);

    // And where the brief comes from, and what the work will be done on — which
    // is the stage's own slug rather than the name the server invented for the
    // row.
    expect(panel.textContent).toContain(ADOPTION.stage!.brief_path);
    expect(panel.textContent).toContain(ADOPTION.stage!.branch);
  });

  it("offers both profiles, the base commit and one adopt press", async () => {
    theAdoption();
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    await drawn(container, ".adoption");

    await waitFor(() => screen.getByLabelText("Grilling"));
    expect(screen.getByLabelText("Implementation")).toBeTruthy();
    expect(screen.getByLabelText("Base branch")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Adopt" })).toBeTruthy();
  });

  /// There is nothing to type and nothing to grill: the brief is the stage
  /// brief, and it arrives when the stage is adopted. The branch is not offered
  /// either — a stage is worked on its own slug, so the name the row carries is
  /// discarded at the press.
  it("offers no brief editor, no start grilling and no branch field", async () => {
    theAdoption();
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    await drawn(container, ".adoption");

    expect(screen.queryByLabelText("Brief")).toBeNull();
    expect(container.querySelector(".start-grilling")).toBeNull();
    expect(screen.queryByLabelText("Branch")).toBeNull();
  });

  /// The stage is the server's reading at the base, so a base recorded is a page
  /// read again — and what it names then is the stage that is next *there*.
  it("names the stage again when the base branch changes", async () => {
    const elsewhere: ConversationView = {
      ...ADOPTING,
      base_commit: "release-1.4",
      adopting: {
        ...ADOPTION,
        stage: {
          label: "03",
          title: "Implementation",
          brief_path: "docs/roadmaps/mvp/03-implementation.md",
          branch: "implementation",
        },
      },
    };

    let recorded = false;
    theAdoption(
      whenever(`/api/ui/conversations/${ADOPTING.id}`, () =>
        json(recorded ? elsewhere : ADOPTING)(),
      ),
      whenever(
        `/api/ui/conversations/${ADOPTING.id}/base`,
        () => {
          recorded = true;
          return json("Recorded")();
        },
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    await waitFor(() =>
      expect(container.querySelector(".adoption .stage")!.textContent).toContain(
        ADOPTION.stage!.title,
      ),
    );

    fireEvent.change(await waitFor(() => screen.getByLabelText("Base branch")), {
      target: { value: elsewhere.base_commit },
    });

    await waitFor(() =>
      expect(container.querySelector(".adoption .stage")!.textContent).toContain(
        "Implementation",
      ),
    );
  });

  /// A roadmap that has since been finished, gone, or picked up reads the same
  /// way here: the roadmap is still named, and there is no stage under it.
  /// Which of those it was is the press's to say by name.
  it("says when there is no stage to adopt at the base commit", async () => {
    const nothing: ConversationView = {
      ...ADOPTING,
      adopting: { ...ADOPTION, title: "", stage: null },
    };
    theAdoption(
      whenever(`/api/ui/conversations/${ADOPTING.id}`, json(nothing)),
    );
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    const panel = await drawn(container, ".adoption");
    expect(panel.textContent).toContain(ADOPTION.roadmap);
    expect(panel.querySelector(".empty")).toBeTruthy();
    expect(panel.querySelector(".stage")).toBeNull();
  });

  /// The press posts to the conversation's own adopt route with nothing in the
  /// body, for the reason starting a grilling sends nothing: which conversation
  /// is in the path, and which stage is the roadmap's own answer at the base
  /// commit — read again by the server when the button is pressed.
  it("posts to the conversation's own adopt route, with nothing in the body", async () => {
    const fetching = theAdoption(
      whenever(
        `/api/ui/conversations/${ADOPTING.id}/adopt`,
        json("Adopted" satisfies Adopted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    fireEvent.click(await drawn(container, ".adoption .adopt"));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${ADOPTING.id}/adopt`),
      ).toEqual({}),
    );
  });

  /// And what a press leaves behind is a conversation that has moved and a
  /// notice with one roadmap fewer in it, so all three are read again.
  it("reads the conversation and the notice again once it has been pressed", async () => {
    const fetching = theAdoption(
      whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)),
      whenever(
        `/api/ui/conversations/${ADOPTING.id}/adopt`,
        json("Adopted" satisfies Adopted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    const before = askedFor(fetching, `/api/ui/conversations/${ADOPTING.id}`);
    fireEvent.click(await drawn(container, ".adoption .adopt"));

    await waitFor(() =>
      expect(
        askedFor(fetching, `/api/ui/conversations/${ADOPTING.id}`),
      ).toBeGreaterThan(before),
    );
  });

  /// A press that was refused says which refusal it was. Every one of them is
  /// something different to go and do — a profile to choose, a box somebody
  /// ticked, a branch somebody is on — so a single "cannot adopt" would leave
  /// the human guessing which.
  it("says which refusal a press came back with", async () => {
    for (const outcome of [
      "NoImplementationProfile",
      "RoadmapComplete",
      "StageInFlight",
      "BranchExists",
    ] satisfies Adopted[]) {
      theAdoption(
        whenever(
          `/api/ui/conversations/${ADOPTING.id}/adopt`,
          json(outcome satisfies Adopted),
          "POST",
        ),
      );
      const { container, unmount } = mount(`/conversations/${ADOPTING.id}`);

      fireEvent.click(await drawn(container, ".adoption .adopt"));

      await waitFor(() =>
        expect(container.querySelector(".adoption .error")!.textContent).toBe(
          ADOPT_REFUSAL[outcome],
        ),
      );

      unmount();
    }
  });
});

describe("a conversation's timeline", () => {
  it("reads the conversation the URL names", async () => {
    const fetching = theWorkbench();
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));
    expect(fetching).toHaveBeenCalledWith(
      `/api/ui/conversations/${OPEN.id}`,
      expect.anything(),
    );
  });

  /// Frozen, which is the state a Brief is read in: while the Conversation is
  /// drafting it is the field the tests below type into.
  it("draws a frozen brief inline, as the server rendered it", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const body = await drawn(container, ".brief-body");

    // The server's own HTML, put in the page: the browser has no markdown
    // parser and never needed one.
    expect(body.innerHTML).toBe(briefOf(GRILLING).html);
    expect(body.querySelector("h1")).toBeTruthy();
  });

  it("is a list of events rather than a brief with a page around it", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    // One kind of Event so far, drawn as one entry of the list the stages after
    // this one add to.
    expect(container.querySelectorAll(".timeline > .timeline-event")).toHaveLength(
      OPEN.timeline.length,
    );
  });

  it("says what to do with a conversation nobody has picked", async () => {
    theWorkbench();
    mount();

    await waitFor(() => screen.getByText("Pick a conversation, or start one."));
  });

  it("shows the server's own wording when a conversation cannot be read", async () => {
    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever(
        `/api/ui/conversations/${OPEN.id}`,
        json({ error: "the Conversation could not be read" }, 500),
      ),
    );
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() =>
      screen.getByText(/the Conversation could not be read/),
    );
  });
});

/// The brief while the Conversation is drafting: a field that is always there
/// and saves itself, rather than a rendering with a way into a form.
describe("writing the brief", () => {
  const field = () => screen.getByLabelText("Brief") as HTMLTextAreaElement;

  /// Where a save of the Brief goes.
  const WRITING = `/api/ui/conversations/${OPEN.id}/brief`;

  it("is a field on what was last written, with nothing to press to open it", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    // The markdown, not the HTML: the source travels beside the rendering for
    // exactly this, so the field needs no parser to fill itself in.
    expect(field().value).toBe(BRIEF.markdown);
    expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();
  });

  /// A copy of what is in the field is what gives it its height — the field
  /// itself never scrolls, and there is no handle to drag.
  it("grows with what is typed into it", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    const growing = container.querySelector(".brief .grow")!;
    expect(growing.getAttribute("data-value")).toBe(BRIEF.markdown);

    fireEvent.input(field(), { target: { value: "# One\n\n# Two\n" } });
    expect(growing.getAttribute("data-value")).toBe("# One\n\n# Two\n");
  });

  it("saves what was typed when the field is left", async () => {
    const written = "# Rate limiting\n\nDecide where the counter lives.\n";
    const fetching = theWorkbench(json("Saved"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    fireEvent.input(field(), { target: { value: written } });
    fireEvent.blur(field());

    await waitFor(() => expect(sent(fetching, WRITING)).toEqual({ markdown: written }));

    // The field stays where it is, and says nothing at all about the save.
    expect(field().value).toBe(written);
    expect(container.querySelector(".brief-standing")).toBeNull();
  });

  it("saves what was typed after a pause in the typing", async () => {
    const fetching = theWorkbench(json("Saved"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    // The clock is this test's from here: what it is about is a pause, and a
    // real one would be a real wait on every run.
    vi.useFakeTimers();
    fireEvent.input(field(), { target: { value: "# Half a" } });
    fireEvent.input(field(), { target: { value: "# Half a thought" } });

    // Mid-sentence, and nothing has gone out: a save a keystroke is what the
    // pause is there to stop.
    await vi.advanceTimersByTimeAsync(100);
    expect(writes(fetching, WRITING)).toBe(0);

    await vi.advanceTimersByTimeAsync(2_000);

    // One save, of the whole of what was typed rather than of the first half.
    expect(writes(fetching, WRITING)).toBe(1);
    expect(sent(fetching, WRITING)).toEqual({ markdown: "# Half a thought" });
  });

  /// A save is a round trip, and the human goes on typing across it. What was
  /// typed while it was in the air is only in the field, so the field has to
  /// send it the moment the save it was waiting on is over — one save at a
  /// time, but never one save and then silence.
  it("saves what was typed while a save was in the air", async () => {
    const answering = holding(json("Saved"));
    const fetching = theWorkbench(whenever(WRITING, answering.held, "POST"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    fireEvent.input(field(), { target: { value: "# Half a" } });
    fireEvent.blur(field());
    await waitFor(() => expect(writes(fetching, WRITING)).toBe(1));

    // Typed into and left again while that first save is still unanswered: one
    // save at a time, so nothing more goes out yet.
    fireEvent.input(field(), { target: { value: "# Half a thought" } });
    fireEvent.blur(field());
    expect(writes(fetching, WRITING)).toBe(1);

    answering.land();

    // And the rest of it goes out on the back of the answer, without waiting
    // for another keystroke.
    await waitFor(() => expect(writes(fetching, WRITING)).toBe(2));
    expect(sent(fetching, WRITING, 1)).toEqual({
      markdown: "# Half a thought",
    });
  });

  /// The card said Saving… / Not saved yet / Saved beside the heading, and now
  /// says nothing at all: a field that keeps itself needs no running commentary,
  /// and the line changed often enough to pull the eye off what was being typed.
  it("says nothing beside the heading in any state of a save", async () => {
    const fetching = theWorkbench(json("Saved"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    const quiet = () => {
      expect(container.querySelector(".brief-standing")).toBeNull();
      expect(container.textContent).not.toContain("Saving");
      expect(container.textContent).not.toContain("Not saved yet");
      expect(container.textContent).not.toContain("Saved");
    };

    // Untouched, typed into and not yet saved, saving, and saved.
    quiet();
    fireEvent.input(field(), { target: { value: "# Something" } });
    quiet();
    fireEvent.blur(field());
    quiet();
    await waitFor(() => expect(writes(fetching, WRITING)).toBe(1));
    quiet();
  });

  /// Leaving a field nothing was typed into is not an edit, and neither is
  /// coming back to one that has already been saved.
  it("says nothing to the server when the field has not moved", async () => {
    const fetching = theWorkbench(json("Saved"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    fireEvent.blur(field());
    fireEvent.input(field(), { target: { value: BRIEF.markdown } });
    fireEvent.blur(field());

    expect(writes(fetching, WRITING)).toBe(0);
  });

  /// The freeze can land between a keystroke and the save it caused, which is
  /// the one thing the human cannot see coming.
  it("keeps what was written and says why when the server refuses it", async () => {
    const fetching = theWorkbench(json("NotDrafting"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    fireEvent.input(field(), { target: { value: "# Too late\n" } });
    fireEvent.blur(field());

    await waitFor(() => screen.getByText(/frozen when grilling started/i));
    // The draft is the only copy of what was written, so it stands.
    expect(field().value).toBe("# Too late\n");
    expect(writes(fetching, WRITING)).toBe(1);
  });

  /// A brief that has frozen does not thaw, so what is written after the
  /// refusal is not worth asking about again — and asking on every pause would
  /// be a request a second for as long as the human kept typing.
  it("stops trying once it has been refused", async () => {
    const fetching = theWorkbench(json("NotDrafting"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    fireEvent.input(field(), { target: { value: "# Too late\n" } });
    fireEvent.blur(field());
    await waitFor(() => screen.getByText(/frozen when grilling started/i));

    fireEvent.input(field(), { target: { value: "# Too late still\n" } });
    fireEvent.blur(field());

    expect(writes(fetching, WRITING)).toBe(1);
    // And the refusal is still on the card, over what is still in the field.
    expect(screen.getByText(/frozen when grilling started/i)).toBeTruthy();
  });

  /// The page reads its Conversation again on every Nudge about it and on every
  /// return to the page, and a field being typed into has to live through both:
  /// a Brief half written is the only copy of it there is, and one that went
  /// every time the world was read again could never be finished.
  it("lives through the page reading the conversation again", async () => {
    // The read that lands while the field is being typed into comes back with
    // something else about the Conversation changed — the branch renamed from
    // another device — so the test can wait for it to have landed rather than
    // sleep until it has. What is asked here is what a read does to the field,
    // and it does the same whether or not anything came back different.
    const RENAMED: ConversationView = { ...OPEN, branch: "work/renamed-away" };
    let standing = OPEN;
    const fetching = serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(READING, () => json(standing)()),
    );
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    fireEvent.input(field(), { target: { value: "# Half a thought" } });

    standing = RENAMED;
    readAgain();
    await waitFor(() => screen.getByRole("heading", { name: RENAMED.branch }));

    // The read landed, and what was typed is still in the field.
    expect(field().value).toBe("# Half a thought");
    expect(askedFor(fetching, READING)).toBeGreaterThan(1);
  });

  /// Frozen, the Brief is a document rather than a field — which is also how an
  /// adopting Conversation has always drawn one.
  it("is no field at all once the grilling has started", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".brief-body");

    expect(screen.queryByLabelText("Brief")).toBeNull();
    expect(container.querySelector(".brief-standing")).toBeNull();
  });
});

/// Setting a conversation up: what has to be settled before anything will run
/// it, drawn under the brief it belongs to.
describe("a conversation's setup", () => {
  /// The brief is the card's headline and the setup follows it, because setting
  /// the work up and kicking it off are one act and both happen in the record.
  it("stands on the brief card, under the brief itself", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const setup = await drawn(container, ".timeline-event > .brief .conversation-setup");

    expect(setup.querySelector(".branch-name")).toBeTruthy();
    expect(setup.querySelector(".base-branch")).toBeTruthy();
    expect(setup.querySelector(".conversation-profiles")).toBeTruthy();

    // Under the words rather than over them: the brief is what the card is,
    // and while it is a draft the words are the field they are typed into.
    const body = container.querySelector(".brief .grow")!;
    expect(
      body.compareDocumentPosition(setup) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// Branch, base and both pairings freeze server-side when grilling starts, so
  /// past that moment there is nothing here that could be changed — and nothing
  /// is drawn rather than drawn disabled.
  it("is gone entirely once the grilling has started", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".timeline-event > .brief");

    expect(container.querySelector(".conversation-setup")).toBeNull();
    expect(screen.queryByLabelText("Branch")).toBeNull();
    expect(screen.queryByLabelText("Base branch")).toBeNull();
    expect(screen.queryByLabelText("Grilling")).toBeNull();
  });

  /// What the conversation is attached to and where it has got to were three
  /// read-only lines in a pane that no longer exists. The record tells that
  /// story, so they are drawn nowhere at all.
  it("shows the repo, the worktree path and the state nowhere", async () => {
    theWorkbenchWith({
      state: "Grilling",
      worktree: { path: "/var/lib/verkstead/worktrees/verkstead-open", missing: false },
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, ".timeline-event > .brief");

    expect(container.querySelector(".conversation-facts")).toBeNull();
    expect(container.querySelector(".conversation-worktree")).toBeNull();
    expect(container.textContent).not.toContain(OPEN.repo.path);
    expect(container.textContent).not.toContain(
      "/var/lib/verkstead/worktrees/verkstead-open",
    );
  });

  /// The branch keeps itself the way the brief above it does: there is nothing
  /// to press, and leaving the field is what sends what is in it.
  it("offers the branch name the server prefilled, and sends a new one", async () => {
    const fetching = theWorkbench(json("Renamed"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Branch"));

    const field = screen.getByLabelText("Branch") as HTMLInputElement;
    expect(field.value).toBe(OPEN.branch);
    expect(screen.queryByRole("button", { name: "Rename" })).toBeNull();

    fireEvent.input(field, { target: { value: "counter-in-redis" } });
    fireEvent.blur(field);

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/branch`),
      ).toEqual({ branch: "counter-in-redis" }),
    );
  });

  it("saves a typed branch name after a pause in the typing", async () => {
    const fetching = theWorkbench(json("Renamed"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Branch"));
    const naming = `/api/ui/conversations/${OPEN.id}/branch`;

    vi.useFakeTimers();
    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "counter-in" },
    });
    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "counter-in-redis" },
    });

    // Mid-name, and nothing has gone out.
    await vi.advanceTimersByTimeAsync(100);
    expect(writes(fetching, naming)).toBe(0);

    await vi.advanceTimersByTimeAsync(2_000);

    // One save, of the whole of what was typed.
    expect(writes(fetching, naming)).toBe(1);
    expect(sent(fetching, naming)).toEqual({ branch: "counter-in-redis" });
  });

  /// The same as the brief above it, because it is the same keeping: what was
  /// typed across a save in flight goes out when that save is over.
  it("saves a name typed while a save was in the air", async () => {
    const naming = `/api/ui/conversations/${OPEN.id}/branch`;
    const answering = holding(json("Renamed"));
    const fetching = theWorkbench(whenever(naming, answering.held, "POST"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Branch"));

    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "counter-in" },
    });
    fireEvent.blur(screen.getByLabelText("Branch"));
    await waitFor(() => expect(writes(fetching, naming)).toBe(1));

    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "counter-in-redis" },
    });
    fireEvent.blur(screen.getByLabelText("Branch"));
    expect(writes(fetching, naming)).toBe(1);

    answering.land();

    await waitFor(() => expect(writes(fetching, naming)).toBe(2));
    expect(sent(fetching, naming, 1)).toEqual({ branch: "counter-in-redis" });
  });

  /// Whether a name is one git would take is the server's to say, so a pause in
  /// the middle of typing one can come back refused for what is not there yet.
  /// The refusal is said in words and goes away by itself once the name is one
  /// the server will have.
  it("says why a branch name was refused, and heals when it is valid", async () => {
    let outcome = "NotABranchName";
    const fetching = theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/branch`,
        () => json(outcome)(),
        "POST",
      ),
    );
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Branch"));

    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "two..dots" },
    });
    fireEvent.blur(screen.getByLabelText("Branch"));

    await waitFor(() => screen.getByText(/will not take that as a branch name/i));

    // The next keystroke fixes it, and the save after that clears the refusal:
    // a name refused is not a field that has stopped trying.
    outcome = "Renamed";
    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "two-dots" },
    });
    fireEvent.blur(screen.getByLabelText("Branch"));

    await waitFor(() =>
      expect(
        writes(fetching, `/api/ui/conversations/${OPEN.id}/branch`),
      ).toBe(2),
    );
    await waitFor(() =>
      expect(screen.queryByText(/will not take that as a branch name/i)).toBeNull(),
    );
  });

  /// The base dropdown, once the branch it is about to be asked for is one of
  /// the options.
  ///
  /// Waited for rather than taken the moment the label is drawn: the branches
  /// arrive on a read of their own, and a `<select>` told to show an option it
  /// has not been given yet falls to the first one it has — which would be the
  /// rule, and a pick nobody made.
  async function basePicker(offering: string): Promise<HTMLSelectElement> {
    const picker = (await waitFor(() =>
      screen.getByLabelText("Base branch"),
    )) as HTMLSelectElement;

    await waitFor(() =>
      expect([...picker.options].map((option) => option.value)).toContain(
        offering,
      ),
    );

    return picker;
  }

  /// The rule first, then the branches — and no way to type a commit, because
  /// there is no longer anything to pin to but a branch.
  it("offers the default rule and then every branch of the repo", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);

    const picker = (await waitFor(() =>
      screen.getByLabelText("Base branch"),
    )) as HTMLSelectElement;

    await waitFor(() => expect(picker.options).toHaveLength(4));
    expect([...picker.options].map((option) => option.value)).toEqual([
      "",
      ...BRANCHES,
    ]);
    expect(picker.options[0]!.textContent).toContain(OPEN.repo.default_branch);

    expect(picker.value, "the pinned branch is what it shows").toBe(
      OPEN.base_commit,
    );
    expect(screen.queryByRole("button", { name: "Record" })).toBeNull();
  });

  it("records the branch that was picked, by name", async () => {
    const fetching = theWorkbench(json("Recorded"));
    mount(`/conversations/${OPEN.id}`);

    fireEvent.change(await basePicker("origin/main"), {
      target: { value: "origin/main" },
    });

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/base`)).toEqual({
        branch: "origin/main",
      }),
    );
  });

  /// The first entry is the override taken away, not a branch called nothing —
  /// and what it goes back to is the rule, which the pane says in words because
  /// a dropdown entry cannot say when it will resolve.
  it("takes the override away when the rule is picked", async () => {
    const fetching = theWorkbench(json("Recorded"));
    mount(`/conversations/${OPEN.id}`);

    fireEvent.change(await basePicker("origin/main"), {
      target: { value: "" },
    });

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/base`)).toEqual({
        branch: null,
      }),
    );
  });

  /// A base the list does not hold — the branch taken away since it was picked,
  /// or the list still on its way — is drawn all the same: falling quietly to
  /// the rule would show one base while the record held another.
  it("still shows a pinned branch the list has lost", async () => {
    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/repos/${OPEN.repo.id}/branches`, json(["main"])),
      whenever(`/api/ui/conversations/${OPEN.id}`, json(OPEN)),
    );
    mount(`/conversations/${OPEN.id}`);

    const picker = await basePicker("main");

    await waitFor(() => expect(picker.options).toHaveLength(3));
    expect([...picker.options].map((option) => option.value)).toEqual([
      "",
      OPEN.base_commit,
      "main",
    ]);
    expect(picker.value).toBe(OPEN.base_commit);
  });

  it("names the branch an unpinned conversation will start from", async () => {
    const rule: ConversationView = { ...OPEN, base_commit: null };
    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/repos/${OPEN.repo.id}/branches`, json(BRANCHES)),
      whenever(`/api/ui/conversations/${OPEN.id}`, json(rule)),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByLabelText("Base branch"));

    expect(
      (screen.getByLabelText("Base branch") as HTMLSelectElement).value,
    ).toBe("");
    expect(container.querySelector(".base-branch .note")!.textContent).toContain(
      OPEN.repo.default_branch,
    );
  });
});

/// The last thing a conversation settles before anything will run it: which
/// account and model grills, and which implements.
describe("a conversation's pairings", () => {
  /// A conversation showing neither choice, which is what a freshly started one
  /// looks like.
  const UNCHOSEN: ConversationView = {
    ...OPEN,
    grilling_pairing: null,
    implementation_pairing: null,
    ready_to_grill: false,
  };

  /// What one row of a picker sends, as the picker writes it.
  const pairing = (profile: ProfileEntry, model: string) =>
    `${profile.id}:${model}`;

  function withConversation(
    view: ConversationView,
    ...answers: Array<() => Promise<Response>>
  ) {
    return serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/conversations/${OPEN.id}`, json(view)),
      ...answers,
    );
  }

  it("shows the two pairings the conversation has chosen", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    const grilling = screen.getByLabelText("Grilling") as HTMLSelectElement;
    const implementing = screen.getByLabelText(
      "Implementation",
    ) as HTMLSelectElement;

    // Separate choices, and in the fixture genuinely separate accounts: grill on
    // fable, implement on opus.
    expect(grilling.value).toBe(
      pairing(OPEN.grilling_pairing!.profile, OPEN.grilling_pairing!.model!),
    );
    expect(implementing.value).toBe(
      pairing(
        OPEN.implementation_pairing!.profile,
        OPEN.implementation_pairing!.model!,
      ),
    );
    expect(grilling.value).not.toBe(implementing.value);
  });

  /// One flat row per profile-and-model combination, labelled with both — a
  /// profile listing two models is two rows, because a session runs one of
  /// them and the pick says which.
  it("offers every profile-and-model combination as one flat list", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    const options = Array.from(
      (screen.getByLabelText("Grilling") as HTMLSelectElement).options,
    ).map((option) => option.text);

    expect(options).toEqual(
      PROFILES.flatMap((profile) =>
        profile.models.map((model) => `${profile.name} — ${model}`),
      ),
    );
  });

  it("sends each choice on its own, to its own role", async () => {
    const fetching = withConversation(UNCHOSEN, json("Chosen"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    fireEvent.change(screen.getByLabelText("Grilling"), {
      target: { value: pairing(PROFILES[0]!, PROFILES[0]!.models[0]!) },
    });
    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/grilling-pairing`),
      ).toEqual({
        profile_id: PROFILES[0]!.id,
        model: PROFILES[0]!.models[0],
      }),
    );

    // The second of a profile's models, which is the half of the choice a
    // profile alone could never have said.
    fireEvent.change(screen.getByLabelText("Implementation"), {
      target: { value: pairing(PROFILES[1]!, PROFILES[1]!.models[1]!) },
    });
    await waitFor(() =>
      expect(
        sent(
          fetching,
          `/api/ui/conversations/${OPEN.id}/implementation-pairing`,
        ),
      ).toEqual({
        profile_id: PROFILES[1]!.id,
        model: PROFILES[1]!.models[1],
      }),
    );
  });

  /// A profile chosen before models were paired with them is half a choice: the
  /// picker draws it as none, and says so where it would have been shown.
  it("reads a profile with no model beside it as nothing chosen", async () => {
    withConversation({
      ...OPEN,
      grilling_pairing: { ...OPEN.grilling_pairing!, model: null },
      ready_to_grill: false,
    });
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByLabelText("Grilling"));
    expect((screen.getByLabelText("Grilling") as HTMLSelectElement).value).toBe(
      "",
    );
    await waitFor(() => screen.getByText(/was chosen before models were/));
  });

  /// Both pairings are fixed when grilling starts, and the refusal is the
  /// server's to make: the picker is drawn the same and says what came back.
  it("says a choice was refused once the grilling has started", async () => {
    withConversation(OPEN, json("NotDrafting"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    fireEvent.change(screen.getByLabelText("Grilling"), {
      target: { value: pairing(PROFILES[0]!, PROFILES[0]!.models[0]!) },
    });

    await waitFor(() =>
      screen.getByText(
        "The grilling has started, so who runs this conversation is settled.",
      ),
    );
  });

  /// A conversation missing either profile is identifiably not ready, and the
  /// answer is the server's rather than a count of the two fields. What is
  /// missing is not said here, though: the button at the end of the record is
  /// the one thing that explains itself, and a verdict up here as well would be
  /// the same complaint twice.
  it("says nothing about readiness until there is something to affirm", async () => {
    withConversation(UNCHOSEN);
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByLabelText("Grilling"));
    expect(container.querySelector(".readiness")).toBeNull();
    expect(screen.queryByText(/Not ready to grill/)).toBeNull();

    // The fixture's own conversation has both, and the server says so.
    expect(OPEN.ready_to_grill).toBe(true);
  });

  /// One row where the pane is wide enough for two, which is the stylesheet's
  /// half of it; what this holds is that they are the one row's to lay out.
  it("draws the two pickers as a single row", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const row = await drawn(container, ".conversation-profiles .pairings");
    expect(row.querySelectorAll(".profile-choice")).toHaveLength(2);
  });

  it("says it is ready when the server says so", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByText("Ready to grill."));
    expect(container.querySelector(".readiness")!.classList).toContain("ready");
  });

  /// A profile whose pair has gone is not one to launch a session under. What is
  /// wrong with it is said where it is chosen, rather than left to be found out
  /// when a session will not start.
  it("says what is wrong with a chosen profile that has broken", async () => {
    const broken: ConversationView = {
      ...OPEN,
      implementation_pairing: {
        ...OPEN.implementation_pairing!,
        profile: {
          ...OPEN.implementation_pairing!.profile,
          broken: "ConfigMissing",
        },
      },
      ready_to_grill: false,
    };
    withConversation(broken);
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByText("Its config file is gone."));
    expect(container.querySelector(".readiness")).toBeNull();
  });

  it("says why a choice was refused, in words", async () => {
    withConversation(UNCHOSEN, json("NoSuchProfile"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    fireEvent.change(screen.getByLabelText("Grilling"), {
      target: { value: pairing(PROFILES[0]!, PROFILES[0]!.models[0]!) },
    });

    await waitFor(() => screen.getByText("That profile has been removed."));
  });

  it("says where to go when there is no profile to choose", async () => {
    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json([])),
      whenever(`/api/ui/conversations/${OPEN.id}`, json(UNCHOSEN)),
    );
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByText(/No agent profiles are saved yet/));
    expect(screen.getByText("add one").getAttribute("href")).toBe("/settings");
  });
});

describe("the panes on a narrow window", () => {
  it("starts on the conversations and walks in and back out", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    // Nothing picked, so the level being shown is the list.
    expect(frame(container).dataset.pane).toBe("conversations");

    fireEvent.click(screen.getByText(DRAFTING.branch));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("timeline"),
    );

    // The third level is the open Event's own, and nothing is open: there is
    // nothing to page into, so nothing offers to.
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));
    expect(screen.queryByRole("button", { name: "Details →" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "← Conversations" }));
    expect(frame(container).dataset.pane).toBe("conversations");
  });

  /// So walking in to the third level is opening something, and the way forward
  /// is what stands afterwards: it is drawn for a selection rather than for a
  /// conversation.
  it("walks on to the details by opening an event, and back out again", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");
    expect(screen.queryByRole("button", { name: "Details →" })).toBeNull();

    fireEvent.click(output);
    expect(frame(container).dataset.pane).toBe("details");

    fireEvent.click(screen.getByRole("button", { name: "← Timeline" }));
    expect(frame(container).dataset.pane).toBe("timeline");

    // Still open, so the way forward is there to be taken again.
    fireEvent.click(screen.getByRole("button", { name: "Details →" }));
    expect(frame(container).dataset.pane).toBe("details");
  });

  /// Opening a Conversation is a navigation, and Back is a way of changing which
  /// one is open that never goes through a click handler.
  it("follows the URL rather than the button that changed it", async () => {
    theWorkbench();
    const { container, history } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    expect(frame(container).dataset.pane).toBe("timeline");

    history.set({ value: "/" });
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("conversations"),
    );
  });

  /// What `data-pane` means is the stylesheet's, and there is nothing to query
  /// it off: jsdom lays nothing out. So the rules themselves are what is read.
  it("is one pane at a time until the window is wide enough for more", () => {
    expect(stylesheet).toContain(".workbench > .pane {\n  display: none;\n}");
    expect(stylesheet).toContain(
      '.workbench[data-pane="conversations"] > .conversations-pane,\n' +
        '.workbench[data-pane="timeline"] > .timeline-pane,\n' +
        '.workbench[data-pane="details"] > .details-pane {\n' +
        "  display: block;\n}",
    );

    // And side by side once there is room: the sidebar joins first, then the
    // third pane.
    expect(stylesheet).toContain("@media (min-width: 60rem) {");
    expect(stylesheet).toContain("@media (min-width: 80rem) {");
  });

  /// Every pane's header stays where it is put, whichever of the two ways the
  /// window is scrolling — the page below 60rem, the pane itself above it.
  /// Layout again, so the rules are what is read.
  it("keeps a pane's header at the top while the pane scrolls", () => {
    expect(stylesheet).toContain(
      ".pane > .pane-head,\n.pane > .pane-chrome {\n" +
        "  position: sticky;\n" +
        "  top: 0;\n" +
        "  z-index: 1;\n",
    );
  });

  /// And the record goes under it rather than being cut off against it: a rem of
  /// paper fading to nothing, hung in the gap the header already kept below
  /// itself so that at rest it covers no part of the first thing in the pane.
  it("fades the record out under whatever is stuck", () => {
    expect(stylesheet).toContain(
      ".pane > .pane-head::after,\n.pane > .pane-chrome::after {\n" +
        '  content: "";\n' +
        "  position: absolute;\n" +
        "  top: 100%;\n" +
        "  right: 0;\n" +
        "  left: 0;\n" +
        "  height: 1rem;\n" +
        "  background: linear-gradient(var(--paper), transparent);\n" +
        "  pointer-events: none;\n}",
    );
  });
});

/// The other shape the middle pane draws: a conversation that has been started,
/// with a move on its timeline and a worktree to say where the work is going on.
const GRILLING = grilling as ConversationView;
const CAPTURE = capture as Capture;
const TRANSCRIPT = transcript as TranscriptView;
const SCREEN = screenOfIt as Screen;

/// The session's output on the grilling conversation's timeline.
const OUTPUT = (() => {
  const found = GRILLING.timeline.find((event) => "AgentOutput" in event);
  if (!found || !("AgentOutput" in found)) {
    throw new Error("the fixture should hold a session's output");
  }
  return found.AgentOutput;
})();

/// Where the details pane fetches the whole of it from.
const CAPTURE_OF_IT = `/api/ui/conversations/${GRILLING.id}/capture/${OUTPUT.id}`;

/// And where it fetches what the session was saying while it printed that.
const TRANSCRIPT_OF_IT = `/api/ui/conversations/${GRILLING.id}/transcript/${OUTPUT.id}`;

/// And where it fetches the grid those bytes leave on a terminal.
const SCREEN_OF_IT = `/api/ui/conversations/${GRILLING.id}/screen/${OUTPUT.id}`;

/// And where it watches that grid while the session is still drawing it.
const WATCHING_IT = `/api/ui/conversations/${GRILLING.id}/screen/${OUTPUT.id}/attach`;

/// The repaint the fixture holds, as the socket sends one.
const PAINTED: Shown = { Painted: SCREEN };

/// A stand-in for the socket a live session's Screen is watched over.
///
/// jsdom has a `WebSocket` and it would dial one, so this stands where it goes
/// — the same shape from the page's side, with what the server says pushed in by
/// the test rather than arriving over a network there is none of here.
class Attached {
  /// What the page reads off the real one to know it may write — see the
  /// measuring in `Screen.tsx`.
  static readonly OPEN = 1;

  /// Every socket opened since the last test, in the order they were opened.
  static opened: Attached[] = [];

  readonly url: string;

  /// What the page has said up it, as it wrote it.
  readonly sent: string[] = [];

  readyState = Attached.OPEN;
  closed = false;

  private readonly listeners = new Map<string, Array<(event: never) => void>>();

  constructor(url: string) {
    this.url = url;
    Attached.opened.push(this);

    // A turn of the event loop later, as a real one is: a page that had a
    // socket open before it finished making one would be a page the browser
    // never gives it.
    queueMicrotask(() => this.fire("open", new Event("open")));
  }

  addEventListener(kind: string, listener: (event: never) => void): void {
    const listening = this.listeners.get(kind) ?? [];
    listening.push(listener);
    this.listeners.set(kind, listening);
  }

  removeEventListener(): void {}

  send(said: string): void {
    this.sent.push(said);
  }

  close(): void {
    this.closed = true;
    this.readyState = 3;
  }

  /// What the server says down it.
  says(shown: Shown): void {
    this.fire("message", { data: JSON.stringify(shown) } as never);
  }

  private fire(kind: string, event: unknown): void {
    for (const listener of this.listeners.get(kind) ?? []) {
      listener(event as never);
    }
  }
}

/// Wait for the page to have attached, and hand back the socket it opened.
function attached(): Promise<Attached> {
  return waitFor(() => {
    const socket = Attached.opened[0];
    if (!socket) {
      throw new Error("nothing has attached to a screen");
    }
    return socket;
  });
}

/// How many rows a record draws, which is no longer how many turns it holds: a
/// call and the answer to it are one card, so an answer its call is drawing has
/// no row of its own. Batches, because an accumulated record is the turns of
/// every reading of it and a pair can straddle the join.
function rows(...readings: Turn[][]): number {
  const said = readings.flat();
  const called = new Set(
    said.flatMap((turn) =>
      turn.kind === "ToolUse" && turn.call !== "" ? [turn.call] : [],
    ),
  );

  return said.filter(
    (turn) => !(turn.kind === "ToolResult" && called.has(turn.call)),
  ).length;
}

/// A record written here rather than by the server, for the shapes one session
/// of one fixture cannot hold at once: a pair that failed, a call still waiting
/// on its tool, an answer whose call is not in the record.
///
/// The turns are the wire's own type, so a field the server adds is a field
/// these have to carry — which is the whole of what keeps a hand-written
/// payload honest.
function recordOf(turns: Turn[], cursor = "9.9.9"): TranscriptView {
  return { turns, bookkeeping: [], whole: true, cursor };
}

/// A session that kept no record of its own conversation, which is what the
/// server says for every stub agent and every backend that writes no log — and
/// what sends the pane to the Capture.
const SAID_NOTHING: TranscriptView = {
  turns: [],
  bookkeeping: [],
  whole: true,
  cursor: "0.0.0",
};

/// What the same session said after that, as the server hands it to a pane that
/// already has the record above: the new turns alone, numbered on from where
/// that reading stopped (ADR 0009).
const MORE = more as TranscriptView;

/// And where the pane asks for it — the cursor the whole reading ended at, which
/// the fixture carries because the server wrote it there. Opaque to the page,
/// and to this test: what a reader does with one is hand it back.
const REST_OF_IT = `${TRANSCRIPT_OF_IT}?after=${encodeURIComponent(
  TRANSCRIPT.cursor,
)}`;

/// The workbench with the grilling conversation open instead of the drafting
/// one.
///
/// Its session left no Transcript, so this is the pane's fallback: the Capture,
/// byte for byte, exactly as it was drawn before there was another record to
/// draw. [`theSpeaking`] is the same conversation with one.
function theGrilling(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
    whenever(TRANSCRIPT_OF_IT, json(SAID_NOTHING)),
    whenever(CAPTURE_OF_IT, json(CAPTURE)),
    whenever(SCREEN_OF_IT, json(SCREEN)),
    ...answers,
  );
}

/// The same, with the session's own record of what it said — one of every kind
/// of turn a log holds, which is what the fixture was written to be.
function theSpeaking(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
    whenever(TRANSCRIPT_OF_IT, json(TRANSCRIPT)),
    whenever(CAPTURE_OF_IT, json(CAPTURE)),
    whenever(SCREEN_OF_IT, json(SCREEN)),
    ...answers,
  );
}

/// The same, with the session's output Event altered — for the states no
/// fixture holds, a running session being one: a fixture is a payload rather
/// than a moment, and one that said it was running would have a page drawing a
/// spinner over something that has not moved since 2026.
function theGrillingOutput(
  over: Partial<AgentOutputEvent>,
  ...answers: Parameters<typeof serving>
) {
  const altered: TimelineEvent[] = GRILLING.timeline.map((event) =>
    "AgentOutput" in event
      ? { AgentOutput: { ...event.AgentOutput, ...over } }
      : event,
  );

  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${GRILLING.id}`,
      json({ ...GRILLING, timeline: altered }),
    ),
    whenever(TRANSCRIPT_OF_IT, json(SAID_NOTHING)),
    whenever(CAPTURE_OF_IT, json(CAPTURE)),
    whenever(SCREEN_OF_IT, json(SCREEN)),
    // Last, so a test can hold one of those paths to an answer of its own: a
    // later answer for a path replaces the earlier.
    ...answers,
  );
}

/// Where a browser would have found an element, for the one assertion that
/// reads a measurement back: jsdom has no layout, so every element on the page
/// is at nothing and nothing wide, and a mark measured off two of them would
/// never be seen to move.
function lay(element: Element, box: { at: number; wide: number }): void {
  Object.defineProperty(element, "offsetLeft", { value: box.at });
  Object.defineProperty(element, "offsetWidth", { value: box.wide });
}

/// The workbench with the opened conversation altered, for the states no fixture
/// holds — a refusal from the server, a worktree that has gone.
function theWorkbenchWith(
  over: Partial<ConversationView>,
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(`/api/ui/conversations/${OPEN.id}`, json({ ...OPEN, ...over })),
    ...answers,
  );
}


/// The sidebar's other draft, which is the opened fixture again under its id
/// and with a Brief of its own.
///
/// A second Conversation of the same kind on purpose: two drafts, each of them
/// a card that is a field, so that anything the middle pane was holding for one
/// of them shows up in the other's card rather than being covered over by a
/// different card altogether.
const SECOND_BRIEF = "# Outbound retries\n\nDecide where the backoff lives.\n";

const SECOND: ConversationView = {
  ...OPEN,
  id: 2,
  timeline: OPEN.timeline.map((event) =>
    "Brief" in event
      ? { Brief: { ...event.Brief, markdown: SECOND_BRIEF } }
      : event,
  ),
};

/// The workbench with three Conversations there to be read: the draft the
/// fixtures open, the sidebar's other draft, and the one being grilled.
function theThree(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever("/api/ui/abandoned-roadmaps", json([])),
    whenever(`/api/ui/conversations/${OPEN.id}`, json(OPEN)),
    whenever(`/api/ui/conversations/${SECOND.id}`, json(SECOND)),
    whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
    whenever(TRANSCRIPT_OF_IT, json(SAID_NOTHING)),
    whenever(CAPTURE_OF_IT, json(CAPTURE)),
    whenever(SCREEN_OF_IT, json(SCREEN)),
    ...answers,
  );
}

/// Switching from one Conversation to another, which used to be dropped: the
/// URL moved and the page did not, and only a reload got the human out of it.
///
/// It went wrong where a switch costs nothing — a Conversation already read
/// once, answered out of the cache. With nothing to wait for there is no moment
/// of loading to tear the page down at, so the second Conversation was merged
/// into the object the first one was drawn from, and everything the middle pane
/// was holding went on standing over it. What settles it is that the whole of
/// the reading half of the page is keyed on the id now, so a switch builds it
/// again from nothing whether or not anything had to be fetched.
describe("switching between conversations", () => {
  /// The field a drafting card is, which is how these tests say which draft is
  /// on screen.
  const field = () => screen.getByLabelText("Brief") as HTMLTextAreaElement;

  it("draws the conversation the URL moved to", async () => {
    theThree();
    const { container, history } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => expect(field().value).toBe(BRIEF.markdown));

    history.set({ value: `/conversations/${GRILLING.id}` });

    // The grilled Conversation's own record, which a draft has nothing like —
    // and no field, because its Brief froze when the grilling started.
    await drawn(container, ".timeline-event .agent-output");
    expect(screen.queryByLabelText("Brief")).toBeNull();
  });

  it("draws one it has read before", async () => {
    theThree();
    const { container, history } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => expect(field().value).toBe(BRIEF.markdown));

    history.set({ value: `/conversations/${GRILLING.id}` });
    await drawn(container, ".timeline-event .agent-output");

    history.set({ value: `/conversations/${OPEN.id}` });
    await waitFor(() => expect(field().value).toBe(BRIEF.markdown));

    // And on again, with both of them in the cache by now: nothing is fetched
    // for this one, so nothing but the change of id can move the page.
    history.set({ value: `/conversations/${GRILLING.id}` });
    await drawn(container, ".timeline-event .agent-output");
  });

  /// Everything the middle pane is holding belongs to the Conversation it is
  /// holding it for. A half-written Brief above all: it is the only copy of
  /// itself there is, and carried into another draft's card it would be one
  /// Conversation's words offered as another's — and saved as them, on the next
  /// pause in the typing.
  it("keeps nothing of the conversation it left", async () => {
    theThree(json("Saved"));
    const { history } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => expect(field().value).toBe(BRIEF.markdown));

    // Both of them read once, so that the switch this is really about has
    // nothing to fetch and no moment of loading to hide behind.
    history.set({ value: `/conversations/${SECOND.id}` });
    await waitFor(() => expect(field().value).toBe(SECOND_BRIEF));

    history.set({ value: `/conversations/${OPEN.id}` });
    await waitFor(() => expect(field().value).toBe(BRIEF.markdown));

    fireEvent.input(field(), { target: { value: "# Half a thought\n" } });
    expect(field().value).toBe("# Half a thought\n");

    history.set({ value: `/conversations/${SECOND.id}` });

    await waitFor(() => expect(field().value).toBe(SECOND_BRIEF));
  });
});


describe("starting the grilling", () => {
  it("offers the button under the timeline once the conversation is ready", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const start = await drawn(container, ".start-grilling .start");

    expect(OPEN.ready_to_grill).toBe(true);
    expect(start.textContent).toContain("Start grilling");

    // Under the timeline, which is where the reason to press it is: at the end
    // of everything that has happened, under the brief it will freeze.
    expect(
      container.querySelector(".timeline")!.compareDocumentPosition(start) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// An unready conversation gets the button all the same, drawn inert. Not
  /// `disabled`, because a disabled button takes no press — and a press is how
  /// the human reaches the explanation on a phone, where a `title` would never
  /// show.
  it("draws the button inert rather than withholding it", async () => {
    theWorkbenchWith({ ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const start = await drawn<HTMLButtonElement>(
      container,
      ".start-grilling .start",
    );

    expect(start.textContent).toContain("Start grilling");
    expect(start.classList).toContain("inert");
    expect(start.getAttribute("aria-disabled")).toBe("true");
    expect(start.disabled).toBe(false);

    // Nothing said until it is asked, and neither of the notes that used to
    // stand in for the button.
    expect(screen.queryByText(/This needs a brief/)).toBeNull();
    expect(screen.queryByText(/the grilling can start/)).toBeNull();
  });

  it("answers a press on the inert button with what is missing, and starts nothing", async () => {
    const fetching = theWorkbenchWith(
      { ready_to_grill: false },
      whenever(
        `/api/ui/conversations/${OPEN.id}/grill`,
        json("Started" satisfies GrillingStarted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(await drawn(container, ".start-grilling .start"));

    await waitFor(() => screen.getByText(/This needs a brief/));
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
  });

  /// The same words for whoever has a pointer to hover with, and gone again
  /// when the pointer is.
  it("shows what is missing on hover too", async () => {
    theWorkbenchWith({ ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const start = await drawn(container, ".start-grilling .start");

    fireEvent.mouseEnter(start);
    await waitFor(() => screen.getByText(/This needs a brief/));

    fireEvent.mouseLeave(start);
    await waitFor(() =>
      expect(screen.queryByText(/This needs a brief/)).toBeNull(),
    );
  });

  it("posts to the conversation's own grill route, with nothing in the body", async () => {
    const fetching = theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/grill`,
        json("Started" satisfies GrillingStarted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(await drawn(container, ".start-grilling .start"));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toEqual(
        {},
      ),
    );
  });

  /// Every refusal is its own sentence, because each of them is something
  /// different for the human to go and do.
  it.each([
    ["NoGrillingProfile", /Choose a grilling profile/],
    ["NoImplementationProfile", /Choose an implementation profile/],
    ["EmptyBrief", /Write the brief first/],
    ["NoBaseCommit", /nothing to branch from/],
    ["BranchExists", /branch already exists/],
    ["ProfileBroken", /not where it was left/],
    ["WorktreeRefused", /Git would not make the worktree/],
    ["NotDrafting", /already been started/],
  ] satisfies Array<[GrillingStarted, RegExp]>)(
    "says in words what %s means",
    async (outcome, said) => {
      theWorkbench(
        whenever(
          `/api/ui/conversations/${OPEN.id}/grill`,
          json(outcome as GrillingStarted),
          "POST",
        ),
      );
      const { container } = mount(`/conversations/${OPEN.id}`);

      fireEvent.click(await drawn(container, ".start-grilling .start"));

      await waitFor(() => screen.getByText(said));
    },
  );

  /// A conversation that has started has nothing to start, so there is nothing
  /// to draw — not a button that would be refused.
  it("offers nothing on a conversation that has already started", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() => screen.getByText("Draft → Grilling"));
    expect(container.querySelector(".start-grilling")).toBeNull();
  });
});

describe("a move on the timeline", () => {
  /// Both ends of it: the record keeps only the state moved to, and the one it
  /// moved from is the move before it — or drafting, where it is the first.
  it("draws the move as the transition it was", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const moved = await drawn(container, ".timeline-event .moved");

    expect(moved.textContent).toBe("Draft → Grilling");
    expect(moved.classList).toContain("grilling");
  });

  /// The brief stays the first event and everything after it follows in the
  /// order it happened, which is also reading order.
  it("comes after the brief it followed", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() => screen.getByText("Draft → Grilling"));

    expect(
      [...container.querySelectorAll(".timeline-event > *")].map(
        (event) => event.className.split(" ")[0],
      ),
    ).toEqual([
      "brief",
      "moved",
      "agent-output",
      // The four Sets that session put to the human, in the order it asked
      // them: the answered one, the one still waiting, the deferred one that is
      // also still waiting, and the one whose stored body this build cannot
      // read — which is a row like any other and in its own place in the
      // record.
      "question-set",
      "question-set",
      "question-set",
      "question-set",
    ]);
  });
});

describe("a session's output on the timeline", () => {
  /// The design's summary: how far the conversation has got, and the last thing
  /// the agent said. An hour of terminal output does not go in the middle pane.
  ///
  /// Turns rather than the lines it printed. A full-screen interface redraws
  /// itself with cursor moves rather than newlines, so a line count read 0 for
  /// every real session — and what a reader wanted from it was how much of a
  /// conversation there is to open, which is what a turn is.
  it("summarises as a turn count and the latest statement", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(OUTPUT.turns).not.toBeNull();
    expect(output.querySelector(".turns")!.textContent).toBe(
      `${OUTPUT.turns} turns`,
    );
    expect(output.querySelector(".latest")!.textContent).toBe(OUTPUT.latest);

    // Nothing of the Capture itself: it is fetched by the pane that shows
    // it, and only once one is opened.
    expect(output.textContent).not.toContain("Reading the brief");
  });

  /// A session whose backend keeps no log has no Transcript to count, and its
  /// row says nothing rather than saying none: every stub agent is one, and a
  /// `0 turns` on it would be a claim about a conversation nothing can see.
  it("shows no metric at all for a session with no transcript", async () => {
    theGrillingOutput({ turns: null });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(output.querySelector(".turns")).toBeNull();
    expect(output.textContent).not.toContain("0 turns");
  });

  /// And one turn is a turn. The count is read off a running session, so it
  /// passes through 1 on its way to the rest of them.
  it("says `1 turn` of a conversation that has taken one", async () => {
    theGrillingOutput({ turns: 1 });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(output.querySelector(".turns")!.textContent).toBe("1 turn");
  });

  /// A session getting on with it: the turning ring at the right edge, which is
  /// the mark the sidebar's card already says the same thing with. The word
  /// `running` it replaced said it once and said nothing about a session that
  /// had stopped talking an hour ago.
  it("turns the ring while the session is still working", async () => {
    theGrillingOutput({ running: true, idle: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(output.querySelector(".mark.working")).toBeTruthy();
    expect(output.querySelector(".mark.idle")).toBeNull();
    expect(output.textContent).not.toContain("running");
  });

  /// And one that is running and has gone quiet: the same ring, empty. What it
  /// exists for is the grilling sitting on a blocking ask for hours, which the
  /// turning ring would have drawn as busy the whole time.
  it("empties the ring while the session is idle", async () => {
    theGrillingOutput({ running: true, idle: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(output.querySelector(".mark.idle")).toBeTruthy();
    expect(output.querySelector(".mark.working")).toBeNull();
  });

  /// A session that has ended is a conversation with a Capture, not one with
  /// an agent in it — and the fixture is exactly that. Nothing is happening to
  /// it, so there is no mark for one to be about.
  it("says nothing about running when the session has ended", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(OUTPUT.running).toBe(false);
    expect(output.querySelector(".mark")).toBeNull();
    expect(output.textContent).not.toContain("running");
  });


  /// The details pane says the same metric as the row it was opened from, and
  /// leaves it out for the same session — the two are one summary shown twice,
  /// and a pane disagreeing with the row it opened from would be two answers to
  /// the one question.
  it("says the same turn count in the details pane", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const summary = await drawn(container, ".details-pane .capture-summary");

    expect(summary.querySelector(".turns")!.textContent).toBe(
      `${OUTPUT.turns} turns`,
    );
  });

  /// And it says the same liveness with the same mark, for the same reason: the
  /// row and the pane are one summary shown twice.
  it("carries the same mark in the details pane", async () => {
    theGrillingOutput({ running: true, idle: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const summary = await drawn(container, ".details-pane .capture-summary");

    expect(summary.querySelector(".mark.idle")).toBeTruthy();
    expect(summary.textContent).not.toContain("running");
  });


  /// And a session that has ended with no Transcript has nothing to say up
  /// there at all, so the pane draws no summary line rather than an empty one.
  it("says nothing there either for a session with no transcript", async () => {
    theGrillingOutput({ turns: null });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    // The record itself, which says the pane is drawn and it is this session's.
    await drawn(container, ".details-pane .record-switch");

    expect(container.querySelector(".details-pane .capture-summary")).toBeNull();
  });

  /// The fallback, and the whole details-pane story for a session whose backend
  /// keeps no log of itself: every stub agent the suite runs is one, and so is
  /// every session that ran before Verkstead started following logs.
  it("shows the whole capture in the details pane, byte for byte", async () => {
    const fetching = theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const shown = await drawn(container, ".details-pane .capture");

    expect(shown.textContent).toBe(CAPTURE.text);
    expect(askedFor(fetching, CAPTURE_OF_IT)).toBeGreaterThan(0);
  });

  /// The pane opens on what the session said rather than on how it looked: the
  /// Screen is the other half of the switch, and nothing is fetched for it until
  /// somebody asks.
  it("opens on the transcript, with the screen a click away", async () => {
    const fetching = theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const showing = await drawn(
      container,
      '.details-pane .record-switch .transcript-tab[aria-pressed="true"]',
    );

    expect(showing.textContent).toBe("Transcript");
    expect(container.querySelector(".details-pane .screen")).toBeNull();
    expect(askedFor(fetching, SCREEN_OF_IT)).toBe(0);
  });

  /// The Screen of a session that has ended: what its terminal last showed,
  /// drawn as a terminal rather than as the bytes that drew it.
  ///
  /// The server holds the terminal that decided the repaint and this one paints
  /// it — which is the whole of the exception to the browser never parsing, and
  /// why what is asserted is the grid rather than the payload.
  it("shows the screen the session left, drawn as a terminal", async () => {
    const fetching = theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    const grid = await drawn(container, ".details-pane .screen .xterm-rows");

    await waitFor(() =>
      expect(grid.textContent).toContain(
        "What should happen to a delivery that has failed forty times?",
      ),
    );

    // The escapes the session dimmed its first line with are instructions to a
    // terminal rather than something a terminal prints, so none of them are on
    // the grid — which is the difference between the Screen and the Capture.
    expect(grid.textContent).toContain("Reading the brief.");
    expect(grid.textContent).not.toContain("2m");

    expect(askedFor(fetching, SCREEN_OF_IT)).toBeGreaterThan(0);
  });

  /// There is nowhere to type: a session that has ended is showing the screen it
  /// left, and the pane says so rather than swallowing keystrokes quietly.
  it("says the screen is read-only and takes no input", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    const said = await drawn(container, ".details-pane .screen .read-only");
    expect(said.textContent).toContain("Read-only");

    const typing = await drawn<HTMLTextAreaElement>(
      container,
      ".details-pane .screen textarea",
    );
    expect(typing.readOnly).toBe(true);
  });

  /// A Transcript is the whole of what a session said, and on a session that has
  /// been talking for an hour that is half a megabyte of it. Read for a tab that
  /// is not showing, it is a wait the human spends on a document nobody asked
  /// for — and it is spent in front of the pane they did ask for, because a
  /// browser gives one origin six connections and the reads queue behind each
  /// other.
  ///
  /// So it is read the moment it is asked for and not before, which is what
  /// makes the reading something the pane put off rather than something it
  /// dropped.
  it("reads the transcript when the reader switches back to it", async () => {
    const fetching = theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    await drawn(container, ".details-pane .turn.prose");
    const first = askedFor(fetching, TRANSCRIPT_OF_IT);

    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));
    await drawn(container, ".details-pane .screen");

    fireEvent.click(await drawn(container, ".details-pane .transcript-tab"));

    await waitFor(() => drawn(container, ".details-pane .turn.prose"));
    expect(askedFor(fetching, TRANSCRIPT_OF_IT)).toBeGreaterThanOrEqual(first);
  });

  /// An empty black rectangle is what a terminal that has failed looks like, so
  /// a Screen that has not been painted yet says which of the two it is. What
  /// makes this worth a line of its own is that nothing else on the pane does:
  /// the terminal is made by the first repaint, so before one there is not even
  /// a grid to be empty.
  it("says it is waiting until a grid has been painted", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);
    theGrillingOutput({ running: true });

    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    const said = await drawn(container, ".details-pane .screen .read-only");
    expect(said.textContent).toContain("Waiting");

    (await attached()).says(PAINTED);

    // And says what it is once there is something to say it about.
    await waitFor(() => expect(said.textContent).toContain("Watching"));
  });

  /// And what it shows instead wherever there is a Transcript: the conversation
  /// the session was having, rather than the bytes it happened to draw.
  it("shows the conversation where the session left one", async () => {
    const fetching = theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const prose = await drawn(container, ".details-pane .turn.prose");

    expect(prose.textContent).toContain("Looking at how the queue is drained");
    // Rendered by the server and put in the page as markup, which is what puts
    // no markdown parser on this side of the wire.
    expect(prose.querySelector("strong")!.textContent).toBe("drained");
    expect(container.querySelector(".details-pane .capture")).toBeNull();
    expect(askedFor(fetching, TRANSCRIPT_OF_IT)).toBeGreaterThan(0);
  });

  /// The one a renderer keying off the line's own type gets wrong: a tool's
  /// answer and a turn from the human arrive under the same type, and reading
  /// a directory listing as though somebody had said it is the whole failure.
  it("draws a turn put to it and a tool's answer as different things", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const put = await drawn(container, ".details-pane .turn.put");
    const answered = await drawn(container, ".details-pane .turn.tool-call");

    expect(put.textContent).toContain("What should the queue do");
    expect(answered.textContent).toContain("crates/server/src/queue.rs");
    expect(put).not.toBe(answered);
  });

  /// What a reader opened this for is what the agent said. What it was thinking
  /// and what it ran are there to be opened rather than scrolled past.
  it("opens with the reasoning and the tool calls folded away", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const reasoning = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .turn.reasoning details",
    );
    const call = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .turn.tool-call details",
    );
    const prose = await drawn(container, ".details-pane .turn.prose");

    expect(reasoning.open).toBe(false);
    expect(call.open).toBe(false);
    // One line about the call, which is the tool and what it was for.
    expect(call.querySelector("summary")!.textContent).toContain("Bash");
    expect(call.querySelector("summary")!.textContent).toContain(
      "Find where a delivery is retried",
    );
    expect(prose.querySelector("details")).toBeNull();
  });

  /// One thing happened, so it is one thing to open. Shut, the card is the tool
  /// and the line about it; open, it is what the tool was called with above
  /// what it said back — the order the two happened in.
  it("draws a call and the answer to it as the one card", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const pair = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .turn.tool-call details",
    );

    expect(pair.querySelector("summary")!.textContent).toContain("Bash");
    expect(pair.querySelector("summary")!.textContent).toContain(
      "Find where a delivery is retried",
    );

    const behind = [...pair.querySelectorAll("pre")];

    expect(behind.map((block) => block.className)).toEqual(["input", "output"]);
    expect(behind[0]!.textContent).toContain("rg -n 'retry'");
    expect(behind[1]!.textContent).toContain("crates/server/src/queue.rs");

    // And the answer is not also standing on its own under it, which is what
    // there being one card is.
    expect(
      container.querySelectorAll(".details-pane .turn.tool-call"),
    ).toHaveLength(1);
    expect(
      container.querySelector(".details-pane .turn.tool-result"),
    ).toBeNull();
  });

  /// Success is quiet: a session calls a hundred tools and ninety-nine of them
  /// work, so a word saying so on every one of them would be a word to read
  /// past. A failure is the exception and says so while the card is still shut,
  /// which is what makes one findable without opening anything.
  it("says failed on a pair that failed, and nothing on one that worked", async () => {
    theSpeaking(
      whenever(
        TRANSCRIPT_OF_IT,
        json(
          recordOf([
            {
              kind: "ToolUse",
              id: 1,
              name: "Bash",
              call: "toolu_a",
              about: "Count the tasks left",
              input: "{}",
            },
            {
              kind: "ToolResult",
              id: 2,
              call: "toolu_a",
              failed: false,
              text: "two",
            },
            {
              kind: "ToolUse",
              id: 3,
              name: "Bash",
              call: "toolu_b",
              about: "Run the tests",
              input: "{}",
            },
            {
              kind: "ToolResult",
              id: 4,
              call: "toolu_b",
              failed: true,
              text: "two tests failed",
            },
          ]),
        ),
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    await drawn(container, ".details-pane .turn.tool-call");

    const [worked, failed] = [
      ...container.querySelectorAll(".details-pane .turn.tool-call summary"),
    ];

    expect(worked!.textContent).toContain("Count the tasks left");
    expect(worked!.querySelector(".failed")).toBeNull();
    expect(failed!.textContent).toContain("Run the tests");
    expect(failed!.querySelector(".failed")!.textContent).toBe("failed");

    // And the red it is said in is the one a stopped run is said in. The
    // stylesheet's, since jsdom resolves no variable and paints nothing.
    expect(stylesheet).toContain(
      ".transcript .tool-call .failed {\n  flex: none;\n  margin-left: auto;\n  color: var(--stopped);\n}",
    );
  });

  /// An answer whose call is not in the record — a log whose first lines are
  /// gone, or a format that has stopped naming the two. Something answered, and
  /// a pane that swallowed it would be a pane missing a turn.
  it("still draws an answer no call is drawing", async () => {
    theSpeaking(
      whenever(
        TRANSCRIPT_OF_IT,
        json(
          recordOf([
            {
              kind: "ToolResult",
              id: 1,
              call: "toolu_gone",
              failed: false,
              text: "04-render.md",
            },
          ]),
        ),
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const orphan = await drawn(container, ".details-pane .turn.tool-result");

    expect(orphan.textContent).toContain("04-render.md");
  });

  /// A pair can straddle a reading: a batch of a Transcript ends wherever the
  /// log had got to, so a call whose tool is still running arrives on its own
  /// and its answer comes with the next one (ADR 0009). The card grows the
  /// answer where it stands rather than a second row appearing beneath it — and
  /// it is the same card, so a reader who had it open still has.
  it("grows the answer into the card its call arrived without", async () => {
    const CALLED = recordOf(
      [
        {
          kind: "ToolUse",
          id: 1,
          name: "Bash",
          call: "toolu_a",
          about: "Run the tests",
          input: '{\n  "command": "cargo test"\n}',
        },
      ],
      "1.1.0",
    );
    const ANSWERED: TranscriptView = {
      ...recordOf(
        [
          {
            kind: "ToolResult",
            id: 2,
            call: "toolu_a",
            failed: false,
            text: "78 passed",
          },
        ],
        "2.2.0",
      ),
      whole: false,
    };

    theGrillingOutput(
      { running: true },
      whenever(TRANSCRIPT_OF_IT, json(CALLED)),
      whenever(
        `${TRANSCRIPT_OF_IT}?after=${encodeURIComponent(CALLED.cursor)}`,
        json(ANSWERED),
      ),
    );
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const waiting = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .turn.tool-call details",
    );
    waiting.open = true;

    // Nothing has come back yet, so there is nothing under what it was called
    // with — and nothing said about how it went either.
    expect(waiting.querySelector("pre.output")).toBeNull();
    expect(waiting.querySelector("summary .failed")).toBeNull();

    await client.invalidateQueries();

    await waitFor(() =>
      expect(waiting.querySelector("pre.output")!.textContent).toContain(
        "78 passed",
      ),
    );
    expect(container.querySelectorAll(".details-pane .turn")).toHaveLength(1);
    expect(waiting.open).toBe(true);
  });

  /// Roughly a third of a log is the backend's own bookkeeping. Folded rather
  /// than dropped: nothing hidden, and nothing in the way.
  it("folds the backend's bookkeeping into one group", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const kept = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .bookkeeping",
    );

    expect(kept.open).toBe(false);
    expect(kept.querySelectorAll("li")).toHaveLength(
      TRANSCRIPT.bookkeeping.length,
    );
    expect(kept.textContent).toContain("attachment");

    // And not among the turns, which is what folding it away is for.
    expect(container.querySelectorAll(".details-pane .turn")).toHaveLength(
      rows(TRANSCRIPT.turns),
    );
  });

  /// ADR 0006's containment, at the end of the wire: the log's format belongs to
  /// somebody else, and one that has moved on should say so here rather than
  /// quietly emptying the pane.
  it("shows a line of a kind it does not know as the JSON it is", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const unread = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .turn.unread details",
    );

    expect(unread.open).toBe(false);
    expect(unread.querySelector("pre")!.textContent).toContain("divination");
  });

  /// The regression this shape of wire is for: a Nudge invalidates every
  /// active query (ADR-0005), so an open pane re-reads a running session's
  /// Transcript — and every fold in it used to snap shut with the rows the
  /// re-read rebuilt, because a `details` keeps its open state in the DOM and
  /// nowhere else. The turns reconcile by `id` now, so a row that did not
  /// change is left alone, folds and all.
  it("keeps a fold open across a Nudge, while new turns arrive beneath it", async () => {
    // Running, so the Transcript is live and read again on every Nudge —
    // which is exactly when a fold has to hold.
    theGrillingOutput(
      { running: true },
      whenever(TRANSCRIPT_OF_IT, json(TRANSCRIPT)),
      whenever(REST_OF_IT, json(MORE)),
    );
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    const reasoning = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .turn.reasoning details",
    );
    reasoning.open = true;

    await client.invalidateQueries();

    // What the session has said since has been drawn under the fold, added to
    // what was already there rather than replacing it…
    await waitFor(() =>
      expect(container.querySelectorAll(".details-pane .turn")).toHaveLength(
        rows(TRANSCRIPT.turns, MORE.turns),
      ),
    );
    // …and the fold is the same element it was, still open.
    expect(
      container.querySelector<HTMLDetailsElement>(
        ".details-pane .turn.reasoning details",
      ),
    ).toBe(reasoning);
    expect(reasoning.open).toBe(true);
  });

  /// The other half of an accumulated record. The backend's bookkeeping is
  /// folded into one group at the end however it arrived, so a line of it that
  /// came in a later batch joins the group rather than starting a second one.
  it("gathers bookkeeping that arrived in pieces into the one group", async () => {
    theGrillingOutput(
      { running: true },
      whenever(TRANSCRIPT_OF_IT, json(TRANSCRIPT)),
      whenever(REST_OF_IT, json(MORE)),
    );
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    await drawn(container, ".details-pane .bookkeeping");

    await client.invalidateQueries();

    await waitFor(() =>
      expect(
        container.querySelectorAll(".details-pane .bookkeeping li"),
      ).toHaveLength(TRANSCRIPT.bookkeeping.length + MORE.bookkeeping.length),
    );
    expect(
      container.querySelectorAll(".details-pane .bookkeeping"),
    ).toHaveLength(1);
  });

  /// The server reads the record whole whenever it cannot carry on from the
  /// cursor it was given, and says so. A page that added one of those to what it
  /// already had would draw the beginning of the session twice.
  it("replaces the record rather than doubling it when a whole one arrives", async () => {
    // The record read whole again, a turn longer than it was — a cursor the
    // server could not carry on from, which is every failure of an incremental
    // read and always a correct answer to one.
    const WHOLE_AGAIN: TranscriptView = {
      ...TRANSCRIPT,
      turns: [
        ...TRANSCRIPT.turns,
        {
          kind: "Prose",
          id: TRANSCRIPT.turns.length + 1,
          html: "<p>And the drain is the place to change it.</p>\n",
        },
      ],
    };

    theGrillingOutput(
      { running: true },
      whenever(TRANSCRIPT_OF_IT, json(TRANSCRIPT)),
      whenever(REST_OF_IT, json(WHOLE_AGAIN)),
    );
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    await drawn(container, ".details-pane .turn");

    await client.invalidateQueries();

    // What was read whole is the record, not something to add to it: the
    // session's beginning is drawn once.
    await waitFor(() =>
      expect(container.querySelectorAll(".details-pane .turn")).toHaveLength(
        rows(WHOLE_AGAIN.turns),
      ),
    );
  });

  /// A session already over when the pane opened is over for good, so its
  /// record is read once and never again — not even on a Nudge, whose
  /// invalidation beats any finite staleTime.
  it("does not read a finished session's transcript again on a Nudge", async () => {
    const fetching = theSpeaking();
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    await drawn(container, ".details-pane .turn.prose");

    expect(OUTPUT.running).toBe(false);
    const before = askedFor(fetching, TRANSCRIPT_OF_IT);
    await client.invalidateQueries();

    expect(askedFor(fetching, TRANSCRIPT_OF_IT)).toBe(before);
  });

  /// The switch is the pane's own control, so it stands in the pane's header
  /// beside the title — where every other pane's Close is, and where this one's
  /// was. Two of them there would be one row with two ways off it, so the Close
  /// goes: "← Timeline" is the way out of every pane on a narrow window, and a
  /// wide one has the conversation's Timeline standing beside this anyway.
  it("puts the switch in the header, and keeps no Close beside it", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    await drawn(container, ".details-pane .pane-head .record-switch");
    await drawn(container, ".details-pane .pane-head .pane-back");

    expect(container.querySelector(".details-pane .close-event")).toBeNull();
  });

  /// Sharing that row is what the switch's width is now about: as wide as its
  /// two labels, and off onto a line of its own when the title leaves it no
  /// room. Both are the stylesheet's, and jsdom lays nothing out.
  it("sizes the switch to its labels and wraps rather than overflowing", () => {
    expect(stylesheet).toContain(".pane-head {\n  display: flex;\n  flex-wrap: wrap;");
    expect(stylesheet).toContain(
      ".record-switch {\n" +
        "  position: relative;\n" +
        "  display: flex;\n" +
        "  flex: 0 0 auto;\n" +
        "  gap: 0.25rem;\n" +
        "  max-width: 100%;\n",
    );
  });

  /// What is under the pressed label is one element that travels, rather than a
  /// background on whichever button is pressed — so switching reads as one thing
  /// moving. Where it travels to is measured, which is why a test has to lay the
  /// two labels out: jsdom has no layout and everything on the page is at
  /// nothing and nothing wide.
  it("moves the mark to whichever label is pressed", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const transcript = await drawn(container, ".details-pane .transcript-tab");
    const screen = await drawn(container, ".details-pane .screen-tab");
    const mark = await drawn<HTMLElement>(
      container,
      ".details-pane .record-switch .indicator",
    );

    // Presentation and nothing else: the two buttons already say which is
    // showing, and saying it twice is saying it wrong.
    expect(mark.getAttribute("aria-hidden")).toBe("true");

    lay(transcript, { at: 4, wide: 78 });
    lay(screen, { at: 86, wide: 56 });

    fireEvent.click(screen);
    await waitFor(() => {
      expect(mark.style.transform).toBe("translateX(86px)");
      expect(mark.style.width).toBe("56px");
    });

    fireEvent.click(transcript);
    await waitFor(() => {
      expect(mark.style.transform).toBe("translateX(4px)");
      expect(mark.style.width).toBe("78px");
    });
  });

  /// And the travel itself is the stylesheet's: jsdom runs no transitions, so
  /// the rule is what is read. A tenth of a second, eased at both ends, across
  /// both what the mark travels — the labels are words of different lengths, so
  /// it changes width on the way as well as place.
  it("slides the mark over a tenth of a second, unless motion is unwelcome", () => {
    expect(stylesheet).toContain(
      "@media (prefers-reduced-motion: no-preference) {\n" +
        "  .record-switch .indicator {\n" +
        "    transition:\n" +
        "      transform 0.1s ease-in-out,\n" +
        "      width 0.1s ease-in-out;\n" +
        "  }\n" +
        "}",
    );
  });

  /// A phone shows one level at a time, and opening an event is walking into
  /// the next one.
  it("walks a narrow window into the details pane", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("details"),
    );
  });

  /// The event that is open is the one the timeline says is open, so that a
  /// narrow window walking back out can see which it came from.
  it("marks the event the details pane is showing", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".agent-output");
    expect(output.classList).not.toContain("selected");

    fireEvent.click(output);

    await waitFor(() => expect(output.classList).toContain("selected"));
    expect(output.getAttribute("aria-pressed")).toBe("true");
  });
});

/// The Screen of a session that is still drawing it: watched over a socket
/// rather than fetched, which is the one place in the app the viewer is sent
/// something instead of asking for it.
///
/// What the server sends is a repaint and then what the session printed, and
/// what goes back up is how wide the pane is. Everything asserted here is the
/// grid or the wire — the terminal is the server's, and this side only paints
/// what it is handed (ADR 0007).
describe("watching a live session's screen", () => {
  /// The workbench with the session still running, and the socket stubbed.
  function watching(): ReturnType<typeof serving> {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);
    return theGrillingOutput({ running: true });
  }

  /// Open the Screen of the running session, and hand back what the page and
  /// the socket are.
  async function watched(): Promise<{
    container: ParentNode;
    socket: Attached;
  }> {
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    return { container, socket: await attached() };
  }

  /// A repaint on connect and what the session prints after it — which is the
  /// whole of the protocol from this side.
  it("paints the repaint it is sent, and then what the session prints", async () => {
    const fetching = watching();
    const { container, socket } = await watched();

    expect(socket.url.startsWith("ws://")).toBe(true);
    expect(socket.url.endsWith(WATCHING_IT)).toBe(true);

    socket.says(PAINTED);

    const grid = await drawn(container, ".details-pane .screen .xterm-rows");

    await waitFor(() =>
      expect(grid.textContent).toContain(
        "What should happen to a delivery that has failed forty times?",
      ),
    );

    socket.says({ Printed: "and then it went quiet\r\n" });

    await waitFor(() =>
      expect(grid.textContent).toContain("and then it went quiet"),
    );

    // Watching, and saying what typing into it does and does not do: typing
    // works in the session, and the press that holds the run off is Stop.
    const said = await drawn(container, ".details-pane .screen .read-only");
    expect(said.textContent).toContain("Watching");
    expect(said.textContent).toContain("press Stop first");

    // Nothing was fetched: a request for the grid as the store last had it
    // would be a request for what the repaint has already replaced.
    expect(askedFor(fetching, SCREEN_OF_IT)).toBe(0);
  });

  /// How big the pane is goes back up, and the server decides what to do with
  /// it: the size that comes back is a repaint, not this side's own guess.
  ///
  /// Both dimensions of it. The rows used to be the server's own count echoed
  /// back, which left the grid whatever height the session was started at; the
  /// pane gives the terminal a height now, so the rows that fit are this
  /// window's answer exactly as the columns are.
  it("says how big the pane it is drawn in is", async () => {
    watching();
    const { socket } = await watched();

    socket.says(PAINTED);

    await waitFor(() => expect(socket.sent.length).toBeGreaterThan(0));

    expect(JSON.parse(socket.sent[0]!)).toEqual({
      Resized: { columns: FITS.cols, rows: FITS.rows },
    });

    // Which is the pane's measurement rather than the grid it was handed: the
    // fixture's Screen is neither of those numbers.
    expect(FITS.rows).not.toBe(SCREEN.rows);
  });

  /// Two watchers of different sizes must not argue. A repaint arriving at a
  /// size this pane never asked for is the latest window having won, and this
  /// one asking for its own back would be the two of them trading repaints for
  /// as long as both stayed open.
  it("does not ask for its size back when somebody else resizes", async () => {
    watching();
    const { socket } = await watched();

    socket.says(PAINTED);
    await waitFor(() => expect(socket.sent).toHaveLength(1));

    // The server's answer to that, and then repaints at a width and a height
    // nothing here asked for: another device, watching the same Screen in a
    // smaller window.
    socket.says({ Painted: { ...SCREEN, columns: FITS.cols, rows: FITS.rows } });
    socket.says({ Painted: { ...SCREEN, columns: 60, rows: FITS.rows } });
    socket.says({ Painted: { ...SCREEN, columns: FITS.cols, rows: 20 } });

    // Said synchronously if it were said at all — the handler measures the pane
    // the moment it has painted the repaint.
    expect(socket.sent).toHaveLength(1);
  });

  /// And the pane it is measuring is a pane with a height: the stylesheet ends
  /// it where the window ends and gives the terminal what is left under the
  /// header, rather than letting it run on down a page that scrolls. Read off
  /// the stylesheet, because jsdom lays nothing out.
  it("gives the Screen the pane's height rather than the page's", () => {
    expect(stylesheet).toContain(
      ".workbench > .details-pane:has(.screen) {\n" +
        "  flex-direction: column;\n" +
        "  height: 100dvh;\n" +
        "  padding-bottom: 1.25rem;\n" +
        "  overflow: hidden;\n" +
        "}",
    );

    // What is above the terminal keeps its size; the Screen takes the rest.
    expect(stylesheet).toContain(
      ".workbench > .details-pane:has(.screen) > :not(.screen) {\n" +
        "  flex: none;\n" +
        "}",
    );
    expect(stylesheet).toContain(
      ".screen {\n" +
        "  display: flex;\n" +
        "  flex: 1;\n" +
        "  flex-direction: column;\n" +
        "  min-height: 0;\n",
    );

    // And the grid a session left behind, which nothing can resize: at its own
    // size, scrolling in the card it sits on rather than scrolling the pane.
    expect(stylesheet).toContain(
      ".screen .terminal-host {\n" +
        "  flex: 1;\n" +
        "  min-height: 0;\n" +
        "  padding: 0.5rem;\n" +
        "  overflow: auto;\n",
    );
  });

  /// A live terminal is the one that must not scroll, and the rule that says so
  /// is doing more than tidying: a scrollbar takes a strip off the pane, the
  /// strip changes what fits, and what fits is what goes up the socket — so a
  /// watcher shown a bigger window's grid would ask for its own back through
  /// the scrollbar, and the smaller of the two would always win.
  it("clips a live terminal rather than letting it scroll", async () => {
    watching();
    const { container, socket } = await watched();

    socket.says(PAINTED);

    const screen = await drawn(container, ".details-pane .screen");
    expect(screen.classList).toContain("live");

    expect(stylesheet).toContain(
      ".screen.live .terminal-host {\n  overflow: hidden;\n}",
    );
  });

  /// And it renders what the session wrote in the case the session wrote it in.
  /// The badge that says a row is live is styled by that word alone, and the
  /// Screen marks itself with the same one — a bare `.live` rule matches the
  /// Screen too, and `text-transform` inherits all the way down into the rows
  /// xterm builds.
  it("leaves a live terminal's text in its own case", async () => {
    watching();
    const { container, socket } = await watched();

    socket.says(PAINTED);

    const screen = await drawn(container, ".details-pane .screen");
    expect(screen.classList).toContain("live");

    // The badge keeps its capitals, and asks for them where badges are.
    expect(stylesheet).toContain(
      ".event-head .live {\n" +
        "  font-size: 0.8rem;\n" +
        "  font-weight: 600;\n" +
        "  text-transform: uppercase;\n",
    );

    // And nothing asks for them by the word alone, here or anywhere else: a
    // state class standing on its own matches every element that carries it.
    expect(stylesheet).not.toMatch(/(^|\n)\.live[\s,{]/);
  });

  /// And the grid of a session that has ended does not carry it: nothing will
  /// resize that one, so scrolling is the only way to the rest of it.
  it("leaves the grid of an ended session scrolling", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);
    theSpeaking();

    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    const screen = await drawn(container, ".details-pane .screen");
    expect(screen.classList).not.toContain("live");
  });

  /// The height belongs to the Screen and not to the pane: switching back to the
  /// Transcript takes the element the rule hangs off away with it, which is what
  /// puts the pane's ordinary scrolling back.
  it("stops binding the pane's height once the Transcript is showing", async () => {
    watching();
    const { container, socket } = await watched();

    socket.says(PAINTED);
    await drawn(container, ".details-pane .screen");

    fireEvent.click(await drawn(container, ".details-pane .transcript-tab"));

    await waitFor(() =>
      expect(container.querySelector(".details-pane .screen")).toBeNull(),
    );
  });

  /// Closing the pane lets the socket go. Watching commits the human to nothing
  /// and a watcher that leaves takes nothing with it — on this side that is one
  /// socket closed and no request made.
  it("lets the socket go when the screen is closed", async () => {
    watching();
    const { container, socket } = await watched();

    socket.says(PAINTED);
    await drawn(container, ".details-pane .screen .xterm-rows");

    fireEvent.click(await drawn(container, ".details-pane .transcript-tab"));

    await waitFor(() => expect(socket.closed).toBe(true));
    expect(Attached.opened).toHaveLength(1);
  });

  /// And a session that has ended has nothing to attach to: its Screen is the
  /// one it last stood on, which is fetched.
  it("fetches the screen of a session that has ended rather than watching it", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);

    const fetching = theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    await drawn(container, ".details-pane .screen .xterm-rows");

    expect(Attached.opened).toHaveLength(0);
    expect(askedFor(fetching, SCREEN_OF_IT)).toBeGreaterThan(0);
  });
});

describe("putting something into a live session's screen", () => {
  /// The terminal the pane drew, as the browser gives a keystroke to one: xterm
  /// takes typing through the hidden textarea it keeps focus in, and turns each
  /// keypress into the bytes a session expects before anything of ours sees it.
  async function typeInto(container: ParentNode, key: string, code: number) {
    const typing = await drawn<HTMLTextAreaElement>(
      container,
      ".details-pane .screen .xterm-helper-textarea",
    );

    fireEvent.keyDown(typing, { key, keyCode: code, which: code });
  }

  /// What a watcher said up the socket, of the kind named.
  function said(socket: Attached, kind: "PutIn" | "Resized"): unknown[] {
    return socket.sent
      .map((wrote) => JSON.parse(wrote) as Record<string, unknown>)
      .filter((wrote) => kind in wrote)
      .map((wrote) => wrote[kind]);
  }

  /// The pane watching a live session, with its socket open and the first
  /// repaint painted — which is what makes the terminal there is to type into.
  async function watching(): Promise<{
    container: ParentNode;
    socket: Attached;
  }> {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);
    theGrillingOutput({ running: true });

    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    const socket = await attached();
    socket.says(PAINTED);

    await drawn(container, ".details-pane .screen .xterm-rows");

    return { container, socket };
  }

  /// Typing goes up the socket as the bytes the terminal made of it. Nothing is
  /// drawn for it here: what the session makes of a keystroke comes back as what
  /// the session printed, which is the one account of what happened.
  it("sends what was typed to the session, and draws nothing for it", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);
    theGrillingOutput({ running: true });

    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    fireEvent.click(await drawn(container, ".details-pane .screen-tab"));

    const socket = await attached();
    socket.says(PAINTED);

    const grid = await drawn(container, ".details-pane .screen .xterm-rows");
    const before = grid.textContent;

    await typeInto(container, "Enter", 13);

    await waitFor(() => expect(said(socket, "PutIn")).toEqual(["\r"]));
    expect(grid.textContent).toBe(before);
  });

  /// A paste goes up the same way. It arrives at the terminal as an event of its
  /// own rather than as a keypress, and what it carries is exactly what somebody
  /// meant to put into the session.
  it("sends a paste up the socket", async () => {
    const { container, socket } = await watching();

    const typing = await drawn<HTMLTextAreaElement>(
      container,
      ".details-pane .screen .xterm-helper-textarea",
    );

    fireEvent.paste(typing, {
      clipboardData: { getData: () => "cargo test" },
    });

    await waitFor(() => expect(said(socket, "PutIn")).toEqual(["cargo test"]));
  });

  /// And so does the mouse. A session whose interface tracks it has the terminal
  /// report every move, click and scroll down the same callback a keystroke
  /// comes out of, and the two are not told apart: neither commits Verkstead to
  /// anything, so what is on the wire is bytes a terminal is being sent. What
  /// the wheel is turned into here is one of them.
  it("sends what the mouse did up the same socket", async () => {
    const { container, socket } = await watching();

    const grid = await drawn(container, ".details-pane .screen .xterm-screen");

    fireEvent.wheel(grid, { deltaY: 120 });

    await waitFor(() => expect(said(socket, "PutIn")).not.toEqual([]));
  });

  /// Typing into a driven session commits Verkstead to nothing: no press to
  /// undo it, and no badge saying the work has stopped. Somebody who wants the
  /// run held off presses Stop first, which is a stop like any other.
  it("neither draws a hand-back nor blocks the conversation on one", async () => {
    const { container } = await watching();

    await typeInto(container, "Enter", 13);

    const note = await drawn(container, ".details-pane .screen .read-only");
    expect(note.textContent).toContain("press Stop first");

    expect(container.querySelector(".details-pane .hand-back")).toBeNull();
    expect(container.querySelector(".timeline-pane .blocked")).toBeNull();
  });
});

describe("aborting a conversation", () => {
  /// Behind a menu on the header, because it throws a worktree away and the
  /// header is somewhere the cursor passes on the way to everything else.
  it("is not one click away", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".conversation-actions > .menu-trigger");

    // Closed, so nothing in it is on the page at all — which is the whole of
    // what standing a destructive action behind a menu means.
    expect(container.querySelector(".abort")).toBeNull();

    const menu = await openActions(container);
    expect(menu.querySelector(".abort")).toBeTruthy();
    expect(container.querySelector(".pane-head .abort")).toBe(
      menu.querySelector(".abort"),
    );
  });

  it("posts to the conversation's own abort route", async () => {
    const fetching = theGrilling(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/abort`,
        json("Aborted" satisfies ConversationAborted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, ".conversation-actions .abort"));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${GRILLING.id}/abort`),
      ).toEqual({}),
    );
  });

  /// What the human is owed before throwing a worktree away: that it is the end
  /// of the conversation rather than a pause, what goes, and what stays.
  it("says it is permanent and that the branch survives it", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await waitFor(() =>
      screen.getByText(/Permanently end the conversation and delete the/),
    );
    expect(screen.getByText(/The branch stays where it is/)).toBeTruthy();
  });

  it("offers nothing to abort on one that is aborted already", async () => {
    theWorkbenchWith({ state: "Aborted", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);

    await waitFor(() => screen.getByText("This conversation has been aborted."));
    expect(container.querySelector(".conversation-actions .abort")).toBeNull();
  });

  it("says when the worktree could not be removed", async () => {
    theGrilling(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/abort`,
        json("WorktreeStuck" satisfies ConversationAborted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, ".conversation-actions .abort"));

    await waitFor(() => screen.getByText(/could not be removed/));
  });
});

/// A conversation Verkstead has finished with, opened again for a second brief
/// round: the frozen brief above the round boundary, and the one being written
/// under it.
describe("reopening a conversation", () => {
  const REOPENED = reopened as ConversationView;

  /// The workbench with the reopened conversation opened instead of the drafting
  /// one.
  function theReopened(...answers: Parameters<typeof serving>) {
    return theWorkbench(
      whenever(`/api/ui/conversations/${REOPENED.id}`, json(REOPENED)),
      ...answers,
    );
  }

  /// Where `Start grilling` sits, and for the same reason: it is the next thing
  /// to do about this conversation, and the end of everything that has happened
  /// is where the next thing belongs.
  it("offers the press under the timeline once the work is finished", async () => {
    theWorkbenchWith({ state: "Done" });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const press = await drawn(container, ".reopen .reopen-conversation");
    expect(press.textContent).toContain("Reopen");

    expect(
      container.querySelector(".timeline")!.compareDocumentPosition(press) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// Done and nowhere else. Aborted is off the ladder and stays there, and every
  /// other state is somewhere the work has got to.
  it("is offered on done and on no other state", async () => {
    for (const state of [
      "Draft",
      "Grilling",
      "Implementing",
      "Wrapping",
      "Aborted",
    ] as const) {
      theWorkbenchWith({ state });
      const { container, unmount } = mount(`/conversations/${OPEN.id}`);

      await drawn(container, ".timeline");
      expect(container.querySelector(".reopen")).toBeNull();
      unmount();
    }
  });

  it("posts to the conversation's own reopen route", async () => {
    const fetching = theWorkbenchWith(
      { state: "Done" },
      whenever(
        `/api/ui/conversations/${OPEN.id}/reopen`,
        json("Reopened" satisfies ConversationReopened),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(await drawn(container, ".reopen .reopen-conversation"));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/reopen`),
      ).toEqual({}),
    );
  });

  /// What the human is owed before pressing it: the frozen brief is not touched,
  /// and the branch is the one the work is already on.
  it("says the brief above it stays where it is", async () => {
    theWorkbenchWith({ state: "Done" });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const panel = await drawn(container, ".reopen");
    expect(panel.textContent).toContain("second round on the same branch");
    expect(panel.textContent).toContain("stays where it is");
  });

  it("says when the branch could not be checked out again", async () => {
    theWorkbenchWith(
      { state: "Done" },
      whenever(
        `/api/ui/conversations/${OPEN.id}/reopen`,
        json("WorktreeRefused" satisfies ConversationReopened),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(await drawn(container, ".reopen .reopen-conversation"));

    await waitFor(() => screen.getByText(/would not check the branch out again/));
  });

  /// One round's brief is a record and the next one's is the field. Both are on
  /// the timeline, and only one of them is written in — which is the whole
  /// reason a brief carries its own `frozen` rather than reading the
  /// conversation's state, since both of these are on a conversation that is
  /// drafting.
  it("draws the frozen brief beside the one being written", async () => {
    theReopened();
    const { container } = mount(`/conversations/${REOPENED.id}`);

    await drawn(container, ".brief");
    const briefs = [...container.querySelectorAll(".brief")];

    expect(briefs).toHaveLength(2);
    expect(briefs[0]!.querySelector("textarea")).toBeNull();
    expect(briefs[1]!.querySelector("textarea")).toBeTruthy();

    // And the setup goes under the round being set up rather than under both.
    expect(briefs[0]!.querySelector(".conversation-setup")).toBeNull();
    expect(briefs[1]!.querySelector(".conversation-setup")).toBeTruthy();
  });

  /// A reader has to be able to tell which brief the work under it was built
  /// from, which is the whole of what the boundary is for.
  it("says where the round boundary falls", async () => {
    theReopened();
    const { container } = mount(`/conversations/${REOPENED.id}`);

    const boundary = await drawn(container, ".timeline-event > .moved.draft");
    expect(
      boundary.textContent,
      "the move says both states, as every move does — and nothing moves *to* \
       drafting except a second round",
    ).toBe("Done → Draft");

    // And it is drawn between the two briefs, which is where the rounds part.
    const briefs = [...container.querySelectorAll(".brief")];
    expect(
      briefs[0]!.compareDocumentPosition(boundary) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      briefs[1]!.compareDocumentPosition(boundary) &
        Node.DOCUMENT_POSITION_PRECEDING,
    ).toBeTruthy();

    // What the boundary looks like is the stylesheet's, and jsdom lays nothing
    // out.
    expect(stylesheet).toContain(".timeline-event > .moved.draft {");
  });

  /// The second round runs the ordinary pipeline from grilling onward, so what
  /// stands under the new brief is the press every first round starts with.
  it("offers the ordinary start grilling under the new brief", async () => {
    theReopened();
    const { container } = mount(`/conversations/${REOPENED.id}`);

    const start = await drawn(container, ".start-grilling .start");
    expect(start.textContent).toContain("Start grilling");
    expect(container.querySelector(".reopen")).toBeNull();
  });
});

/// Where the two stops are pressed.
const STOPPING = `/api/ui/conversations/${GRILLING.id}/stop`;
const AT_ONCE = `/api/ui/conversations/${GRILLING.id}/force-stop`;

/// The grilling conversation as the server would say it stands right now — a
/// session running, or nothing running, or a run that has already halted.
function theGrillingStanding(
  over: Partial<ConversationView>,
  ...answers: Parameters<typeof serving>
) {
  return theGrilling(
    whenever(
      `/api/ui/conversations/${GRILLING.id}`,
      json({ ...GRILLING, ...over }),
    ),
    ...answers,
  );
}

describe("stopping a conversation", () => {
  /// The two stops sit in the same menu as the abort, in the order of what each
  /// one costs: pause after this task, stop now, end the conversation. Each says
  /// what it does, because *stop* and *force stop* are two words apart and hours
  /// of work apart.
  it("offers the three ways of stopping, each saying what it does", async () => {
    theGrillingStanding({ ready_to_stop: true, working: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);
    const offered = [...menu.querySelectorAll("button")].map(
      (button) => button.className,
    );

    expect(offered).toEqual(["stop", "force-stop", "abort"]);

    expect(
      screen.getByText("Pause after the current task until you resume."),
    ).toBeTruthy();
    expect(
      screen.getByText("Halt any running tasks and stop immediately."),
    ).toBeTruthy();
    expect(
      screen.getByText(/Permanently end the conversation and delete the/),
    ).toBeTruthy();
  });

  /// Force stop ends a session, so it is offered where there is one. With
  /// nothing running the ordinary stop halts the run at once anyway, and a
  /// second button promising the same thing would be one to think about for
  /// nothing.
  it("offers no force stop where nothing is running", async () => {
    theGrillingStanding({ ready_to_stop: true, working: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await drawn(container, ".conversation-actions .stop");
    expect(
      container.querySelector(".conversation-actions .force-stop"),
    ).toBeNull();
  });

  /// And neither is offered on a conversation that has already stopped. Getting
  /// one going again is what resume is for; there is nothing here left to stop.
  it("offers neither stop on a conversation that has already halted", async () => {
    theGrillingStanding({ ready_to_stop: false, working: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await drawn(container, ".conversation-actions .abort");
    expect(container.querySelector(".conversation-actions .stop")).toBeNull();
    expect(
      container.querySelector(".conversation-actions .force-stop"),
    ).toBeNull();
  });

  /// Nothing goes with either press. Which conversation it is is the whole of
  /// what there is to say, and which of the two stops it was is the route.
  it("posts to the conversation's own stop route", async () => {
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(
        STOPPING,
        json("Stopping" satisfies ConversationStopped),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, ".conversation-actions .stop"));

    await waitFor(() => expect(sent(fetching, STOPPING)).toEqual({}));
  });

  it("posts to the force stop route", async () => {
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(AT_ONCE, json("Stopped" satisfies ConversationStopped), "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(
      await drawn(container, ".conversation-actions .force-stop"),
    );

    await waitFor(() => expect(sent(fetching, AT_ONCE)).toEqual({}));
  });

  /// A stop that is waiting for a task to finish says so where it was pressed.
  /// Nothing on the timeline has changed yet — the session is still running, and
  /// the notice comes when it stops — so this is the only thing that can tell
  /// the human the press landed.
  it("says a stop is waiting for the task to finish", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(
        STOPPING,
        json("Stopping" satisfies ConversationStopped),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, ".conversation-actions .stop"));

    const waiting = await drawn(container, ".conversation-actions .waiting");

    expect(waiting.textContent).toContain("finishes its task first");
  });

  /// And a press that found the run already stopped says that instead, rather
  /// than looking as though it did something.
  it("says in words that it had already stopped", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(
        STOPPING,
        json("AlreadyHalted" satisfies ConversationStopped),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, ".conversation-actions .stop"));

    const refused = await drawn(container, ".conversation-actions .error");

    expect(refused.textContent).toBe(STOP_REFUSAL.AlreadyHalted);
    expect(refused.textContent).toContain("Resume");
  });
});

/// The three readable Question Sets the grilling conversation's session put to
/// the human: one answered, one still waiting on the session that asked it, and
/// one deferred — waiting too, with nothing standing still until it is
/// answered. All three are needed, because what a row draws turns on which.
const ASKED = (() => {
  const found = GRILLING.timeline.flatMap((event) =>
    "QuestionSet" in event ? [event.QuestionSet] : [],
  );
  if (found.length !== 3) {
    throw new Error(
      "the fixture should hold an answered Set, a waiting one and a deferred one",
    );
  }
  return found;
})();

const ANSWERED_SET = ASKED.find((asked) => "Answered" in asked.standing)!;

const WAITING_SET = ASKED.find(
  (asked) => "Waiting" in asked.standing && asked.standing.Waiting !== "deferred",
)!;

const DEFERRED_SET = ASKED.find(
  (asked) => "Waiting" in asked.standing && asked.standing.Waiting === "deferred",
)!;

/// And the third row a Set gets: the one whose stored body this build cannot
/// read, which is neither answered nor waiting and is on the record all the
/// same.
const UNREADABLE_SET = (() => {
  const found = GRILLING.timeline.flatMap((event) =>
    "UnreadableSet" in event ? [event.UnreadableSet] : [],
  );
  if (found.length !== 1) {
    throw new Error("the fixture should hold one Set this build cannot read");
  }
  return found[0]!;
})();

/// The whole document behind each, which is what the details pane fetches. The
/// two Set fixtures are the same shapes read back from the same endpoint — the
/// standing is what decides whether the pane draws a sheet or a record, and
/// these are the two.
///
/// Served as the whole reading rather than as the Set inside it, because that is
/// what the endpoint answers with: the pane is told which of the two kinds it is
/// holding before it draws anything.
const DOCUMENT = readable(answeredSet);
const SHEET = readable(answeringSet);

/// The workbench with the grilling conversation open and both of its Sets
/// answerable, which is what the details pane fetches when one is opened.
function theGrillingSets(...answers: Parameters<typeof serving>) {
  return theGrilling(
    whenever(`/api/ui/sets/${ANSWERED_SET.set_id}`, json(reads(DOCUMENT))),
    whenever(`/api/ui/sets/${WAITING_SET.set_id}`, json(reads(SHEET))),
    ...answers,
  );
}

/// One Question Set's summary as the interview it reads as: a line per
/// question, each the label it answers to, what it asked, and what became of it.
function interviewed(card: ParentNode): string[][] {
  return [...card.querySelectorAll(".asked .ask")].map((ask) =>
    [".n", ".question", ".answer"].map(
      (part) => ask.querySelector(part)?.textContent ?? "",
    ),
  );
}

describe("a question set on the timeline", () => {
  it("reads as an interview of question line and answer line", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, ".question-set");

    expect(interviewed(card)).toEqual(
      ANSWERED_SET.rows.map((row) => [
        row.name,
        row.question,
        // A question the human left open — and the Heading, which was never
        // asked. The line says so rather than leaving a blank, because a blank
        // on a settled Set would read as an Answer of nothing.
        row.answer === "" ? "unanswered" : row.answer,
      ]),
    );

    // Every pair of the Set, and no table: a long set earns a long card, and
    // the columns never fit the middle pane.
    expect(interviewed(card)).toHaveLength(ANSWERED_SET.rows.length);
    expect(card.querySelector("table")).toBeNull();
  });

  /// The one thing the old table said about the shape of a Set, kept: a
  /// Sub-question sits under the Question it belongs to, its lettered label and
  /// its answer carried in with it.
  it("sets a sub-question under the question it belongs to", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, ".question-set");

    expect(
      [...card.querySelectorAll(".asked .ask")].map((ask) =>
        ask.classList.contains("nested"),
      ),
    ).toEqual(ANSWERED_SET.rows.map((row) => row.nested));
  });

  /// A line each, whatever was asked and whatever came back. The card is the
  /// summary of the Set and the whole of it is a press away, so a question that
  /// ran to a paragraph would push the rest of the interview off the pane.
  /// jsdom lays nothing out, so the rules are what is read.
  it("holds each question and each answer to one truncated line", async () => {
    expect(stylesheet).toContain(
      ".question-set .asked .question,\n" +
        ".question-set .asked .answer {\n" +
        "  grid-column: 2;\n" +
        "  min-width: 0;\n" +
        "  display: block;\n" +
        "  overflow: hidden;\n" +
        "  white-space: nowrap;\n" +
        "  text-overflow: ellipsis;\n" +
        "}",
    );

    // And the track they sit in has to be allowed to be narrower than its
    // longest word, or there is nothing for the ellipsis to happen in.
    expect(stylesheet).toContain(
      "  grid-template-columns: var(--asked-label) minmax(0, 1fr);",
    );
  });

  /// The question is what the exchange is about, and the answer under it is
  /// read against it.
  it("sets the question in bold and the answer under it plainly", async () => {
    expect(stylesheet).toContain(
      ".question-set .asked .question {\n  font-weight: 600;\n}",
    );
  });

  it("says which set it is, so a timeline of rounds reads as a conversation", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, ".question-set");

    expect(card.querySelector(".set-title")!.textContent).toBe(
      ANSWERED_SET.title,
    );
  });

  /// The one thing on a timeline that is asking for something rather than
  /// recording it.
  it("marks the one still waiting on the human", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".question-set");
    const cards = [...container.querySelectorAll(".question-set")];

    // The answered one, the one still waiting, the deferred one — which is
    // waiting too, the human being the one who has not answered either — and
    // the unreadable one, which is waiting on nobody, whatever the record says
    // about it, because nothing here can put its questions in front of anybody.
    expect(cards.map((card) => card.classList.contains("waiting"))).toEqual([
      false,
      true,
      true,
      false,
    ]);
    expect(screen.getAllByText("waiting on you")).toHaveLength(2);
  });

  /// Both are something to answer, so both say so. What the second word adds is
  /// that no session is standing still until this one is answered — which is
  /// the difference between a question holding the work up and one the work
  /// went on without.
  it("says which of the two waiting sets was deferred", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".question-set");
    const cards = [...container.querySelectorAll(".question-set")];

    expect(
      cards.map((card) => card.querySelector(".deferred") !== null),
    ).toEqual([false, false, true, false]);

    const deferred = cards[2]!;

    expect(deferred.querySelector(".set-title")!.textContent).toBe(
      DEFERRED_SET.title,
    );
    expect(deferred.querySelector(".deferred")!.textContent).toBe("deferred");
  });

  /// A column of blanks would read as a Set that was answered with nothing.
  it("draws no answers at all on one nothing has been decided about", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".question-set");
    const waiting = [...container.querySelectorAll(".question-set")][1]!;

    expect(
      interviewed(waiting).map(([, , answer]) => answer),
    ).toEqual(WAITING_SET.rows.map(() => "—"));
  });

  /// The summary is a line each; the document is a Preface, every Option of
  /// every Question, and the Diff the ask was about.
  it("opens the whole document in the details pane", async () => {
    const fetching = theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".question-set"));

    const pane = screen.getByLabelText("Details");
    await waitFor(() => {
      if (!pane.querySelector(".preface")) {
        throw new Error("the document has not been drawn");
      }
    });

    expect(fetching).toHaveBeenCalledWith(
      `/api/ui/sets/${ANSWERED_SET.set_id}`,
      expect.anything(),
    );
    expect(pane.querySelector("h1")!.textContent).toBe(DOCUMENT.title);
    expect(frame(container).dataset.pane).toBe("details");
  });

  /// The point of the whole stage: the loop closes without leaving the GUI.
  it("answers the one still waiting, which is what ends the session's wait", async () => {
    const fetching = theGrillingSets(
      whenever(`/api/ui/sets/${SHEET.id}/response`, json("Accepted" satisfies Submitted)),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".question-set");
    fireEvent.click([...container.querySelectorAll(".question-set")][1]!);

    const pane = screen.getByLabelText("Details");
    const chosen = await waitFor(() => {
      const radio = pane.querySelector<HTMLInputElement>(
        'input[name="Q1-option"][value="2"]',
      );
      if (!radio) {
        throw new Error("the sheet has not been drawn");
      }
      return radio;
    });

    fireEvent.click(chosen);
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));
    // The rest of the Set is being left open, which the sheet asks about before
    // it sends — the same warning it gives on a page of its own.
    fireEvent.click(screen.getByRole("button", { name: "Send anyway" }));

    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        `/api/ui/sets/${SHEET.id}/response`,
        expect.objectContaining({ method: "POST" }),
      ),
    );

    expect(sent(fetching, `/api/ui/sets/${SHEET.id}/response`)).toMatchObject({
      answers: expect.arrayContaining([{ label: "Q1", selected: 2 }]),
    });
  });

  /// The pane caps what it holds at the same 60rem every other column is read
  /// at and centres it, so there is a margin here for the nav to stand in —
  /// which is what a Set as long as this one is read with anywhere else.
  it("brings the page's own table of contents into the pane", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".question-set"));

    const pane = screen.getByLabelText("Details");
    await waitFor(() => {
      if (!pane.querySelector(".preface")) {
        throw new Error("the document has not been drawn");
      }
    });

    const nav = pane.querySelector("nav.contents");
    expect(nav, "expected the Set's contents in the pane").toBeTruthy();

    // The same entries the page lists, in the same order — this is the Set
    // page's own nav rather than a second reading of the document.
    expect(
      [...nav!.querySelectorAll("a.contents-link")].map((line) =>
        line.getAttribute("href"),
      ),
    ).toEqual(["#preface", "#questions", "#q1", "#q2", "#q3", "#postscript"]);

    // And it picks its shape from the pane rather than from the window.
    expect(nav!.classList.contains("contents-paned")).toBe(true);
  });

  /// The floating header names where the reader is across the top of the column
  /// it belongs to. The pane has a header of its own up there already.
  it("leaves the floating header to the page", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".question-set"));

    const pane = screen.getByLabelText("Details");
    await drawn(pane, "nav.contents");

    expect(pane.querySelector(".page-header")).toBeNull();
  });
});

describe("a question set the build cannot read", () => {
  it("is a row saying so rather than a gap in the record", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".question-set");
    const row = container.querySelector(".question-set.unreadable")!;

    expect(row.querySelector(".unreadable-badge")!.textContent).toBe(
      "cannot be read",
    );
    // Serde's own sentence, which names the field that has left the schema.
    expect(row.querySelector(".unreadable-why")!.textContent).toContain(
      "accepted_by",
    );
    // No table, because there is nothing to draw one from — and nothing asking
    // the human for anything either.
    expect(row.querySelector(".asked")).toBeNull();
    expect(row.classList.contains("waiting")).toBe(false);
  });

  it("opens the stored body in the details pane, the way any Set opens", async () => {
    theGrillingSets(
      whenever(`/api/ui/sets/${UNREADABLE_SET.set_id}`, json(unreadableSet)),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".question-set");
    fireEvent.click(container.querySelector(".question-set.unreadable")!);

    const pane = screen.getByLabelText("Details");
    const stored = await waitFor(() => {
      const found = pane.querySelector(".stored-json");
      if (!found) {
        throw new Error("the stored body has not been drawn");
      }
      return found;
    });

    expect(stored.textContent).toBe(unreadable(unreadableSet).body);
    // The one thing the timeline's rows are not: a sheet to fill in.
    expect(pane.querySelector(".questions")).toBeNull();
  });
});

/// The third shape the middle pane draws: a conversation the grilling has handed
/// over, its closing proposal answered with a direction picked on it, and the
/// work being built.
const BUILDING = building as ConversationView;

/// The workbench with that conversation open.
function theBuilding(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${BUILDING.id}`,
      json({ ...BUILDING, ...over }),
    ),
    ...answers,
  );
}

/// The handoff that grilling wrote, which is the same conversation's own.
const HANDOFF = (() => {
  const event = BUILDING.timeline.find((entry) => "Handoff" in entry);
  if (!event || !("Handoff" in event)) {
    throw new Error("the fixture should carry a handoff");
  }
  return event.Handoff;
})();

describe("a grilling that has handed over", () => {
  it("draws no chooser anywhere: the pick rode the closing set", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    // The timeline is up, so the pane has drawn the whole of what it was handed.
    await drawn(container, ".timeline");

    expect(BUILDING.state).toBe("Implementing");
    expect(container.querySelector(".direction-chooser")).toBeNull();
    expect(container.querySelector(".directions")).toBeNull();
  });

  it("shows the answered proposal set as the record of the choice", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const asked = await drawn(container, ".timeline-event > .question-set");

    // Answered, and answered with a pick: what the human decided is on the set
    // they decided it on, and there is no second event beside it saying so.
    expect(asked.querySelector(".live")).toBeNull();
    expect(asked.textContent).toContain("Ready to build the usage-limit pause");
  });

  it("goes from grilling to implementing with no rung in between", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, ".timeline");

    const moved = [
      ...container.querySelectorAll(".timeline-event > .moved"),
    ].map((line) => line.textContent);

    expect(moved).toEqual(["Draft → Grilling", "Grilling → Implementing"]);
  });

  /// A move records only the state it went to, and an abort is off the ladder
  /// rather than on it — so what it stopped in is the move before it, which is
  /// the whole of what makes the line worth reading.
  it("names the state an abort stopped in", async () => {
    theBuilding({
      state: "Aborted",
      timeline: [
        ...BUILDING.timeline,
        { Moved: { id: 9001, at: "2026-08-24T11:00:00Z", state: "Aborted" } },
      ],
    });
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, ".timeline");

    const moved = [
      ...container.querySelectorAll(".timeline-event > .moved"),
    ].map((line) => line.textContent);

    expect(moved.at(-1)).toBe("Implementing → Aborted");
  });

  it("draws the handoff the grilling wrote as the document it is", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const handoff = await drawn(container, ".timeline-event > .handoff");

    expect(handoff.querySelector("h2")?.textContent).toBe("Handoff");
    expect(handoff.querySelector(".markdown")?.innerHTML).toContain(
      "<h1>Pausing on a usage limit</h1>",
    );

    // Nothing to edit and nothing to answer: it is the agent's account of a
    // conversation that is over. The card opens, which is a different thing —
    // see the documents on a timeline, below.
    expect(handoff.querySelector("textarea")).toBeNull();
    expect(handoff.querySelector("button")).toBeNull();
  });
});

describe("disagreeing with a proposal", () => {
  it("draws no chooser while the grilling is still running", async () => {
    // What a refused proposal leaves behind: the set is on the timeline and
    // answered, and the conversation is still grilling.
    theBuilding({ state: "Grilling", direction: null });
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, ".timeline");

    expect(container.querySelector(".direction-chooser")).toBeNull();
    expect(container.querySelector(".directions")).toBeNull();
  });
});

/// The commits on that conversation's timeline: what a session leaves behind
/// besides its output.
const COMMITS = BUILDING.timeline.flatMap((event) =>
  "Commit" in event ? [event.Commit] : [],
);

/// One commit, as the details pane fetches it: no summary, which is the
/// bookkeeping commit and every commit recorded before summaries were kept.
///
/// The diff is built from the answering set's own attached diff rather than
/// written by hand: it is the same `DiffView`, rendered by the same server-side
/// renderer that a commit's diff goes through — which is the whole reason a
/// commit needs no diff machinery of its own.
const COMMIT_PANE: CommitPane = {
  summary: null,
  diagrams: false,
  diff: readable(answeringSet).diff,
};

/// The same commit with something to say for itself: the summary as the server
/// renders one — prose, and the source block a Diagram is held for.
const SUMMARISED: CommitPane = {
  ...COMMIT_PANE,
  summary:
    '<p>A bucket per account.</p>\n<div class="wide"><pre class="mermaid">flowchart LR\n  in --&gt; limiter --&gt; out\n</pre></div>',
  diagrams: true,
};

/// The other commit on that timeline, summarised in its turn: what the pane is
/// handed when the human reads one commit and then the next.
const SUMMARISED_TOO: CommitPane = {
  ...COMMIT_PANE,
  summary:
    '<p>A queue per repository.</p>\n<div class="wide"><pre class="mermaid">flowchart LR\n  work --&gt; queue --&gt; runner\n</pre></div>',
  diagrams: true,
};

/// Where the details pane fetches it from.
const DIFF_OF_IT = `/api/ui/conversations/${BUILDING.id}/commit/${COMMITS[0]!.id}`;

/// And where it fetches the other one, for the tests that open both.
const DIFF_OF_THE_OTHER = `/api/ui/conversations/${BUILDING.id}/commit/${COMMITS[1]!.id}`;

/// The workbench with that conversation open and its commits to hand.
function theCommits(...answers: Parameters<typeof serving>) {
  return theBuilding({}, whenever(DIFF_OF_IT, json(COMMIT_PANE)), ...answers);
}

describe("a commit on the timeline", () => {
  it("summarises as what it was called and how much it moved", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const row = await drawn(container, ".timeline-event > .commit");
    const commit = COMMITS[0]!;

    expect(row.querySelector(".subject")!.textContent).toBe(commit.subject);
    expect(row.querySelector(".sha")!.textContent).toBe(
      commit.sha.slice(0, 7),
    );
    expect(row.querySelector(".files")!.textContent).toBe(
      `${commit.files} files`,
    );
    expect(row.querySelector(".added")!.textContent).toBe(
      `+${commit.insertions}`,
    );
    expect(row.querySelector(".removed")!.textContent).toBe(
      `−${commit.deletions}`,
    );
  });

  /// The card's own account of the commit, under the counts. Clamped by the
  /// stylesheet, so what is asked here is that the prose is on the card at all
  /// and that it is prose — the fixture's summary opens with a Diagram, and a
  /// card filled with the words of the fence would be the whole of what is left
  /// to read.
  it("carries a snippet of what the commit said about itself", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, ".timeline-event > .commit");

    const said = COMMITS.find((commit) => commit.snippet !== null)!;
    const row = [
      ...container.querySelectorAll(".timeline-event > .commit"),
    ].find((card) => card.querySelector(".subject")!.textContent === said.subject)!;

    expect(row.querySelector(".snippet")!.textContent).toBe(said.snippet);
    expect(row.querySelector(".snippet")!.textContent).not.toContain(
      "flowchart",
    );
  });

  /// Every bookkeeping commit and every commit recorded before summaries were
  /// kept. Nothing marks the absence: the card is the one it has always been.
  it("draws the card it always drew for a commit that said nothing", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, ".timeline-event > .commit");

    const silent = COMMITS.find((commit) => commit.snippet === null)!;
    const row = [
      ...container.querySelectorAll(".timeline-event > .commit"),
    ].find((card) => card.querySelector(".subject")!.textContent === silent.subject)!;

    expect(row.querySelector(".snippet")).toBeNull();
    expect(row.innerHTML).toBe(
      '<span class="event-head">' +
        '<span class="what">Commit</span>' +
        `<span class="sha">${silent.sha.slice(0, 7)}</span>` +
        "</span>" +
        `<span class="subject">${silent.subject}</span>` +
        '<span class="changed">' +
        `<span class="files">${silent.files} files</span>` +
        `<span class="added">+${silent.insertions}</span>` +
        `<span class="removed">−${silent.deletions}</span>` +
        "</span>",
    );
  });

  it("draws one row per commit, in timeline order", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, ".timeline-event > .commit");

    const subjects = [
      ...container.querySelectorAll(".timeline-event > .commit .subject"),
    ].map((it) => it.textContent);

    expect(COMMITS).toHaveLength(2);
    expect(subjects).toEqual(COMMITS.map((commit) => commit.subject));
  });

  /// There is nothing to decide about a commit. The design gives it no
  /// per-commit review — feedback consolidates in the wrap-up phase — so the
  /// row opens a diff and offers nothing else.
  it("asks the human for nothing", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const row = await drawn(container, ".timeline-event > .commit");

    expect(row.querySelectorAll("button")).toHaveLength(0);
    expect(row.textContent).not.toContain("Approve");
  });

  /// The pane is the open Event and nothing else, so closing the diff leaves it
  /// bare — and a narrow window walks back out to the record with it, there
  /// being no level left to be on.
  it("empties the pane and walks back out when it is closed", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));
    await drawn(container, ".details-pane .diff-files");

    fireEvent.click(await drawn(container, ".details-pane .close-event"));

    await waitFor(() =>
      expect(screen.getByLabelText("Details").textContent).toBe(""),
    );
    expect(frame(container).dataset.pane).toBe("timeline");
  });

  it("shows that commit's diff in the details pane, as the server rendered it", async () => {
    const fetching = theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const diff = await drawn(container, ".details-pane .diff-files");

    // Put in the page as it arrived: the folds, the per-file anchors and the
    // highlighting are all the renderer's, and nothing here reads the diff.
    const folds = [...diff.querySelectorAll("details.diff-file")];

    expect(folds.map((fold) => fold.id)).toEqual(["diff-1", "diff-2"]);
    expect(folds[0]!.querySelector(".diff-path")!.textContent).toBe(
      COMMIT_PANE.diff!.paths[0],
    );
    expect(diff.querySelector(".diff-line.add")).toBeTruthy();
    expect(diff.querySelector(".tok-storage")).toBeTruthy();
    expect(askedFor(fetching, DIFF_OF_IT)).toBeGreaterThan(0);
  });

  /// The message is the event's to say: the diff arrives headerless, because
  /// the renderer splits on `diff --git` and would drop anything above it.
  it("says which commit it is above the diff", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const header = await drawn(container, ".details-pane .commit-header");

    expect(header.textContent).toContain(COMMITS[0]!.subject);
    expect(header.textContent).toContain(COMMITS[0]!.sha.slice(0, 7));
  });

  /// What the commit said about itself, between the header and the diff — the
  /// server rendered and sanitized it, so the pane only has to put it in the
  /// page.
  it("shows the commit's summary above the diff", async () => {
    theBuilding(
      {},
      whenever(
        DIFF_OF_IT,
        json({
          ...COMMIT_PANE,
          summary: "<p>A bucket per account.</p>",
        } satisfies CommitPane),
      ),
    );
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const summary = await drawn(container, ".details-pane .commit-summary");

    expect(summary.innerHTML).toBe("<p>A bucket per account.</p>");

    // Read in the order it is written in: what the commit says about itself,
    // then what it changed.
    const diff = await drawn(container, ".details-pane .diff");

    expect(
      summary.compareDocumentPosition(diff) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// A Diagram in the summary is the one thing the pane draws for itself, and it
  /// draws it the Set page's way: over the source block the server left, once the
  /// summary is in the page. What it drew is `diagrams.test.ts`'s subject; what
  /// is asked here is that it was reached for, and over this block alone — a Set
  /// page open behind the workbench draws its own.
  it("draws the Diagram in a summary that holds one", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json(SUMMARISED)));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const summary = await drawn(container, ".details-pane .commit-summary");

    // The source block the renderer draws over — and what the reader is left
    // with if it never draws.
    expect(summary.querySelector("pre.mermaid")!.textContent).toContain(
      "flowchart LR",
    );

    await waitFor(() => expect(drawing).toHaveBeenCalledOnce());
    expect(drawing.mock.calls[0]![0]).toEqual({ root: summary });
  });

  /// And draws it again for the next commit opened, which is not a second mount:
  /// the pane is not rebuilt per commit, so the second summary's markup lands in
  /// the block the first one was drawn in.
  ///
  /// Read three commits deep on purpose. The first switch is masked — the second
  /// commit is still being fetched for a tick, and a pane with no summary yet
  /// takes the block out of the page and puts a fresh one back — but a commit the
  /// cache already holds comes back with no such gap, and that is the one a
  /// drawing hung on the mount would never follow.
  it("draws the Diagram again for each commit the pane is switched to", async () => {
    theBuilding(
      {},
      whenever(DIFF_OF_IT, json(SUMMARISED)),
      whenever(DIFF_OF_THE_OTHER, json(SUMMARISED_TOO)),
    );
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, ".timeline-event > .commit");

    /// The card for one of the two, which is the only way to tell them apart on
    /// the timeline.
    const card = (subject: string) =>
      [...container.querySelectorAll(".timeline-event > .commit")].find(
        (row) => row.querySelector(".subject")!.textContent === subject,
      )!;

    /// The summary block once it is holding the commit that was clicked, rather
    /// than the one before it: which commit the pane is showing is exactly what
    /// this test is about, so waiting for the block alone would prove nothing.
    const showing = (words: string) =>
      waitFor(() => {
        const block = container.querySelector(".details-pane .commit-summary");
        if (!block?.textContent?.includes(words)) {
          throw new Error(`the pane is not showing ${words} yet`);
        }
        return block;
      });

    fireEvent.click(card(COMMITS[0]!.subject));
    const first = await showing("A bucket per account.");
    await waitFor(() => expect(drawing).toHaveBeenCalledOnce());
    expect(drawing.mock.calls[0]![0]).toEqual({ root: first });

    fireEvent.click(card(COMMITS[1]!.subject));
    const second = await showing("A queue per repository.");
    await waitFor(() => expect(drawing).toHaveBeenCalledTimes(2));
    expect(drawing.mock.calls[1]![0]).toEqual({ root: second });

    // Back to the first, which the cache still holds and hands back whole.
    fireEvent.click(card(COMMITS[0]!.subject));
    const again = await showing("A bucket per account.");
    await waitFor(() => expect(drawing).toHaveBeenCalledTimes(3));
    expect(drawing.mock.calls[2]![0]).toEqual({ root: again });

    // And every drawing before the one on the page was stopped: each is watching
    // the colour scheme, and one nobody took down redraws nodes the block has
    // since let go of.
    expect(drawing.mock.results[0]!.value).toHaveBeenCalledOnce();
    expect(drawing.mock.results[1]!.value).toHaveBeenCalledOnce();
    expect(drawing.mock.results[2]!.value).not.toHaveBeenCalled();
  });

  /// Which is every other commit: mermaid is megabytes, and a pane with nothing
  /// to draw pays none of them.
  it("never reaches for the renderer where the summary holds no Diagram", async () => {
    theBuilding(
      {},
      whenever(
        DIFF_OF_IT,
        json({
          ...COMMIT_PANE,
          summary: "<p>A bucket per account.</p>",
        } satisfies CommitPane),
      ),
    );
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));
    await drawn(container, ".details-pane .commit-summary");

    expect(drawing).not.toHaveBeenCalled();
  });

  /// The ordinary commit, and every commit recorded before summaries were kept:
  /// the pane is the header and the diff, exactly as it always was.
  it("draws nothing where the commit carried no summary", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));
    await drawn(container, ".details-pane .diff-files");

    expect(container.querySelector(".details-pane .commit-summary")).toBeNull();
  });

  it("says so plainly when the commit changed no files", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json({ diff: null })));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    // Waited for rather than read once: the pane says `Loading…` in the same
    // place while the diff is in flight.
    await waitFor(() =>
      expect(
        container.querySelector(".details-pane .empty")!.textContent,
      ).toContain("changed no files"),
    );

    expect(container.querySelector(".details-pane .diff")).toBeNull();
  });

  /// A commit whose repository no longer has it is a 404, and the pane says the
  /// server's own words rather than drawing an empty diff.
  it("shows the server's wording when the diff cannot be read", async () => {
    theBuilding(
      {},
      whenever(DIFF_OF_IT, () =>
        Promise.resolve(
          new Response(
            JSON.stringify({
              error: "there is no such commit on that Conversation",
            }),
            { status: 404, headers: { "content-type": "application/json" } },
          ),
        ),
      ),
    );
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const error = await drawn(container, ".details-pane .error");

    expect(error.textContent).toContain(
      "there is no such commit on that Conversation",
    );
  });

  /// A commit's diff cannot change, so it is read once and never again — not
  /// even on a Nudge, whose invalidation beats any finite staleTime. What that
  /// buys besides the request is the per-file folds: the diff's markup is
  /// reassigned wholesale whenever the query's data moves, and data that is
  /// never re-read never moves.
  it("does not read the diff again on a Nudge, so its folds hold", async () => {
    const fetching = theCommits();
    const { container, client } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));
    const fold = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane details.diff-file",
    );
    fold.open = false;

    const before = askedFor(fetching, DIFF_OF_IT);
    await client.invalidateQueries();

    expect(askedFor(fetching, DIFF_OF_IT)).toBe(before);
    expect(
      container.querySelector<HTMLDetailsElement>(
        ".details-pane details.diff-file",
      ),
    ).toBe(fold);
    expect(fold.open).toBe(false);
  });

  /// The event that is open is the one the timeline says is open, so a narrow
  /// window walking back out can see which it came from.
  it("marks the commit the details pane is showing", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const row = await drawn(container, ".timeline-event > .commit");
    expect(row.classList).not.toContain("selected");

    fireEvent.click(row);

    await waitFor(() => expect(row.classList).toContain("selected"));
    expect(row.getAttribute("aria-pressed")).toBe("true");
  });
});

/// The table of contents in a details pane, on the two panes that hold one: a
/// commit's diff and a Question Set.
///
/// The nav itself is the Set page's own — its entries, its scroll-spy and its
/// jump are asked about where the page is — so what these ask is what the pane
/// adds: that a commit's folds are listed at all, and that which of the two
/// shapes is drawn is the pane's width's answer rather than the window's.
describe("the contents of a details pane", () => {
  /// What jsdom lays out, which is nothing: every element is nought wide, so a
  /// nav measuring the pane it is in would always fold into its bar. This
  /// answers for the pane alone and leaves everything else the nothing jsdom
  /// knows about it.
  const measured = Object.getOwnPropertyDescriptor(
    Element.prototype,
    "clientWidth",
  );

  /// How wide the details pane is standing, in rem.
  function paneStands(rem: number): void {
    Object.defineProperty(Element.prototype, "clientWidth", {
      configurable: true,
      get(this: Element) {
        return this.classList.contains("details-pane") ? rem * 16 : 0;
      },
    });
  }

  /// What the pane's nav is watching, since jsdom has no observer of its own:
  /// the ids handed to the one the scroll-spy makes.
  function spying(): string[] {
    const watched: string[] = [];

    vi.stubGlobal(
      "IntersectionObserver",
      class {
        observe(target: Element) {
          watched.push(target.id);
        }
        disconnect() {}
        unobserve() {}
      },
    );

    return watched;
  }

  beforeEach(() => {
    // Asked for no motion, the jump asks with `scrollIntoView` — which is the
    // ask a test with no layout under it can see.
    Element.prototype.scrollIntoView = vi.fn();
    vi.stubGlobal("matchMedia", () => ({ matches: true }));
  });

  afterEach(() => {
    if (measured !== undefined) {
      Object.defineProperty(Element.prototype, "clientWidth", measured);
    }
  });

  it("names every file of a commit's diff, in diff order", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const nav = await drawn(container, ".details-pane nav.contents");

    // The section the diff is, and then its folds — the same anchors the
    // renderer stamped on them, in the order the paths beside it name.
    expect(
      [...nav.querySelectorAll("a.contents-link")].map((line) =>
        line.getAttribute("href"),
      ),
    ).toEqual(["#commit-diff", "#diff-1", "#diff-2"]);

    // The whole path rides behind the line, which is where a nav this narrow
    // can be read out in full.
    expect(
      [...nav.querySelectorAll(".contents-entry a")].map((line) =>
        line.getAttribute("title"),
      ),
    ).toEqual(COMMIT_PANE.diff!.paths);
  });

  /// And above them, what the commit said about itself — one nav over the whole
  /// pane, in the order the pane is read in.
  it("lists a commit's summary above its diff, and jumps to both", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json(SUMMARISED)));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const nav = await drawn(container, ".details-pane nav.contents");

    expect(
      [...nav.querySelectorAll("a.contents-link")].map((line) =>
        line.getAttribute("href"),
      ),
    ).toEqual(["#commit-summary", "#commit-diff", "#diff-1", "#diff-2"]);

    // And both lines land on something: the ids are the pane's own sections
    // rather than names the nav made up.
    for (const anchor of ["commit-summary", "commit-diff"]) {
      const section = container.querySelector(`.details-pane #${anchor}`);
      expect(section, `expected the pane to hold #${anchor}`).toBeTruthy();

      const landed = vi.fn();
      section!.scrollIntoView = landed;
      nav.querySelector<HTMLAnchorElement>(`a[href="#${anchor}"]`)!.click();

      expect(landed).toHaveBeenCalled();
    }
  });

  it("jumps into a fold of the commit, unfolding it first", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const nav = await drawn(container, ".details-pane nav.contents");
    const fold = container.querySelector<HTMLDetailsElement>(
      ".details-pane #diff-2",
    )!;
    fold.open = false;

    nav.querySelector<HTMLAnchorElement>('a[href="#diff-2"]')!.click();

    expect(fold.open, "landing on a closed fold is landing on nothing").toBe(
      true,
    );
    expect(fold.scrollIntoView).toHaveBeenCalled();
  });

  it("tracks the reader's place among the commit's folds", async () => {
    const watched = spying();
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));
    await drawn(container, ".details-pane nav.contents");

    await waitFor(() =>
      expect(watched).toEqual(
        expect.arrayContaining(["commit-diff", "diff-1", "diff-2"]),
      ),
    );
  });

  it("folds into its bar where the pane has no margin to stand in", async () => {
    paneStands(50);
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const nav = await drawn(container, ".details-pane nav.contents");
    expect(nav.classList.contains("contents-paned")).toBe(true);
    expect(nav.classList.contains("contents-roomy")).toBe(false);
  });

  it("stands in the margin once the pane is wide enough for one", async () => {
    paneStands(90);
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const nav = await drawn(container, ".details-pane nav.contents");
    expect(nav.classList.contains("contents-roomy")).toBe(true);
  });

  /// The same answer on the other pane that holds a nav, from the same
  /// measurement: it is the pane's width, not what the pane is holding.
  it("answers the same way for a question set's pane", async () => {
    paneStands(90);
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".question-set"));

    const nav = await drawn(container, ".details-pane nav.contents");
    expect(nav.classList.contains("contents-roomy")).toBe(true);
  });

  /// The window is wide enough for the page's own sidebar at every width these
  /// tests run at; what decides the nav's shape here is the pane.
  it("keeps the bar on a narrow set pane, whatever the window is doing", async () => {
    paneStands(40);
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".question-set"));

    const nav = await drawn(container, ".details-pane nav.contents");
    expect(nav.classList.contains("contents-roomy")).toBe(false);
    expect(nav.querySelector(".contents-bar")).toBeTruthy();
  });
});

/// The fifth shape the middle pane draws: a conversation being built from a
/// backlog, with `.tasks/` in its worktree read back as the pinned task list.
const TASKED = tasks as ConversationView;

/// The list itself, off that payload.
const BACKLOG = (() => {
  const pinned = TASKED.pinned[0];
  if (!pinned || !("TaskList" in pinned)) {
    throw new Error("the fixture should carry a pinned task list");
  }
  return pinned.TaskList;
})();

/// The workbench with that conversation open.
function theTasked(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${TASKED.id}`,
      json({ ...TASKED, ...over }),
    ),
    ...answers,
  );
}

describe("the pinned task list", () => {
  it("draws every task of the backlog, in the list's own order", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, ".pinned .task-list");

    expect(BACKLOG.tasks).toHaveLength(4);
    expect(
      [...list.querySelectorAll(".tasks li")].map((row) => [
        row.querySelector(".n")!.textContent,
        row.querySelector(".what")!.textContent,
      ]),
    ).toEqual(BACKLOG.tasks.map((task) => [task.number, task.title]));
  });

  it("says which tasks are done", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, ".pinned .task-list");
    const rows = [...list.querySelectorAll(".tasks li")];

    expect(rows.map((row) => row.classList.contains("done"))).toEqual(
      BACKLOG.tasks.map((task) => task.done),
    );

    // Drawn the way the file it is read out of writes it, a box per row.
    expect(rows.map((row) => row.querySelector(".box")!.textContent)).toEqual(
      BACKLOG.tasks.map((task) => (task.done ? "☑" : "☐")),
    );

    // In words as well as in a class, so a row read aloud says it too — the box
    // is the look of it and the word is what anything reading gets, which is
    // why it is out of the layout rather than out of the document.
    expect(rows.map((row) => row.querySelector(".state")!.textContent)).toEqual(
      BACKLOG.tasks.map((task) => (task.done ? "done" : "to do")),
    );
    expect(stylesheet).toContain(
      ".pinned .task-list .state,\n" +
        ".pinned .stage-list .state {\n" +
        "  position: absolute;",
    );
  });

  /// `[ ] Some task            01`: the box and the title lead, and the number
  /// is at the far end of the row, out of the way of the reading.
  it("puts the number at the right edge of each row", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, ".pinned .task-list");

    // The order of the row is the order it reads in: nothing is moved by the
    // stylesheet that the document does not already say.
    expect(
      [...list.querySelectorAll(".tasks li")].map((row) =>
        [...row.children]
          .map((part) => part.className)
          .filter((name) => name !== "state"),
      ),
    ).toEqual(BACKLOG.tasks.map(() => ["box", "what", "n"]));

    // And what holds it against that edge, which jsdom lays out no more than it
    // does the rest.
    expect(stylesheet).toContain(
      ".pinned .task-list .n,\n" +
        ".pinned .stage-list .n {\n" +
        "  margin-left: auto;\n" +
        "  flex: none;",
    );
  });

  it("says what the backlog is and how far through it the work is", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const head = await drawn(container, ".pinned .task-list .event-head");

    expect(head.textContent).toContain("Task list");
    expect(head.querySelector(".feature")!.textContent).toBe(BACKLOG.feature);
    expect(head.querySelector(".progress")!.textContent).toBe("2 of 4 done");
  });

  /// Pinned is a thing an event *is*, decided by its kind: it is drawn outside
  /// the record, so it does not scroll away with it.
  it("is drawn above the record rather than in it", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const pinned = await drawn(container, ".pinned");

    expect(pinned.closest(".timeline")).toBeNull();
    expect(container.querySelector(".timeline .tasks")).toBeNull();
  });

  /// Nothing pins or unpins one: the set is fixed, so there is no control for
  /// it and no details pane to open — the whole of a task list is the list.
  it("asks the human for nothing", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, ".pinned .task-list");

    expect(list.querySelectorAll("button")).toHaveLength(0);
    expect(list.textContent).not.toContain("Pin");
  });

  /// What holds it in view is the block it shares with the header, so that is
  /// where the rule is read. jsdom lays nothing out, so the rule itself is what
  /// is read, as the panes' own is.
  it("stays in view while the record scrolls past it", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const pinned = await drawn(container, ".pinned");
    const chrome = pinned.closest(".pane-chrome");

    // One block, with the header in it: that is what makes them stay together
    // with no strip of scrolling record between them.
    expect(chrome).not.toBeNull();
    expect(chrome!.querySelector(".pane-head")).not.toBeNull();

    expect(stylesheet).toContain(
      ".pane > .pane-head,\n.pane > .pane-chrome {\n  position: sticky;\n  top: 0;",
    );
  });

  /// The bug this fixed: the pinned block and the menu hanging off the header
  /// were on one layer, so the pinned items were drawn over the menu and there
  /// was no way to press what was under them.
  it("draws what hangs off the header over the pinned items", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const menu = await openActions(container);

    // Both are inside the one stuck block, so which is over which is settled
    // between them rather than against the record.
    expect(menu.closest(".pane-chrome")).not.toBeNull();
    expect(stylesheet).toContain(
      ".pane-chrome > .pane-head {\n  position: relative;\n  z-index: 1;\n}",
    );
  });

  it("draws nothing at all where the worktree holds no backlog", async () => {
    // Every other fixture here is a conversation with no `.tasks/`, which is
    // the ordinary case: the server pins nothing and there is nothing to draw.
    expect(OPEN.pinned).toEqual([]);

    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, ".timeline");

    expect(container.querySelector(".pinned")).toBeNull();
    expect(container.querySelector(".task-list")).toBeNull();
  });
});

/// The sixth shape the middle pane draws: a conversation whose direction was a
/// staged roadmap, with `docs/roadmaps/` in its worktree read back as the
/// pinned stage list.
const STAGED = roadmap as ConversationView;

/// The roadmap itself, off that payload.
const ROADMAP = (() => {
  const pinned = STAGED.pinned[0];
  if (!pinned || !("StageList" in pinned)) {
    throw new Error("the fixture should carry a pinned stage list");
  }
  return pinned.StageList;
})();

/// The workbench with that conversation open.
function theStaged(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${STAGED.id}`,
      json({ ...STAGED, ...over }),
    ),
    ...answers,
  );
}

describe("the pinned stage list", () => {
  it("draws every stage of the roadmap, in the roadmap's own order", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, ".pinned .stage-list");

    expect(ROADMAP.stages).toHaveLength(4);
    expect(
      [...list.querySelectorAll(".stages li")].map((row) => [
        row.querySelector(".n")!.textContent,
        row.querySelector(".what")!.textContent,
      ]),
    ).toEqual(ROADMAP.stages.map((stage) => [stage.number, stage.title]));
  });

  /// The roadmap's rows read the way the backlog's do, number at the far end.
  it("puts the number at the right edge of each row", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, ".pinned .stage-list");

    expect(
      [...list.querySelectorAll(".stages li")].map((row) =>
        [...row.children]
          .map((part) => part.className)
          .filter((name) => name !== "state"),
      ),
    ).toEqual(ROADMAP.stages.map(() => ["box", "what", "n"]));
  });

  it("says which stages are checked", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, ".pinned .stage-list");
    const rows = [...list.querySelectorAll(".stages li")];

    expect(rows.map((row) => row.classList.contains("done"))).toEqual(
      ROADMAP.stages.map((stage) => stage.done),
    );

    // Boxes and words both, as a task's row carries them.
    expect(rows.map((row) => row.querySelector(".box")!.textContent)).toEqual(
      ROADMAP.stages.map((stage) => (stage.done ? "☑" : "☐")),
    );
    expect(rows.map((row) => row.querySelector(".state")!.textContent)).toEqual(
      ROADMAP.stages.map((stage) => (stage.done ? "done" : "to do")),
    );
  });

  it("says which roadmap it is and how far through it the effort is", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const head = await drawn(container, ".pinned .stage-list .event-head");

    expect(head.textContent).toContain("Roadmap");
    expect(head.querySelector(".feature")!.textContent).toBe(ROADMAP.title);
    expect(head.querySelector(".progress")!.textContent).toBe("2 of 4 done");
  });

  /// Its directory is its identity, so a roadmap that wrote no heading is still
  /// named — by the directory whoever starts a stage is pointed at.
  it("falls back to the roadmap's directory where it wrote no heading", async () => {
    theStaged({ pinned: [{ StageList: { ...ROADMAP, title: "" } }] });
    const { container } = mount(`/conversations/${STAGED.id}`);

    const head = await drawn(container, ".pinned .stage-list .event-head");

    expect(head.querySelector(".feature")!.textContent).toBe(ROADMAP.name);
  });

  /// Pinned beside the backlog and the pull request, and drawn the same way:
  /// outside the record, with nothing to pin, unpin or open.
  it("is drawn above the record and asks the human for nothing", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, ".pinned .stage-list");

    expect(list.closest(".timeline")).toBeNull();
    expect(container.querySelector(".timeline .stages")).toBeNull();
    expect(list.querySelectorAll("button")).toHaveLength(0);
  });

  /// What Verkstead did on its own account while nobody was watching — here,
  /// the stage it started when this roadmap's wrap-up settled.
  ///
  /// A line in the record rather than a card above it, because it is a sentence
  /// and not a document: there is nothing to open, and nothing to answer.
  it("draws what verkstead did unasked as a line in the record", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const notice = await drawn(container, ".timeline-event > .notice");

    expect(notice.textContent).toContain("Stage 01");
    expect(notice.querySelector("code")?.textContent).toBe("mvp");
    expect(notice.closest(".timeline")).not.toBeNull();
    expect(notice.querySelectorAll("button")).toHaveLength(0);
  });

  it("draws nothing at all where the branch has written no roadmap", async () => {
    // Every other fixture here is a conversation whose branch touched none,
    // which is the ordinary case: the server pins nothing.
    expect(TASKED.pinned.some((event) => "StageList" in event)).toBe(false);

    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    await drawn(container, ".pinned .task-list");

    expect(container.querySelector(".stage-list")).toBeNull();
  });
});

/// A conversation whose driving has halted, and the notice saying what stopped.
const STOPPED = halted as ConversationView;

/// The notice itself, off that payload: what stopped, why, and the evidence.
const SAID = (() => {
  const event = STOPPED.timeline.find((entry) => "Notice" in entry);
  if (!event || !("Notice" in event)) {
    throw new Error("the fixture should carry the notice of a halt");
  }
  return event.Notice;
})();

/// The workbench with that conversation open.
function theStopped(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${STOPPED.id}`,
      json({ ...STOPPED, ...over }),
    ),
    ...answers,
  );
}

describe("the notice of a halt", () => {
  /// Inline and whole, unlike a capture or a diff: what a stop has to say is a
  /// paragraph and two blocks of terminal text, gathered when the run stopped
  /// because a worktree and a session's output both move on. So it is on the
  /// event rather than behind a fetch.
  it("says what stopped, why, and what the evidence was", async () => {
    const fetching = theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const notice = await drawn(container, ".timeline .notice");

    expect(notice.textContent).toContain(
      "The task in .tasks/03-commit-events.md",
    );
    expect(notice.textContent).toContain("the session exited with status 1");
    expect(notice.textContent).toContain("crates/store/src/commits.rs");
    expect(notice.textContent).toContain("could not compile");

    // The columns are the whole of what makes a status readable, and neither
    // git nor a terminal writes markdown.
    expect(notice.querySelectorAll("pre").length).toBe(2);

    // And nothing was fetched for it: no request the page made names the
    // event, the way a Capture and a diff are each named by one.
    expect(
      fetching.mock.calls
        .map(([asked]) => String(asked))
        .filter((path) => path.includes(`/${SAID.id}`)),
    ).toEqual([]);
  });

  /// A line and not a card: there is nothing to open and nothing to answer.
  /// What gets the work going again is Resume, at the foot of the timeline.
  it("holds nothing to press", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const notice = await drawn(container, ".timeline .notice");

    expect(notice.querySelector("button")).toBeNull();
    expect(notice.classList.contains("openable")).toBe(false);
  });
});

/// A conversation waiting an account's window out, and the pause it stopped at.
const WAITING = paused as ConversationView;

const OUT_OF_WINDOW = (() => {
  const event = WAITING.timeline.find((entry) => "Pause" in entry);
  if (!event || !("Pause" in event)) {
    throw new Error("the fixture should carry a pause");
  }
  return event.Pause;
})();

/// The workbench with that conversation open.
function thePaused(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${WAITING.id}`,
      json({ ...WAITING, ...over }),
    ),
    ...answers,
  );
}

/// Where the human says not to wait for the window.
const RESUME_PATH = `/api/ui/conversations/${WAITING.id}/pause/${OUT_OF_WINDOW.id}/resume`;

describe("a pause on the timeline", () => {
  /// The two facts that decide whether the human does anything about it: which
  /// account ran out, and when it comes back.
  it("names the account that ran out and when the window comes back", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    const waiting = await drawn(container, ".timeline .pause");

    expect(waiting.querySelector(".what")!.textContent).toContain(
      `${OUT_OF_WINDOW.profile} is out of window`,
    );
    expect(waiting.querySelector(".what")!.textContent).toContain(
      "2026-08-03 05:00 UTC",
    );

    // And the backend's own sentence underneath, which is the record of why
    // this was raised.
    expect(waiting.querySelector(".how")!.textContent).toBe(OUT_OF_WINDOW.said);
  });

  /// A display may not carry one. The pause is still the whole record: the wait
  /// is then the human's to end.
  it("says only that the account ran out where no reset time could be read", async () => {
    thePaused({
      timeline: WAITING.timeline.map((entry) =>
        "Pause" in entry
          ? { Pause: { ...entry.Pause, resets_at: null } }
          : entry,
      ),
    });
    const { container } = mount(`/conversations/${WAITING.id}`);

    const waiting = await drawn(container, ".timeline .pause");

    expect(waiting.querySelector(".what")!.textContent).toBe(
      `${OUT_OF_WINDOW.profile} is out of window`,
    );
    expect(waiting.querySelector(".resume")).toBeTruthy();
  });

  /// One press rather than three remedies: Verkstead is not driving anything
  /// here and has nothing to retry, so the only choice is whether to keep
  /// waiting.
  it("offers one press, and says the worktree is untouched", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    const waiting = await drawn(container, ".timeline .pause");

    expect(waiting.querySelector(".resume")!.textContent).toBe(
      "Go on without waiting",
    );
    expect(waiting.querySelectorAll(".remedy")).toHaveLength(0);
    expect(waiting.textContent).toContain(
      "the worktree is left exactly as the session left it",
    );
  });

  it("sends the press with nothing beside it", async () => {
    const fetching = thePaused(
      {},
      whenever(RESUME_PATH, json("Resumed" satisfies PauseResumed), "POST"),
    );
    const { container } = mount(`/conversations/${WAITING.id}`);

    const waiting = await drawn(container, ".timeline .pause");
    fireEvent.click(waiting.querySelector(".resume")!);

    await waitFor(() => expect(sent(fetching, RESUME_PATH)).toEqual({}));

    // And nothing is said about a press that worked: the event reading back
    // resumed is what says it.
    expect(waiting.querySelector(".error")).toBeNull();
  });

  /// The record is what a timeline is: a long run against a busy account
  /// collects one of these a day, each saying how that day's wait ended.
  it("shows what ended the wait once something has, and stops offering", async () => {
    thePaused({
      blocked_on: null,
      timeline: WAITING.timeline.map((entry) =>
        "Pause" in entry
          ? {
              Pause: {
                ...entry.Pause,
                resumed: {
                  by: "Reset" as const,
                  at: "2026-08-03T05:00:04.000Z",
                },
              },
            }
          : entry,
      ),
    });
    const { container } = mount(`/conversations/${WAITING.id}`);

    const waiting = await drawn(container, ".timeline .pause");

    expect(waiting.querySelector(".resumed")!.textContent).toBe(
      "The window came back",
    );
    expect(waiting.querySelector(".resuming")).toBeNull();
    expect(waiting.classList.contains("open")).toBe(false);
  });

  /// The window came back while the page was open, or a second press. Not an
  /// error, and said in words rather than retried.
  it("says so when the wait was already over", async () => {
    thePaused(
      {},
      whenever(
        RESUME_PATH,
        json("AlreadyResumed" satisfies PauseResumed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${WAITING.id}`);

    const waiting = await drawn(container, ".timeline .pause");
    fireEvent.click(waiting.querySelector(".resume")!);

    await waitFor(() =>
      expect(waiting.querySelector(".error")!.textContent).toContain(
        "The first ending stands",
      ),
    );
  });

  /// A paused run is a run that has stopped, so it carries the same badge an
  /// interruption does — and the badge stays put, because what there is to press
  /// is drawn whole in the list and there is no pane behind it.
  it("carries blocked on you, and the badge opens no pane", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    const badge = await drawn<HTMLButtonElement>(container, ".blocked");
    expect(badge.textContent).toBe("Blocked on you");

    fireEvent.click(badge);

    const waiting = await drawn(container, ".timeline .pause.selected");
    expect(waiting).toBeTruthy();
    expect(frame(container).dataset.pane).toBe("timeline");
  });
});

describe("a conversation blocked on the human", () => {
  it("says so where the conversation is named", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const badge = await drawn(container, ".pane-head .blocked");

    expect(badge.textContent).toBe("Blocked on you");
    expect(STOPPED.blocked_on).toBe(SAID.id);
  });

  it("draws no badge where nothing is stopping the work", async () => {
    expect(OPEN.blocked_on).toBeNull();

    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, ".timeline");

    expect(container.querySelector(".blocked")).toBeNull();
  });
});

/// The sixth shape the middle pane draws: a conversation whose backlog is
/// worked through, with the pull request the finish step opened pinned where
/// the task list was.
const WRAPPING = wrapping as ConversationView;

/// The pull request itself, off that payload.
const OPENED = (() => {
  const pinned = WRAPPING.pinned[0];
  if (!pinned || !("PullRequest" in pinned)) {
    throw new Error("the fixture should carry a pinned pull request");
  }
  return pinned.PullRequest;
})();

/// What is on it, which the details pane fetches from the server, which asks
/// GitHub through the host's `gh`.
const CARRIED: PullRequestDetails = {
  commits: [
    {
      sha: "d41f8a3b6c2e91750f4a8c3d5b7e2f10a9c6d4b8",
      subject: "chore: finish rate-limiting",
    },
    {
      sha: "5c2a9e14b7f36d80a1c4e9b2f7d53081a6e4c9b2",
      subject: "feat: count the requests",
    },
  ],
  comments: [
    {
      author: "tobico",
      at: "2026-08-21T09:00:00.000Z",
      html: "<p>The counter wants a <strong>test</strong>.</p>",
    },
  ],
};

/// Where the pane fetches it from.
const WHAT_IS_ON_IT = `/api/ui/conversations/${WRAPPING.id}/pull-request/${OPENED.id}`;

/// The workbench with that conversation open and its pull request to hand.
function theWrapping(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${WRAPPING.id}`,
      json({ ...WRAPPING, ...over }),
    ),
    whenever(WHAT_IS_ON_IT, json(CARRIED)),
    ...answers,
  );
}

describe("the pinned pull request", () => {
  it("says what it is called and what number it answers to", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, ".pinned .pull-request");

    expect(opened.textContent).toContain("Pull request");
    expect(opened.querySelector(".number")!.textContent).toBe(
      `#${OPENED.number}`,
    );
    expect(opened.querySelector(".open-pull-request")!.textContent).toBe(
      OPENED.title,
    );
  });

  /// Merging is the human's act and it happens over there, so getting there is
  /// a link rather than anything this page's panes have to answer for.
  it("links out to GitHub itself", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const out = await drawn<HTMLAnchorElement>(
      container,
      ".pinned .pull-request .out",
    );

    expect(out.href).toBe(OPENED.url);
  });

  /// Pinned is a thing an event is: it is drawn outside the record, and the
  /// move into wrapping is what says on the record that it arrived.
  it("is drawn above the record rather than in it", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, ".pinned .pull-request");

    expect(pinned.closest(".timeline")).toBeNull();
    expect(container.querySelector(".timeline .pull-request")).toBeNull();

    const moves = [...container.querySelectorAll(".timeline .moved")].map(
      (line) => line.textContent,
    );
    expect(moves.at(-1)).toBe("Implementing → Wrapping");
  });

  it("shows what is on it in the details pane, fetched rather than remembered", async () => {
    const fetching = theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, ".pinned .pull-request");
    fireEvent.click(opened.querySelector(".open-pull-request")!);

    const commits = await drawn(container, ".details-pane .pr-commits");

    expect(
      [...commits.querySelectorAll(".commits li")].map((row) => [
        row.querySelector(".sha")!.textContent,
        row.querySelector(".subject")!.textContent,
      ]),
    ).toEqual(CARRIED.commits.map((it) => [it.sha.slice(0, 7), it.subject]));

    const comments = await drawn(container, ".details-pane .pr-comments");

    expect(comments.querySelector(".author")!.textContent).toBe(
      CARRIED.comments[0]!.author,
    );
    // Put in the page as it arrived: a comment is markdown from the public
    // internet, and the server is what rendered and sanitized it.
    expect(comments.querySelector(".markdown")!.innerHTML).toBe(
      CARRIED.comments[0]!.html,
    );

    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBeGreaterThan(0);
  });

  /// Every way `gh` cannot answer is a different afternoon for the human, so
  /// the pane says the server's own wording rather than "could not load".
  it("shows the server's wording when gh cannot answer", async () => {
    theWrapping(
      {},
      whenever(WHAT_IS_ON_IT, () =>
        Promise.resolve(
          new Response(
            JSON.stringify({
              error:
                "this machine's `gh` is not logged in, so Verkstead cannot ask GitHub anything",
            }),
            { status: 502, headers: { "content-type": "application/json" } },
          ),
        ),
      ),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, ".pinned .pull-request");
    fireEvent.click(opened.querySelector(".open-pull-request")!);

    const error = await drawn(container, ".details-pane .error");

    expect(error.textContent).toContain("is not logged in");
  });

  /// Nothing is fetched until somebody opens it: reading this is an API call
  /// GitHub answers, and the conversation around it is read again on every
  /// Nudge about it.
  it("asks GitHub nothing until it is opened", async () => {
    const fetching = theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    await drawn(container, ".pinned .pull-request");

    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBe(0);
  });

/// A conversation with all three kinds pinned at once: the backlog it was built
/// from, the roadmap the branch wrote to, and the pull request it ended on.
///
/// Composed rather than a fixture of its own, because what is being read here is
/// how the timeline draws several pinned cards, and each of the three is already
/// a golden fixture the server wrote.
const ALL_THREE = [
  { TaskList: BACKLOG },
  { StageList: ROADMAP },
  { PullRequest: OPENED },
];

/// A finger going down or coming up at a place across the card.
///
/// Built by hand rather than through `fireEvent`, because jsdom has no
/// `TouchEvent` to build one with: what the handler reads is `changedTouches`,
/// so that is what this carries. It bubbles, which is how the handler on the
/// card hears an event dispatched on the card.
function touching(kind: string, clientX: number): Event {
  const event = new Event(kind, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "changedTouches", { value: [{ clientX }] });
  return event;
}

/// A finger dragged across the showing card, from one place to another.
function swipe(card: Element, from: number, to: number) {
  card.dispatchEvent(touching("touchstart", from));
  card.dispatchEvent(touching("touchend", to));
}

describe("the pinned carousel", () => {
  it("shows one of several pinned cards at a time", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, ".pinned");

    expect(
      pinned.querySelectorAll(".task-list, .stage-list, .pull-request"),
    ).toHaveLength(1);
    expect(pinned.querySelector(".task-list")).not.toBeNull();
  });

  /// The dots are the whole of what the carousel says about itself: how many
  /// there are, and which one of them is being read. Each is named for the card
  /// it turns to, so a reader who cannot see them is told the same thing.
  it("counts them beneath the card and marks the one showing", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const dots = await drawn(container, ".pinned .carousel > .dots");
    const buttons = [...dots.querySelectorAll("button")];

    expect(buttons.map((dot) => dot.getAttribute("aria-label"))).toEqual([
      "Task list",
      "Roadmap",
      "Pull request",
    ]);
    expect(buttons.map((dot) => dot.getAttribute("aria-current"))).toEqual([
      "true",
      null,
      null,
    ]);
  });

  it("turns to any of them when its dot is pressed", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const dots = await drawn(container, ".pinned .carousel > .dots");
    fireEvent.click(dots.querySelectorAll("button")[2]!);

    await waitFor(() =>
      expect(container.querySelector(".pinned .pull-request")).not.toBeNull(),
    );
    expect(container.querySelector(".pinned .task-list")).toBeNull();
    expect(
      dots.querySelectorAll("button")[2]!.getAttribute("aria-current"),
    ).toBe("true");
  });

  /// The arrows count round both ends: with three cards, one that stopped at
  /// the end would be a dead control most of the time.
  it("steps between them with the arrows, and counts round the ends", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const carousel = await drawn(container, ".pinned .carousel");

    fireEvent.click(carousel.querySelector(".step.on")!);
    await waitFor(() =>
      expect(carousel.querySelector(".stage-list")).not.toBeNull(),
    );

    // Back past the front, which is the far end of the list.
    fireEvent.click(carousel.querySelector(".step.back")!);
    fireEvent.click(carousel.querySelector(".step.back")!);
    await waitFor(() =>
      expect(carousel.querySelector(".pull-request")).not.toBeNull(),
    );
  });

  /// Where there is no pointer to reach an arrow with there are no arrows: the
  /// swipe is what they are, and two buttons lying over the card would be two
  /// buttons in the way of it.
  it("keeps the arrows for pointer devices", async () => {
    expect(stylesheet).toContain(
      ".pinned .carousel > .step {\n  display: none;\n}",
    );
    expect(stylesheet).toContain(
      "@media (hover: hover) {\n  .pinned .carousel > .step {\n    display: grid;",
    );
  });

  it("turns the card on a swipe across it", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const showing = await drawn(container, ".pinned .carousel > .showing");

    // Leftwards is onwards, the way a page turns.
    swipe(showing, 200, 200 - SWIPE);
    await waitFor(() =>
      expect(showing.querySelector(".stage-list")).not.toBeNull(),
    );

    swipe(showing, 200, 200 + SWIPE);
    await waitFor(() =>
      expect(showing.querySelector(".task-list")).not.toBeNull(),
    );

    // A press that slid a little is still a press, and turns nothing.
    swipe(showing, 200, 200 - (SWIPE - 1));
    expect(showing.querySelector(".task-list")).not.toBeNull();
  });

  /// Which card the reader is put in front of: the one the work has stopped on,
  /// which is what they opened the conversation to deal with.
  it("fronts the card the work is blocked on", async () => {
    theWrapping({ pinned: ALL_THREE, blocked_on: OPENED.id });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, ".pinned");

    expect(pinned.querySelector(".pull-request")).not.toBeNull();
    expect(pinned.querySelector(".task-list")).toBeNull();
  });

  /// And with nothing stopping it, the fixed order — which is the order the
  /// server hands them over in, and the order the work goes through them in.
  it("otherwise fronts the first, which is the task list", async () => {
    expect(WRAPPING.blocked_on).toBeNull();

    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, ".pinned");

    expect(pinned.querySelector(".task-list")).not.toBeNull();
  });

  /// And with no backlog to be first, the roadmap — the order is the server's,
  /// which is the order the work goes through them in.
  it("fronts the roadmap where there is no backlog before it", async () => {
    theWrapping({ pinned: ALL_THREE.slice(1) });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, ".pinned");

    expect(pinned.querySelector(".stage-list")).not.toBeNull();
    expect(pinned.querySelector(".pull-request")).toBeNull();
  });

  /// One pinned card is not a carousel: there is nothing to turn to, and dots
  /// counting to one would be furniture around a card nothing can be done with.
  it("draws no carousel at all around a single pinned card", async () => {
    expect(WRAPPING.pinned).toHaveLength(1);

    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, ".pinned .pull-request");

    expect(pinned.closest(".carousel")).toBeNull();
    expect(container.querySelector(".pinned .dots")).toBeNull();
    expect(container.querySelector(".pinned .step")).toBeNull();
  });

  /// The card that is showing keeps everything a pinned card ever had: the
  /// sticky block it travels with, and — for the pull request — the details
  /// pane it opens.
  it("keeps the showing card's place and its behaviour", async () => {
    const fetching = theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const carousel = await drawn(container, ".pinned .carousel");
    expect(carousel.closest(".pane-chrome")).not.toBeNull();
    expect(carousel.closest(".timeline")).toBeNull();

    const dots = carousel.querySelector(".dots")!;
    fireEvent.click(dots.querySelectorAll("button")[2]!);

    const opened = await drawn(container, ".pinned .pull-request");
    fireEvent.click(opened.querySelector(".open-pull-request")!);

    await drawn(container, ".details-pane .pr-commits");
    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBeGreaterThan(0);
  });
});
});

/// What the human asked for by hand, which the same conversation carries on the
/// end of its record: a manual task, set going outside the pipeline.
const ASKED_BY_HAND = (() => {
  const event = WRAPPING.timeline.find((entry) => "ManualTask" in entry);
  if (!event || !("ManualTask" in event)) {
    throw new Error("the fixture should carry a manual task");
  }
  return event.ManualTask;
})();

describe("a manual task", () => {
  /// A card in the record, like the brief and the handoff: it is a document
  /// somebody wrote, and the words are the whole of it.
  it("draws what was asked for as a card in the record", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, ".timeline-event > .manual-task");

    expect(asked.querySelector(".event-head")!.textContent).toContain(
      "Manual task",
    );
    expect(asked.closest(".timeline")).not.toBeNull();
  });

  /// Put in the page as the server rendered it, like every other piece of
  /// markdown on this wire — so what the human set in backticks reads as code
  /// rather than as backticks.
  it("shows the instruction as the server rendered it", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, ".timeline-event > .manual-task");

    expect(asked.querySelector(".markdown")!.innerHTML).toBe(
      ASKED_BY_HAND.html,
    );
    expect(asked.querySelector("code")!.textContent).toBe("main");
  });

  /// Read-only: what its session went on to do arrives as the events any work
  /// arrives as, under this one. Opening it is not answering it — the card is a
  /// way into the whole instruction and nothing more.
  it("asks the human for nothing", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, ".timeline-event > .manual-task");

    expect(asked.querySelectorAll("button")).toHaveLength(0);
    expect(asked.querySelectorAll("textarea")).toHaveLength(0);
  });
});

/// Where a resume is pressed.
const RESUMING = `/api/ui/conversations/${WRAPPING.id}/resume`;

describe("the resume button", () => {
  /// Drawn on the server's word alone. What drives a conversation is a register
  /// of running tasks, which lives in the server — a page working it out from
  /// the state and the session it can see would be a second opinion about a
  /// question only one side can answer.
  it("is drawn where nothing is driving the conversation", async () => {
    theWrapping({ ready_to_resume: true });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const resume = await drawn(container, ".resume");

    expect(resume.querySelector(".resume-conversation")!.textContent).toContain(
      "Resume",
    );
  });

  /// And gone where something is. There is nothing to start again, and a button
  /// offering to would be one that could only refuse.
  it("goes where something is driving it already", async () => {
    theWrapping({ ready_to_resume: false });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    await drawn(container, ".timeline");

    expect(container.querySelector(".resume")).toBeNull();
  });

  /// Nothing goes with the press. What should be running is recomputed from
  /// where the work now stands, which is the whole reason there is one button
  /// rather than a choice of them.
  it("sends the press with nothing on it", async () => {
    const fetching = theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json("Resumed" as Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const resume = await drawn(container, ".resume");
    fireEvent.click(resume.querySelector(".resume-conversation")!);

    await waitFor(() => expect(sent(fetching, RESUMING)).toEqual({}));
  });

  /// A press that found nothing to start says so where it was pressed. This is
  /// the whole of what resume is for: a conversation nothing is driving, and
  /// the reason nothing is.
  it("says in words that there was nothing to start", async () => {
    theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json("NothingToWork" as Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const resume = await drawn(container, ".resume");
    fireEvent.click(resume.querySelector(".resume-conversation")!);

    const refused = await drawn(container, ".resume .error");

    expect(refused.textContent).toBe(RESUME_REFUSAL.NothingToWork);
    expect(refused.textContent).toContain("no backlog left");
  });

  /// And a second press on a conversation the first one got going is refused as
  /// driven, which is the same press arriving twice rather than a mistake.
  it("says in words that something is driving it now", async () => {
    theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json("AlreadyDriven" as Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const resume = await drawn(container, ".resume");
    fireEvent.click(resume.querySelector(".resume-conversation")!);

    const refused = await drawn(container, ".resume .error");

    expect(refused.textContent).toBe(RESUME_REFUSAL.AlreadyDriven);
  });
});

/// Where a manual task is submitted from.
const SET_GOING = `/api/ui/conversations/${WRAPPING.id}/manual-task`;

/// The conversation's own implementation pairing, which is what the composer
/// starts on.
const IMPLEMENTATION = WRAPPING.implementation_pairing!;

/// And a pairing of another saved profile, which is what a one-off pick picks.
const OTHER = PROFILES.find(
  (profile) => profile.id !== IMPLEMENTATION.profile.id,
)!;

/// One row of a pairing picker, as the picker writes its value.
const running = (profile: ProfileEntry, model: string) =>
  `${profile.id}:${model}`;

describe("the manual task composer", () => {
  /// Offered wherever nothing is running: this conversation is wrapping up, and
  /// a wrapping lull is exactly the quiet moment it is there for.
  it("is drawn at the end of the timeline when nothing is running", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    await drawn(container, ".manual-task-composer select");
    const composer = container.querySelector(".manual-task-composer")!;

    expect(composer.querySelector("textarea")).toBeTruthy();
    expect(
      composer.querySelector(".start-manual-task")!.textContent,
    ).toContain("Set it going");
    expect(
      composer.compareDocumentPosition(container.querySelector(".timeline")!) &
        Node.DOCUMENT_POSITION_PRECEDING,
    ).toBeTruthy();
  });

  /// And gone while one runs. The rule is the register and nothing else: an
  /// agent is in the worktree, and a second one would be two editing each
  /// other's files.
  it("goes while a session is running", async () => {
    theWrapping({ working: true });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    await drawn(container, ".timeline");

    expect(container.querySelector(".manual-task-composer")).toBeNull();
  });

  /// Drafting is one of the two states with no worktree, so there is nowhere
  /// for a session to run and nothing to offer.
  it("is not drawn on a conversation with no worktree", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, ".start-grilling");

    expect(container.querySelector(".manual-task-composer")).toBeNull();
  });

  /// The dropdown starts on the conversation's implementation pairing, because
  /// that is what its work runs under — but it is a start rather than a rule.
  /// What it offers is every profile-and-model combination, one flat row each.
  it("starts on the conversation's implementation pairing", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const picker = await drawn<HTMLSelectElement>(
      container,
      ".manual-task-composer select",
    );

    expect(picker.value).toBe(
      running(IMPLEMENTATION.profile, IMPLEMENTATION.model!),
    );
    expect([...picker.options].map((option) => option.textContent)).toEqual(
      PROFILES.flatMap((profile) =>
        profile.models.map((model) => `${profile.name} — ${model}`),
      ),
    );
  });

  /// The instruction and the pairing, on the wire together. One press does both:
  /// what it says and what it runs as are the whole of a manual task.
  it("sends what was typed and the pairing picked beside it", async () => {
    const fetching = theWrapping(
      {},
      whenever(SET_GOING, json("Started" as ManualTaskStarted), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const picker = await drawn<HTMLSelectElement>(
      container,
      ".manual-task-composer select",
    );
    const composer = container.querySelector(".manual-task-composer")!;
    fireEvent.input(composer.querySelector("textarea")!, {
      target: { value: "Rebase onto main." },
    });
    fireEvent.change(picker, {
      target: { value: running(OTHER, OTHER.models[0]!) },
    });
    fireEvent.click(composer.querySelector(".start-manual-task")!);

    await waitFor(() =>
      expect(sent(fetching, SET_GOING)).toEqual({
        instruction: "Rebase onto main.",
        profile_id: OTHER.id,
        model: OTHER.models[0],
      }),
    );

    // The pick is one-off: it is what this task runs as, and nothing writes it
    // back to the conversation.
    expect(
      askedFor(
        fetching,
        `/api/ui/conversations/${WRAPPING.id}/implementation-pairing`,
      ),
    ).toBe(0);
  });

  /// Emptied on the way out, because the instruction is on the timeline now:
  /// what is left in the box would otherwise read as something still to ask for.
  it("empties the box once the task is going", async () => {
    theWrapping({}, whenever(SET_GOING, json("Started"), "POST"));
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const composer = await drawn(container, ".manual-task-composer");
    const typing = composer.querySelector<HTMLTextAreaElement>("textarea")!;
    fireEvent.input(typing, { target: { value: "Rebase onto main." } });
    fireEvent.click(composer.querySelector(".start-manual-task")!);

    await waitFor(() => expect(typing.value).toBe(""));
  });

  /// Nothing typed is nothing to ask for, and the button says so by not being
  /// pressable rather than by refusing afterwards.
  it("will not submit an empty instruction", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const composer = await drawn(container, ".manual-task-composer");

    expect(
      composer.querySelector<HTMLButtonElement>(".start-manual-task")!.disabled,
    ).toBe(true);
  });

  /// A submit that raced a session loses, and the page says which of the named
  /// refusals it was: the composer that was pressed was drawn a moment ago.
  it("says in words that an agent was already running", async () => {
    theWrapping(
      {},
      whenever(SET_GOING, json("AlreadyRunning" as ManualTaskStarted), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const composer = await drawn(container, ".manual-task-composer");
    fireEvent.input(composer.querySelector("textarea")!, {
      target: { value: "Rebase onto main." },
    });
    fireEvent.click(composer.querySelector(".start-manual-task")!);

    const refused = await drawn(container, ".manual-task-composer .error");

    expect(refused.textContent).toBe(MANUAL_TASK_REFUSAL.AlreadyRunning);
    expect(refused.textContent).toContain("already running");
  });

  /// Every named refusal has words of its own, because each of them is
  /// something different to go and do about it.
  it("has a sentence for every way of being refused", () => {
    for (const [outcome, said] of Object.entries(MANUAL_TASK_REFUSAL)) {
      if (outcome === "Started") {
        continue;
      }
      expect(said, `${outcome} should say something`).not.toBe("");
    }
  });
});

/// The three documents on a timeline: the frozen brief, the handoff the grilling
/// wrote, and the instruction a manual task was set going with.
///
/// Each of them is as long as whoever wrote it made it, so the card shows the
/// first five lines under a fade and the whole of it is a press away. Where the
/// fifth line falls is a fact about a laid-out box and jsdom has no layout, so
/// the clamp itself is asserted off the stylesheet — the way a drawn diagram's
/// rules are — and what is asked here is that each document is put inside it and
/// that pressing the card opens the whole.
describe("the documents on a timeline", () => {
  /// The details pane, and what it has drawn.
  const details = () => screen.getByLabelText("Details");

  it("puts the frozen brief in a clamp, and opens the whole of it", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const brief = await drawn(container, ".timeline-event > .brief");

    expect(brief.querySelector(".clamp > .brief-body")).toBeTruthy();

    fireEvent.click(brief);

    const opened = await drawn(details(), ".document");

    expect(details().querySelector("h1")!.textContent).toBe("Brief");
    // The whole of it, and not inside a clamp: the pane is where a document
    // that would not fit on a card is read.
    expect(opened.innerHTML).toBe(briefOf(GRILLING).html);
    expect(details().querySelector(".clamp")).toBeNull();
  });

  it("puts the handoff in a clamp, and opens the whole of it", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const handoff = await drawn(container, ".timeline-event > .handoff");

    expect(handoff.querySelector(".clamp > .handoff-body")).toBeTruthy();

    fireEvent.click(handoff);

    const opened = await drawn(details(), ".document");

    expect(details().querySelector("h1")!.textContent).toBe("Handoff");
    expect(opened.innerHTML).toBe(HANDOFF.html);
    expect(details().querySelector(".clamp")).toBeNull();
  });

  it("puts a manual task's instruction in a clamp, and opens the whole of it", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, ".timeline-event > .manual-task");

    expect(asked.querySelector(".clamp > .manual-task-body")).toBeTruthy();

    fireEvent.click(asked);

    const opened = await drawn(details(), ".document");

    expect(details().querySelector("h1")!.textContent).toBe("Manual task");
    expect(opened.innerHTML).toBe(ASKED_BY_HAND.html);
    expect(details().querySelector(".clamp")).toBeNull();
  });

  /// The same affordance the events that are buttons have, said on an article
  /// because rendered markdown cannot live inside a button: the role, the
  /// keyboard, and the selection drawn on the card that is open.
  it("presses like a button, and says which card is open", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const handoff = await drawn(container, ".timeline-event > .handoff");

    expect(handoff.getAttribute("role")).toBe("button");
    expect(handoff.getAttribute("tabindex")).toBe("0");
    expect(handoff.getAttribute("aria-pressed")).toBe("false");
    expect(handoff.classList.contains("openable")).toBe(true);

    fireEvent.keyDown(handoff, { key: "Enter" });

    await drawn(details(), ".document");

    await waitFor(() =>
      expect(handoff.classList.contains("selected")).toBe(true),
    );
    expect(handoff.getAttribute("aria-pressed")).toBe("true");
  });

  /// A short document opens too. One affordance whether or not the fade is
  /// drawn, because a card the human has to judge the length of before pressing
  /// is a card they will not press.
  it("opens a document too short to be cut off", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, ".timeline-event > .manual-task");

    // One paragraph, which is nowhere near the clamp.
    expect(ASKED_BY_HAND.html.split("\n").length).toBeLessThan(5);
    expect(asked.getAttribute("role")).toBe("button");

    fireEvent.click(asked);

    await drawn(details(), ".document");
  });

  /// The brief while it is still a draft is a field with the setup under it, and
  /// every press on that card belongs to one of those.
  it("leaves the drafting brief a field, unclamped and unpressable", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const brief = await drawn(container, ".timeline-event > .brief");
    await drawn(container, ".brief .grow textarea");

    expect(brief.querySelector(".clamp")).toBeNull();
    expect(brief.getAttribute("role")).toBeNull();
    expect(brief.getAttribute("tabindex")).toBeNull();
    expect(brief.classList.contains("openable")).toBe(false);

    fireEvent.click(brief);

    expect(details().querySelector(".document")).toBeNull();
  });

  /// A notice is a sentence rather than a document: one line already, with
  /// nothing to cut off and nothing to open.
  it("leaves a notice line whole", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const notice = await drawn(container, ".timeline-event > .notice");

    expect(notice.querySelector(".clamp")).toBeNull();
    expect(notice.getAttribute("role")).toBeNull();
  });
});

/// The clamp itself, which is the stylesheet's: how tall a card's document is
/// allowed to be, and where the fade over the cut comes from.
describe("a clamped document", () => {
  /// The declarations of the block `selector` opens — the same reading the
  /// diagram rules are asserted by, and for the same reason: jsdom has no
  /// layout, so a rule about one is read rather than measured.
  function block(selector: string): string {
    const opened = stylesheet.indexOf(`${selector} {`);
    expect(opened, `the stylesheet should have a \`${selector}\` rule`).not.toBe(-1);

    return stylesheet.slice(opened, stylesheet.indexOf("}", opened));
  }

  /// The page is set at a line height of 1.5, which is what turns a count of
  /// lines into a height. The two halves are written down in different languages
  /// and this is where they are held to each other.
  const LINE_HEIGHT = 1.5;

  it("shows five lines of it and hides the rest", () => {
    const clamp = block(".clamp");

    expect(CLAMPED_LINES).toBe(5);
    expect(clamp).toContain(`max-height: ${CLAMPED_LINES * LINE_HEIGHT}em`);
    expect(clamp).toContain("overflow: hidden");

    // And that line height is the body's, rather than a number this test made
    // up: a page set looser or tighter would clamp at a different height.
    expect(stylesheet).toContain(`font: 16px/${LINE_HEIGHT} system-ui`);
  });

  it("fades the cut into the card, and only where there is a cut", () => {
    const cut = block(".clamp.cut::after");

    expect(cut).toContain("linear-gradient(to bottom, transparent, var(--card))");
    // The fade must not swallow the press: the whole card opens the pane.
    expect(cut).toContain("pointer-events: none");

    // On `.cut` and nowhere else, which is what makes a short document show
    // whole with no fade over its last line.
    expect(stylesheet).not.toContain(".clamp::after");
  });
});
