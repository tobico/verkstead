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
  BacklogPane,
  BriefEvent,
  Capture,
  CommitPane,
  ConversationArchived,
  ConversationClosed,
  ConversationEntry,
  ConversationSteered,
  ConversationStopped,
  ConversationUnarchived,
  ConversationView,
  GrillingStarted,
  ProfileEntry,
  PullRequestDetails,
  Resumed,
  Screen,
  Shown,
  ShowingArchived,
  SteerOpened,
  Submitted,
  TimelineEvent,
  TranscriptView,
  Turn,
} from "../src/api/types";
// The one menu, which all three of this page's ⋯ and dropdowns are drawn as.
import app from "../src/App.module.css";
import dropdown from "../src/Menu.module.css";
import notices from "../src/notices.module.css";
// The set page as it is drawn inside a details pane: its nav, its sections, and
// the record of a Set this build cannot read.
import contents from "../src/set/Contents.module.css";
import sheet from "../src/set/Sheet.module.css";
import illegible from "../src/set/Unreadable.module.css";
// The element defaults, which is where the page's own line height is set.
import base from "../src/styles/base.css?raw";
// What can be done to a Conversation as a whole, both ways: the hashed names
// the menu's rows are queried by, and the words its refusals are said in.
import { STOP_REFUSAL } from "../src/workbench/Actions";
import actions from "../src/workbench/Actions.module.css";
import { ADOPT_REFUSAL } from "../src/workbench/Adoption";
import adoption from "../src/workbench/Adoption.module.css";
// The detail panes, each a module of its own: a commit, a document read whole,
// one session's record and the terminal it was printed on, and a pull request.
import backlogPane from "../src/workbench/Backlog.module.css";
import backlogCss from "../src/workbench/Backlog.module.css?raw";
import commitPane from "../src/workbench/Commit.module.css";
// The Diff section the commit pane draws, which is the Set page's own component
// and so the Set page's own module.
import diffSection from "../src/set/Diff.module.css";
// The sidebar, both ways: the hashed names its rows are queried by, and the
// source of the rules that say what a card's state looks like.
import sidebar from "../src/workbench/Conversations.module.css";
import sidebarCss from "../src/workbench/Conversations.module.css?raw";
import documentPane from "../src/workbench/Document.module.css";
// The ring a running session is marked by, wherever it is drawn.
import marks from "../src/workbench/Mark.module.css";
import marksCss from "../src/workbench/Mark.module.css?raw";
import outputPane from "../src/workbench/Output.module.css";
import outputCss from "../src/workbench/Output.module.css?raw";
// The pane chrome, both ways: the hashed names to query the page by, and the
// source to read the rules that jsdom lays nothing out for.
import paneHead from "../src/workbench/PaneHead.module.css";
import paneHeadCss from "../src/workbench/PaneHead.module.css?raw";
// The pause card, which is one of the record's and draws itself.
import prPane from "../src/workbench/PullRequest.module.css";
// The Screen, both ways: it is the one pane with a height of its own, and the
// rules that give it one are what jsdom cannot lay out.
import screenPane from "../src/workbench/Screen.module.css";
import screenCss from "../src/workbench/Screen.module.css?raw";
// And the timeline, both ways again: it is the biggest of these, and a good
// deal of what it says about a card is a rule rather than an element.
// What is still the human's to settle on the brief card.
import setup from "../src/workbench/Setup.module.css";
import steerModal from "../src/workbench/Steer.module.css";
import timeline from "../src/workbench/Timeline.module.css";
import timelineCss from "../src/workbench/Timeline.module.css?raw";
// And the frame the three panes stand in, both ways: it holds the layout rules
// jsdom lays nothing out for, and the pane names everything else is found by.
import shell from "../src/workbench/Workbench.module.css";
import shellCss from "../src/workbench/Workbench.module.css?raw";
import { CLAMPED_LINES, RESUME_REFUSAL, SWIPE } from "../src/workbench/Timeline";
import { STEER_REFUSAL } from "../src/workbench/Steer";
import {
  BRANCHES,
  HIDING_ARCHIVED,
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
import stopped from "./fixtures/conversation-stopped.json" with { type: "json" };
import paused from "./fixtures/conversation-paused.json" with { type: "json" };
import secondRound from "./fixtures/conversation-second-round.json" with {
  type: "json",
};
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
  return container.querySelector(`.${shell.workbench}`)!;
}

/// Open the conversation's action menu: press the trigger, and wait for what it
/// drops.
async function openActions(container: ParentNode): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, `.${actions.conversationActions} > .${dropdown.trigger}`));
  return drawn(container, `.${actions.conversationActions} > .${dropdown.drop}`);
}

/// Open the sidebar's ⋯, which is what the rest of Verkstead is behind: press
/// the trigger, and wait for what it drops.
async function openWorkbenchActions(
  container: ParentNode,
): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, `.${sidebar.workbenchActions} > .${dropdown.trigger}`));
  return drawn(container, `.${sidebar.workbenchActions} > .${dropdown.drop}`);
}

/// Drop the new-conversation menu, which is where both ways of starting one
/// live: press the button, and wait for what it drops.
async function openNewConversation(
  container: ParentNode,
): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, `.${sidebar.newConversation} > .${dropdown.trigger}`));
  return drawn(container, `.${sidebar.newConversation} > .${dropdown.drop}`);
}

/// The repos in that menu, in the order they are offered — waited for, because
/// the menu opens whether or not the list has arrived.
async function repoRows(container: ParentNode): Promise<HTMLButtonElement[]> {
  await drawn(container, `.${dropdown.drop} > [role="menuitem"]`);
  return [...container.querySelectorAll<HTMLButtonElement>(`.${dropdown.drop} > [role="menuitem"]`)];
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

    const heading = await drawn(container, `.${shell.conversationsPane} h1`);

    expect(heading.textContent).toContain("Verkstead");
    expect(heading.textContent).not.toContain("Conversations");

    // Cut from the one artwork, served from `assets/` at the site root — the
    // same file the manifest names, rather than a copy of it under `web/`.
    const icon = heading.querySelector("img")!;
    expect(icon.getAttribute("src")).toBe("/icons/icon-192.png");

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
      [...container.querySelectorAll(`.${sidebar.conversationRow} .${sidebar.title}`)].map(
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

    // The line of provenance under the name, which holds the repo and nothing
    // else yet.
    expect(row.querySelector(`.${sidebar.meta}`)!.textContent).toBe(
      DRAFTING.repo,
    );

    // And nothing about where it has got to, in words: that is drawn now — see
    // *how a card says where its conversation has got to*.
    expect(row.textContent).not.toContain(DRAFTING.state);
  });

  it("says so plainly when nothing is being worked on", async () => {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

  /// And the one row of that menu that is about this list rather than about the
  /// rest of Verkstead: whether the conversations put away are drawn in it.
  ///
  /// The server's answer rather than this device's, which is what makes it the
  /// same list on a phone opened afterwards.
  it("draws the archived toggle where the server has it", async () => {
    theWorkbench(
      whenever(
        "/api/ui/conversations/archived",
        json({ showing: true } satisfies ShowingArchived),
      ),
    );
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    await openWorkbenchActions(container);

    const toggle = await waitFor(() =>
      screen.getByLabelText<HTMLInputElement>("Show archived conversations"),
    );
    expect(toggle.checked).toBe(true);
  });

  /// A switch says where it stands rather than asking for a flip, so what goes
  /// out is the position the human has just put it in.
  it("sends the position the switch was put in", async () => {
    const fetching = theWorkbench(
      whenever(
        "/api/ui/conversations/archived",
        json(undefined, 204),
        "POST",
      ),
    );
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    await openWorkbenchActions(container);
    const toggle = await waitFor(() =>
      screen.getByLabelText<HTMLInputElement>("Show archived conversations"),
    );
    fireEvent.click(toggle);

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations/archived")).toEqual({
        showing: true,
      }),
    );
  });

  /// The list is drawn from the setting, so flipping it reads the list again —
  /// which is where the archived conversations come from and go back to.
  it("reads the list again once the setting has been saved", async () => {
    const fetching = theWorkbench(
      whenever(
        "/api/ui/conversations/archived",
        json(undefined, 204),
        "POST",
      ),
    );
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    await openWorkbenchActions(container);
    const read = askedFor(fetching, "/api/ui/conversations");
    fireEvent.click(
      await waitFor(() => screen.getByLabelText("Show archived conversations")),
    );

    await waitFor(() =>
      expect(askedFor(fetching, "/api/ui/conversations")).toBeGreaterThan(read),
    );
  });

  /// Nothing of it until it is pressed, which is the point of putting it there:
  /// the head of the pane gives up a mark's worth of room and no more.
  it("keeps the way out of the workbench behind that menu", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    expect(
      container.querySelector(`.${sidebar.workbenchActions} > .${dropdown.drop}`),
    ).toBeNull();
    // The way to the settings is in the menu and nowhere else, so with the menu
    // shut there is no way out of this page on it at all.
    expect(screen.queryByText("Settings")).toBeNull();
  });

  /// The menu still opens with nothing to start a conversation in — and what is
  /// in it is the page that fixes that, because a menu that opened on nothing
  /// would say only that the button was broken.
  it("says where to go when there is no repo to start one against", async () => {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json([])),
    );
    const { container } = mount();
    await openNewConversation(container);

    await waitFor(() => screen.getByText(/No repos are registered yet/));
    expect(container.querySelector(`.${dropdown.drop} > [role="menuitem"]`)).toBeNull();
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
  await drawn(container, `.${sidebar.conversationRow}`);
  return [...container.querySelectorAll<HTMLElement>(`.${sidebar.conversationRow}`)];
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

    expect(card!.querySelector(`.${marks.mark}.${marks.working}`)).toBeTruthy();
    expect(card!.querySelector(`.${marks.mark}.${marks.waiting}`)).toBeNull();
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

    expect(card!.querySelector(`.${marks.mark}.${marks.idle}`)).toBeTruthy();
    expect(card!.querySelector(`.${marks.mark}.${marks.working}`)).toBeNull();
  });

  /// The disc and nothing beside it: the glyph inside it and the accent border
  /// round the whole card have both gone, because a waiting card that was also
  /// the open one had two treatments arguing over the one edge.
  it("marks a conversation waiting on the human with the disc alone", async () => {
    theSidebar({ state: "Grilling", working: false, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    const disc = card!.querySelector(`.${marks.mark}.${marks.waiting}`);
    expect(disc).toBeTruthy();
    expect(disc!.textContent).toBe("");
    expect(card!.querySelector(`.${marks.mark}.${marks.working}`)).toBeNull();

    // Nothing on the row says it either: the card's border is the ordinary one.
    expect(card!.className).toBe(sidebar.conversationRow);
    expect(sidebarCss, "the waiting card's own border is retired").not.toContain(
      ".conversationRow.waiting",
    );
  });

  /// The mark is a shape, and a shape is nothing to a screen reader beside the
  /// label that already said it.
  it("keeps the mark out of what is read aloud", async () => {
    theSidebar({ state: "Grilling", working: false, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(
      card!.querySelector(`.${marks.mark}.${marks.waiting}`)!.getAttribute("aria-hidden"),
    ).toBe("true");
  });

  /// A Blocking Ask is exactly this: the session that asked is still alive and
  /// idling on the answer. Of the two things true of it, the one the human can do
  /// something about is the ask.
  it("shows the dot and not the spinner when it is both", async () => {
    theSidebar({ state: "Grilling", working: true, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(`.${marks.mark}.${marks.waiting}`)).toBeTruthy();
    expect(card!.querySelector(`.${marks.mark}.${marks.working}`)).toBeNull();
  });

  /// And over the empty one, which is the same case a step further on: the
  /// session that asked has been quiet since it asked. The mark that outranks
  /// both is the one the human can do something about.
  it("shows the dot and not the empty ring when it is both", async () => {
    theSidebar({ state: "Grilling", working: true, idle: true, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(`.${marks.mark}.${marks.waiting}`)).toBeTruthy();
    expect(card!.querySelector(`.${marks.mark}.${marks.idle}`)).toBeNull();
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
    await drawn(container, `.${sidebar.conversationRow} .${marks.mark}.${marks.working}`);

    rows = [{ ...rows[0]!, idle: true }];
    await nudged(client);

    expect(container.querySelector(`.${sidebar.conversationRow} .${marks.mark}.${marks.idle}`)).toBeTruthy();
    expect(container.querySelector(`.${sidebar.conversationRow} .${marks.mark}.${marks.working}`)).toBeNull();

    // And back, on the session speaking again.
    rows = [{ ...rows[0]!, idle: false }];
    await nudged(client);

    expect(
      container.querySelector(`.${sidebar.conversationRow} .${marks.mark}.${marks.working}`),
    ).toBeTruthy();
    expect(container.querySelector(`.${sidebar.conversationRow} .${marks.mark}.${marks.idle}`)).toBeNull();
  });

  it("marks nothing on a conversation that is neither", async () => {
    theSidebar({ state: "Implementing", working: false, waiting: false });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(`.${marks.mark}`)).toBeNull();
  });

  it("draws a draft as a draft, and marks nothing on it", async () => {
    theSidebar({ state: "Draft", working: false, waiting: false });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.classList.contains(sidebar.draft!)).toBe(true);
    expect(card!.querySelector(`.${marks.mark}`)).toBeNull();

    // What "draft" means is the stylesheet's, and jsdom lays nothing out.
    expect(sidebarCss).toContain(
      ".conversationRow.draft .open {\n  border-style: dotted;\n}",
    );
  });

  /// Which of the two it was is the details pane's to say. The sidebar's business
  /// is that there is nothing here to do.
  it("dims finished and closed work identically", async () => {
    theSidebar({ state: "Done" }, { state: "Closed" }, { state: "Wrapping" });
    const { container } = mount();

    expect((await cards(container)).map((card) => card.className)).toEqual([
      `${sidebar.conversationRow} ${sidebar.ended}`,
      `${sidebar.conversationRow} ${sidebar.ended}`,
      sidebar.conversationRow,
    ]);
  });

  /// How far down is the stylesheet's, and jsdom lays nothing out: what these
  /// two say is that a closed card recedes far enough to read as closed, and
  /// that being the open one is the accent border and nothing beside it.
  it("takes a closed card well down, and marks the open one with a border", () => {
    expect(sidebarCss).toContain(
      ".conversationRow.ended .open {\n  opacity: 0.45;\n}",
    );
    expect(sidebarCss).toContain(
      ".conversationRow.selected .open {\n" +
        "  border-color: var(--accent);\n" +
        "  border-style: solid;\n" +
        "  opacity: 1;\n" +
        "}",
    );
    expect(
      sidebarCss,
      "the inset stripe is retired everywhere it was drawn",
    ).not.toContain("box-shadow: inset 0.2rem");
  });

  /// Being the open one is the strongest thing a card's edge has to say, so it
  /// is written last of the border rules and takes back what the two above it
  /// did: the draft's dotted line, and the finished card's fade. What is dimmed
  /// on a finished card that is open is what is inside it.
  it("lets the open card's border outrank every other treatment", () => {
    expect(sidebarCss.indexOf(".conversationRow.selected .open")).toBeGreaterThan(
      sidebarCss.indexOf(".conversationRow.draft .open"),
    );
    expect(sidebarCss.indexOf(".conversationRow.selected .open")).toBeGreaterThan(
      sidebarCss.indexOf(".conversationRow.ended .open"),
    );
    expect(sidebarCss).toContain(
      ".conversationRow.selected.ended .open > * {\n  opacity: 0.45;\n}",
    );
  });

  /// Dimmed and still a row to press: a Done Conversation can be steered.
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

  /// A session that has gone quiet is one the human may have to do something
  /// about, so its ring is drawn in the accent rather than in the edge grey it
  /// used to take — which against a card border read as an edge come loose. The
  /// accent is a themed variable, so it is right in both themes at once.
  it("draws the ring of a quiet session in the accent", () => {
    expect(marksCss).toContain(".mark.idle {\n  border: 1.5px solid var(--accent);\n}");
  });

  /// The spinner is motion, and motion is something to be able to turn off —
  /// everywhere it is drawn, which is every mark on the page rather than the
  /// sidebar's alone.
  it("holds the spinner still where motion is unwelcome", () => {
    expect(marksCss).toContain(
      "@media (prefers-reduced-motion: reduce) {\n" +
        "  .mark.working {\n" +
        "    animation: none;\n" +
        "  }\n" +
        "}",
    );
  });
});

/// The order the sidebar is in, which is the human's own: they drag a card and
/// the whole list goes to the server, so it survives a reload, a restart and a
/// second device without any of the three being a case this page knows about.
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
      (card) => card.querySelector(`.${sidebar.title}`)!.textContent,
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
    const drawn = () => [...list.querySelectorAll<HTMLElement>(`.${sidebar.conversationRow}`)];

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

  /// The card of a row, which is the whole of what a row is a target for now:
  /// the press that opens a Conversation and the press that moves one land on
  /// the same button, and what tells them apart is what the hand does next.
  function card(row: HTMLElement): HTMLElement {
    return row.querySelector<HTMLElement>(`.${sidebar.open}`)!;
  }

  /// Drag one card to a height on the pane with the mouse, and let go. Past the
  /// grace on the way out, since a press that never travels is a click.
  function dragTo(row: HTMLElement, y: number) {
    const open = card(row);

    fireEvent.pointerDown(open, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      clientX: 0,
      clientY: 0,
    });
    fireEvent.pointerMove(open, { pointerId: 1, clientX: 0, clientY: y });
    fireEvent.pointerUp(open, { pointerId: 1 });
  }

  /// A finger put on a card, and the card given long enough to lift under it.
  async function holdOn(row: HTMLElement): Promise<HTMLElement> {
    const open = card(row);

    fireEvent.pointerDown(open, {
      button: 0,
      pointerId: 1,
      pointerType: "touch",
      clientX: 0,
      clientY: 0,
    });

    // Longer than a card takes to lift, which is the whole of what the finger
    // has to do — see `LIFT` in `Conversations.tsx`.
    await new Promise((done) => setTimeout(done, 450));

    return open;
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

  /// A card that could only be dragged would be a control half the people using
  /// it could not reach.
  it("moves a row a step at a time from the keyboard", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    fireEvent.keyDown(card(rows[2]!), { key: "ArrowUp" });

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
    fireEvent.keyDown(card(rows[0]!), { key: "ArrowUp" });

    expect(await order(container)).toEqual(["first", "second", "third"]);
    expect(
      askedFor(fetching, "/api/ui/conversations/order"),
      "there is nowhere to move it to, so there is nothing to save",
    ).toBe(0);
  });

  /// There is no grip beside the card to be dragged, and none to be tabbed to
  /// either: the card is the only control on the row.
  it("carries no second control on the row", async () => {
    three();
    const { container } = mount();

    expect((await cards(container)).map((row) => row.querySelectorAll("button").length))
      .toEqual([1, 1, 1]);
  });

  /// The grip had a label of its own saying which row it moved. Nothing beside
  /// the card carries one now, so the card says which keys move it instead.
  it("says on the card which keys move it", async () => {
    three();
    const { container } = mount();

    expect(
      (await cards(container)).map((row) => card(row).getAttribute("aria-keyshortcuts")),
    ).toEqual(["ArrowUp ArrowDown", "ArrowUp ArrowDown", "ArrowUp ArrowDown"]);
  });

  /// A press that let go about where it landed is a click, whatever the pixel
  /// or two a hand moves between the two.
  it("opens the conversation a press did not move", async () => {
    three();
    const { container, history } = mount();

    const rows = await cards(container);
    laidOut(rows);

    const open = card(rows[2]!);
    fireEvent.pointerDown(open, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      clientX: 0,
      clientY: 0,
    });
    fireEvent.pointerMove(open, { pointerId: 1, clientX: 2, clientY: 2 });
    fireEvent.pointerUp(open, { pointerId: 1 });
    fireEvent.click(open);

    expect(await order(container)).toEqual(["first", "second", "third"]);
    expect(history.get()).toBe("/conversations/3");
  });

  /// And one that moved a card does not also open it: the card is where they
  /// put it, and opening the Conversation as well would answer one gesture
  /// twice.
  it("does not open the conversation a drag moved", async () => {
    three();
    const { container, history } = mount();

    const rows = await cards(container);
    laidOut(rows);

    dragTo(rows[2]!, 10);
    fireEvent.click(card(rows[2]!));

    expect(await order(container)).toEqual(["third", "first", "second"]);
    expect(history.get(), "the drag was the whole of what they said").toBe("/");
  });

  /// A finger drags a card by holding it still first. No distance tells a drag
  /// from a scroll on a phone, so what tells the two apart is the time before
  /// the finger moves.
  it("lifts a card under a finger that holds still", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    const open = await holdOn(rows[2]!);
    fireEvent.pointerMove(open, { pointerId: 1, clientX: 0, clientY: 10 });
    fireEvent.pointerUp(open, { pointerId: 1 });

    expect(await order(container)).toEqual(["third", "first", "second"]);
    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations/order")).toEqual({
        order: [3, 1, 2],
      }),
    );
  });

  /// A finger that travels before its card has lifted is scrolling the list,
  /// and the list is the browser's to scroll.
  it("leaves a swipe to the list", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    const open = card(rows[2]!);
    fireEvent.pointerDown(open, {
      button: 0,
      pointerId: 1,
      pointerType: "touch",
      clientX: 0,
      clientY: 0,
    });
    fireEvent.pointerMove(open, { pointerId: 1, clientX: 0, clientY: 40 });
    fireEvent.pointerUp(open, { pointerId: 1 });

    expect(await order(container)).toEqual(["first", "second", "third"]);
    expect(
      askedFor(fetching, "/api/ui/conversations/order"),
      "nothing was moved, so there is nothing to save",
    ).toBe(0);
  });

  /// Once a card has lifted, though, the list must not scroll out from under
  /// it — and the refusal is over the moment the hand is.
  it("refuses the scroll only while a card is held", async () => {
    three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    const open = await holdOn(rows[2]!);
    expect(scrolled(), "the held card, not the list").toBe(true);

    fireEvent.pointerUp(open, { pointerId: 1 });
    expect(scrolled(), "the list again, as on any other day").toBe(false);
  });

  /// Whether the page took the scroll away from the browser: a touch move at
  /// the document, and whether anything refused it.
  function scrolled(): boolean {
    const swipe = new Event("touchmove", { bubbles: true, cancelable: true });
    document.dispatchEvent(swipe);
    return swipe.defaultPrevented;
  }

  /// A card that refused the scroll before the finger landed would refuse the
  /// swipe that scrolls the list with it, which is why nothing here says
  /// `touch-action` and the drag refuses it from the lift instead.
  it("leaves the scroll to the browser until a card has lifted", () => {
    expect(sidebarCss).not.toContain("touch-action:");
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
    fireEvent.keyDown(card(rows[2]!), { key: "ArrowUp" });

    await waitFor(() =>
      screen.getByText(/The order could not be saved/, { exact: false }),
    );
    expect(await order(container)).toEqual(["first", "second", "third"]);
  });
});

/// A right-click on a card, which is the fourth thing a press on one can be:
/// what there is to do about the Conversation it stands for, in place, without
/// opening it first.
///
/// The rows are the Conversation pane's own — see \`Actions.tsx\`, which is the
/// one component both menus are drawn through — so what is asked here is that
/// the sidebar reaches them at all, that they are about the card that was
/// pressed rather than about whatever is open, and that a finger is left with
/// the gesture it already had.
describe("what a right-click on a card offers", () => {
  /// The drafting conversation open, and the grilling one a row of the sidebar
  /// — two different Conversations, which is the whole point of the menu.
  function theSidebarOver(...answers: Parameters<typeof serving>) {
    return theWorkbench(
      whenever(
        `/api/ui/conversations/${GRILLING.id}`,
        json({ ...GRILLING, ready_to_stop: true, working: true }),
      ),
      ...answers,
    );
  }

  /// The card of a row, which is the one target a row has.
  function card(row: HTMLElement): HTMLElement {
    return row.querySelector<HTMLElement>(`.${sidebar.open}`)!;
  }

  /// The grilling conversation's row, which is the one this file's fixtures put
  /// in the sidebar beside the open one.
  async function grillingCard(container: ParentNode): Promise<HTMLElement> {
    const rows = await cards(container);
    const row = rows.find(
      (card) => card.dataset.id === String(GRILLING.id),
    );
    expect(row, "the fixture sidebar should hold the grilling conversation")
      .toBeTruthy();
    return card(row!);
  }

  /// Right-click a card at a place on the window, and say whether the browser's
  /// own menu was taken off the press.
  function rightClick(on: HTMLElement, x = 120, y = 200): boolean {
    return !fireEvent.contextMenu(on, { clientX: x, clientY: y });
  }

  /// What it drops, or nothing where nothing was right-clicked. Looked for in
  /// the sidebar, the Conversation's own ⋯ being drawn through the same
  /// component and painted by the same class.
  function drop(container: ParentNode): HTMLElement | null {
    return container.querySelector<HTMLElement>(
      `.${shell.conversationsPane} .${actions.conversationActions} > .${dropdown.drop}`,
    );
  }

  /// The same, waited for.
  async function opened(container: ParentNode): Promise<HTMLElement> {
    return drawn(
      container,
      `.${shell.conversationsPane} .${actions.conversationActions} > .${dropdown.drop}`,
    );
  }

  it("drops nothing until a card is right-clicked", async () => {
    theSidebarOver();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await cards(container);

    expect(drop(container)).toBeNull();
  });

  /// The rows the Conversation pane would offer that same Conversation, in the
  /// same order — which is what having one component for both means.
  it("offers exactly what the pane's own menu would", async () => {
    theSidebarOver();
    const { container } = mount(`/conversations/${OPEN.id}`);

    rightClick(await grillingCard(container));

    // The card carries seven fields and the rows need a good deal more than
    // seven, so the Conversation is read before there is anything to draw.
    const menu = await opened(container);
    await drawn(menu, `.${actions.close}`);

    expect(
      [...menu.querySelectorAll("button")].map((button) => button.className),
    ).toEqual([actions.stop, actions.forceStop, actions.steer, actions.close]);
  });

  /// Which is the whole reason it is worth having: the list is where the human
  /// is when they want to end something that is not what they are reading.
  it("acts on the card that was pressed, not on the one that is open", async () => {
    const fetching = theSidebarOver(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/close`,
        json("Closed" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    rightClick(await grillingCard(container));
    fireEvent.click(await drawn(await opened(container), `.${actions.close}`));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${GRILLING.id}/close`),
      ).toEqual({}),
    );
    expect(
      askedFor(fetching, `/api/ui/conversations/${OPEN.id}/close`),
      "the conversation that is open was not the one pressed",
    ).toBe(0);
  });

  /// Where the pointer was, because that is the whole of what a context menu
  /// knows about where it belongs.
  it("comes down where the pointer was", async () => {
    theSidebarOver();
    const { container } = mount(`/conversations/${OPEN.id}`);

    rightClick(await grillingCard(container), 140, 260);

    const menu = await opened(container);
    expect(menu.style.left).toBe("140px");
    expect(menu.style.top).toBe("260px");
  });

  /// The browser's own menu is not what the press is asking for.
  it("takes the browser's menu off the press", async () => {
    theSidebarOver();
    const { container } = mount(`/conversations/${OPEN.id}`);

    expect(rightClick(await grillingCard(container))).toBe(true);
  });

  /// A right-click is not a click and not the start of a drag — see `grab` in
  /// `Conversations.tsx`, which takes the primary button and nothing else — so
  /// the Conversation under it is neither opened nor moved.
  it("neither opens the conversation nor moves it", async () => {
    const fetching = theSidebarOver();
    const { container, history } = mount(`/conversations/${OPEN.id}`);

    rightClick(await grillingCard(container));
    await opened(container);

    expect(history.get()).toBe(`/conversations/${OPEN.id}`);
    expect(
      askedFor(fetching, "/api/ui/conversations/order"),
      "nothing was dragged, so there is nothing to save",
      ).toBe(0);
  });

  /// A finger has no right-click, and the long press it might have been is
  /// already how a card is picked up to be dragged. So a phone is left exactly
  /// where it was, and the ⋯ on the Conversation is the way to all of this.
  it("leaves a finger the gesture it already had", async () => {
    theSidebarOver();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const held = await grillingCard(container);
    fireEvent.pointerDown(held, {
      button: 0,
      pointerId: 1,
      pointerType: "touch",
      clientX: 0,
      clientY: 0,
    });

    // Longer than a card takes to lift — see `LIFT` in `Conversations.tsx`.
    await new Promise((done) => setTimeout(done, 450));

    expect(drop(container), "a held card, not a menu").toBeNull();
    expect(
      held.closest(`.${sidebar.conversationRow}`)!.classList.contains(sidebar.held!),
    ).toBe(true);

    fireEvent.pointerUp(held, { pointerId: 1 });
  });

  /// A phone fires `contextmenu` from that same long press, which is the one
  /// way this could have reached a finger. The press that started the gesture
  /// is what says which hand it was — the event itself says nothing — and a
  /// finger's is left alone, the browser's own answer to it included.
  it("opens nothing from a phone's long press", async () => {
    theSidebarOver();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const held = await grillingCard(container);
    fireEvent.pointerDown(held, {
      button: 0,
      pointerId: 1,
      pointerType: "touch",
      clientX: 0,
      clientY: 0,
    });

    expect(
      rightClick(held),
      "the browser's own answer to a long press is the browser's",
    ).toBe(false);
    expect(drop(container)).toBeNull();

    fireEvent.pointerUp(held, { pointerId: 1 });
  });

  /// The way out that needs no aim, which every menu has because it is the one
  /// component — see `tests/menus.test.tsx`, where the rest of that is asked.
  it("goes on a press away from it", async () => {
    theSidebarOver();
    const { container } = mount(`/conversations/${OPEN.id}`);

    rightClick(await grillingCard(container));
    await opened(container);

    fireEvent.click(
      container.querySelector(
        `.${shell.conversationsPane} .${actions.conversationActions} .${dropdown.backdrop}`,
      )!,
    );

    await waitFor(() => expect(drop(container)).toBeNull());
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
    await drawn(container, `.${sidebar.newConversation} > .${dropdown.trigger}`);

    expect(container.querySelector(`.${sidebar.newConversation} > .${dropdown.drop}`)).toBeNull();
  });

  it("closes once a repo has been chosen", async () => {
    theWorkbench(json({ Started: { id: OPEN.id } }));
    const { container } = mount();
    await openNewConversation(container);

    fireEvent.click((await repoRows(container))[0]!);

    await waitFor(() =>
      expect(container.querySelector(`.${sidebar.newConversation} > .${dropdown.drop}`)).toBeNull(),
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
      expect(container.querySelector(`.${sidebar.newConversation} > .${dropdown.drop}`)).toBeNull(),
    );
    expect(document.activeElement).toBe(
      container.querySelector(`.${sidebar.newConversation} > .${dropdown.trigger}`),
    );
  });

  /// A press away from it lands on the backdrop rather than on the page, so the
  /// press that takes the menu back cannot also open a conversation.
  it("closes on a press outside it", async () => {
    theWorkbench();
    const { container } = mount();
    await openNewConversation(container);

    fireEvent.click(await drawn(container, `.${sidebar.newConversation} > .${dropdown.backdrop}`));

    await waitFor(() =>
      expect(container.querySelector(`.${sidebar.newConversation} > .${dropdown.drop}`)).toBeNull(),
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

    const said = await drawn(container, `.${sidebar.newConversation} > .${dropdown.drop} .${notices.error}`);
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

    const group = await drawn(container, `.${sidebar.menuGroup}`);
    expect(group.querySelector(`.${sidebar.menuHeading}`)!.textContent).toBe(
      "Adopt a roadmap",
    );

    const rows = [
      ...group.querySelectorAll<HTMLButtonElement>(`.${sidebar.adoptRoadmap}`),
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

    expect(container.querySelector(`.${sidebar.adoptRoadmap}`)).toBeNull();
  });

  /// Beneath the repos, which is the order the two are decided in: starting
  /// work is the ordinary thing, and adopting is what is also there.
  it("is drawn beneath the repos", async () => {
    theWorkbench(whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)));
    const { container } = mount();
    await openNewConversation(container);

    const group = await drawn(container, `.${sidebar.menuGroup}`);
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

    expect(container.querySelector(`.${sidebar.menuGroup}`)).toBeNull();
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
    await drawn(container, `.${sidebar.menuGroup}`);

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

    const rows = await drawn(container, `.${sidebar.menuGroup}`);
    fireEvent.click(
      rows.querySelectorAll<HTMLButtonElement>(`.${sidebar.adoptRoadmap}`)[1]!,
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

    const panel = await drawn(container, `.${adoption.adoption}`);
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

    await drawn(container, `.${adoption.adoption}`);

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

    await drawn(container, `.${adoption.adoption}`);

    expect(screen.queryByLabelText("Brief")).toBeNull();
    expect(container.querySelector(`.${timeline.startGrilling}`)).toBeNull();
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
      expect(container.querySelector(`.${adoption.adoption} .${adoption.stage}`)!.textContent).toContain(
        ADOPTION.stage!.title,
      ),
    );

    fireEvent.change(await waitFor(() => screen.getByLabelText("Base branch")), {
      target: { value: elsewhere.base_commit },
    });

    await waitFor(() =>
      expect(container.querySelector(`.${adoption.adoption} .${adoption.stage}`)!.textContent).toContain(
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

    const panel = await drawn(container, `.${adoption.adoption}`);
    expect(panel.textContent).toContain(ADOPTION.roadmap);
    expect(panel.querySelector(`.${notices.empty}`)).toBeTruthy();
    expect(panel.querySelector(`.${adoption.stage}`)).toBeNull();
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

    fireEvent.click(await drawn(container, `.${adoption.adoption} .${adoption.adopt}`));

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
    fireEvent.click(await drawn(container, `.${adoption.adoption} .${adoption.adopt}`));

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

      fireEvent.click(await drawn(container, `.${adoption.adoption} .${adoption.adopt}`));

      await waitFor(() =>
        expect(container.querySelector(`.${adoption.adoption} .${notices.error}`)!.textContent).toBe(
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

    const body = await drawn(container, `.${timeline.briefBody}`);

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
    expect(container.querySelectorAll(`.${timeline.timeline} > .${timeline.timelineEvent}`)).toHaveLength(
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
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

    const growing = container.querySelector(
      `.${timeline.brief} .${app.grow}`,
    )!;
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
    expect(container.textContent).not.toContain("Saved");
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
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

    await drawn(container, `.${timeline.briefBody}`);

    expect(screen.queryByLabelText("Brief")).toBeNull();
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

    const card = await drawn(
      container,
      `.${timeline.timelineEvent} > .${timeline.brief} .${setup.conversationSetup}`,
    );

    expect(card.querySelector(`.${setup.branchName}`)).toBeTruthy();
    expect(card.querySelector(`.${setup.baseBranch}`)).toBeTruthy();
    expect(card.querySelector(`.${setup.conversationProfiles}`)).toBeTruthy();

    // The two branch fields are one row's to lay out, the way the pairings
    // below them are; the stylesheet is what wraps it where the pane is narrow.
    const row = card.querySelector(`.${setup.branches}`)!;
    expect(row.querySelector(`.${setup.branchName}`)).toBeTruthy();
    expect(row.querySelector(`.${setup.baseBranch}`)).toBeTruthy();

    // Under the words rather than over them: the brief is what the card is,
    // and while it is a draft the words are the field they are typed into.
    const body = container.querySelector(`.${timeline.brief} .${app.grow}`)!;
    expect(
      body.compareDocumentPosition(card) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// Branch, base and both pairings freeze server-side when grilling starts, so
  /// past that moment there is nothing here that could be changed — and nothing
  /// is drawn rather than drawn disabled.
  it("is gone entirely once the grilling has started", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);

    expect(container.querySelector(`.${setup.conversationSetup}`)).toBeNull();
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

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);

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
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

  /// The rule entry is the whole of what names the branch an unpinned
  /// conversation starts from: the hint that used to repeat it underneath is
  /// gone, the dropdown having said it already.
  it("names the branch an unpinned conversation will start from", async () => {
    const rule: ConversationView = { ...OPEN, base_commit: null };
    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/repos/${OPEN.repo.id}/branches`, json(BRANCHES)),
      whenever(`/api/ui/conversations/${OPEN.id}`, json(rule)),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    const picker = (await waitFor(() =>
      screen.getByLabelText("Base branch"),
    )) as HTMLSelectElement;

    expect(picker.value).toBe("");
    expect(picker.options[0]!.textContent).toContain(OPEN.repo.default_branch);
    expect(
      container.querySelector(`.${setup.baseBranch} .${notices.note}`),
    ).toBeNull();
  });

  /// Neither wording of the hint under the dropdown survives — the pinned one
  /// either, which is what the fixture's own conversation would draw.
  it("says nothing under the dropdown about when the base resolves", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByLabelText("Base branch"));

    expect(
      container.querySelector(`.${setup.baseBranch} .${notices.note}`),
    ).toBeNull();
    expect(screen.queryByText(/branches from/)).toBeNull();
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
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

  /// Readiness is the business of the button at the end of the record, which is
  /// enabled or else explains what is missing. Said up here as well it would be
  /// the same verdict twice, so the setup says nothing about it either way.
  it("says nothing about readiness, ready or not", async () => {
    withConversation(UNCHOSEN);
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByLabelText("Grilling"));
    expect(screen.queryByText(/Not ready to grill/)).toBeNull();
    expect(screen.queryByText("Ready to grill.")).toBeNull();
  });

  it("says nothing about readiness when the server says it is ready", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByLabelText("Grilling"));
    expect(OPEN.ready_to_grill, "the fixture is the ready one").toBe(true);
    expect(screen.queryByText("Ready to grill.")).toBeNull();
  });

  /// One row where the pane is wide enough for two, which is the stylesheet's
  /// half of it; what this holds is that they are the one row's to lay out.
  it("draws the two pickers as a single row", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const row = await drawn(container, `.${setup.conversationProfiles} .${setup.pairings}`);
    expect(row.querySelectorAll(`.${setup.profileChoice}`)).toHaveLength(2);
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
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByText("Its config file is gone."));
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
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

    // The way back out is a navigation — it takes the Conversation off the URL
    // — so the level changes when the router has moved rather than when the
    // button was pressed.
    fireEvent.click(screen.getByRole("button", { name: "← Conversations" }));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("conversations"),
    );
  });

  /// Walking back out takes the Conversation off the URL as well as off the
  /// frame, so that pressing the same card again is a change of selection
  /// rather than a navigation to where the page already stands — which is what
  /// pages a phone forward into it a second time.
  it("walks back in to the conversation it just came out of", async () => {
    theWorkbench();
    const { container, history } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    fireEvent.click(screen.getByText(DRAFTING.branch));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("timeline"),
    );
    expect(history.get()).toBe(`/conversations/${DRAFTING.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    fireEvent.click(screen.getByRole("button", { name: "← Conversations" }));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("conversations"),
    );
    expect(history.get()).toBe("/");

    fireEvent.click(screen.getByText(DRAFTING.branch));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("timeline"),
    );
  });

  /// So walking in to the third level is opening something, and the way forward
  /// is what stands afterwards: it is drawn for a selection rather than for a
  /// conversation.
  it("walks on to the details by opening an event, and back out again", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);
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
    expect(shellCss).toContain(".workbench > .pane {\n  display: none;\n}");
    expect(shellCss).toContain(
      '.workbench[data-pane="conversations"] > .conversationsPane,\n' +
        '.workbench[data-pane="timeline"] > .timelinePane,\n' +
        '.workbench[data-pane="details"] > .detailsPane {\n' +
        "  display: block;\n}",
    );

    // And side by side once there is room: the sidebar joins first, then the
    // third pane.
    expect(shellCss).toContain("@media (min-width: 60rem) {");
    expect(shellCss).toContain("@media (min-width: 80rem) {");
  });

  /// Every pane's header stays where it is put, whichever of the two ways the
  /// window is scrolling — the page below 60rem, the pane itself above it.
  /// Layout again, so the rules are what is read. One rule now for both the
  /// header and the block the timeline wraps it in: the two of them wear the
  /// frame's own name for whatever a pane sticks to its top edge.
  it("keeps a pane's header at the top while the pane scrolls", () => {
    expect(shellCss).toContain(
      ".pane > .paneChrome {\n" +
        "  position: sticky;\n" +
        "  top: 0;\n" +
        "  z-index: 1;\n",
    );
  });

  /// And the record goes under it rather than being cut off against it: a rem of
  /// paper fading to nothing, hung in the gap the header already kept below
  /// itself so that at rest it covers no part of the first thing in the pane.
  it("fades the record out under whatever is stuck", () => {
    const fade =
      '  content: "";\n' +
      "  position: absolute;\n" +
      "  top: 100%;\n" +
      "  right: 0;\n" +
      "  left: 0;\n" +
      "  height: 1rem;\n" +
      "  background: linear-gradient(var(--paper), transparent);\n" +
      "  pointer-events: none;\n}";

    expect(shellCss).toContain(".pane > .paneChrome::after {\n" + fade);
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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
    await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);
    expect(screen.queryByLabelText("Brief")).toBeNull();
  });

  it("draws one it has read before", async () => {
    theThree();
    const { container, history } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => expect(field().value).toBe(BRIEF.markdown));

    history.set({ value: `/conversations/${GRILLING.id}` });
    await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    history.set({ value: `/conversations/${OPEN.id}` });
    await waitFor(() => expect(field().value).toBe(BRIEF.markdown));

    // And on again, with both of them in the cache by now: nothing is fetched
    // for this one, so nothing but the change of id can move the page.
    history.set({ value: `/conversations/${GRILLING.id}` });
    await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);
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

    const start = await drawn(container, `.${timeline.startGrilling} .${timeline.start}`);

    expect(OPEN.ready_to_grill).toBe(true);
    expect(start.textContent).toContain("Start grilling");

    // Under the timeline, which is where the reason to press it is: at the end
    // of everything that has happened, under the brief it will freeze.
    expect(
      container.querySelector(`.${timeline.timeline}`)!.compareDocumentPosition(start) &
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
      `.${timeline.startGrilling} .${timeline.start}`,
    );

    expect(start.textContent).toContain("Start grilling");
    expect(start.classList).toContain(timeline.inert);
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

    fireEvent.click(await drawn(container, `.${timeline.startGrilling} .${timeline.start}`));

    await waitFor(() => screen.getByText(/This needs a brief/));
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
  });

  /// The same words for whoever has a pointer to hover with, and gone again
  /// when the pointer is.
  it("shows what is missing on hover too", async () => {
    theWorkbenchWith({ ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const start = await drawn(container, `.${timeline.startGrilling} .${timeline.start}`);

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

    fireEvent.click(await drawn(container, `.${timeline.startGrilling} .${timeline.start}`));

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

      fireEvent.click(await drawn(container, `.${timeline.startGrilling} .${timeline.start}`));

      await waitFor(() => screen.getByText(said));
    },
  );

  /// A conversation that has started has nothing to start, so there is nothing
  /// to draw — not a button that would be refused.
  it("offers nothing on a conversation that has already started", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() => screen.getByText("Draft → Grilling"));
    expect(container.querySelector(`.${timeline.startGrilling}`)).toBeNull();
  });
});

describe("a move on the timeline", () => {
  /// Both ends of it: the record keeps only the state moved to, and the one it
  /// moved from is the move before it — or drafting, where it is the first.
  it("draws the move as the transition it was", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const moved = await drawn(container, `.${timeline.timelineEvent} .${timeline.moved}`);

    expect(moved.textContent).toBe("Draft → Grilling");
    expect(moved.classList).toContain(timeline.grilling);
  });

  /// The brief stays the first event and everything after it follows in the
  /// order it happened, which is also reading order.
  it("comes after the brief it followed", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() => screen.getByText("Draft → Grilling"));

    expect(
      [...container.querySelectorAll(`.${timeline.timelineEvent} > *`)].map(
        (event) => event.className.split(" ")[0],
      ),
    ).toEqual([
      timeline.brief,
      timeline.moved,
      timeline.agentOutput,
      // The four Sets that session put to the human, in the order it asked
      // them: the answered one, the one still waiting, the deferred one that is
      // also still waiting, and the one whose stored body this build cannot
      // read — which is a row like any other and in its own place in the
      // record.
      timeline.questionSet,
      timeline.questionSet,
      timeline.questionSet,
      timeline.questionSet,
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

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(OUTPUT.turns).not.toBeNull();
    expect(output.querySelector(`.${timeline.turns}`)!.textContent).toBe(
      `${OUTPUT.turns} turns`,
    );
    expect(output.querySelector(`.${timeline.latest}`)!.textContent).toBe(OUTPUT.latest);

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

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(output.querySelector(`.${timeline.turns}`)).toBeNull();
    expect(output.textContent).not.toContain("0 turns");
  });

  /// And one turn is a turn. The count is read off a running session, so it
  /// passes through 1 on its way to the rest of them.
  it("says `1 turn` of a conversation that has taken one", async () => {
    theGrillingOutput({ turns: 1 });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(output.querySelector(`.${timeline.turns}`)!.textContent).toBe("1 turn");
  });

  /// A session getting on with it: the turning ring at the right edge, which is
  /// the mark the sidebar's card already says the same thing with. The word
  /// `running` it replaced said it once and said nothing about a session that
  /// had stopped talking an hour ago.
  it("turns the ring while the session is still working", async () => {
    theGrillingOutput({ running: true, idle: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(output.querySelector(`.${marks.mark}.${marks.working}`)).toBeTruthy();
    expect(output.querySelector(`.${marks.mark}.${marks.idle}`)).toBeNull();
    expect(output.textContent).not.toContain("running");
  });

  /// And one that is running and has gone quiet: the same ring, empty. What it
  /// exists for is the grilling sitting on a blocking ask for hours, which the
  /// turning ring would have drawn as busy the whole time.
  it("empties the ring while the session is idle", async () => {
    theGrillingOutput({ running: true, idle: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(output.querySelector(`.${marks.mark}.${marks.idle}`)).toBeTruthy();
    expect(output.querySelector(`.${marks.mark}.${marks.working}`)).toBeNull();
  });

  /// A session that has ended is a conversation with a Capture, not one with
  /// an agent in it — and the fixture is exactly that. Nothing is happening to
  /// it, so there is no mark for one to be about.
  it("says nothing about running when the session has ended", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(OUTPUT.running).toBe(false);
    expect(output.querySelector(`.${marks.mark}`)).toBeNull();
    expect(output.textContent).not.toContain("running");
  });


  /// The details pane says the same metric as the row it was opened from, and
  /// leaves it out for the same session — the two are one summary shown twice,
  /// and a pane disagreeing with the row it opened from would be two answers to
  /// the one question.
  it("says the same turn count in the details pane", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const summary = await drawn(container, `.${shell.detailsPane} .${outputPane.captureSummary}`);

    expect(summary.querySelector(`.${outputPane.turns}`)!.textContent).toBe(
      `${OUTPUT.turns} turns`,
    );
  });

  /// And it says the same liveness with the same mark, for the same reason: the
  /// row and the pane are one summary shown twice.
  it("carries the same mark in the details pane", async () => {
    theGrillingOutput({ running: true, idle: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const summary = await drawn(container, `.${shell.detailsPane} .${outputPane.captureSummary}`);

    expect(summary.querySelector(`.${marks.mark}.${marks.idle}`)).toBeTruthy();
    expect(summary.textContent).not.toContain("running");
  });


  /// And a session that has ended with no Transcript has nothing to say up
  /// there at all, so the pane draws no summary line rather than an empty one.
  it("says nothing there either for a session with no transcript", async () => {
    theGrillingOutput({ turns: null });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    // The record itself, which says the pane is drawn and it is this session's.
    await drawn(container, `.${shell.detailsPane} .${outputPane.recordSwitch}`);

    expect(container.querySelector(`.${shell.detailsPane} .${outputPane.captureSummary}`)).toBeNull();
  });

  /// The fallback, and the whole details-pane story for a session whose backend
  /// keeps no log of itself: every stub agent the suite runs is one, and so is
  /// every session that ran before Verkstead started following logs.
  it("shows the whole capture in the details pane, byte for byte", async () => {
    const fetching = theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const shown = await drawn(container, `.${shell.detailsPane} .${outputPane.capture}`);

    expect(shown.textContent).toBe(CAPTURE.text);
    expect(askedFor(fetching, CAPTURE_OF_IT)).toBeGreaterThan(0);
  });

  /// The pane opens on what the session said rather than on how it looked: the
  /// Screen is the other half of the switch, and nothing is fetched for it until
  /// somebody asks.
  it("opens on the transcript, with the screen a click away", async () => {
    const fetching = theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const showing = await drawn(
      container,
      `.${shell.detailsPane} .${outputPane.recordSwitch} .${outputPane.transcriptTab}[aria-pressed="true"]`,
    );

    expect(showing.textContent).toBe("Transcript");
    expect(container.querySelector(`.${shell.detailsPane} .${screenPane.screen}`)).toBeNull();
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    const grid = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .xterm-rows`);

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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    const said = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .${screenPane.readOnly}`);
    expect(said.textContent).toContain("Read-only");

    const typing = await drawn<HTMLTextAreaElement>(
      container,
      `.${shell.detailsPane} .${screenPane.screen} textarea`,
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.prose}`);
    const first = askedFor(fetching, TRANSCRIPT_OF_IT);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));
    await drawn(container, `.${shell.detailsPane} .${screenPane.screen}`);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.transcriptTab}`));

    await waitFor(() => drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.prose}`));
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    const said = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .${screenPane.readOnly}`);
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const prose = await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.prose}`);

    expect(prose.textContent).toContain("Looking at how the queue is drained");
    // Rendered by the server and put in the page as markup, which is what puts
    // no markdown parser on this side of the wire.
    expect(prose.querySelector("strong")!.textContent).toBe("drained");
    expect(container.querySelector(`.${shell.detailsPane} .${outputPane.capture}`)).toBeNull();
    expect(askedFor(fetching, TRANSCRIPT_OF_IT)).toBeGreaterThan(0);
  });

  /// The one a renderer keying off the line's own type gets wrong: a tool's
  /// answer and a turn from the human arrive under the same type, and reading
  /// a directory listing as though somebody had said it is the whole failure.
  it("draws a turn put to it and a tool's answer as different things", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const put = await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.put}`);
    const answered = await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolCall}`);

    expect(put.textContent).toContain("What should the queue do");
    expect(answered.textContent).toContain("crates/server/src/queue.rs");
    expect(put).not.toBe(answered);
  });

  /// What a reader opened this for is what the agent said. What it was thinking
  /// and what it ran are there to be opened rather than scrolled past.
  it("opens with the reasoning and the tool calls folded away", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const reasoning = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} .${outputPane.turn}.${outputPane.reasoning} details`,
    );
    const call = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolCall} details`,
    );
    const prose = await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.prose}`);

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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const pair = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolCall} details`,
    );

    expect(pair.querySelector("summary")!.textContent).toContain("Bash");
    expect(pair.querySelector("summary")!.textContent).toContain(
      "Find where a delivery is retried",
    );

    const behind = [...pair.querySelectorAll("pre")];

    expect(behind.map((block) => block.className)).toEqual([
      outputPane.input,
      outputPane.output,
    ]);
    expect(behind[0]!.textContent).toContain("rg -n 'retry'");
    expect(behind[1]!.textContent).toContain("crates/server/src/queue.rs");

    // And the answer is not also standing on its own under it, which is what
    // there being one card is.
    expect(
      container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolCall}`),
    ).toHaveLength(1);
    expect(
      container.querySelector(`.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolResult}`),
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolCall}`);

    const [worked, failed] = [
      ...container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolCall} summary`),
    ];

    expect(worked!.textContent).toContain("Count the tasks left");
    expect(worked!.querySelector(`.${outputPane.failed}`)).toBeNull();
    expect(failed!.textContent).toContain("Run the tests");
    expect(failed!.querySelector(`.${outputPane.failed}`)!.textContent).toBe(
      "failed",
    );

    // And the red it is said in is the one a stopped run is said in. The
    // stylesheet's, since jsdom resolves no variable and paints nothing.
    expect(outputCss).toContain(
      ".transcript .toolCall .failed {\n  flex: none;\n  margin-left: auto;\n  color: var(--stopped);\n}",
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const orphan = await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolResult}`);

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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const waiting = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} .${outputPane.turn}.${outputPane.toolCall} details`,
    );
    waiting.open = true;

    // Nothing has come back yet, so there is nothing under what it was called
    // with — and nothing said about how it went either.
    expect(waiting.querySelector(`pre.${outputPane.output}`)).toBeNull();
    expect(waiting.querySelector(`summary .${outputPane.failed}`)).toBeNull();

    await client.invalidateQueries();

    await waitFor(() =>
      expect(
        waiting.querySelector(`pre.${outputPane.output}`)!.textContent,
      ).toContain("78 passed"),
    );
    expect(container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}`)).toHaveLength(1);
    expect(waiting.open).toBe(true);
  });

  /// Roughly a third of a log is the backend's own bookkeeping. Folded rather
  /// than dropped: nothing hidden, and nothing in the way.
  it("folds the backend's bookkeeping into one group", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const kept = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} .${outputPane.bookkeeping}`,
    );

    expect(kept.open).toBe(false);
    expect(kept.querySelectorAll("li")).toHaveLength(
      TRANSCRIPT.bookkeeping.length,
    );
    expect(kept.textContent).toContain("attachment");

    // And not among the turns, which is what folding it away is for.
    expect(container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}`)).toHaveLength(
      rows(TRANSCRIPT.turns),
    );
  });

  /// ADR 0006's containment, at the end of the wire: the log's format belongs to
  /// somebody else, and one that has moved on should say so here rather than
  /// quietly emptying the pane.
  it("shows a line of a kind it does not know as the JSON it is", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const unread = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} .${outputPane.turn}.${outputPane.unread} details`,
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    const reasoning = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} .${outputPane.turn}.${outputPane.reasoning} details`,
    );
    reasoning.open = true;

    await client.invalidateQueries();

    // What the session has said since has been drawn under the fold, added to
    // what was already there rather than replacing it…
    await waitFor(() =>
      expect(container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}`)).toHaveLength(
        rows(TRANSCRIPT.turns, MORE.turns),
      ),
    );
    // …and the fold is the same element it was, still open.
    expect(
      container.querySelector<HTMLDetailsElement>(
        `.${shell.detailsPane} .${outputPane.turn}.${outputPane.reasoning} details`,
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.bookkeeping}`);

    await client.invalidateQueries();

    await waitFor(() =>
      expect(
        container.querySelectorAll(`.${shell.detailsPane} .${outputPane.bookkeeping} li`),
      ).toHaveLength(TRANSCRIPT.bookkeeping.length + MORE.bookkeeping.length),
    );
    expect(
      container.querySelectorAll(`.${shell.detailsPane} .${outputPane.bookkeeping}`),
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.turn}`);

    await client.invalidateQueries();

    // What was read whole is the record, not something to add to it: the
    // session's beginning is drawn once.
    await waitFor(() =>
      expect(container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}`)).toHaveLength(
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.prose}`);

    expect(OUTPUT.running).toBe(false);
    const before = askedFor(fetching, TRANSCRIPT_OF_IT);
    await client.invalidateQueries();

    expect(askedFor(fetching, TRANSCRIPT_OF_IT)).toBe(before);
  });

  /// The switch is the pane's own control, so it stands in the pane's header
  /// beside the title, with "← Timeline" over the top of it — which is every
  /// details pane's header now that none of them carries a Close.
  it("puts the switch in the header, beside the way back", async () => {
    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const head = await drawn(container, `.${shell.detailsPane} .${paneHead.head}`);
    await drawn(container, `.${shell.detailsPane} .${paneHead.head} .${outputPane.recordSwitch}`);
    await drawn(container, `.${shell.detailsPane} .${paneHead.head} .${paneHead.back}`);

    expect(head.textContent).not.toContain("Close");
  });

  /// Sharing that row is what the switch's width is now about: as wide as its
  /// two labels, and off onto a line of its own when the title leaves it no
  /// room. Both are the stylesheet's, and jsdom lays nothing out.
  it("sizes the switch to its labels and wraps rather than overflowing", () => {
    expect(paneHeadCss).toContain(".head {\n  display: flex;\n  flex-wrap: wrap;");
    expect(outputCss).toContain(
      ".recordSwitch {\n" +
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const transcript = await drawn(container, `.${shell.detailsPane} .${outputPane.transcriptTab}`);
    const screen = await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`);
    const mark = await drawn<HTMLElement>(
      container,
      `.${shell.detailsPane} .${outputPane.recordSwitch} .${outputPane.indicator}`,
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
    expect(outputCss).toContain(
      "@media (prefers-reduced-motion: no-preference) {\n" +
        "  .recordSwitch .indicator {\n" +
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("details"),
    );
  });

  /// The event that is open is the one the timeline says is open, so that a
  /// narrow window walking back out can see which it came from.
  it("marks the event the details pane is showing", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.agentOutput}`);
    expect(output.classList).not.toContain(timeline.selected);

    fireEvent.click(output);

    await waitFor(() => expect(output.classList).toContain(timeline.selected));
    expect(output.getAttribute("aria-pressed")).toBe("true");
  });
});

/// And that same session again, held against the foot of the pane for as long
/// as it is running.
///
/// A second appearance rather than a move: the card stays where it is on the
/// record, and this is the way back to it from however far down the human has
/// read. What holds it there is one rule of the frame's, which is what makes it
/// right in both scrolling regimes — a narrow window scrolls the page, a wide
/// one scrolls the pane, and sticky hugs the bottom edge in either.
describe("the strip for the session running now", () => {
  /// Where the strip is found, which is the pane rather than the record: it is
  /// no part of the list of what has happened.
  const strip = (container: HTMLElement) =>
    container.querySelector(`.${shell.timelinePane} > .${timeline.session}`);

  it("holds the running session against the foot of the pane", async () => {
    theGrillingOutput({ running: true, idle: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const pinned = await drawn(container, `.${timeline.session}`);

    expect(pinned.textContent).toContain("Agent output");
    expect(pinned.querySelector(`.${marks.mark}.${marks.working}`)).toBeTruthy();

    // Outside the record, and inside the pane: the strip is a second appearance
    // of the session rather than an event of its own.
    expect(pinned.closest(`.${timeline.timeline}`)).toBeNull();
    expect(strip(container)).toBe(pinned);
  });

  /// jsdom lays nothing out, so what says it stays down there is the rule, as
  /// it is for the block stuck to the pane's top edge. One rule for both ways a
  /// pane scrolls, and beneath the chrome in stacking terms so that a menu
  /// coming down from the header passes over it.
  it("stays against the bottom edge while the record scrolls past it", async () => {
    theGrillingOutput({ running: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const pinned = await drawn(container, `.${timeline.session}`);

    expect(pinned.classList).toContain(shell.paneFoot);
    expect(shellCss).toContain(
      ".pane > .paneFoot {\n  position: sticky;\n  bottom: 0;\n  z-index: 0;",
    );
  });

  /// The mark says what the card's says, live: the two are one session read at
  /// two distances, and a strip disagreeing with the row would be two answers
  /// to the one question.
  it("empties the ring while the session is idle", async () => {
    theGrillingOutput({ running: true, idle: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const pinned = await drawn(container, `.${timeline.session}`);

    expect(pinned.querySelector(`.${marks.mark}.${marks.idle}`)).toBeTruthy();
    expect(pinned.querySelector(`.${marks.mark}.${marks.working}`)).toBeNull();
  });

  /// And the press is the card's own press: the same session's output in the
  /// details pane, and the card on the record marked as the one that is open.
  it("opens the session's output", async () => {
    theGrillingOutput({ running: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.session}`));

    await drawn(container, `.${shell.detailsPane} .${outputPane.captureSummary}`);

    const card = container.querySelector(`.${timeline.agentOutput}`)!;
    expect(card.classList).toContain(timeline.selected);
  });

  /// And nothing at all where nothing is running, which is every conversation
  /// between one step and the next: there is no session to be found, so there
  /// is nothing to hold in view.
  it("draws no strip when no session is running", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(OUTPUT.running).toBe(false);
    expect(strip(container)).toBeNull();
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

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

    const grid = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .xterm-rows`);

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
    const said = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .${screenPane.readOnly}`);
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
    expect(shellCss).toContain(
      ".workbench > .detailsPane:has(.paneScreen) {\n" +
        "  flex-direction: column;\n" +
        "  height: 100dvh;\n" +
        "  padding-bottom: 1.25rem;\n" +
        "  overflow: hidden;\n" +
        "}",
    );

    // What is above the terminal keeps its size; the Screen takes the rest.
    expect(shellCss).toContain(
      ".workbench > .detailsPane:has(.paneScreen) > :not(.paneScreen) {\n" +
        "  flex: none;\n" +
        "}",
    );
    expect(screenCss).toContain(
      ".screen {\n" +
        "  display: flex;\n" +
        "  flex: 1;\n" +
        "  flex-direction: column;\n" +
        "  min-height: 0;\n",
    );

    // And the grid a session left behind, which nothing can resize: at its own
    // size, scrolling in the card it sits on rather than scrolling the pane.
    expect(screenCss).toContain(
      ".screen .terminalHost {\n" +
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

    const screen = await drawn(container, `.${shell.detailsPane} .${screenPane.screen}`);
    expect(screen.classList).toContain(screenPane.live!);

    expect(screenCss).toContain(
      ".screen.live .terminalHost {\n  overflow: hidden;\n}",
    );
  });

  /// And it leaves the session's own type alone. The badge that says a row is
  /// live is styled by that word alone, and the Screen marks itself with the
  /// same one. The two are hashed apart now that each is in a module of its
  /// own, but neither sheet may ask for the word bare inside itself — a rule
  /// that did would match every element of that module carrying it, and the
  /// inherited half of what a badge is set in would reach every row xterm
  /// builds.
  it("leaves a live terminal's text in its own type", async () => {
    watching();
    const { container, socket } = await watched();

    socket.says(PAINTED);

    const screen = await drawn(container, `.${shell.detailsPane} .${screenPane.screen}`);
    expect(screen.classList).toContain(screenPane.live!);

    // The badge asks for its type where badges are, and in sentence case:
    // nothing in the viewer is set in capitals any more.
    expect(timelineCss).toContain(
      ".eventHead .live {\n" +
        "  font-size: 0.8rem;\n" +
        "  font-weight: 600;\n",
    );

    // And nothing asks for them by the word alone, here or anywhere else: a
    // state class standing on its own matches every element that carries it.
    expect(timelineCss).not.toMatch(/(^|\n)\.live[\s,{]/);
    expect(screenCss).not.toMatch(/(^|\n)\.live[\s,{]/);
  });

  /// And the grid of a session that has ended does not carry it: nothing will
  /// resize that one, so scrolling is the only way to the rest of it.
  it("leaves the grid of an ended session scrolling", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);
    theSpeaking();

    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    const screen = await drawn(container, `.${shell.detailsPane} .${screenPane.screen}`);
    expect(screen.classList).not.toContain(screenPane.live!);
  });

  /// The height belongs to the Screen and not to the pane: switching back to the
  /// Transcript takes the element the rule hangs off away with it, which is what
  /// puts the pane's ordinary scrolling back.
  it("stops binding the pane's height once the Transcript is showing", async () => {
    watching();
    const { container, socket } = await watched();

    socket.says(PAINTED);
    await drawn(container, `.${shell.detailsPane} .${screenPane.screen}`);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.transcriptTab}`));

    await waitFor(() =>
      expect(container.querySelector(`.${shell.detailsPane} .${screenPane.screen}`)).toBeNull(),
    );
  });

  /// Closing the pane lets the socket go. Watching commits the human to nothing
  /// and a watcher that leaves takes nothing with it — on this side that is one
  /// socket closed and no request made.
  it("lets the socket go when the screen is closed", async () => {
    watching();
    const { container, socket } = await watched();

    socket.says(PAINTED);
    await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .xterm-rows`);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.transcriptTab}`));

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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .xterm-rows`);

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
      `.${shell.detailsPane} .${screenPane.screen} .xterm-helper-textarea`,
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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    const socket = await attached();
    socket.says(PAINTED);

    await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .xterm-rows`);

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

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    const socket = await attached();
    socket.says(PAINTED);

    const grid = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .xterm-rows`);
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
      `.${shell.detailsPane} .${screenPane.screen} .xterm-helper-textarea`,
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

    const grid = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .xterm-screen`);

    fireEvent.wheel(grid, { deltaY: 120 });

    await waitFor(() => expect(said(socket, "PutIn")).not.toEqual([]));
  });

  /// Typing into a driven session commits Verkstead to nothing: no press to
  /// undo it, and no badge saying the work has stopped. Somebody who wants the
  /// run held off presses Stop first, which is a stop like any other.
  it("neither draws a hand-back nor blocks the conversation on one", async () => {
    const { container } = await watching();

    await typeInto(container, "Enter", 13);

    const note = await drawn(container, `.${shell.detailsPane} .${screenPane.screen} .${screenPane.readOnly}`);
    expect(note.textContent).toContain("press Stop first");

    expect(container.querySelector('[class*="handBack"]')).toBeNull();
    expect(
      container.querySelector(`.${shell.timelinePane} .${timeline.blocked}`),
    ).toBeNull();
  });
});

describe("closing a conversation", () => {
  /// Behind a menu on the header, because it throws a worktree away and the
  /// header is somewhere the cursor passes on the way to everything else.
  it("is not one click away", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${actions.conversationActions} > .${dropdown.trigger}`);

    // Closed, so nothing in it is on the page at all — which is the whole of
    // what standing a destructive action behind a menu means.
    expect(container.querySelector(`.${actions.close}`)).toBeNull();

    const menu = await openActions(container);
    expect(menu.querySelector(`.${actions.close}`)).toBeTruthy();
    expect(container.querySelector(`.${paneHead.head} .${actions.close}`)).toBe(
      menu.querySelector(`.${actions.close}`),
    );
  });

  it("posts to the conversation's own close route", async () => {
    const fetching = theGrilling(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/close`,
        json("Closed" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.close}`));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${GRILLING.id}/close`),
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

  it("offers nothing to close on one that is closed already", async () => {
    theWorkbenchWith({ state: "Closed", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);

    await waitFor(() => screen.getByText("This conversation has been closed."));
    expect(container.querySelector(`.${actions.conversationActions} .${actions.close}`)).toBeNull();
  });

  it("says when the worktree could not be removed", async () => {
    theGrilling(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/close`,
        json("WorktreeStuck" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.close}`));

    await waitFor(() => screen.getByText(/could not be removed/));
  });
});

/// Archiving stands where Close does once Close has been pressed: the way to
/// put a finished conversation out of the list without touching any of it.
describe("archiving a conversation", () => {
  /// A conversation still being worked on belongs on the list it is worked
  /// from, so the row is not there to press.
  it("is offered on a closed conversation and on no other", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);
    expect(menu.querySelector(`.${actions.archive}`)).toBeNull();
  });

  it("stands where close was, on one that has been closed", async () => {
    theWorkbenchWith({ state: "Closed", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const menu = await openActions(container);
    await waitFor(() => expect(menu.querySelector(`.${actions.archive}`)).toBeTruthy());
    expect(menu.querySelector(`.${actions.close}`)).toBeNull();
  });

  it("posts to the conversation's own archive route", async () => {
    const fetching = theWorkbenchWith(
      { state: "Closed", ready_to_grill: false },
      whenever(
        `/api/ui/conversations/${OPEN.id}/archive`,
        json("Archived" satisfies ConversationArchived),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.archive}`));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/archive`)).toEqual({}),
    );
  });

  /// What the human is owed before a press that makes something disappear: that
  /// it is the list it goes off, and that nothing of the record goes with it.
  it("says the record stays where it is", async () => {
    theWorkbenchWith({ state: "Closed", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);

    await waitFor(() => screen.getByText(/Take it off the conversations list/));
    expect(screen.getByText(/Its record stays where it is/)).toBeTruthy();
  });

  /// A page drawn against a conversation that has since been steered back into
  /// the work: the press is refused, and the refusal is what says so.
  it("says when it is not a conversation to put away", async () => {
    theWorkbenchWith(
      { state: "Closed", ready_to_grill: false },
      whenever(
        `/api/ui/conversations/${OPEN.id}/archive`,
        json("NotClosed" satisfies ConversationArchived),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.archive}`));

    await waitFor(() => screen.getByText(/nothing to put away/));
  });
});

/// And the way back out of it, which stands in the same place on a conversation
/// that has already been put away: archiving is reversible, and this is the
/// reversal.
describe("unarchiving a conversation", () => {
  /// One row or the other, never both — the two say opposite things about the
  /// same conversation.
  it("stands where archive was, on one already put away", async () => {
    theWorkbenchWith({
      state: "Closed",
      ready_to_grill: false,
      archived: true,
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const menu = await openActions(container);
    await waitFor(() =>
      expect(menu.querySelector(`.${actions.unarchive}`)).toBeTruthy(),
    );
    expect(menu.querySelector(`.${actions.archive}`)).toBeNull();
  });

  it("posts to the conversation's own unarchive route", async () => {
    const fetching = theWorkbenchWith(
      { state: "Closed", ready_to_grill: false, archived: true },
      whenever(
        `/api/ui/conversations/${OPEN.id}/unarchive`,
        json("Unarchived" satisfies ConversationUnarchived),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.unarchive}`));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/unarchive`)).toEqual({}),
    );
  });

  /// What the human is owed before a press that puts something back: that it is
  /// the list it returns to, and that it stays there.
  it("says the conversation goes back on the list", async () => {
    theWorkbenchWith({
      state: "Closed",
      ready_to_grill: false,
      archived: true,
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);

    await waitFor(() =>
      screen.getByText(/Put it back on the conversations list/),
    );
  });

  /// The one thing left to refuse: a page drawn against a conversation that has
  /// since gone.
  it("says when the conversation is gone", async () => {
    theWorkbenchWith(
      { state: "Closed", ready_to_grill: false, archived: true },
      whenever(
        `/api/ui/conversations/${OPEN.id}/unarchive`,
        json("NoSuchConversation" satisfies ConversationUnarchived),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.unarchive}`));

    await waitFor(() => screen.getByText("This conversation is gone."));
  });
});

/// A conversation Verkstead had finished with, steered into a second round: the
/// first round's brief above the boundary and the round steered into below it.
///
/// A steer is the one way back into work that is over — nothing under the
/// timeline offers a second door — so what these ask is that the round it opens
/// reads as one, and that a finished conversation still has the steer to press.
describe("a second round", () => {
  const SECOND = secondRound as ConversationView;

  /// The workbench with the steered conversation opened instead of the drafting
  /// one.
  function theSecondRound(...answers: Parameters<typeof serving>) {
    return theWorkbench(
      whenever(`/api/ui/conversations/${SECOND.id}`, json(SECOND)),
      ...answers,
    );
  }

  /// One brief per round, both of them a record: the round steered into is past
  /// drafting from the moment it lands, so neither is a field. What the first
  /// round was built from stays on the timeline beside what the second is.
  it("draws a brief for each round, neither of them a field", async () => {
    theSecondRound();
    const { container } = mount(`/conversations/${SECOND.id}`);

    await drawn(container, `.${timeline.brief}`);
    const briefs = [...container.querySelectorAll(`.${timeline.brief}`)];

    expect(briefs).toHaveLength(2);
    expect(briefs[0]!.querySelector("textarea")).toBeNull();
    expect(briefs[1]!.querySelector("textarea")).toBeNull();

    // And no setup under either: the branch and the base commit were settled by
    // the first round, and there is nothing here left to say about them.
    expect(container.querySelector(`.${setup.conversationSetup}`)).toBeNull();
  });

  /// A reader has to be able to tell which brief the work under it was built
  /// from, which is the whole of what the boundary is for. The human's own line
  /// and the move under it are the pair that says so.
  it("says where the round boundary falls", async () => {
    theSecondRound();
    const { container } = mount(`/conversations/${SECOND.id}`);

    const boundary = await drawn(container, `.${timeline.timelineEvent} > .${timeline.steered}`);
    expect(boundary.textContent).toBe("You steered this into Grilling");

    // And it is drawn between the two briefs, which is where the rounds part.
    const briefs = [...container.querySelectorAll(`.${timeline.brief}`)];
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
    expect(timelineCss).toContain(".timelineEvent > .steered,");
  });

  /// And the boundary on a timeline reopened before a steer was the way back
  /// in. Nothing writes a move to drafting any more, but the records that carry
  /// one are the record, and the rounds on them still have to be told apart.
  it("keeps the boundary a reopened round was drawn with", async () => {
    theBuilding({
      state: "Draft",
      timeline: [
        ...BUILDING.timeline,
        { Moved: { id: 9007, at: "2026-08-24T11:00:00Z", state: "Draft" } },
      ],
    });
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const boundary = await drawn(container, `.${timeline.timelineEvent} > .${timeline.moved}.${timeline.draft}`);
    expect(boundary.textContent).toBe("Implementing → Draft");

    expect(timelineCss).toContain(".timelineEvent > .moved.draft {");
  });

  /// And the finished end of the ladder still reads as having something to do
  /// from here: nothing stands under the timeline, and the steer that opens a
  /// round like this one is in the menu on the header.
  it("offers the steer and nothing else on a finished conversation", async () => {
    theWorkbenchWith({ state: "Done" });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${timeline.timeline}`);
    expect(container.querySelector(`.${timeline.startGrilling}`)).toBeNull();

    await openActions(container);
    await drawn(container, `.${actions.conversationActions} .${actions.steer}`);
  });
});

/// Where the two stops are pressed.
const STOPPING = `/api/ui/conversations/${GRILLING.id}/stop`;
const AT_ONCE = `/api/ui/conversations/${GRILLING.id}/force-stop`;

/// The grilling conversation as the server would say it stands right now — a
/// session running, or nothing running, or a run that has already stopped.
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
  /// The two stops sit in the same menu as the steer and the close, in the order
  /// of what each one costs: pause after this task, stop now, move the work
  /// somewhere else, end the conversation. Each says what it does, because
  /// *stop* and *force stop* are two words apart and hours of work apart.
  it("offers the four ways of stopping, each saying what it does", async () => {
    theGrillingStanding({ ready_to_stop: true, working: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);
    const offered = [...menu.querySelectorAll("button")].map(
      (button) => button.className,
    );

    expect(offered).toEqual([
      actions.stop,
      actions.forceStop,
      actions.steer,
      actions.close,
    ]);

    expect(
      screen.getByText("Stop after the current task until you resume."),

    ).toBeTruthy();
    expect(
      screen.getByText("End any running task and stop immediately."),
    ).toBeTruthy();
    expect(
      screen.getByText("Stop the run and move this conversation somewhere else."),
    ).toBeTruthy();
    expect(
      screen.getByText(/Permanently end the conversation and delete the/),
    ).toBeTruthy();
  });

  /// Force stop ends a session, so it is offered where there is one. With
  /// nothing running the ordinary stop stops the run at once anyway, and a
  /// second button promising the same thing would be one to think about for
  /// nothing.
  it("offers no force stop where nothing is running", async () => {
    theGrillingStanding({ ready_to_stop: true, working: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await drawn(container, `.${actions.conversationActions} .${actions.stop}`);
    expect(
      container.querySelector(`.${actions.conversationActions} .${actions.forceStop}`),
    ).toBeNull();
  });

  /// And neither is offered on a conversation that has already stopped. Getting
  /// one going again is what resume is for; there is nothing here left to stop.
  it("offers neither stop on a conversation that has already stopped", async () => {
    theGrillingStanding({ ready_to_stop: false, working: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await drawn(container, `.${actions.conversationActions} .${actions.close}`);
    expect(container.querySelector(`.${actions.conversationActions} .${actions.stop}`)).toBeNull();
    expect(
      container.querySelector(`.${actions.conversationActions} .${actions.forceStop}`),
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
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.stop}`));

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
      await drawn(container, `.${actions.conversationActions} .${actions.forceStop}`),
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
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.stop}`));

    const waiting = await drawn(container, `.${actions.conversationActions} .${actions.waiting}`);

    expect(waiting.textContent).toContain("finishes its task first");
  });

  /// And a press that found the run already stopped says that instead, rather
  /// than looking as though it did something.
  it("says in words that it had already stopped", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(
        STOPPING,
        json("AlreadyStopped" satisfies ConversationStopped),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.stop}`));

    const refused = await drawn(container, `.${actions.conversationActions} .${notices.error}`);

    expect(refused.textContent).toBe(STOP_REFUSAL.AlreadyStopped);
    expect(refused.textContent).toContain("Resume");
  });
});

/// Where a steer is clicked, and where the modal it opens is submitted.
const STEERING = `/api/ui/conversations/${GRILLING.id}/steer`;
const STEER_SUBMIT = `/api/ui/conversations/${GRILLING.id}/steer/submit`;

/// What the click answers with when it found a session still running, and when
/// it found none — which is the whole of what the modal is drawn from.
const OVER_A_SESSION = json({ Opened: { working: true } } satisfies SteerOpened);
const OVER_NOTHING = json({ Opened: { working: false } } satisfies SteerOpened);

/// Click Steer in the actions menu, and wait for the modal it opens.
///
/// The modal is looked for on the document rather than in the container: a
/// native `dialog` opened with `showModal` is drawn in the top layer, which is
/// not inside the page's own tree.
async function openSteer(container: ParentNode): Promise<HTMLElement> {
  const menu = await openActions(container);
  fireEvent.click(await drawn(menu, `.${actions.steer}`));
  return drawn(document.body, `.${steerModal.steerConversation}`);
}

/// The states the modal is offering to send the conversation into, in the order
/// it draws them.
function targets(modal: ParentNode): string[] {
  return [
    ...modal.querySelectorAll<HTMLInputElement>(`.${steerModal.steerTarget} input`),
  ].map((input) => input.value);
}

describe("steering a conversation", () => {
  /// Every state is somewhere to steer *from* — a draft nothing has run in, a
  /// run in flight, work Verkstead has finished with — so the row is drawn
  /// wherever the menu is, unlike the two stops beside it. Which states it can
  /// be steered *to* is the modal's to offer.
  it("offers the row whatever state the conversation is in", async () => {
    theGrillingStanding({ ready_to_stop: false, working: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await drawn(container, `.${actions.conversationActions} .${actions.steer}`);
    expect(container.querySelector(`.${actions.conversationActions} .${actions.stop}`)).toBeNull();
  });

  /// The click is a press before it is a modal: it stops the drive, so nothing
  /// new is launched while the human composes and the world the modal was drawn
  /// against is the world the submit arrives in.
  it("stops the drive on the click, and reads the conversation back", async () => {
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const before = askedFor(fetching, `/api/ui/conversations/${GRILLING.id}`);

    await openSteer(container);

    expect(sent(fetching, STEERING)).toEqual({});
    await waitFor(() =>
      expect(
        askedFor(fetching, `/api/ui/conversations/${GRILLING.id}`),
      ).toBeGreaterThan(before),
    );
  });

  /// Wrapping up is a move onto a pull request that is already there rather than
  /// a way of opening one, so it is offered only where the record holds one. A
  /// target that could only be refused by name is worse than one that was never
  /// offered.
  ///
  /// Each says what it means, because the words between two of these are the
  /// difference between an hour of work and none.
  it("offers wrapping up only where the work is on a pull request", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const { container, unmount } = mount(`/conversations/${GRILLING.id}`);

    expect(targets(await openSteer(container))).toEqual([
      "Grilling",
      "Implementing",
      "Done",
    ]);
    expect(screen.getByText(/Finished with. Nothing runs/)).toBeTruthy();
    unmount();

    theGrillingStanding(
      { ready_to_stop: true, working: true, pinned: WRAPPING.pinned },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const wrapped = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(wrapped.container);

    expect(targets(modal)).toEqual([
      "Grilling",
      "Implementing",
      "Wrapping",
      "Done",
    ]);
    expect(screen.getByText(/The branch looked at again/)).toBeTruthy();
  });

  /// Implementing either carries on what the branch already holds or does what
  /// the human writes, so the instruction is required exactly where nothing
  /// stands to be carried on — and the submit is held shut until it says
  /// something rather than offered and then refused.
  ///
  /// The server says whether anything stands: what does includes the finish step
  /// a list of ticked tasks still has to run, which no reading of the entries
  /// here could see.
  it("requires the instruction where nothing stands to be carried on", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const { container, unmount } = mount(`/conversations/${GRILLING.id}`);

    const bare = await openSteer(container);

    fireEvent.click(await drawn(bare, `.${steerModal.steerTarget} input[value="Implementing"]`));

    const held = (await drawn(
      bare,
      `.${steerModal.steerButtons} .${steerModal.steer}`,
    )) as HTMLButtonElement;

    await waitFor(() => expect(held.disabled).toBe(true));
    expect(
      (bare.querySelector("#steer-instruction") as HTMLTextAreaElement)
        .placeholder,
    ).toContain("nothing on this branch to carry on");

    fireEvent.input(await drawn(bare, "#steer-instruction"), {
      target: { value: "Rebase this onto main." },
    });

    await waitFor(() => expect(held.disabled).toBe(false));
    unmount();

    // And where something does stand, writing nothing means carry it on: the
    // field is still there, and the submit was never held shut.
    theGrillingStanding(
      { ready_to_stop: true, working: true, ready_to_continue: true },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const standing = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(standing.container);

    fireEvent.click(
      await drawn(modal, `.${steerModal.steerTarget} input[value="Implementing"]`),
    );

    const press = (await drawn(
      modal,
      `.${steerModal.steerButtons} .${steerModal.steer}`,
    )) as HTMLButtonElement;

    await waitFor(() => expect(press.disabled).toBe(false));
    expect(
      (modal.querySelector("#steer-instruction") as HTMLTextAreaElement)
        .placeholder,
    ).toContain("carry on with what the branch already holds");
  });

  /// And a grilling starts from a brief, so the brief is required exactly where
  /// none is written down — a draft nobody has typed into — and optional where
  /// one is, empty there meaning grill the one that stands.
  ///
  /// The round's own brief is the newest on the timeline, which is what the
  /// server refuses by too: a round steered into freezes the brief it lands
  /// with, so an empty one is an interview about nothing that nothing can go
  /// back and write into.
  it("requires the brief where the conversation has none written", async () => {
    theGrillingStanding(
      {
        ready_to_stop: true,
        working: false,
        timeline: GRILLING.timeline.map((event) =>
          "Brief" in event
            ? { Brief: { ...event.Brief, markdown: "", html: "" } }
            : event,
        ),
      },
      whenever(STEERING, OVER_NOTHING, "POST"),
    );
    const { container, unmount } = mount(`/conversations/${GRILLING.id}`);

    const bare = await openSteer(container);

    fireEvent.click(await drawn(bare, `.${steerModal.steerTarget} input[value="Grilling"]`));

    const held = (await drawn(
      bare,
      `.${steerModal.steerButtons} .${steerModal.steer}`,
    )) as HTMLButtonElement;

    await waitFor(() => expect(held.disabled).toBe(true));
    expect(
      (bare.querySelector("#steer-brief") as HTMLTextAreaElement).placeholder,
    ).toContain("Nothing is written down yet");

    fireEvent.input(await drawn(bare, "#steer-brief"), {
      target: { value: "# Retries\n\nThe backoff is wrong.\n" },
    });

    await waitFor(() => expect(held.disabled).toBe(false));
    unmount();

    // And where one stands, writing nothing means grill the one that is there:
    // the field is still drawn, and the submit was never held shut.
    theGrillingStanding(
      { ready_to_stop: true, working: false },
      whenever(STEERING, OVER_NOTHING, "POST"),
    );
    const standing = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(standing.container);

    fireEvent.click(await drawn(modal, `.${steerModal.steerTarget} input[value="Grilling"]`));

    const press = (await drawn(
      modal,
      `.${steerModal.steerButtons} .${steerModal.steer}`,
    )) as HTMLButtonElement;

    await waitFor(() => expect(press.disabled).toBe(false));
    expect(
      (modal.querySelector("#steer-brief") as HTMLTextAreaElement).placeholder,
    ).toContain("brief that is already there");
  });

  /// And the instruction is drawn under implementing alone: what a hand-written
  /// job under a wrap-up would mean is nothing at all.
  it("draws the instruction only under implementing", async () => {
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: false, ready_to_continue: true },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(
        STEER_SUBMIT,
        json("Steered" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);

    expect(modal.querySelector("#steer-instruction")).toBeNull();

    fireEvent.click(
      await drawn(modal, `.${steerModal.steerTarget} input[value="Implementing"]`),
    );

    fireEvent.input(await drawn(modal, "#steer-instruction"), {
      target: { value: "Note the window the count is against." },
    });
    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.steer}`));

    const building = GRILLING.implementation_pairing!;

    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toEqual({
        target: "Implementing",
        interrupt: false,
        pairing: { profile_id: building.profile.id, model: building.model },
        // No round is being opened, so neither half of what one carries is
        // sent.
        brief: null,
        digest: false,
        instruction: "Note the window the count is against.",
      }),
    );
  });

  /// The pairing is the conversation's rather than one session's, so it is
  /// prefilled from what the work already runs under and what is picked is sent
  /// to be recorded as the conversation's own.
  ///
  /// Which of the two is prefilled follows the target: a grilling runs under the
  /// grilling pairing whatever else has happened since, and everything that
  /// builds runs under the other. Drawn only under a target something runs in:
  /// done runs nothing, so there is nothing there to pick.
  it("prefills the pairing of the role steered into", async () => {
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: false, pinned: WRAPPING.pinned },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(
        STEER_SUBMIT,
        json("Steered" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    // It opens on grilling, that being the first target offered, so the picker
    // is drawn from the start — filled in with what the conversation is grilled
    // under.
    const modal = await openSteer(container);

    const picker = (await drawn(modal, "#steer-pairing")) as HTMLSelectElement;
    const interviewing = GRILLING.grilling_pairing!;

    await waitFor(() =>
      expect(picker.value).toBe(
        `${interviewing.profile.id}:${interviewing.model!}`,
      ),
    );

    // Nothing runs in done, so there is nothing there to pick.
    fireEvent.click(await drawn(modal, `.${steerModal.steerTarget} input[value="Done"]`));
    await waitFor(() =>
      expect(modal.querySelector("#steer-pairing")).toBeNull(),
    );

    // And wrapping up is work being built, so it is the other pairing that is
    // prefilled there — a different choice about different work.
    fireEvent.click(await drawn(modal, `.${steerModal.steerTarget} input[value="Wrapping"]`));

    const building = GRILLING.implementation_pairing!;

    await waitFor(() =>
      expect(
        (modal.querySelector("#steer-pairing") as HTMLSelectElement).value,
      ).toBe(`${building.profile.id}:${building.model!}`),
    );

    fireEvent.change(await drawn(modal, "#steer-pairing"), {
      target: { value: `${PROFILES[0]!.id}:${PROFILES[0]!.models[0]!}` },
    });
    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.steer}`));

    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toEqual({
        target: "Wrapping",
        interrupt: false,
        pairing: { profile_id: PROFILES[0]!.id, model: PROFILES[0]!.models[0] },
        // No round is being opened and no session is being written a job, so
        // none of what those carry is sent: a brief under a wrap-up would be a
        // document about nothing.
        brief: null,
        digest: false,
        instruction: null,
      }),
    );
  });

  /// Grilling is the one target that carries a payload: a brief for the round it
  /// opens, and a choice about how much of the last interview primes it. Both
  /// are optional, and both default to the quietest thing they could mean.
  it("sends the new round's brief and what it is primed with", async () => {
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: false },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(
        STEER_SUBMIT,
        json("Steered" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    // It opens on grilling, so both are drawn from the start.
    const modal = await openSteer(container);

    fireEvent.input(await drawn(modal, "#steer-brief"), {
      target: { value: "# Retries\n\nThe backoff is wrong.\n" },
    });
    fireEvent.click(await drawn(modal, `.${steerModal.steerDigest} input`));
    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.steer}`));

    const own = GRILLING.grilling_pairing!;

    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toEqual({
        target: "Grilling",
        interrupt: false,
        pairing: { profile_id: own.profile.id, model: own.model },
        brief: "# Retries\n\nThe backoff is wrong.\n",
        digest: true,
        instruction: null,
      }),
    );
  });

  /// And neither is drawn under a target that opens no round: what a brief under
  /// a wrap-up would mean is nothing at all.
  it("draws the brief only under grilling", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: false, pinned: WRAPPING.pinned },
      whenever(STEERING, OVER_NOTHING, "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);
    await drawn(modal, `.${steerModal.steerBrief}`);

    fireEvent.click(await drawn(modal, `.${steerModal.steerTarget} input[value="Wrapping"]`));

    await waitFor(() => expect(modal.querySelector(`.${steerModal.steerBrief}`)).toBeNull());
  });

  /// The checkbox is about the world rather than about the move, so it is drawn
  /// against what the click found: with nothing running it would promise
  /// something about a session that is not there.
  it("offers the interrupt only where a session is running", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const { container, unmount } = mount(`/conversations/${GRILLING.id}`);

    const over = await openSteer(container);
    expect(over.querySelector(`.${steerModal.steerInterrupt}`)).toBeTruthy();
    unmount();

    theGrillingStanding(
      { ready_to_stop: true, working: false },
      whenever(STEERING, OVER_NOTHING, "POST"),
    );
    const quiet = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(quiet.container);
    expect(modal.querySelector(`.${steerModal.steerInterrupt}`)).toBeNull();
  });

  /// What the submit carries: where the work goes, and whether to end what is
  /// running where it stands.
  it("sends the target and the interrupt, and closes on the move", async () => {
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: true },
      whenever(STEERING, OVER_A_SESSION, "POST"),
      whenever(
        STEER_SUBMIT,
        json("Steered" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);

    fireEvent.click(await drawn(modal, `.${steerModal.steerTarget} input[value="Done"]`));
    fireEvent.click(await drawn(modal, `.${steerModal.steerInterrupt} input`));
    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.steer}`));

    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toEqual({
        target: "Done",
        interrupt: true,
        // Nothing runs in done and no round is opened there, so none of what
        // those carry is sent.
        pairing: null,
        brief: null,
        digest: false,
        instruction: null,
      }),
    );

    await waitFor(() =>
      expect(document.body.querySelector(`.${steerModal.steerConversation}`)).toBeNull(),
    );
  });

  /// Cancel is no press at all: the conversation stays where the click left it,
  /// stopped, with resume drawn on it. That is accepted rather than a bug — the
  /// click is what froze the world while the human was composing.
  it("sends nothing when it is cancelled, and leaves resume drawn", async () => {
    // The conversation as the server says it stands once the click has landed:
    // stopped, so there is nothing left to stop and one press that undoes it.
    const fetching = theGrillingStanding(
      { ready_to_stop: false, ready_to_resume: true, working: true },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);
    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.cancel}`));

    await waitFor(() =>
      expect(document.body.querySelector(`.${steerModal.steerConversation}`)).toBeNull(),
    );

    expect(
      fetching.mock.calls.filter(([asked]) => String(asked) === STEER_SUBMIT),
    ).toEqual([]);

    const press = await drawn(container, `.${timeline.resume} .${timeline.resumeConversation}`);
    expect(press.textContent).toContain("Resume");
  });

  /// And a submit the server refused says so where it was pressed, rather than
  /// closing as though it had gone.
  it("says in words when the move was refused", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: false },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(
        STEER_SUBMIT,
        json("NoSuchConversation" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);
    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.steer}`));

    const refused = await drawn(document.body, `.${steerModal.steerConversation} .${steerModal.failure}`);

    expect(refused.textContent).toBe(STEER_REFUSAL.NoSuchConversation);
  });

  /// The record of one: the human's own line, and the machine's plain move under
  /// it. A timeline of moves alone could never be read back for the difference
  /// between the pipeline arriving somewhere and somebody putting it there.
  it("draws the steer beside the move it wrote", async () => {
    theGrillingStanding({
      state: "Done",
      timeline: [
        ...GRILLING.timeline,
        {
          Steer: {
            id: 9001,
            at: "2026-08-24T11:00:00Z",
            target: "Done",
            html: null,
          },
        },
        { Moved: { id: 9002, at: "2026-08-24T11:00:00Z", state: "Done" } },
      ],
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const steered = await drawn(container, `.${timeline.timelineEvent} > .${timeline.steered}`);

    expect(steered.textContent).toBe("You steered this into Done");

    const moved = [
      ...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.moved}`),
    ].map((line) => line.textContent);

    expect(moved.at(-1)).toBe("Grilling → Done");
  });

  /// And where it carries an instruction it is a card rather than a line: what
  /// the session was sent off to do is a document like the brief and the
  /// handoff, and is read the same way — clamped beside the move, whole in the
  /// details pane.
  it("draws the instruction a steer carried, and opens the whole of it", async () => {
    theGrillingStanding({
      state: "Implementing",
      timeline: [
        ...GRILLING.timeline,
        {
          Steer: {
            id: 9003,
            at: "2026-08-24T11:00:00Z",
            target: "Implementing",
            html: "<p>Note the window the count is against.</p>",
          },
        },
        { Moved: { id: 9004, at: "2026-08-24T11:00:00Z", state: "Implementing" } },
      ],
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, `.${timeline.timelineEvent} > .${timeline.steeredWith}`);

    expect(card.textContent).toContain("You steered this into Implementing");
    expect(card.querySelector(`.${timeline.steerBody}`)!.innerHTML).toBe(
      "<p>Note the window the count is against.</p>",
    );

    fireEvent.click(card);

    const pane = await drawn(container, `.${shell.detailsPane} .${documentPane.document}`);

    expect(pane.textContent).toContain("Note the window the count is against.");
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
  return [...card.querySelectorAll(`.${timeline.asked} .${timeline.ask}`)].map((ask) =>
    [`.${timeline.n}`, `.${timeline.question}`, `.${timeline.answer}`].map(
      (part) => ask.querySelector(part)?.textContent ?? "",
    ),
  );
}

describe("a question set on the timeline", () => {
  it("reads as an interview of question line and answer line", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, `.${timeline.questionSet}`);

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

    const card = await drawn(container, `.${timeline.questionSet}`);

    expect(
      [...card.querySelectorAll(`.${timeline.asked} .${timeline.ask}`)].map((ask) =>
        ask.classList.contains(timeline.nested!),
      ),
    ).toEqual(ANSWERED_SET.rows.map((row) => row.nested));
  });

  /// A line each, whatever was asked and whatever came back. The card is the
  /// summary of the Set and the whole of it is a press away, so a question that
  /// ran to a paragraph would push the rest of the interview off the pane.
  /// jsdom lays nothing out, so the rules are what is read.
  it("holds each question and each answer to one truncated line", async () => {
    expect(timelineCss).toContain(
      ".questionSet .asked .question,\n" +
        ".questionSet .asked .answer {\n" +
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
    expect(timelineCss).toContain(
      "  grid-template-columns: var(--asked-label) minmax(0, 1fr);",
    );
  });

  /// The question is what the exchange is about, and the answer under it is
  /// read against it.
  it("sets the question in bold and the answer under it plainly", async () => {
    expect(timelineCss).toContain(
      ".questionSet .asked .question {\n  font-weight: 600;\n}",
    );
  });

  it("says which set it is, so a timeline of rounds reads as a conversation", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, `.${timeline.questionSet}`);

    expect(card.querySelector(`.${timeline.setTitle}`)!.textContent).toBe(
      ANSWERED_SET.title,
    );
  });

  /// The one thing on a timeline that is asking for something rather than
  /// recording it.
  it("marks the one still waiting on the human", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.questionSet}`);
    const cards = [...container.querySelectorAll(`.${timeline.questionSet}`)];

    // The answered one, the one still waiting, the deferred one — which is
    // waiting too, the human being the one who has not answered either — and
    // the unreadable one, which is waiting on nobody, whatever the record says
    // about it, because nothing here can put its questions in front of anybody.
    expect(cards.map((card) => card.classList.contains(timeline.waiting!))).toEqual([
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

    await drawn(container, `.${timeline.questionSet}`);
    const cards = [...container.querySelectorAll(`.${timeline.questionSet}`)];

    expect(
      cards.map((card) => card.querySelector(`.${timeline.deferred}`) !== null),
    ).toEqual([false, false, true, false]);

    const deferred = cards[2]!;

    expect(deferred.querySelector(`.${timeline.setTitle}`)!.textContent).toBe(
      DEFERRED_SET.title,
    );
    expect(deferred.querySelector(`.${timeline.deferred}`)!.textContent).toBe("deferred");
  });

  /// A column of blanks would read as a Set that was answered with nothing.
  it("draws no answers at all on one nothing has been decided about", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.questionSet}`);
    const waiting = [...container.querySelectorAll(`.${timeline.questionSet}`)][1]!;

    expect(
      interviewed(waiting).map(([, , answer]) => answer),
    ).toEqual(WAITING_SET.rows.map(() => "—"));
  });

  /// The summary is a line each; the document is a Preface, every Option of
  /// every Question, and the Diff the ask was about.
  it("opens the whole document in the details pane", async () => {
    const fetching = theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.questionSet}`));

    const pane = screen.getByLabelText("Details");
    await waitFor(() => {
      if (!pane.querySelector(`.${sheet.preface}`)) {
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

    await drawn(container, `.${timeline.questionSet}`);
    fireEvent.click([...container.querySelectorAll(`.${timeline.questionSet}`)][1]!);

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

    fireEvent.click(await drawn(container, `.${timeline.questionSet}`));

    const pane = screen.getByLabelText("Details");
    await waitFor(() => {
      if (!pane.querySelector(`.${sheet.preface}`)) {
        throw new Error("the document has not been drawn");
      }
    });

    const nav = pane.querySelector(`nav.${contents.contents}`);
    expect(nav, "expected the Set's contents in the pane").toBeTruthy();

    // The same entries the page lists, in the same order — this is the Set
    // page's own nav rather than a second reading of the document.
    expect(
      [...nav!.querySelectorAll(`a.${contents.link}`)].map((line) =>
        line.getAttribute("href"),
      ),
    ).toEqual(["#preface", "#questions", "#q1", "#q2", "#q3", "#postscript"]);

    // And it picks its shape from the pane rather than from the window.
    expect(nav!.classList.contains(contents.paned!)).toBe(true);
  });

  /// The floating header names where the reader is across the top of the column
  /// it belongs to. The pane has a header of its own up there already.
  it("leaves the floating header to the page", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.questionSet}`));

    const pane = screen.getByLabelText("Details");
    await drawn(pane, `nav.${contents.contents}`);

    expect(pane.querySelector(`.${contents.header}`)).toBeNull();
  });
});

describe("a question set the build cannot read", () => {
  it("is a row saying so rather than a gap in the record", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.questionSet}`);
    const row = container.querySelector(`.${timeline.questionSet}.${timeline.unreadable}`)!;

    expect(row.querySelector(`.${illegible.unreadableBadge}`)!.textContent).toBe(
      "cannot be read",
    );
    // Serde's own sentence, which names the field that has left the schema.
    expect(row.querySelector(`.${illegible.unreadableWhy}`)!.textContent).toContain(
      "accepted_by",
    );
    // No table, because there is nothing to draw one from — and nothing asking
    // the human for anything either.
    expect(row.querySelector(`.${timeline.asked}`)).toBeNull();
    expect(row.classList.contains(timeline.waiting!)).toBe(false);
  });

  it("opens the stored body in the details pane, the way any Set opens", async () => {
    theGrillingSets(
      whenever(`/api/ui/sets/${UNREADABLE_SET.set_id}`, json(unreadableSet)),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.questionSet}`);
    fireEvent.click(container.querySelector(`.${timeline.questionSet}.${timeline.unreadable}`)!);

    const pane = screen.getByLabelText("Details");
    const stored = await waitFor(() => {
      const found = pane.querySelector(`.${illegible.storedJson}`);
      if (!found) {
        throw new Error("the stored body has not been drawn");
      }
      return found;
    });

    expect(stored.textContent).toBe(unreadable(unreadableSet).body);
    // The one thing the timeline's rows are not: a sheet to fill in.
    expect(pane.querySelector(`.${sheet.questions}`)).toBeNull();
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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
    await drawn(container, `.${timeline.timeline}`);

    expect(BUILDING.state).toBe("Implementing");
    expect(container.querySelector(`.${sheet.directions}`)).toBeNull();
  });

  it("shows the answered proposal set as the record of the choice", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const asked = await drawn(container, `.${timeline.timelineEvent} > .${timeline.questionSet}`);

    // Answered, and answered with a pick: what the human decided is on the set
    // they decided it on, and there is no second event beside it saying so.
    expect(asked.querySelector(`.${timeline.live}`)).toBeNull();
    expect(asked.textContent).toContain("Ready to build the usage-limit pause");
  });

  it("goes from grilling to implementing with no rung in between", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timeline}`);

    const moved = [
      ...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.moved}`),
    ].map((line) => line.textContent);

    expect(moved).toEqual(["Draft → Grilling", "Grilling → Implementing"]);
  });

  /// A move records only the state it went to, and a close is off the ladder
  /// rather than on it — so what it stopped in is the move before it, which is
  /// the whole of what makes the line worth reading.
  it("names the state a close stopped in", async () => {
    theBuilding({
      state: "Closed",
      timeline: [
        ...BUILDING.timeline,
        { Moved: { id: 9001, at: "2026-08-24T11:00:00Z", state: "Closed" } },
      ],
    });
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timeline}`);

    const moved = [
      ...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.moved}`),
    ].map((line) => line.textContent);

    expect(moved.at(-1)).toBe("Implementing → Closed");
  });

  it("draws the handoff the grilling wrote as the document it is", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const handoff = await drawn(container, `.${timeline.timelineEvent} > .${timeline.handoff}`);

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

    await drawn(container, `.${timeline.timeline}`);

    expect(container.querySelector(`.${sheet.directions}`)).toBeNull();
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

    const row = await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);
    const commit = COMMITS[0]!;

    expect(row.querySelector(`.${timeline.subject}`)!.textContent).toBe(commit.subject);
    expect(row.querySelector(`.${timeline.sha}`)!.textContent).toBe(
      commit.sha.slice(0, 7),
    );
    expect(row.querySelector(`.${timeline.files}`)!.textContent).toBe(
      `${commit.files} files`,
    );
    expect(row.querySelector(`.${timeline.added}`)!.textContent).toBe(
      `+${commit.insertions}`,
    );
    expect(row.querySelector(`.${timeline.removed}`)!.textContent).toBe(
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

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    const said = COMMITS.find((commit) => commit.snippet !== null)!;
    const row = [
      ...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.commit}`),
    ].find((card) => card.querySelector(`.${timeline.subject}`)!.textContent === said.subject)!;

    expect(row.querySelector(`.${timeline.snippet}`)!.textContent).toBe(said.snippet);
    expect(row.querySelector(`.${timeline.snippet}`)!.textContent).not.toContain(
      "flowchart",
    );
  });

  /// Every bookkeeping commit and every commit recorded before summaries were
  /// kept. Nothing marks the absence: the card is the one it has always been.
  it("draws the card it always drew for a commit that said nothing", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    const silent = COMMITS.find((commit) => commit.snippet === null)!;
    const row = [
      ...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.commit}`),
    ].find((card) => card.querySelector(`.${timeline.subject}`)!.textContent === silent.subject)!;

    expect(row.querySelector(`.${timeline.snippet}`)).toBeNull();
    expect(row.innerHTML).toBe(
      `<span class="${timeline.eventHead}">` +
        `<span class="${timeline.what}">Commit</span>` +
        `<span class="${timeline.sha}">${silent.sha.slice(0, 7)}</span>` +
        "</span>" +
        `<span class="${timeline.subject}">${silent.subject}</span>` +
        `<span class="${timeline.changed}">` +
        `<span class="${timeline.files}">${silent.files} files</span>` +
        `<span class="${timeline.added}">+${silent.insertions}</span>` +
        `<span class="${timeline.removed}">−${silent.deletions}</span>` +
        "</span>",
    );
  });

  it("draws one row per commit, in timeline order", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    const subjects = [
      ...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.commit} .${timeline.subject}`),
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

    const row = await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    expect(row.querySelectorAll("button")).toHaveLength(0);
    expect(row.textContent).not.toContain("Approve");
  });

  /// "← Timeline" is the only way off the pane: there is no Close anywhere in
  /// the details panel, and on a narrow window the way back is the way out.
  it("walks back out to the record, and offers no Close", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));
    const pane = await drawn(container, `.${shell.detailsPane} .${paneHead.head}`);

    expect(pane.textContent).not.toContain("Close");

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${paneHead.back}`));

    await waitFor(() => expect(frame(container).dataset.pane).toBe("timeline"));
  });

  it("shows that commit's diff in the details pane, as the server rendered it", async () => {
    const fetching = theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const diff = await drawn(container, `.${shell.detailsPane} .${diffSection.diffFiles}`);

    // Put in the page as it arrived: the folds, the per-file anchors and the
    // highlighting are all the renderer's, and nothing here reads the diff.
    const folds = [...diff.querySelectorAll("details.diffFile")];

    expect(folds.map((fold) => fold.id)).toEqual(["diff-1", "diff-2"]);
    expect(folds[0]!.querySelector(".diffPath")!.textContent).toBe(
      COMMIT_PANE.diff!.paths[0],
    );
    expect(diff.querySelector(".diffLine.add")).toBeTruthy();
    expect(diff.querySelector(".tok-storage")).toBeTruthy();
    expect(askedFor(fetching, DIFF_OF_IT)).toBeGreaterThan(0);
  });

  /// The message is the event's to say: the diff arrives headerless, because
  /// the renderer splits on `diff --git` and would drop anything above it.
  it("says which commit it is above the diff", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const header = await drawn(container, `.${shell.detailsPane} .${commitPane.header}`);

    expect(header.textContent).toContain(COMMITS[0]!.subject);
    expect(header.textContent).toContain(COMMITS[0]!.sha.slice(0, 7));
  });

  /// What the commit said about itself, between the header and the diff — the
  /// server rendered and sanitized it, so the pane only has to put it in the
  /// page. Headed and boxed the way a Set's Preface is, which is the same kind
  /// of thing read the same way.
  it("shows the commit's message above the diff, headed and boxed", async () => {
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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const body = await drawn(container, `.${shell.detailsPane} .${commitPane.messageBody}`);

    expect(body.innerHTML).toBe("<p>A bucket per account.</p>");

    // In the box, under a heading of its own — the heading outside the box, as
    // a Preface's is.
    const message = body.closest(`.${commitPane.message}`)!;

    expect(message.querySelector("h2")!.textContent).toBe("Message");
    expect(message.querySelector("h2")!.nextElementSibling).toBe(body);

    // Read in the order it is written in: what the commit says about itself,
    // then what it changed.
    const diff = await drawn(container, `.${shell.detailsPane} .${diffSection.diff}`);

    expect(
      message.compareDocumentPosition(diff) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// A Diagram in the message is the one thing the pane draws for itself, and it
  /// draws it the Set page's way: over the source block the server left, once the
  /// message is in the page. What it drew is `diagrams.test.ts`'s subject; what
  /// is asked here is that it was reached for, and over this block alone — a Set
  /// page open behind the workbench draws its own.
  it("draws the Diagram in a message that holds one", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json(SUMMARISED)));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const body = await drawn(container, `.${shell.detailsPane} .${commitPane.messageBody}`);

    // The source block the renderer draws over — and what the reader is left
    // with if it never draws.
    expect(body.querySelector("pre.mermaid")!.textContent).toContain(
      "flowchart LR",
    );

    await waitFor(() => expect(drawing).toHaveBeenCalledOnce());
    expect(drawing.mock.calls[0]![0]).toEqual({ root: body });
  });

  /// And draws it again for the next commit opened, which is not a second mount:
  /// the pane is not rebuilt per commit, so the second message's markup lands in
  /// the block the first one was drawn in.
  ///
  /// Read three commits deep on purpose. The first switch is masked — the second
  /// commit is still being fetched for a tick, and a pane with no message yet
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

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    /// The card for one of the two, which is the only way to tell them apart on
    /// the timeline.
    const card = (subject: string) =>
      [...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.commit}`)].find(
        (row) => row.querySelector(`.${timeline.subject}`)!.textContent === subject,
      )!;

    /// The message block once it is holding the commit that was clicked, rather
    /// than the one before it: which commit the pane is showing is exactly what
    /// this test is about, so waiting for the block alone would prove nothing.
    const showing = (words: string) =>
      waitFor(() => {
        const block = container.querySelector(`.${shell.detailsPane} .${commitPane.messageBody}`);
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
  it("never reaches for the renderer where the message holds no Diagram", async () => {
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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));
    await drawn(container, `.${shell.detailsPane} .${commitPane.messageBody}`);

    expect(drawing).not.toHaveBeenCalled();
  });

  /// The ordinary commit, and every commit recorded before summaries were kept:
  /// the pane is the header and the diff, exactly as it always was.
  it("draws nothing where the commit carried no message", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));
    await drawn(container, `.${shell.detailsPane} .${diffSection.diffFiles}`);

    expect(container.querySelector(`.${shell.detailsPane} .${commitPane.messageBody}`)).toBeNull();
  });

  it("says so plainly when the commit changed no files", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json({ diff: null })));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    // Waited for rather than read once: the pane says `Loading…` in the same
    // place while the diff is in flight.
    await waitFor(() =>
      expect(
        container.querySelector(`.${shell.detailsPane} .${notices.empty}`)!.textContent,
      ).toContain("changed no files"),
    );

    expect(container.querySelector(`.${shell.detailsPane} .diff`)).toBeNull();
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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const error = await drawn(container, `.${shell.detailsPane} .${notices.error}`);

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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));
    const fold = await drawn<HTMLDetailsElement>(
      container,
      `.${shell.detailsPane} details.diffFile`,
    );
    fold.open = false;

    const before = askedFor(fetching, DIFF_OF_IT);
    await client.invalidateQueries();

    expect(askedFor(fetching, DIFF_OF_IT)).toBe(before);
    expect(
      container.querySelector<HTMLDetailsElement>(
        `.${shell.detailsPane} details.diffFile`,
      ),
    ).toBe(fold);
    expect(fold.open).toBe(false);
  });

  /// The event that is open is the one the timeline says is open, so a narrow
  /// window walking back out can see which it came from.
  it("marks the commit the details pane is showing", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const row = await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);
    expect(row.classList).not.toContain(timeline.selected);

    fireEvent.click(row);

    await waitFor(() => expect(row.classList).toContain(timeline.selected));
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
        return this.classList.contains(shell.detailsPane!) ? rem * 16 : 0;
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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);

    // The section the diff is, and then its folds — the same anchors the
    // renderer stamped on them, in the order the paths beside it name.
    expect(
      [...nav.querySelectorAll(`a.${contents.link}`)].map((line) =>
        line.getAttribute("href"),
      ),
    ).toEqual(["#commit-diff", "#diff-1", "#diff-2"]);

    // The whole path rides behind the line, which is where a nav this narrow
    // can be read out in full.
    expect(
      [...nav.querySelectorAll(`.${contents.entry} a`)].map((line) =>
        line.getAttribute("title"),
      ),
    ).toEqual(COMMIT_PANE.diff!.paths);
  });

  /// And above them, what the commit said about itself — one nav over the whole
  /// pane, in the order the pane is read in.
  it("lists a commit's message above its diff, and jumps to both", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json(SUMMARISED)));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);

    expect(
      [...nav.querySelectorAll(`a.${contents.link}`)].map((line) =>
        line.getAttribute("href"),
      ),
    ).toEqual(["#commit-message", "#commit-diff", "#diff-1", "#diff-2"]);

    // And the line that jumps to it is named for the section it lands on, which
    // is the heading the reader arrives at.
    expect(
      nav.querySelector(`a[href="#commit-message"]`)!.textContent,
    ).toBe("Message");

    // And both lines land on something: the ids are the pane's own sections
    // rather than names the nav made up.
    for (const anchor of ["commit-message", "commit-diff"]) {
      const section = container.querySelector(`.${shell.detailsPane} #${anchor}`);
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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);
    const fold = container.querySelector<HTMLDetailsElement>(
      `.${shell.detailsPane} #diff-2`,
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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));
    await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);

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

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);
    expect(nav.classList.contains(contents.paned!)).toBe(true);
    expect(nav.classList.contains(contents.roomy!)).toBe(false);
  });

  it("stands in the margin once the pane is wide enough for one", async () => {
    paneStands(90);
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);
    expect(nav.classList.contains(contents.roomy!)).toBe(true);
  });

  /// The same answer on the other pane that holds a nav, from the same
  /// measurement: it is the pane's width, not what the pane is holding.
  it("answers the same way for a question set's pane", async () => {
    paneStands(90);
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.questionSet}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);
    expect(nav.classList.contains(contents.roomy!)).toBe(true);
  });

  /// The window is wide enough for the page's own sidebar at every width these
  /// tests run at; what decides the nav's shape here is the pane.
  it("keeps the bar on a narrow set pane, whatever the window is doing", async () => {
    paneStands(40);
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.questionSet}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);
    expect(nav.classList.contains(contents.roomy!)).toBe(false);
    expect(nav.querySelector(`.${contents.bar}`)).toBeTruthy();
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

    const list = await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);

    expect(BACKLOG.tasks).toHaveLength(4);
    expect(
      [...list.querySelectorAll(`.${timeline.tasks} li`)].map((row) => [
        row.querySelector(`.${timeline.n}`)!.textContent,
        row.querySelector(`.${timeline.what}`)!.textContent,
      ]),
    ).toEqual(BACKLOG.tasks.map((task) => [task.number, task.title]));
  });

  it("says which tasks are done", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);
    const rows = [...list.querySelectorAll(`.${timeline.tasks} li`)];

    expect(rows.map((row) => row.classList.contains(timeline.done!))).toEqual(
      BACKLOG.tasks.map((task) => task.done),
    );

    // Drawn the way the file it is read out of writes it, a box per row.
    expect(rows.map((row) => row.querySelector(`.${timeline.box}`)!.textContent)).toEqual(
      BACKLOG.tasks.map((task) => (task.done ? "☑" : "☐")),
    );

    // In words as well as in a class, so a row read aloud says it too — the box
    // is the look of it and the word is what anything reading gets, which is
    // why it is out of the layout rather than out of the document.
    expect(rows.map((row) => row.querySelector(`.${timeline.state}`)!.textContent)).toEqual(
      BACKLOG.tasks.map((task) => (task.done ? "done" : "to do")),
    );
    expect(timelineCss).toContain(
      ".taskList .state,\n" +
        ".stageList .state {\n" +
        "  position: absolute;",
    );
  });

  /// `[ ] Some task            01`: the box and the title lead, and the number
  /// is at the far end of the row, out of the way of the reading.
  it("puts the number at the right edge of each row", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);

    // The order of the row is the order it reads in: nothing is moved by the
    // stylesheet that the document does not already say.
    expect(
      [...list.querySelectorAll(`.${timeline.tasks} li`)].map((row) =>
        [...row.children]
          .map((part) => part.className)
          .filter((name) => name !== timeline.state),
      ),
    ).toEqual(
      BACKLOG.tasks.map(() => [timeline.box, timeline.what, timeline.n]),
    );

    // And what holds it against that edge, which jsdom lays out no more than it
    // does the rest.
    expect(timelineCss).toContain(
      ".taskList .n,\n" +
        ".stageList .n {\n" +
        "  margin-left: auto;\n" +
        "  flex: none;",
    );
  });

  it("says what the backlog is and how far through it the work is", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const head = await drawn(container, `.${timeline.pinned} .${timeline.taskList} .${timeline.eventHead}`);

    expect(head.textContent).toContain("Task list");
    expect(head.querySelector(`.${timeline.feature}`)!.textContent).toBe(BACKLOG.feature);
    expect(head.querySelector(`.${timeline.progress}`)!.textContent).toBe("2 of 4 done");
  });

  /// Pinned is a thing an event *is*, decided by its kind: it is drawn outside
  /// the record, so it does not scroll away with it.
  it("is drawn above the record as well as in it", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(pinned.closest(`.${timeline.timeline}`)).toBeNull();

    // And a second copy on the record, at the row that says the backlog landed
    // — one card in two places, so the record keeps the moment the work stopped
    // being a plan.
    expect(
      container.querySelectorAll(`.${timeline.timeline} .${timeline.taskList}`),
    ).toHaveLength(1);
  });

  /// The row fixes where the backlog landed and nothing else: the card at it is
  /// the same live reading the pinned one is drawn from, so the two cannot come
  /// to disagree about how far through the work is.
  it("draws the same list at the row where the backlog landed", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const listed = await drawn(
      container,
      `.${timeline.timeline} .${timeline.taskList}`,
    );

    expect(
      [...listed.querySelectorAll(`.${timeline.tasks} li`)].map((row) => [
        row.querySelector(`.${timeline.what}`)!.textContent,
        row.querySelector(`.${timeline.state}`)!.textContent,
      ]),
    ).toEqual(
      BACKLOG.tasks.map((task) => [task.title, task.done ? "done" : "to do"]),
    );

    expect(
      listed.querySelector(`.${timeline.progress}`)!.textContent,
    ).toBe("2 of 4 done");
  });

  /// The reading is the worktree's, and a worktree can be taken away. The row
  /// stays — it is the record of a moment, and the moment happened — with no
  /// card to draw at it.
  it("draws nothing at that row once there is no backlog left to read", async () => {
    theTasked({
      pinned: [],
      timeline: TASKED.timeline.map((event) =>
        "TaskList" in event
          ? { TaskList: { ...event.TaskList, list: null } }
          : event,
      ),
    });
    const { container } = mount(`/conversations/${TASKED.id}`);

    await drawn(container, `.${timeline.timeline}`);

    expect(container.querySelector(`.${timeline.taskList}`)).toBeNull();
  });

  /// Nothing pins or unpins one: the set is fixed, so there is no control for
  /// it. What the card does have is the one press its whole surface is — the
  /// documents its entries name, in the details pane.
  it("is one press and offers nothing else", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);

    expect(list.querySelectorAll("button")).toHaveLength(0);
    expect(list.textContent).not.toContain("Pin");
    expect(list.getAttribute("role")).toBe("button");
    expect(list.getAttribute("aria-pressed")).toBe("false");
  });

  /// What holds it in view is the block it shares with the header, so that is
  /// where the rule is read. jsdom lays nothing out, so the rule itself is what
  /// is read, as the panes' own is.
  it("stays in view while the record scrolls past it", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);
    const chrome = pinned.closest(`.${shell.paneChrome}`);

    // One block, with the header in it: that is what makes them stay together
    // with no strip of scrolling record between them.
    expect(chrome).not.toBeNull();
    expect(chrome!.querySelector(`.${paneHead.head}`)).not.toBeNull();

    expect(shellCss).toContain(
      ".pane > .paneChrome {\n  position: sticky;\n  top: 0;",
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
    expect(menu.closest(`.${shell.paneChrome}`)).not.toBeNull();
    expect(shellCss).toContain(
      ".paneChrome > .paneChrome {\n  position: relative;\n  z-index: 1;\n}",
    );
  });

  it("draws nothing at all where the worktree holds no backlog", async () => {
    // Every other fixture here is a conversation with no `.tasks/`, which is
    // the ordinary case: the server pins nothing and there is nothing to draw.
    expect(OPEN.pinned).toEqual([]);

    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${timeline.timeline}`);

    expect(container.querySelector(`.${timeline.pinned}`)).toBeNull();
    expect(container.querySelector(`.${timeline.taskList}`)).toBeNull();
  });
});

/// The backlog opened, as the details pane fetches it: one document per entry,
/// with the finished tasks' files gone.
///
/// Written by hand rather than taken from a fixture: the fixtures are of the
/// conversation endpoint, and this is a pane's own payload — the same way a
/// commit's is written above.
const BACKLOG_PANE: BacklogPane = {
  feature: BACKLOG.feature,
  diagrams: true,
  tasks: BACKLOG.tasks.map((task) => ({
    number: task.number,
    title: task.title,
    html: task.done
      ? null
      : `<h1>${task.number}. ${task.title}</h1>\n<h2>What to build</h2>\n` +
        `<p>The ${task.title.toLowerCase()} part of it.</p>\n` +
        '<div class="wide"><pre class="mermaid">flowchart LR\n  in --&gt; out\n</pre></div>',
  })),
};

/// Where the details pane fetches it from — the conversation alone, there being
/// no event to name a backlog by.
const THE_BACKLOG = `/api/ui/conversations/${TASKED.id}/backlog`;

describe("the task list opened", () => {
  /// What the card is pressed for: the documents its entries name, which is the
  /// one thing about a backlog the card cannot show.
  it("draws every task document as its own boxed section, in backlog order", async () => {
    const fetching = theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${backlogPane.task}`);

    const sections = [
      ...container.querySelectorAll(`.${shell.detailsPane} .${backlogPane.task}`),
    ];

    expect(sections.map((section) => section.id)).toEqual(
      BACKLOG.tasks.map((task) => `task-${task.number}`),
    );
    expect(
      sections.map((section) => [
        section.querySelector(`.${backlogPane.n}`)!.textContent,
        section.querySelector(`.${backlogPane.what}`)!.textContent,
      ]),
    ).toEqual(BACKLOG.tasks.map((task) => [task.number, task.title]));

    // The Preface's own treatment: the heading outside the box, the rendered
    // markdown in it, put in the page as the server wrote it.
    const outstanding = sections[BACKLOG.tasks.findIndex((task) => !task.done)]!;
    const body = outstanding.querySelector(`.${backlogPane.document}`)!;

    expect(body.classList).toContain("markdown");
    expect(body.querySelector("h2")!.textContent).toBe("What to build");
    expect(outstanding.querySelector("h2")!.closest(`.${backlogPane.document}`)).toBeNull();
    expect(backlogCss).toContain(
      ".document,\n.finished {\n  padding: 1rem;\n  background: var(--card);",
    );

    expect(askedFor(fetching, THE_BACKLOG)).toBeGreaterThan(0);
  });

  /// A done task's file is gone from `.tasks/` — which is what says it is done —
  /// so its entry is drawn with the note rather than with a document.
  it("says so where the document is finished and removed", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${backlogPane.task}`);

    const sections = [
      ...container.querySelectorAll(`.${shell.detailsPane} .${backlogPane.task}`),
    ];

    expect(
      sections.map((section) => section.querySelector(`.${backlogPane.finished}`) !== null),
    ).toEqual(BACKLOG.tasks.map((task) => task.done));

    const done = sections[BACKLOG.tasks.findIndex((task) => task.done)]!;

    expect(done.querySelector(`.${backlogPane.finished}`)!.textContent).toBe(
      "Finished, and the document removed.",
    );
    expect(done.querySelector(`.${backlogPane.document}`)).toBeNull();
  });

  /// The set page's own table of contents, one line per task: a finished one is
  /// listed too, because it is part of what the backlog is.
  it("offers a jump to each task", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    const nav = await drawn(container, `.${shell.detailsPane} .${contents.contents}`);
    const lines = [...nav.querySelectorAll(`.${contents.sections} > li a`)];

    expect(lines.map((line) => line.getAttribute("href"))).toEqual(
      BACKLOG.tasks.map((task) => `#task-${task.number}`),
    );
    expect(lines.map((line) => line.textContent)).toEqual(
      BACKLOG.tasks.map((task) => `${task.number} ${task.title}`),
    );
  });

  /// One backlog in two places, so opening either opens the one pane and both
  /// read as selected while it is open — the pull request's own arrangement.
  it("opens from the row on the record as well as from the pinned card", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.timeline} .${timeline.taskList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${backlogPane.task}`);

    const both = [...container.querySelectorAll(`.${timeline.taskList}`)];

    expect(both).toHaveLength(2);
    expect(both.every((card) => card.classList.contains(timeline.selected!))).toBe(true);
    expect(both.every((card) => card.getAttribute("aria-pressed") === "true")).toBe(true);
  });

  /// "← Timeline" is the only way off it: there is no Close anywhere in the
  /// details panel.
  it("is titled for the card, and walks back out to the record", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    const head = await drawn(container, `.${shell.detailsPane} .${paneHead.head}`);

    expect(head.querySelector("h1")!.textContent).toBe("Task list");
    expect(head.textContent).not.toContain("Close");
    expect(
      (await drawn(container, `.${shell.detailsPane} .${backlogPane.feature}`)).textContent,
    ).toBe(BACKLOG.feature);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${paneHead.back}`));

    await waitFor(() => expect(frame(container).dataset.pane).toBe("timeline"));
  });

  /// The server refuses cleanly where the worktree or the backlog has gone, and
  /// the pane says what it was told rather than spinning.
  it("says what went wrong where there is no backlog left to read", async () => {
    theTasked(
      {},
      whenever(
        THE_BACKLOG,
        json({ error: "there is no backlog on that Conversation" }, 404),
      ),
    );
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    const line = await drawn(container, `.${shell.detailsPane} .${notices.error}`);

    expect(line.textContent).toContain("there is no backlog on that Conversation");
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

    const list = await drawn(container, `.${timeline.pinned} .${timeline.stageList}`);

    expect(ROADMAP.stages).toHaveLength(4);
    expect(
      [...list.querySelectorAll(`.${timeline.stages} li`)].map((row) => [
        row.querySelector(`.${timeline.n}`)!.textContent,
        row.querySelector(`.${timeline.what}`)!.textContent,
      ]),
    ).toEqual(ROADMAP.stages.map((stage) => [stage.number, stage.title]));
  });

  /// The roadmap's rows read the way the backlog's do, number at the far end.
  it("puts the number at the right edge of each row", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.stageList}`);

    expect(
      [...list.querySelectorAll(`.${timeline.stages} li`)].map((row) =>
        [...row.children]
          .map((part) => part.className)
          .filter((name) => name !== timeline.state),
      ),
    ).toEqual(
      ROADMAP.stages.map(() => [timeline.box, timeline.what, timeline.n]),
    );
  });

  it("says which stages are checked", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.stageList}`);
    const rows = [...list.querySelectorAll(`.${timeline.stages} li`)];

    expect(rows.map((row) => row.classList.contains(timeline.done!))).toEqual(
      ROADMAP.stages.map((stage) => stage.done),
    );

    // Boxes and words both, as a task's row carries them.
    expect(rows.map((row) => row.querySelector(`.${timeline.box}`)!.textContent)).toEqual(
      ROADMAP.stages.map((stage) => (stage.done ? "☑" : "☐")),
    );
    expect(rows.map((row) => row.querySelector(`.${timeline.state}`)!.textContent)).toEqual(
      ROADMAP.stages.map((stage) => (stage.done ? "done" : "to do")),
    );
  });

  it("says which roadmap it is and how far through it the effort is", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const head = await drawn(container, `.${timeline.pinned} .${timeline.stageList} .${timeline.eventHead}`);

    expect(head.textContent).toContain("Roadmap");
    expect(head.querySelector(`.${timeline.feature}`)!.textContent).toBe(ROADMAP.title);
    expect(head.querySelector(`.${timeline.progress}`)!.textContent).toBe("2 of 4 done");
  });

  /// Its directory is its identity, so a roadmap that wrote no heading is still
  /// named — by the directory whoever starts a stage is pointed at.
  it("falls back to the roadmap's directory where it wrote no heading", async () => {
    theStaged({ pinned: [{ StageList: { ...ROADMAP, title: "" } }] });
    const { container } = mount(`/conversations/${STAGED.id}`);

    const head = await drawn(container, `.${timeline.pinned} .${timeline.stageList} .${timeline.eventHead}`);

    expect(head.querySelector(`.${timeline.feature}`)!.textContent).toBe(ROADMAP.name);
  });

  /// Pinned beside the backlog and the pull request, and drawn the same way:
  /// above the record and again on it, with nothing to pin, unpin or open.
  it("is drawn above the record and asks the human for nothing", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.stageList}`);

    expect(list.closest(`.${timeline.timeline}`)).toBeNull();
    expect(list.querySelectorAll("button")).toHaveLength(0);
  });

  /// And on the record at the row that says the roadmap landed, drawn from the
  /// same reading the pinned copy is — a stage ticking moves both at once.
  it("draws the same roadmap at the row where it landed", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const listed = await drawn(
      container,
      `.${timeline.timeline} .${timeline.stageList}`,
    );

    expect(
      [...listed.querySelectorAll(`.${timeline.stages} li`)].map((row) => [
        row.querySelector(`.${timeline.what}`)!.textContent,
        row.querySelector(`.${timeline.state}`)!.textContent,
      ]),
    ).toEqual(
      ROADMAP.stages.map((stage) => [
        stage.title,
        stage.done ? "done" : "to do",
      ]),
    );

    expect(listed.querySelectorAll("button")).toHaveLength(0);
  });

  /// Read off the worktree like the backlog's, so a worktree that has gone
  /// leaves the row with no card to draw at it.
  it("draws nothing at that row once there is no roadmap left to read", async () => {
    theStaged({
      pinned: [],
      timeline: STAGED.timeline.map((event) =>
        "StageList" in event
          ? { StageList: { ...event.StageList, roadmaps: [] } }
          : event,
      ),
    });
    const { container } = mount(`/conversations/${STAGED.id}`);

    await drawn(container, `.${timeline.timeline}`);

    expect(container.querySelector(`.${timeline.stageList}`)).toBeNull();
  });

  /// What Verkstead did on its own account while nobody was watching — here,
  /// the stage it started when this roadmap's wrap-up settled.
  ///
  /// In the record rather than pinned above it, because it is a moment and not
  /// the standing state of anything — and a card there like every other moment,
  /// with nothing to open and nothing to answer.
  it("draws what verkstead did unasked as a card in the record", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const notice = await drawn(container, `.${timeline.timelineEvent} > .${timeline.notice}`);

    expect(notice.textContent).toContain("Stage 01");
    expect(notice.querySelector("code")?.textContent).toBe("mvp");
    expect(notice.closest(`.${timeline.timeline}`)).not.toBeNull();
    expect(notice.querySelectorAll("button")).toHaveLength(0);

    // The card surface is the one every timeline event is given, so the notice
    // asks for nothing that would take it back out of the run of them.
    expect(timelineCss).toContain(
      ".timelineEvent > .notice {\n" +
        "  font-size: 0.9rem;\n" +
        "  color: var(--ink-soft);\n" +
        "}",
    );
  });

  it("draws nothing at all where the branch has written no roadmap", async () => {
    // Every other fixture here is a conversation whose branch touched none,
    // which is the ordinary case: the server pins nothing.
    expect(TASKED.pinned.some((event) => "StageList" in event)).toBe(false);

    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);

    expect(container.querySelector(`.${timeline.stageList}`)).toBeNull();
  });
});

/// A conversation whose driving has stopped, and the notice saying what stopped.
const STOPPED = stopped as ConversationView;

/// The notice itself, off that payload: what stopped, why, and the evidence.
const SAID = (() => {
  const event = STOPPED.timeline.find((entry) => "Notice" in entry);
  if (!event || !("Notice" in event)) {
    throw new Error("the fixture should carry the notice of a stop");
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${STOPPED.id}`,
      json({ ...STOPPED, ...over }),
    ),
    ...answers,
  );
}

describe("the notice of a stop", () => {
  /// Inline and whole, unlike a capture or a diff: what a stop has to say is a
  /// paragraph and two blocks of terminal text, gathered when the run stopped
  /// because a worktree and a session's output both move on. So it is on the
  /// event rather than behind a fetch.
  it("says what stopped, why, and what the evidence was", async () => {
    const fetching = theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const notice = await drawn(container, `.${timeline.timeline} .${timeline.notice}`);

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

    const notice = await drawn(container, `.${timeline.timeline} .${timeline.notice}`);

    expect(notice.querySelector("button")).toBeNull();
    expect(notice.classList.contains(timeline.openable!)).toBe(false);
  });
});

/// A conversation stopped because the account it was spending ran out of
/// window, carrying a Pause a Verkstead of before left on its timeline.
const WAITING = paused as ConversationView;

/// The line that session printed, off the notice the old Pause reads back as.
const PRINTED = "Usage limit reached · continuing automatically at 3pm · esc to cancel";

/// The workbench with that conversation open.
function thePaused(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${WAITING.id}`,
      json({ ...WAITING, ...over }),
    ),
    ...answers,
  );
}

describe("a run stopped because an account ran out of window", () => {
  /// One stopped shape: the notice saying what stopped and why, the badge
  /// pointing at it, and the one Resume at the foot of the timeline. The same
  /// three things a run stopped by a press draws — see the stop above.
  it("draws the card, the badge and the button every stop draws", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    await drawn(container, `.${timeline.timeline} .${timeline.notice}`);

    const notice = [...container.querySelectorAll(`.${timeline.timeline} .${timeline.notice}`)].find(
      (drawn) => drawn.textContent!.includes("stopped"),
    );

    expect(notice!.textContent).toContain("Implementing the work");
    expect(notice!.textContent).toContain("opus");
    expect(notice!.textContent).toContain("is out of window");


    expect((await drawn(container, `.${timeline.blocked}`)).textContent).toBe(
      "Blocked on you",
    );
    expect(
      (await drawn(container, `.${timeline.resume} .${timeline.resumeConversation}`)).textContent,
    ).toBe("Resume");
  });

  /// And the one thing that tells it from any other stop: when the account
  /// comes back, in the words the session printed them in — `3pm` stays `3pm`.
  /// Words to read rather than a countdown: no stop resumes itself, so this one
  /// waits for the same press, and the reset is what says when to make it.
  it("says when the account comes back, beside the resume", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    const resets = await drawn(container, `.${timeline.resume} .${timeline.resets}`);

    expect(resets.textContent).toContain("out of window until 3pm");
    expect(resets.closest(`.${timeline.resume}`)!.querySelector(`.${timeline.resumeConversation}`))
      .not.toBeNull();
  });

  /// A conversation stopped by a press carries no such words, which is the
  /// whole of the difference between the two.
  it("is the only thing a conversation stopped by a press draws differently", async () => {
    expect(STOPPED.resets).toBeNull();

    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    await drawn(container, `.${timeline.resume} .${timeline.resumeConversation}`);

    expect(container.querySelector(`.${timeline.resets}`)).toBeNull();
  });

  /// The record is kept and read rather than rewritten (ADR-0006): a Pause a
  /// Verkstead of before wrote still says which account ran out and what the
  /// session printed about it, drawn as the line every stop is said in.
  it("still reads a pause written before this stage", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    await drawn(container, `.${timeline.timeline} .${timeline.notice}`);

    const notices = [...container.querySelectorAll(`.${timeline.timeline} .${timeline.notice}`)];
    const wait = notices.find((notice) =>
      notice.textContent!.includes(PRINTED),
    );

    expect(wait).toBeTruthy();
    expect(wait!.textContent).toContain("opus");
  });

  /// One press and one place to make it. Nothing on the record offers to go on
  /// without waiting, and nothing in the viewer answers a pause by its event.
  it("offers no way to go on without waiting", async () => {
    const fetching = thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    await drawn(container, `.${timeline.timeline} .${timeline.notice}`);

    expect(container.querySelector('[class*="pause"]')).toBeNull();
    expect(container.textContent).not.toContain("Go on without waiting");

    for (const notice of container.querySelectorAll(`.${timeline.timeline} .${timeline.notice}`)) {
      expect(notice.querySelector("button")).toBeNull();
    }

    fireEvent.click(await drawn(container, `.${timeline.blocked}`));

    await waitFor(() =>
      expect(
        fetching.mock.calls
          .map(([asked]) => String(asked))
          .filter((path) => path.includes("/pause/")),
      ).toEqual([]),
    );
  });

  /// The badge marks the notice where it stands in the record rather than
  /// paging a narrow window away to a details level a notice has nothing to
  /// show in.
  it("marks the notice it is blocked on where the notice stands", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.blocked}`));

    const marked = await drawn(container, `.${timeline.timeline} .${timeline.notice}.${timeline.selected}`);

    expect(marked.textContent).toContain("Implementing the work");
    expect(frame(container).dataset.pane).toBe("timeline");

    // Said by the card's own border, the way every other selected card says it
    // — in the colour that means stopped, because that is what this one is.
    expect(timelineCss).toContain(
      ".timelineEvent > .notice.selected {\n  border-color: var(--stopped);\n}",
    );
  });
});

describe("a conversation blocked on the human", () => {
  it("says so where the conversation is named", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const badge = await drawn(container, `.${paneHead.head} .${timeline.blocked}`);

    expect(badge.textContent).toBe("Blocked on you");
    expect(STOPPED.blocked_on).toBe(SAID.id);
  });

  it("draws no badge where nothing is stopping the work", async () => {
    expect(OPEN.blocked_on).toBeNull();

    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${timeline.timeline}`);

    expect(container.querySelector(`.${timeline.blocked}`)).toBeNull();
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
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
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

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(opened.textContent).toContain("Pull request");
    expect(opened.querySelector(`.${timeline.number}`)!.textContent).toBe(
      `#${OPENED.number}`,
    );
    expect(opened.querySelector(`.${timeline.openPullRequest}`)!.textContent).toBe(
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
      `.${timeline.pinned} .${timeline.pullRequest} .${timeline.out}`,
    );

    expect(out.href).toBe(OPENED.url);
  });

  /// Pinned and on the record both: the sticky block holds it in view for as
  /// long as the work is on it, and the record has it where it happened.
  it("is drawn in the pinned block and on the record", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(pinned.closest(`.${timeline.timeline}`)).toBeNull();

    const listed = container.querySelector<HTMLElement>(
      `.${timeline.timeline} .${timeline.pullRequest}`,
    )!;
    expect(listed.querySelector(`.${timeline.openPullRequest}`)!.textContent).toBe(
      OPENED.title,
    );

    // Where it happened, which is the move into wrapping the same pull request
    // wrote: the record reads on past it.
    const record = [...container.querySelectorAll(`.${timeline.timeline} > li`)];
    expect(record.findIndex((row) => row.contains(listed))).toBeLessThan(
      record.length - 1,
    );

    const moves = [...container.querySelectorAll(`.${timeline.timeline} .${timeline.moved}`)].map(
      (line) => line.textContent,
    );
    expect(moves.at(-1)).toBe("Implementing → Wrapping");
  });

  /// One card in two places, so opening either is opening the pull request:
  /// the pane is the same pane, and both copies read as selected because there
  /// is one selection and it is this event.
  it("opens the same pane from either copy, and marks both", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const listed = await drawn(
      container,
      `.${timeline.timeline} .${timeline.pullRequest}`,
    );
    fireEvent.click(listed.querySelector(`.${timeline.openPullRequest}`)!);

    await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);

    expect(
      [...container.querySelectorAll(`.${timeline.pullRequest}`)].map((card) =>
        card.classList.contains(timeline.selected!),
      ),
    ).toEqual([true, true]);
  });

  it("shows what is on it in the details pane, fetched rather than remembered", async () => {
    const fetching = theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    fireEvent.click(opened.querySelector(`.${timeline.openPullRequest}`)!);

    const commits = await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);

    expect(
      [...commits.querySelectorAll(`.${prPane.carried} li`)].map((row) => [
        row.querySelector(`.${prPane.sha}`)!.textContent,
        row.querySelector(`.${prPane.subject}`)!.textContent,
      ]),
    ).toEqual(CARRIED.commits.map((it) => [it.sha.slice(0, 7), it.subject]));

    const comments = await drawn(container, `.${shell.detailsPane} .${prPane.comments}`);

    expect(comments.querySelector(`.${prPane.author}`)!.textContent).toBe(
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

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    fireEvent.click(opened.querySelector(`.${timeline.openPullRequest}`)!);

    const error = await drawn(container, `.${shell.detailsPane} .${notices.error}`);

    expect(error.textContent).toContain("is not logged in");
  });

  /// Nothing is fetched until somebody opens it: reading this is an API call
  /// GitHub answers, and the conversation around it is read again on every
  /// Nudge about it.
  it("asks GitHub nothing until it is opened", async () => {
    const fetching = theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

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

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(
      pinned.querySelectorAll(`.${timeline.taskList}, .${timeline.stageList}, .${timeline.pullRequest}`),
    ).toHaveLength(1);
    expect(pinned.querySelector(`.${timeline.taskList}`)).not.toBeNull();
  });

  /// The dots are the whole of what the carousel says about itself: how many
  /// there are, and which one of them is being read. Each is named for the card
  /// it turns to, so a reader who cannot see them is told the same thing.
  ///
  /// Above the card rather than beneath it: the cards are not the same height
  /// as each other, so dots underneath would move every time the card changed.
  it("counts them above the card and marks the one showing", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const dots = await drawn(container, `.${timeline.pinned} .${timeline.carousel} > .${timeline.dots}`);
    const buttons = [...dots.querySelectorAll("button")];

    // First of the carousel's own children, which is what puts them above the
    // deck the cards are dealt into.
    expect(dots.previousElementSibling).toBeNull();
    expect(dots.nextElementSibling?.className).toContain(timeline.deck);

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

    const dots = await drawn(container, `.${timeline.pinned} .${timeline.carousel} > .${timeline.dots}`);
    fireEvent.click(dots.querySelectorAll("button")[2]!);

    await waitFor(() =>
      expect(container.querySelector(`.${timeline.pinned} .${timeline.pullRequest}`)).not.toBeNull(),
    );
    // The card it turned off is held in the deck while the slide runs, and gone
    // once it has.
    await waitFor(() =>
      expect(container.querySelector(`.${timeline.pinned} .${timeline.taskList}`)).toBeNull(),
    );
    expect(
      dots.querySelectorAll("button")[2]!.getAttribute("aria-current"),
    ).toBe("true");
  });

  /// The arrows count round both ends: with three cards, one that stopped at
  /// the end would be a dead control most of the time.
  it("steps between them with the arrows, and counts round the ends", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const carousel = await drawn(container, `.${timeline.pinned} .${timeline.carousel}`);

    fireEvent.click(carousel.querySelector(`.${timeline.step}.${timeline.on}`)!);
    await waitFor(() =>
      expect(carousel.querySelector(`.${timeline.stageList}`)).not.toBeNull(),
    );

    // Back past the front, which is the far end of the list.
    fireEvent.click(carousel.querySelector(`.${timeline.step}.${timeline.back}`)!);
    fireEvent.click(carousel.querySelector(`.${timeline.step}.${timeline.back}`)!);
    await waitFor(() =>
      expect(carousel.querySelector(`.${timeline.pullRequest}`)).not.toBeNull(),
    );
  });

  /// Where there is no pointer to reach an arrow with there are no arrows: the
  /// swipe is what they are, and two buttons lying over the card would be two
  /// buttons in the way of it.
  it("keeps the arrows for pointer devices", async () => {
    expect(timelineCss).toContain(".deck > .step {\n  display: none;\n}");
    expect(timelineCss).toContain(
      "@media (hover: hover) {\n  .deck > .step {\n    display: grid;",
    );
  });

  /// The arrows lie over the card's own edges, so the card stands back from
  /// them — and only where there are arrows to stand back from.
  it("gives the cards room for the arrows where there are arrows", async () => {
    const [, hovering] = timelineCss.split("@media (hover: hover) {");

    expect(hovering).toContain(
      "  .deck .taskList,\n  .deck .stageList,\n  .deck .pullRequest {\n    padding-inline: 2.4rem;\n  }",
    );
  });

  /// The slide is the stylesheet's, gated the way the record's tab indicator
  /// is: where motion is not wanted the card being left is not drawn at all,
  /// and the swap is the instant one this replaced.
  it("slides between cards only where motion is welcome", async () => {
    expect(timelineCss).toContain(".deck > .leaving {\n  display: none;\n}");

    const [, moving] = timelineCss.split(
      "@media (prefers-reduced-motion: no-preference) {",
    );

    expect(moving).toContain(".deck > .arriving.onward {\n    animation: arriveOnward");
    expect(moving).toContain(".deck > .leaving.onward {\n    animation: leaveOnward");
    expect(moving).toContain(".deck > .arriving.backward {\n    animation: arriveBackward");
    expect(moving).toContain(".deck > .leaving.backward {\n    animation: leaveBackward");
  });

  /// Both cards are in the deck while a turn runs, each wearing the part it is
  /// playing and the way the deck is travelling — which is all the stylesheet
  /// needs to move the pair together.
  it("holds both cards while a turn runs, and only that long", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const deck = await drawn(container, `.${timeline.pinned} .${timeline.deck}`);
    fireEvent.click(deck.querySelector(`.${timeline.step}.${timeline.on}`)!);

    // Read on the spot rather than waited for: the pair are there the moment the
    // arrow is pressed, and one of them is gone again a fifth of a second later.
    const leaving = deck.querySelector(`.${timeline.leaving}`);
    expect(leaving?.className).toContain(timeline.onward);
    expect(leaving?.querySelector(`.${timeline.taskList}`)).not.toBeNull();

    const arriving = deck.querySelector(`.${timeline.arriving}`);
    expect(arriving?.className).toContain(timeline.onward);
    expect(arriving?.querySelector(`.${timeline.stageList}`)).not.toBeNull();

    // And back the other way, which the pair say too.
    await waitFor(() => expect(deck.querySelector(`.${timeline.leaving}`)).toBeNull());
    fireEvent.click(deck.querySelector(`.${timeline.step}.${timeline.back}`)!);

    expect(deck.querySelector(`.${timeline.leaving}`)?.className).toContain(
      timeline.backward,
    );

    await waitFor(() => expect(deck.querySelector(`.${timeline.leaving}`)).toBeNull());
    expect(deck.querySelector(`.${timeline.arriving}`)).toBeNull();
    expect(deck.querySelector(`.${timeline.taskList}`)).not.toBeNull();
  });

  it("turns the card on a swipe across it", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const deck = await drawn(container, `.${timeline.pinned} .${timeline.carousel} > .${timeline.deck}`);

    // Leftwards is onwards, the way a page turns.
    swipe(deck, 200, 200 - SWIPE);
    await waitFor(() =>
      expect(deck.querySelector(`.${timeline.stageList}`)).not.toBeNull(),
    );

    swipe(deck, 200, 200 + SWIPE);
    await waitFor(() =>
      expect(deck.querySelector(`.${timeline.taskList}`)).not.toBeNull(),
    );

    // A press that slid a little is still a press, and turns nothing.
    await waitFor(() => expect(deck.querySelector(`.${timeline.leaving}`)).toBeNull());
    swipe(deck, 200, 200 - (SWIPE - 1));
    expect(deck.querySelector(`.${timeline.stageList}`)).toBeNull();
  });

  /// Which card the reader is put in front of: the one the work has stopped on,
  /// which is what they opened the conversation to deal with.
  it("fronts the card the work is blocked on", async () => {
    theWrapping({ pinned: ALL_THREE, blocked_on: OPENED.id });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(pinned.querySelector(`.${timeline.pullRequest}`)).not.toBeNull();
    expect(pinned.querySelector(`.${timeline.taskList}`)).toBeNull();
  });

  /// And with nothing stopping it, the fixed order — which is the order the
  /// server hands them over in, and the order the work goes through them in.
  it("otherwise fronts the first, which is the task list", async () => {
    expect(WRAPPING.blocked_on).toBeNull();

    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(pinned.querySelector(`.${timeline.taskList}`)).not.toBeNull();
  });

  /// And with no backlog to be first, the roadmap — the order is the server's,
  /// which is the order the work goes through them in.
  it("fronts the roadmap where there is no backlog before it", async () => {
    theWrapping({ pinned: ALL_THREE.slice(1) });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(pinned.querySelector(`.${timeline.stageList}`)).not.toBeNull();
    expect(pinned.querySelector(`.${timeline.pullRequest}`)).toBeNull();
  });

  /// One pinned card is not a carousel: there is nothing to turn to, and dots
  /// counting to one would be furniture around a card nothing can be done with.
  it("draws no carousel at all around a single pinned card", async () => {
    expect(WRAPPING.pinned).toHaveLength(1);

    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(pinned.closest(`.${timeline.carousel}`)).toBeNull();
    expect(container.querySelector(`.${timeline.pinned} .${timeline.dots}`)).toBeNull();
    expect(container.querySelector(`.${timeline.pinned} .${timeline.step}`)).toBeNull();
  });

  /// The card that is showing keeps everything a pinned card ever had: the
  /// sticky block it travels with, and — for the pull request — the details
  /// pane it opens.
  it("keeps the showing card's place and its behaviour", async () => {
    const fetching = theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const carousel = await drawn(container, `.${timeline.pinned} .${timeline.carousel}`);
    expect(carousel.closest(`.${shell.paneChrome}`)).not.toBeNull();
    expect(carousel.closest(`.${timeline.timeline}`)).toBeNull();

    const dots = carousel.querySelector(`.${timeline.dots}`)!;
    fireEvent.click(dots.querySelectorAll("button")[2]!);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    fireEvent.click(opened.querySelector(`.${timeline.openPullRequest}`)!);

    await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);
    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBeGreaterThan(0);
  });
});
});

/// What somebody asked for by hand once, which the same conversation carries on
/// the end of its record: a manual task, from before a steer was the way to set
/// a session going.
const ASKED_BY_HAND = (() => {
  const event = WRAPPING.timeline.find((entry) => "ManualTask" in entry);
  if (!event || !("ManualTask" in event)) {
    throw new Error("the fixture should carry a manual task");
  }
  return event.ManualTask;
})();

describe("a manual task on an old record", () => {
  /// A card in the record, like the notice beside it: nothing sets another
  /// going, so there is nothing to press and nothing to open.
  it("draws what was asked for as a card in the record", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, `.${timeline.timelineEvent} > .${timeline.manualTask}`);

    expect(asked.closest(`.${timeline.timeline}`)).not.toBeNull();
    expect(asked.querySelector(`.${timeline.eventHead}`)).toBeNull();
    expect(asked.classList.contains("openable")).toBe(false);
    expect(asked.getAttribute("role")).toBeNull();

    // And it takes the card surface every timeline event is given, asking for
    // nothing that would draw it back out of the run of them.
    expect(timelineCss).toContain(
      ".timelineEvent > .manualTask {\n  font-size: 0.9rem;\n}",
    );
  });

  /// Put in the page as the server rendered it, like every other piece of
  /// markdown on this wire — so what was set in backticks reads as code rather
  /// than as backticks.
  it("shows the instruction as the server rendered it", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, `.${timeline.timelineEvent} > .${timeline.manualTask}`);

    expect(asked.innerHTML).toBe(ASKED_BY_HAND.html);
    expect(asked.querySelector("code")!.textContent).toBe("main");
  });

  /// Read-only, and nothing to do about it: what its session went on to do
  /// arrived as the events any work arrives as, under this one.
  it("asks the human for nothing", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, `.${timeline.timelineEvent} > .${timeline.manualTask}`);

    expect(asked.querySelectorAll("button")).toHaveLength(0);
    expect(asked.querySelectorAll("textarea")).toHaveLength(0);

    fireEvent.click(asked);

    await drawn(container, `.${timeline.timeline}`);
    expect(container.querySelector(`.${shell.detailsPane} .${documentPane.document}`)).toBeNull();
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

    const resume = await drawn(container, `.${timeline.resume}`);

    expect(resume.querySelector(`.${timeline.resumeConversation}`)!.textContent).toContain(
      "Resume",
    );
  });

  /// And gone where something is. There is nothing to start again, and a button
  /// offering to would be one that could only refuse.
  it("goes where something is driving it already", async () => {
    theWrapping({ ready_to_resume: false });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    await drawn(container, `.${timeline.timeline}`);

    expect(container.querySelector(`.${timeline.resume}`)).toBeNull();
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

    const resume = await drawn(container, `.${timeline.resume}`);
    fireEvent.click(resume.querySelector(`.${timeline.resumeConversation}`)!);

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

    const resume = await drawn(container, `.${timeline.resume}`);
    fireEvent.click(resume.querySelector(`.${timeline.resumeConversation}`)!);

    const refused = await drawn(container, `.${timeline.resume} .${notices.error}`);

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

    const resume = await drawn(container, `.${timeline.resume}`);
    fireEvent.click(resume.querySelector(`.${timeline.resumeConversation}`)!);

    const refused = await drawn(container, `.${timeline.resume} .${notices.error}`);

    expect(refused.textContent).toBe(RESUME_REFUSAL.AlreadyDriven);
  });
});

/// The three documents on a timeline: the frozen brief, the handoff the grilling
/// wrote, and the instruction a steer sent a session off with.
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

  /// A one-paragraph instruction, for the document that is nowhere near long
  /// enough to be cut off by the clamp.
  const SHORT_INSTRUCTION = "<p>Note the window the count is against.</p>";

  it("puts the frozen brief in a clamp, and opens the whole of it", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const brief = await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);

    expect(brief.querySelector(`.${timeline.clamp} > .${timeline.briefBody}`)).toBeTruthy();

    fireEvent.click(brief);

    const opened = await drawn(details(), `.${documentPane.document}`);

    expect(details().querySelector("h1")!.textContent).toBe("Brief");
    // The whole of it, and not inside a clamp: the pane is where a document
    // that would not fit on a card is read.
    expect(opened.innerHTML).toBe(briefOf(GRILLING).html);
    expect(details().querySelector(`.${timeline.clamp}`)).toBeNull();
  });

  it("puts the handoff in a clamp, and opens the whole of it", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const handoff = await drawn(container, `.${timeline.timelineEvent} > .${timeline.handoff}`);

    expect(handoff.querySelector(`.${timeline.clamp} > .${timeline.handoffBody}`)).toBeTruthy();

    fireEvent.click(handoff);

    const opened = await drawn(details(), `.${documentPane.document}`);

    expect(details().querySelector("h1")!.textContent).toBe("Handoff");
    expect(opened.innerHTML).toBe(HANDOFF.html);
    expect(details().querySelector(`.${timeline.clamp}`)).toBeNull();
  });

  /// The same affordance the events that are buttons have, said on an article
  /// because rendered markdown cannot live inside a button: the role, the
  /// keyboard, and the selection drawn on the card that is open.
  it("presses like a button, and says which card is open", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const handoff = await drawn(container, `.${timeline.timelineEvent} > .${timeline.handoff}`);

    expect(handoff.getAttribute("role")).toBe("button");
    expect(handoff.getAttribute("tabindex")).toBe("0");
    expect(handoff.getAttribute("aria-pressed")).toBe("false");
    expect(handoff.classList.contains(timeline.openable!)).toBe(true);

    fireEvent.keyDown(handoff, { key: "Enter" });

    await drawn(details(), `.${documentPane.document}`);

    await waitFor(() =>
      expect(handoff.classList.contains(timeline.selected!)).toBe(true),
    );
    expect(handoff.getAttribute("aria-pressed")).toBe("true");
  });

  /// A short document opens too. One affordance whether or not the fade is
  /// drawn, because a card the human has to judge the length of before pressing
  /// is a card they will not press.
  it("opens a document too short to be cut off", async () => {
    theGrillingStanding({
      state: "Implementing",
      timeline: [
        ...GRILLING.timeline,
        {
          Steer: {
            id: 9005,
            at: "2026-08-24T11:00:00Z",
            target: "Implementing",
            html: SHORT_INSTRUCTION,
          },
        },
        { Moved: { id: 9006, at: "2026-08-24T11:00:00Z", state: "Implementing" } },
      ],
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, `.${timeline.timelineEvent} > .${timeline.steeredWith}`);

    // One paragraph, which is nowhere near the clamp.
    expect(SHORT_INSTRUCTION.split("\n").length).toBeLessThan(5);
    expect(card.getAttribute("role")).toBe("button");

    fireEvent.click(card);

    await drawn(details(), `.${documentPane.document}`);
  });

  /// The brief while it is still a draft is a field with the setup under it, and
  /// every press on that card belongs to one of those.
  it("leaves the drafting brief a field, unclamped and unpressable", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const brief = await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);
    await drawn(container, `.${timeline.brief} .${app.grow} textarea`);

    expect(brief.querySelector(`.${timeline.clamp}`)).toBeNull();
    expect(brief.getAttribute("role")).toBeNull();
    expect(brief.getAttribute("tabindex")).toBeNull();
    expect(brief.classList.contains(timeline.openable!)).toBe(false);

    fireEvent.click(brief);

    expect(details().querySelector(`.${documentPane.document}`)).toBeNull();
  });

  /// A notice is a sentence rather than a document: one line already, with
  /// nothing to cut off and nothing to open.
  it("leaves a notice line whole", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const notice = await drawn(container, `.${timeline.timelineEvent} > .${timeline.notice}`);

    expect(notice.querySelector(`.${timeline.clamp}`)).toBeNull();
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
    const opened = timelineCss.indexOf(`${selector} {`);
    expect(opened, `the stylesheet should have a \`${selector}\` rule`).not.toBe(-1);

    return timelineCss.slice(opened, timelineCss.indexOf("}", opened));
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
    expect(base).toContain(`font: 16px/${LINE_HEIGHT} system-ui`);
  });

  it("fades the cut into the card, and only where there is a cut", () => {
    const cut = block(".clamp.cut::after");

    expect(cut).toContain("linear-gradient(to bottom, transparent, var(--card))");
    // The fade must not swallow the press: the whole card opens the pane.
    expect(cut).toContain("pointer-events: none");

    // On `.cut` and nowhere else, which is what makes a short document show
    // whole with no fade over its last line.
    expect(timelineCss).not.toContain(".clamp::after");
  });
});
