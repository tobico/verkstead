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

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  AbandonedRepo,
  Adopted,
  AgentOutputEvent,
  AgentType,
  BacklogPane,
  BriefEvent,
  Capture,
  CheckRollup,
  CommitPane,
  CompanionView,
  ConversationArchived,
  ConversationClosed,
  ConversationEntry,
  ConversationSteered,
  ConversationStopped,
  ConversationUnarchived,
  ConversationView,
  GrillingStarted,
  Merging,
  NoticeEvent,
  PairingView,
  ProfileEntry,
  PinnedEvent,
  PullRequestDetails,
  Resolved,
  Resumed,
  RoadmapPane,
  Screen,
  ShareCommented,
  SharePublished,
  Shown,
  ShowingArchived,
  StageListEvent,
  SteerOpened,
  Submitted,
  TaskListEvent,
  TimelineEvent,
  TranscriptView,
  Turn,
} from "../src/api/types";
// The one menu, which all three of this page's ⋯ and dropdowns are drawn as.
// The app's own retry rule, which is what a read that gave up on its deadline
// is answered by.
import { retrying } from "../src/api/client";
// The app itself, for the one test in this file whose subject is the app's own
// query client rather than anything a page draws.
import { App } from "../src/App";
import app from "../src/App.module.css";
// The card a Set's Preface and a commit's Message are both drawn as, both ways:
// the hashed names to query the two panes by, and the source to read the box's
// own rules off, jsdom laying nothing out to read them from.
import card from "../src/Card.module.css";
import cardCss from "../src/Card.module.css?raw";
// And the pressable card every one of those two panes is reached from — a
// Conversation in the sidebar, an Event on the record — both ways again: the
// hashed names a card is queried by, and the source the card's own paint is
// read off.
import pressable from "../src/CardButton.module.css";
import pressableCss from "../src/CardButton.module.css?raw";
// And the pressable icon, which is the other thing in a pane that opens into a
// subpane: the gear at the head of the sidebar is one.
import button from "../src/IconButton.module.css";
// The brand mark of the harness a session runs under, two ways: the hashed name
// a drawn mark is queried by, and the four files it is drawn out of — read here
// as lobehub published them, so that a test naming a mark and the component
// drawing it are two independent statements about the same art, the way the
// check marks below are named from Font Awesome rather than off `Checks.tsx`.
// Reading one back off the page is `marked` in `./marking.ts`, the mark being
// drawn on more pages than this one.
import harnessMark from "../src/HarnessMark.module.css";
import claudeMarkFile from "../src/marks/claude-color.svg?raw";
import codexMarkFile from "../src/marks/codex.svg?raw";
import grokMarkFile from "../src/marks/grok.svg?raw";
import opencodeMarkFile from "../src/marks/opencode.svg?raw";
import dropdown from "../src/Menu.module.css";
import menuCss from "../src/Menu.module.css?raw";
import notices from "../src/notices.module.css";
// And the layer an outcome that reached outside this machine is said on.
import toasts from "../src/Toasts.module.css";
// The set page as it is drawn inside a details pane: its nav, its sections, and
// the record of a Set this build cannot read.
import contents from "../src/set/Contents.module.css";
import sheet from "../src/set/Sheet.module.css";
import illegible from "../src/set/Unreadable.module.css";
import { under } from "../src/pairing";
// The listbox the app draws for itself, whose own names the setup row's
// triggers are built out of — the line the reading stands on above all.
import chrome from "../src/picking.module.css";
// The element defaults, which is where the page's own line height is set.
import base from "../src/styles/base.css?raw";
// What can be done to a Conversation as a whole, both ways: the hashed names
// the menu's rows are queried by, and the words its refusals are said in.
import {
  ARCHIVE_REFUSAL,
  CLOSE_REFUSAL,
  RESUME_REFUSAL,
  STOP_REFUSAL,
  UNARCHIVE_REFUSAL,
} from "../src/workbench/Actions";
import actions from "../src/workbench/Actions.module.css";
import { ADOPT_REFUSAL } from "../src/workbench/Adoption";
import adoption from "../src/workbench/Adoption.module.css";
// The detail panes, each a module of its own: the brief and what the
// conversation was configured with, a commit, a document read whole, one
// session's record and the terminal it was printed on, and a pull request.
import briefPane from "../src/workbench/Brief.module.css";
// The composer, which is the details pane a conversation is drafted in: the
// brief as a field, the setup under it and the press that starts the work.
import composer from "../src/workbench/Composer.module.css";
import composerCss from "../src/workbench/Composer.module.css?raw";
import commitPane from "../src/workbench/Commit.module.css";
import commitPaneCss from "../src/workbench/Commit.module.css?raw";
// The Diff section the commit pane draws, which is the Set page's own component
// and so the Set page's own module.
import diffSection from "../src/set/Diff.module.css";
// The sidebar, both ways: the hashed names its rows are queried by, and the
// source of the rules that say what a card's state looks like.
import sidebar from "../src/workbench/Conversations.module.css";
import sidebarCss from "../src/workbench/Conversations.module.css?raw";
import documentPane from "../src/workbench/Document.module.css";
// And the one module the two plan panes share: a backlog's task documents and
// a roadmap's stage briefs are the same stack of boxed sections.
import documents from "../src/workbench/Documents.module.css";
import documentsCss from "../src/workbench/Documents.module.css?raw";
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
import { RESOLVE_REFUSAL } from "../src/workbench/PullRequest";
import prPane from "../src/workbench/PullRequest.module.css";
// Sharing the Conversation, which is a details pane of its own: the published
// share it draws, and the presses that came out of the actions menu.
import sharePane from "../src/workbench/Share.module.css";
// The mark a pull request's checks are said in, both ways: the hashed names to
// query the card by, and the words the icon is read aloud in. The three shapes
// themselves come straight from Font Awesome, so that a test naming one and the
// component drawing it are two independent statements about the same icon.
import { faCircle } from "@fortawesome/free-regular-svg-icons";
import { faCheck, faXmark } from "@fortawesome/free-solid-svg-icons";
import {
  SAID as CHECKS_SAID,
  SPOKEN as CHECKS_SPOKEN,
} from "../src/workbench/Checks";
import checkMarks from "../src/workbench/Checks.module.css";
// And the mark a pull request that will not merge is said in, the same two ways
// — the hashed name, and the words the icon is read aloud in. Its own shape,
// named here rather than read off the component, for the reason the three above
// are.
import { faCodeMerge } from "@fortawesome/free-solid-svg-icons";
import { CONFLICTED, IN_WORDS } from "../src/workbench/Merging";
import conflictMark from "../src/workbench/Merging.module.css";
// The Screen, both ways: it is the one pane with a height of its own, and the
// rules that give it one are what jsdom cannot lay out.
import screenPane from "../src/workbench/Screen.module.css";
import screenCss from "../src/workbench/Screen.module.css?raw";
// What a Conversation is called where nobody has named its branch, which the
// sidebar and the pane header are both drawn with.
import { AUTOMATIC, DRAFT, titled } from "../src/workbench/naming";
// The words a lifecycle state is said in, which the status button draws beside
// the status and the sidebar's row reads aloud.
import { STATE } from "../src/workbench/states";
// And the timeline, both ways again: it is the biggest of these, and a good
// deal of what it says about a card is a rule rather than an element.
// What is still the human's to settle on the brief card.
import setup from "../src/workbench/Setup.module.css";
import setupCss from "../src/workbench/Setup.module.css?raw";
import steerModal from "../src/workbench/Steer.module.css";
// The status button at the foot of the sticky block over the Conversation pane,
// both ways: the hashed names its line is queried by, and the source of the
// paint that says when it is in the accent.
import statusButton from "../src/workbench/StatusButton.module.css";
import statusButtonCss from "../src/workbench/StatusButton.module.css?raw";
import timeline from "../src/workbench/Timeline.module.css";
import timelineCss from "../src/workbench/Timeline.module.css?raw";
import truncatedCss from "../src/Truncated.module.css?raw";
// And the frame the three panes stand in, both ways: it holds the layout rules
// jsdom lays nothing out for, and the pane names everything else is found by.
import shell from "../src/Panes.module.css";
import shellCss from "../src/Panes.module.css?raw";
// And the one sentence a Verkstead with no session to start says, wherever it
// says it — the fifth thing every press that wanted a session comes back with.
import { NO_SESSIONS } from "../src/workbench/sessions";
import { ABBREVIATED, CLAMPED_LINES, SWIPE } from "../src/workbench/Timeline";
import {
  COMPANION_BRANCH_REFUSAL,
  COMPANION_MODE_REFUSAL,
  COMPANION_REFUSAL,
  COMPANION_REMOVAL_REFUSAL,
  REPO_SWITCH_REFUSAL,
} from "../src/workbench/Setup";
import { STEER_REFUSAL } from "../src/workbench/Steer";
import {
  BRANCHES,
  COMPANION_BRANCHES,
  HIDING_ARCHIVED,
  OPEN,
  PROFILES,
  REPOS,
  SIDEBAR,
  drawn,
  mount,
  mountSidebar,
  nudged,
  theWorkbench,
} from "./bench";
import { art, marked } from "./marking";
import {
  offered,
  pick,
  picker,
  rows as offers,
  showing,
} from "./pickers";
import {
  askedFor,
  hangs,
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

/// The composer pane's own heading, which is what a draft is called: the branch
/// somebody named, and *Draft* until one is named. The opened fixture has a
/// name, so this is that name — see `naming.ts`, and the test that pins both
/// halves of the rule.
const composerHead = () =>
  screen.getByRole("heading", { name: titled(OPEN) });

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
  return container.querySelector(`.${shell.panes}`)!;
}

/// Where a Conversation is opened for the tests that are about a card *before*
/// anything has been pressed.
///
/// Its own path lands on the end of the record — the last openable card is
/// selected and the pane opens on it — which is the point of it and no use at
/// all to a test asking what an unpressed card looks like. A path that names a
/// details pane is a cold load of that pane and the landing leaves it alone; a
/// path naming an Event the record has not got leaves the pane empty, as a
/// stale link does. There is no Event 0, so this is one.
function unopened(conversation: { id: number }): string {
  return `/conversations/${conversation.id}/events/0`;
}

/// The composer, waited for where the Conversation opens on it.
///
/// Every fixture the tests below open is a Draft whose record is the Brief and
/// nothing else, so there is no Timeline drawn to press a card on: the landing
/// opens the composer and a narrow window is already standing at it. Which
/// level is showing matters to a test that asks about roles rather than
/// classes — the frame draws one pane at a time and the rest are
/// `display: none`, and an accessible name is only found in the one being
/// shown. Pressing a Brief card to open this pane is a record with more on it
/// than the Brief, and is asked where that is the question.
async function openComposer(container: ParentNode): Promise<HTMLElement> {
  return drawn(container, `.${shell.detailsPane} .${composer.composer}`);
}

/// And on into the Repo panel, which is where the branch, the base and the
/// companion repos are settled: the first option of the composer's row, and one
/// flat card rather than a menu of levels.
async function openRepo(container: ParentNode): Promise<HTMLElement> {
  await openComposer(container);

  fireEvent.click(
    await drawn<HTMLButtonElement>(container, `.${setup.repoOption} > button`),
  );

  return drawn(container, `.${setup.repoOption} > [role="group"]`);
}

/// Open the conversation's action menu: press the trigger, and wait for what it
/// drops.
async function openActions(container: ParentNode): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, `.${actions.conversationActions} > .${dropdown.trigger}`));
  return drawn(container, `.${actions.conversationActions} > .${dropdown.drop}`);
}

/// The status button's first line, in its parts: the status word, the state
/// understated beside it, and the word for what a Cleanup took where one has
/// been through the record.
///
/// Where there is no status word to say — a Draft, a Done or a Closed
/// conversation — the state takes the bold and stands alone, so `word` is the
/// state and `state` is `null`. Which is a fact worth reading off the two
/// elements rather than off one string: they are drawn differently on purpose.
async function standing(container: ParentNode): Promise<{
  word: string | null;
  state: string | null;
  trimmed: string | null;
  attention: boolean;
}> {
  const line = await drawn(
    container,
    `.${statusButton.status} .${statusButton.standing}`,
  );

  return {
    word: line.querySelector(`.${statusButton.title}`)?.textContent ?? null,
    state: line.querySelector(`.${statusButton.state}`)?.textContent ?? null,
    trimmed:
      line.querySelector(`.${statusButton.trimmed}`)?.textContent ?? null,
    attention: line.classList.contains(statusButton.attention!),
  };
}

/// The gear at the head of the sidebar, which is what the rest of Verkstead is
/// behind. Found by the name it is read aloud by, an icon saying nothing for
/// itself.
async function gear(container: ParentNode): Promise<HTMLButtonElement> {
  return drawn<HTMLButtonElement>(container, 'button[aria-label="Settings"]');
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
    ).toEqual(SIDEBAR.map((entry) => titled(entry)));
  });

  /// Each row's name is held to one line and cut at the front, exactly as the
  /// header of the pane it opens cuts the same name: a sidebar is a list to run
  /// an eye down, and a wrapping name made every row a different height. The
  /// whole of it is in the tooltip.
  it("holds each row's name to one line, cut at the front", async () => {
    theWorkbench();
    const { container } = mount();

    const row = await drawn(
      container,
      `.${sidebar.conversationRow} .${sidebar.title}`,
    );

    expect(row.getAttribute("title")).toBe(titled(SIDEBAR[0]!));
    expect(row.querySelector("bdi")!.textContent).toBe(titled(SIDEBAR[0]!));
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
  /// Repos and the Agent Profiles were folded onto the settings page. A gear at
  /// the head of the pane, where the ⋯ that held it stood: one press rather than
  /// the two a menu of one row was.
  it("reaches the rest of Verkstead from the gear", async () => {
    theWorkbench();
    const { container, history } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    fireEvent.click(await gear(container));
    await waitFor(() => expect(history.get()).toBe("/settings"));
  });

  /// And it is another thing in this pane that is selected and opened into the
  /// pane beside it, which is what the cards under it are — so it says which of
  /// the two it is the same way they do. Not open on the workbench, and open
  /// wherever under the settings the human has got to.
  it("reads the gear as open only under the settings", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));
    expect((await gear(container)).getAttribute("aria-pressed")).toBe("false");

    for (const at of ["/settings", "/settings/repos/1"]) {
      const page = mountSidebar(at);
      const there = await gear(page.container);
      expect(there.getAttribute("aria-pressed")).toBe("true");
      expect(there.classList).toContain(button.open);
      page.unmount();
    }
  });

  /// The ⋯ that held both of them is gone with them: what it was for was the way
  /// out of the workbench, and a menu that stood in front of one press is a
  /// press in front of a press.
  it("leaves no ⋯ at the head of the conversations", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    expect(
      container.querySelector(
        `.${shell.conversationsPane} .${dropdown.trigger}.${dropdown.mark}`,
      ),
    ).toBeNull();
  });

  /// Two words rather than the sentence it used to be, and never on two lines:
  /// the switch stands under the list of conversations, so what else it could be
  /// showing does not have to be said — and a label that wrapped would leave the
  /// switch on a line of its own with nothing beside it to say what it was for.
  it("keeps the archived switch on one line", () => {
    const at = sidebarCss.indexOf("\n.showArchived label > span {");
    expect(at, "expected the sheet to hold the switch's own label rule").toBeGreaterThan(-1);
    expect(sidebarCss.slice(at, sidebarCss.indexOf("\n}", at))).toContain(
      "white-space: nowrap;",
    );
  });

  /// And the one thing at the foot of the pane, which is about this list rather
  /// than about the rest of Verkstead: whether the conversations put away are
  /// drawn in it.
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
    mount();

    const toggle = await waitFor(() =>
      screen.getByLabelText<HTMLInputElement>("Show archived"),
    );
    // Drawn where the server has it rather than where the server had it: the
    // switch is on the page before its read has landed, and off is where one
    // nobody has answered for stands.
    await waitFor(() => expect(toggle.checked).toBe(true));
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
    mount();

    const toggle = await waitFor(() =>
      screen.getByLabelText<HTMLInputElement>("Show archived"),
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
    mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    const read = askedFor(fetching, "/api/ui/conversations");
    fireEvent.click(
      await waitFor(() => screen.getByLabelText("Show archived")),
    );

    await waitFor(() =>
      expect(askedFor(fetching, "/api/ui/conversations")).toBeGreaterThan(read),
    );
  });

  /// The switch is the foot of the pane rather than the last row of the list:
  /// last in the column, with whatever room the conversations leave over taken
  /// above it, and stuck to the bottom edge once there is no room to take. So a
  /// short list stands it against the bottom of the screen and a long one keeps
  /// it there with the cards going under it — which is worth the strip of list
  /// it covers, a list with no end in sight being exactly the one nobody should
  /// have to reach the end of to answer this.
  ///
  /// It is the frame's own foot, which is why the sticking is asked of
  /// `Panes.module.css` here and this pane is asked only that it wears the name.
  ///
  /// Where a thing sits is the stylesheet's, and jsdom lays nothing out: what is
  /// asked here is that it is last in the pane, that it is the pane's foot, and
  /// that the pane is the column that gives a foot its room.
  it("sticks the archived switch to the foot of the pane", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    const pane = container.querySelector(`.${shell.conversationsPane}`)!;
    const foot = pane.lastElementChild!;

    expect(foot.classList.contains(sidebar.showArchived!)).toBe(true);
    expect(foot.classList.contains(shell.paneFoot!)).toBe(true);

    const stuck = shellCss.indexOf("\n.pane > .paneFoot {");
    expect(shellCss.slice(stuck, shellCss.indexOf("\n}", stuck))).toContain(
      "position: sticky;\n  bottom: 0;",
    );

    // Against the bottom edge rather than a padding above it, which is where a
    // negative margin left it: what a sticky box may never be pushed past is
    // its containing block, so the room under a pane's last line is the pane's
    // to stop keeping rather than the foot's to take back.
    expect(shellCss).toContain(
      ".pane:has(> .paneFoot) {\n  padding-bottom: 0;\n}",
    );
    expect(shellCss).toContain("  margin: 1rem -1.25rem 0;\n");

    // And the room over it, which is the column's to give: a list too short to
    // scroll has nothing to stick against, and the switch belongs at the bottom
    // of the pane rather than under the last card.
    const room = shellCss.indexOf("\n.conversationsPane > .paneFoot {");
    expect(shellCss.slice(room, shellCss.indexOf("\n}", room))).toContain(
      "margin-top: auto;",
    );

    const column = shellCss.indexOf("\n.conversationsPane {");
    expect(
      column,
      "expected the frame to make a column of the conversations pane",
    ).toBeGreaterThan(-1);
    expect(shellCss.slice(column, shellCss.indexOf("\n}", column))).toContain(
      "flex-direction: column;",
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

  /// News nobody has looked at draws the same disc, because it says the same
  /// thing to somebody glancing down the list: *look here*. What put it there
  /// is a wrap-up that carried the work to Done while nobody was watching, and
  /// the push that went out about it.
  it("marks a conversation with news on it with the same disc", async () => {
    theSidebar({ state: "Done", working: false, waiting: false, unseen: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(`.${marks.mark}.${marks.waiting}`)).toBeTruthy();
  });

  /// One mark and two reasons for it, so the words are where they are told
  /// apart: a screen reader gets the disc as a sentence rather than as a shape,
  /// and *waiting on you* about a Conversation that only wants reading would be
  /// a job nobody had.
  it("says which of the two the disc is about", async () => {
    theSidebar(
      { branch: "over", state: "Done", working: false, waiting: false, unseen: true },
      { branch: "asking", state: "Grilling", working: false, waiting: true },
      // Both at once, which is a finished Conversation with an ask still
      // answerable on it. The one the human can do something about is the ask.
      { branch: "both", state: "Done", working: false, waiting: true, unseen: true },
    );
    const { container } = mount();

    expect(
      (await cards(container)).map((card) =>
        card.querySelector("button")!.getAttribute("aria-label"),
      ),
    ).toEqual([
      "over, verkstead, Done, not looked at yet",
      "asking, verkstead, Grilling, waiting on you",
      "both, verkstead, Done, waiting on you",
    ]);
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

    // What "draft" means is the stylesheet's, and jsdom lays nothing out. The
    // name rather than the card: a card is a `CardButton` now and is flat, so
    // there is no outline left to draw one as the outline of. Both halves of
    // what the name is drawn as, because the rule says both: leant over, and
    // dimmed to the soft ink a card that has nothing settled on it is written
    // in.
    expect(sidebarCss).toContain(
      ".conversationRow.draft .title {\n  font-style: italic;\n  color: var(--ink-soft);\n}",
    );
  });

  /// A branch name Verkstead invented says nothing about the work, so a row
  /// carrying one is called what it is: a Draft, with the name itself drawn
  /// nowhere. A name somebody typed is the row's name exactly as before.
  it("calls a conversation nobody has named a draft", async () => {
    theSidebar(
      { branch: "amber-kestrel", branch_named: false, state: "Draft" },
      { branch: "rate-limiting", branch_named: true, state: "Draft" },
    );
    const { container } = mount();
    const rows = await cards(container);

    expect(
      rows.map((card) => card.querySelector(`.${sidebar.title}`)!.textContent),
    ).toEqual([DRAFT, "rate-limiting"]);
    expect(container.textContent).not.toContain("amber-kestrel");
  });

  /// And what it is read aloud as opens on the same word: the label is the card
  /// said in words, so a card reading Draft cannot be a label reading anything
  /// else. The state it repeats is said once rather than twice over.
  it("reads an unnamed row aloud as the draft it is drawn as", async () => {
    theSidebar(
      { branch: "amber-kestrel", branch_named: false, state: "Draft" },
      { branch: "rate-limiting", branch_named: true, state: "Draft" },
    );
    const { container } = mount();

    expect(
      (await cards(container)).map((card) =>
        card.querySelector("button")!.getAttribute("aria-label"),
      ),
    ).toEqual(["Draft, verkstead", "rate-limiting, verkstead, Draft"]);
  });

  /// And the Draft carries past the draft. Starting the work is not what makes
  /// an invented name worth reading: the first session is told to replace it, so
  /// the row goes on saying Draft until somebody has settled the name — by
  /// renaming the branch, or by the session ending and leaving it.
  it("keeps calling work on an unnamed branch a draft while it is being named", async () => {
    theSidebar(
      { branch: "amber-kestrel", branch_named: false, naming: true, state: "Grilling" },
      { branch: "brave-otter", branch_named: false, naming: false, state: "Grilling" },
      { branch: "rate-limiting", branch_named: true, naming: false, state: "Implementing" },
    );
    const { container } = mount();
    const rows = await cards(container);

    expect(
      rows.map((card) => card.querySelector(`.${sidebar.title}`)!.textContent),
    ).toEqual([DRAFT, "brave-otter", "rate-limiting"]);
    expect(container.textContent).not.toContain("amber-kestrel");
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
  /// that being the open one is the fill every pressable card in the app says
  /// it with — `CardButton.module.css`'s, rather than a second answer this
  /// sheet gives to the same question.
  it("takes a closed card well down, and marks the open one with a fill", () => {
    expect(sidebarCss).toContain(
      ".conversationRow.ended .open {\n  opacity: 0.45;\n}",
    );
    expect(pressableCss).toContain(
      ".open {\n  background: var(--card);\n}",
    );
    expect(
      sidebarCss,
      "the sidebar says nothing of its own about which card is open",
    ).not.toContain("background-color");
    expect(
      sidebarCss,
      "the inset stripe is retired everywhere it was drawn",
    ).not.toContain("box-shadow: inset 0.2rem");
  });

  /// The fade a finished card takes is the one thing this sheet still says
  /// about the open one, and it says it by taking it back: a card being read is
  /// read at full strength whatever state the work is in. Written after the
  /// fade, which is what makes it outrank it. What is dimmed on a finished card
  /// that is open is what is inside it.
  it("lifts the finished card's fade off the one that is open", () => {
    expect(sidebarCss.indexOf(".conversationRow.selected .open")).toBeGreaterThan(
      sidebarCss.indexOf(".conversationRow.ended .open"),
    );
    expect(sidebarCss).toContain(
      ".conversationRow.selected .open {\n  opacity: 1;\n}",
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

  /// Which rows are still under the hand, by branch. Empty every moment nobody
  /// is dragging, which is what every way a drag can end has to leave behind.
  async function holding(container: ParentNode): Promise<(string | null)[]> {
    return (await cards(container))
      .filter((row) => row.classList.contains(sidebar.held!))
      .map((row) => row.querySelector(`.${sidebar.title}`)!.textContent);
  }

  /// A press on a card, and the hand moved far enough down the pane to lift it
  /// and carry it to the top. What ends the drag is the caller's to say.
  function pickUp(row: HTMLElement): HTMLElement {
    const open = card(row);

    fireEvent.pointerDown(open, {
      button: 0,
      pointerId: 1,
      pointerType: "mouse",
      clientX: 0,
      clientY: 0,
    });
    fireEvent.pointerMove(open, { pointerId: 1, clientX: 0, clientY: 10 });

    return open;
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

  /// The hand can outrun the card it is dragging — out of the row, out of the
  /// sidebar, out of the window — and let go somewhere the card will never hear
  /// about. So the release is listened for at the window rather than at the
  /// card, and this is the drag that says so: nothing is left held, the order
  /// the hand made is what goes, and the next press is a drag of its own.
  it("lets go of a card released away from it", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    pickUp(rows[2]!);
    fireEvent.pointerUp(window, { pointerId: 1 });

    expect(await order(container)).toEqual(["third", "first", "second"]);
    expect(await holding(container), "nothing is under the hand now").toEqual([]);
    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations/order")).toEqual({
        order: [3, 1, 2],
      }),
    );

    // And the list is there to be dragged again, rather than held by a hand
    // that left the window a moment ago.
    dragTo(rows[0]!, 500);
    expect(await order(container)).toEqual(["third", "second", "first"]);
  });

  /// The card loses the pointer the first time the list moves it — a card that
  /// moves in the DOM has the capture taken back off it — so the browser says
  /// so in the middle of every drag there is, one row in. It is not an ending:
  /// the hand is still on the card, the drag goes on to the next row, and what
  /// is sent is the whole of what the hand made rather than its first step.
  it("carries on dragging the card the list moved under it", async () => {
    const fetching = three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    const open = pickUp(rows[2]!);
    expect(await order(container), "one row so far").toEqual([
      "third",
      "first",
      "second",
    ]);

    // What the browser says once the row it captured to has moved in the list.
    fireEvent.lostPointerCapture(open, { pointerId: 1 });
    expect(await holding(container), "still under the hand").toEqual(["third"]);

    // And the hand carries on, down to the second row of the list.
    fireEvent.pointerMove(open, { pointerId: 1, clientX: 0, clientY: 70 });
    expect(await order(container), "and on to the row below").toEqual([
      "first",
      "third",
      "second",
    ]);

    fireEvent.pointerUp(window, { pointerId: 1 });

    expect(await holding(container), "let go where the hand did").toEqual([]);
    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations/order")).toEqual({
        order: [1, 3, 2],
      }),
    );
  });

  /// And a cancel is a release as far as the drag is concerned, wherever it
  /// lands — including the refusal of the scroll that a lifted card hung on the
  /// document.
  it("puts a card down on a cancel that lands away from it", async () => {
    three();
    const { container } = mount();

    const rows = await cards(container);
    laidOut(rows);

    const open = await holdOn(rows[2]!);
    fireEvent.pointerMove(open, { pointerId: 1, clientX: 0, clientY: 10 });
    expect(scrolled(), "the held card, not the list").toBe(true);

    fireEvent.pointerCancel(window, { pointerId: 1 });

    expect(await holding(container), "nothing is under the hand now").toEqual([]);
    expect(scrolled(), "the list again, as on any other day").toBe(false);
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

  /// And the card is a card again straight afterwards. What swallows the click
  /// a drag ends on is spent by that one click: a keyboard press is a click
  /// with no pointer behind it, so anything left standing would leave the card
  /// the drag ended on deaf to Enter until the hand went back to it.
  it("opens from the keyboard on the card a drag just moved", async () => {
    three();
    const { container, history } = mount();

    const rows = await cards(container);
    laidOut(rows);

    dragTo(rows[2]!, 10);
    fireEvent.click(card(rows[2]!));

    expect(await order(container)).toEqual(["third", "first", "second"]);
    expect(history.get(), "the drag itself opened nothing").toBe("/");

    // Enter on the card the hand let go of, which is a click and nothing else
    // — there is no press in front of it to clear anything.
    fireEvent.click(card((await cards(container))[0]!));

    await waitFor(() => expect(history.get()).toBe("/conversations/3"));
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
    ).toEqual([
      actions.resume,
      actions.stop,
      actions.forceStop,
      actions.steer,
      actions.close,
      actions.closeAndArchive,
    ]);
  });

  /// Resume among them, which the sidebar gets for nothing: it is a row of the
  /// one set, so the press that gets a conversation driving again is here as
  /// well as under the status button — and it acts on the card that was
  /// right-clicked, like every other row.
  it("resumes the card that was right-clicked", async () => {
    const resuming = `/api/ui/conversations/${GRILLING.id}/resume`;
    const fetching = theSidebarOver(
      whenever(resuming, json("Resumed" satisfies Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    rightClick(await grillingCard(container));
    fireEvent.click(await drawn(await opened(container), `.${actions.resume}`));

    await waitFor(() => expect(sent(fetching, resuming)).toEqual({}));
    expect(
      askedFor(fetching, `/api/ui/conversations/${OPEN.id}/resume`),
      "the conversation that is open was not the one pressed",
    ).toBe(0);
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

    // The Draft it was opened at, still standing on its composer: the
    // right-click moved neither the selection nor the pane.
    expect(history.get()).toBe(
      `/conversations/${OPEN.id}/events/${BRIEF.id}`,
    );
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

    await openComposer(container);
    await drawn(container, `.${adoption.adoption}`);

    await waitFor(() => screen.getByLabelText("Grilling"));
    expect(screen.getByLabelText("Implementation")).toBeTruthy();

    // And the base inside the Repo panel, which is where every fact about the
    // repository is settled.
    await openRepo(container);
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
    expect(container.querySelector(`.${composer.startGrilling}`)).toBeNull();
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

    await openRepo(container);
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

    await waitFor(() => composerHead());
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
    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${timeline.timeline}`);

    // An entry per Event, drawn as the list the stages after this one add to.
    expect(container.querySelectorAll(`.${timeline.timeline} > .${timeline.timelineEvent}`)).toHaveLength(
      RECORDED.timeline.length,
    );
  });

  /// The pane is titled by its branch, and says which Repo that branch is in
  /// understated beside it — the same two facts in the same order the sidebar's
  /// card says them, so the card and the header of the pane it opens read as
  /// the one name said twice.
  it("says the repo understated beside the branch it is titled by", async () => {
    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const head = await drawn(container, `.${shell.middlePane} .${paneHead.head}`);
    const name = head.querySelector("h1")!;

    expect(name.querySelector(`.${timeline.paneTitle}`)!.textContent).toBe(
      OPEN.branch,
    );
    expect(name.querySelector(`.${timeline.paneRepo}`)!.textContent).toBe(
      OPEN.repo.name,
    );

    // And the two are told apart out loud as well as on screen: the heading is
    // named by everything under it run together, so the space between them is
    // written into the markup rather than left to the stylesheet's gap.
    expect(
      screen.getByRole("heading", {
        name: `${OPEN.branch} ${OPEN.repo.name}`,
      }),
    ).toBe(name);
  });

  /// And on a Draft above all, where the title is the word *Draft* and the Repo
  /// is the whole of what tells this header from the next draft's.
  it("says it on a draft too, where the title is the word Draft", async () => {
    theRecordedWith({ branch_named: false, state: "Draft" });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const head = await drawn(container, `.${shell.middlePane} .${paneHead.head}`);
    const name = head.querySelector("h1")!;

    await waitFor(() =>
      expect(name.querySelector(`.${timeline.paneTitle}`)!.textContent).toBe(
        DRAFT,
      ),
    );
    expect(name.querySelector(`.${timeline.paneRepo}`)!.textContent).toBe(
      OPEN.repo.name,
    );
  });

  /// The subtitle is understated rather than a second title, and neither half
  /// of the name wraps: the header row carries the pane's own controls at its
  /// far end, and a heading that grew downwards was what pushed them onto a
  /// line of their own. The branch is what gives way instead — cut, not wrapped
  /// — and the Repo keeps its place beside it. The stylesheet's, jsdom laying
  /// nothing out.
  it("draws the repo quietly and holds the name to one line", () => {
    expect(timelineCss).toContain(
      ".paneName {\n" +
        "  display: flex;\n" +
        "  flex-wrap: nowrap;\n" +
        "  align-items: baseline;\n" +
        "  gap: 0 0.5rem;\n" +
        "  min-width: 0;\n" +
        "}",
    );
    expect(timelineCss).toContain(
      ".paneName .paneRepo {\n" +
        "  flex: none;\n" +
        "  font-size: 0.9rem;\n" +
        "  font-weight: 400;\n" +
        "  color: var(--ink-soft);\n" +
        "  white-space: nowrap;\n" +
        "}",
    );
  });

  /// And the controls stay at the top right of the pane however long the branch
  /// is called: the group does not shrink, the heading is allowed to, and the
  /// cutting happens inside the name.
  it("holds the pane's controls at the end of the header row", () => {
    expect(timelineCss).toContain(
      ".paneControls {\n" +
        "  display: flex;\n" +
        "  flex: none;\n" +
        "  align-items: center;\n" +
        "  gap: 0.8rem;\n" +
        "}",
    );
    // The zero basis is the load-bearing half: a wrapping row decides where to
    // break on each item's basis, before any shrinking, so an `auto` basis puts
    // the controls on their own line the moment the title alone is wider than
    // the row — however narrow it would then have been allowed to become.
    expect(paneHeadCss).toContain(
      ".head h1 {\n  flex: 1 1 0;\n  min-width: 0;\n  margin: 0;\n}",
    );
  });

  /// The branch itself is cut at the front, with the whole of it in a tooltip:
  /// what tells two branch names apart is their tails, so an ellipsis at the end
  /// would be a column of names that all read the same. The `<bdi>` inside is
  /// what keeps the name the right way round in the right-to-left line the cut
  /// is made with — see `Truncated.tsx`.
  it("cuts a long branch name at the front and keeps the whole in a tooltip", async () => {
    const branch = "timeline-pinned-polish-and-then-some-more-of-it-2026";

    theRecordedWith({ branch, branch_named: true });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const title = await drawn(
      container,
      `.${shell.middlePane} .${paneHead.head} h1 .${timeline.paneTitle}`,
    );

    expect(title.getAttribute("title")).toBe(branch);
    expect(title.querySelector("bdi")!.textContent).toBe(branch);
    expect(truncatedCss).toContain("  direction: rtl;");
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

/// The one route out of a Conversation whose page will not load.
///
/// The read behind the pane has half a dozen ways to fail, and every one of them
/// used to leave the human with a line of error and nothing to press — on the
/// very Conversation they most wanted to be rid of. The presses themselves never
/// needed the reading; only the menu did. So the header is drawn without it, and
/// what it carries is the way out. See `Hatch.tsx`.
describe("the escape hatch on a conversation that will not load", () => {
  /// The workbench with the open Conversation's own read refusing, which is the
  /// state the hatch exists for. `list` replaces the sidebar's answer, for the
  /// tests about what the hatch reads off it.
  function theBrokenConversation(...answers: Parameters<typeof serving>) {
    return serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever("/api/ui/abandoned-roadmaps", json([])),
      whenever(
        `/api/ui/conversations/${OPEN.id}`,
        json({ error: "the Conversation could not be read" }, 500),
      ),
      ...answers,
    );
  }

  /// The sidebar with the open Conversation in whichever state a test is about.
  const listedAs = (state: ConversationEntry["state"]) =>
    whenever(
      "/api/ui/conversations",
      json(
        SIDEBAR.map((entry) =>
          entry.id === OPEN.id ? { ...entry, state } : entry,
        ),
      ),
    );

  const CLOSE_AND_ARCHIVE = `/api/ui/conversations/${OPEN.id}/close-and-archive`;
  const ARCHIVE = `/api/ui/conversations/${OPEN.id}/archive`;

  /// A header where there could be no header: the branch off the sidebar's own
  /// list, the ⋯ the ordinary pane carries, and the error still under it.
  it("draws a header with the ⋯ on it, over the error", async () => {
    theBrokenConversation();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const head = await drawn(container, `.${shell.middlePane} .${paneHead.head}`);
    expect(head.querySelector("h1")?.textContent).toBe(DRAFTING.branch);

    await drawn(container, `.${actions.conversationActions} > .${dropdown.trigger}`);
    await waitFor(() => screen.getByText(/the Conversation could not be read/));
  });

  /// And the way back out, which is the whole reason the hatch is on the pane
  /// rather than on the sidebar's right-click: a phone has no right-click, and
  /// a page it cannot leave is worse than one it cannot read.
  it("carries the way back to the conversations", async () => {
    theBrokenConversation();
    const { container, history } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(await drawn(container, `.${shell.middlePane} .${shell.paneBack}`));

    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// One row, and the one that covers every state: close refuses nothing but a
  /// Conversation that is gone, and an already-closed one still archives.
  it("offers close and archive on a conversation that is not closed", async () => {
    theBrokenConversation();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const menu = await openActions(container);

    expect(menu.querySelector(`.${actions.closeAndArchive}`)).toBeTruthy();
    expect(menu.querySelector(`.${actions.archive}`)).toBeNull();
    expect(menu.querySelector(`.${actions.close}`)).toBeNull();
    expect(menu.querySelector(`.${actions.steer}`)).toBeNull();
    expect(menu.querySelectorAll("button")).toHaveLength(1);
  });

  /// And the same row where the sidebar cannot say either — the list not in
  /// hand, or holding no row for this one. The human's own words: if we cannot
  /// tell which, show only Close and archive, since it covers everything.
  it("offers it too when the list says nothing about this conversation", async () => {
    theBrokenConversation(whenever("/api/ui/conversations", json([])));
    const { container } = mount(`/conversations/${OPEN.id}`);

    const menu = await openActions(container);

    expect(menu.querySelector(`.${actions.closeAndArchive}`)).toBeTruthy();
    expect(menu.querySelector(`.${actions.archive}`)).toBeNull();
  });

  /// Archive stands there instead only where the list says the Conversation is
  /// closed — because Archive on one that is not answers `NotClosed` and goes
  /// nowhere, which is a dead end rather than an escape.
  it("offers archive alone where the list says it is closed", async () => {
    theBrokenConversation(listedAs("Closed"));
    const { container } = mount(`/conversations/${OPEN.id}`);

    const menu = await openActions(container);

    // Waited for rather than read at once: the rows are reactive, so the one
    // the list settles on arrives whenever the list does.
    await drawn(container, `.${actions.archive}`);
    expect(menu.querySelector(`.${actions.closeAndArchive}`)).toBeNull();
    expect(menu.querySelectorAll("button")).toHaveLength(1);
  });

  it("posts to the conversation's own close-and-archive route", async () => {
    const fetching = theBrokenConversation(
      whenever(CLOSE_AND_ARCHIVE, json("Closed" satisfies ConversationClosed), "POST"),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.closeAndArchive}`));

    await waitFor(() => expect(sent(fetching, CLOSE_AND_ARCHIVE)).toEqual({}));
  });

  it("posts to the archive route where that is the row it drew", async () => {
    const fetching = theBrokenConversation(
      listedAs("Closed"),
      whenever(ARCHIVE, json("Archived" satisfies ConversationArchived), "POST"),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.archive}`));

    await waitFor(() => expect(sent(fetching, ARCHIVE)).toEqual({}));
  });

  /// A press that lands leaves, unlike the ordinary menu's, which stays where it
  /// is for the re-read to correct. There is nothing here to stay for: the page
  /// could not be read before the press and will not be read after it. On a
  /// narrow window that is the way back to the list and on a wide one the empty
  /// pane, which is one navigation.
  it("leaves the page a press has landed on", async () => {
    theBrokenConversation(
      whenever(CLOSE_AND_ARCHIVE, json("Closed" satisfies ConversationClosed), "POST"),
    );
    const { container, history } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.closeAndArchive}`));

    await waitFor(() => expect(history.get()).toBe("/"));
    await waitFor(() => screen.getByText("Pick a conversation, or start one."));
  });

  /// The other way a page ends up in front of the hatch, and the one the
  /// incident behind all this really was: a read that never comes back at all.
  /// The server has a deadline of its own on the fetch that hung, and this is
  /// the net under the hang classes nobody has met yet.
  describe("a read that never comes back", () => {
    /// The deadline, as `AbortSignal.timeout` is asked for it.
    const DEADLINE = 30_000;

    /// The signal the read is really given, so the test can fire the deadline
    /// itself rather than sitting through it.
    function atOurOwnPace() {
      const controller = new AbortController();
      const timeout = vi
        .spyOn(AbortSignal, "timeout")
        .mockReturnValue(controller.signal);

      return {
        timeout,
        expire: () =>
          controller.abort(new DOMException("timed out", "TimeoutError")),
      };
    }

    it("gives the conversation's own read a deadline, and nothing else one", async () => {
      const fetching = theWorkbench();
      mount(`/conversations/${OPEN.id}`);
      await waitFor(() => composerHead());

      const signalOf = (path: string) =>
        fetching.mock.calls.find(([asked]) => String(asked) === path)?.[1]
          ?.signal;

      expect(signalOf(READING)).toBeInstanceOf(AbortSignal);
      expect(signalOf("/api/ui/conversations")).toBeUndefined();
    });

    it("draws the hatch on a read hung past its deadline", async () => {
      const { timeout, expire } = atOurOwnPace();

      theWorkbench(whenever(READING, hangs()));
      const { container } = mount(`/conversations/${OPEN.id}`);

      await waitFor(() => screen.getByText("Loading…"));
      expect(timeout).toHaveBeenCalledWith(DEADLINE);

      expire();

      const menu = await openActions(container);
      expect(menu.querySelector(`.${actions.closeAndArchive}`)).toBeTruthy();

      timeout.mockRestore();
    });

    /// And it is not read again on the strength of having given up: three more
    /// deadlines' worth of nothing before the page is allowed to say anything,
    /// and what it would say is what it already knew. An ordinary failure is
    /// still worth the ordinary three attempts, which is the other half of the
    /// rule and the half only this can ask about.
    it("is not one the app makes again", () => {
      const gave_up = new DOMException("timed out", "TimeoutError");

      expect(retrying(0, gave_up)).toBe(false);
      expect(retrying(0, new Error("the server fell over"))).toBe(true);
      expect(retrying(2, new Error("the server fell over"))).toBe(true);
      expect(retrying(3, new Error("the server fell over"))).toBe(false);
    });

    /// And the app really reads that way — the hatch drawn off the one request,
    /// with no second one behind it.
    ///
    /// Through `App` rather than through [`mount`], which is the whole point of
    /// this one: what is being asked about is the app's own query client, and
    /// every mount in this file builds a client that retries nothing, so a page
    /// driven through one would only be asking about the client it built. It is
    /// the reason `resuming.test.tsx` drives `App` too.
    ///
    /// Which matters because the rule is one line, and without it the ordinary
    /// three retries stand: four deadlines' worth of nothing, with a backoff
    /// between each, before the page may say anything at all. That is two
    /// minutes of *Loading…* rather than thirty seconds — near enough to the
    /// hang this whole branch is about that nothing should be able to take the
    /// line away quietly.
    it("draws the hatch off that one read, the app making no second one", async () => {
      const { timeout, expire } = atOurOwnPace();

      const fetching = theWorkbench(whenever(READING, hangs()));
      window.history.pushState({}, "", `/conversations/${OPEN.id}`);
      const { container } = render(() => <App />);

      await waitFor(() => screen.getByText("Loading…"));

      expire();

      const menu = await openActions(container);
      expect(menu.querySelector(`.${actions.closeAndArchive}`)).toBeTruthy();
      expect(askedFor(fetching, READING)).toBe(1);

      timeout.mockRestore();
    });
  });

  /// And a refusal goes where every other refusal in this menu goes: a card
  /// over the page, saying the refusal's own sentence.
  ///
  /// It matters more here than anywhere else the card is drawn. Everywhere else
  /// a refusal is a page drawn against a Conversation that has moved and the
  /// re-read behind the press is the correction; here the reading is the thing
  /// that failed, so a press that went quietly nowhere would leave the human on
  /// a page that will not load with the one way off it apparently doing
  /// nothing.
  it("says over the page that the press did nothing, and stays where it is", async () => {
    theBrokenConversation(
      whenever(
        CLOSE_AND_ARCHIVE,
        json("NoSuchConversation" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container, history } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.closeAndArchive}`));

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);
    expect(said.textContent).toBe(CLOSE_REFUSAL.NoSuchConversation);

    // The menu goes as the card comes up, as it does in the ordinary menu: a
    // dropdown left hanging behind a card is one nobody can see to close.
    expect(container.querySelector(`.${dropdown.drop}`)).toBeNull();
    expect(history.get()).toBe(`/conversations/${OPEN.id}`);
  });

  /// And a request that fell over on the way out is answered the same way: the
  /// press was made and nothing came of it, which is the whole of what the card
  /// is for.
  it("says over the page when the request itself fell over", async () => {
    theBrokenConversation(
      whenever(
        CLOSE_AND_ARCHIVE,
        json({ error: "the server is not answering" }, 503),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.closeAndArchive}`));

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toContain("could not be closed");
    expect(said.textContent).toContain("the server is not answering");
  });
});

/// The composer: the details pane a Conversation is drafted in, which is where
/// the brief is written, the setup settled and the work started from.
///
/// A pane like every other, reached by pressing the Brief card and standing at a
/// path of its own — and the only home of the start button, which the foot of
/// the timeline no longer draws.
describe("the composer pane", () => {
  /// One rule of the stylesheet, by the selector it is written against.
  const rule = (sheet: string, selector: string): string => {
    const opened = sheet.indexOf(`${selector} {`);
    expect(opened, `the stylesheet should have a \`${selector}\` rule`).not.toBe(
      -1,
    );

    return sheet.slice(opened, sheet.indexOf("}", opened));
  };

  /// What the pane is called: the branch the work will be done on, which is
  /// what a draft is named by everywhere else — the sidebar's row, the summary
  /// under a frozen brief — and *Draft* while the name is still the one
  /// Verkstead invented. The composer is the one pane that was called after the
  /// document in it rather than after the work.
  it("is titled by the branch, and Draft until one is named", async () => {
    const titleOf = (container: ParentNode) =>
      container.querySelector(`.${shell.detailsPane} h1`)!.textContent;

    theWorkbench();
    const { container, unmount } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);
    expect(OPEN.branch_named).toBe(true);
    await waitFor(() => expect(titleOf(container)).toBe(OPEN.branch));

    unmount();

    // A name nobody chose is nothing to read, so the pane says what the record
    // is instead.
    theWorkbenchWith({ branch_named: false });
    const again = mount(`/conversations/${OPEN.id}`);

    await drawn(again.container, `.${shell.detailsPane} .${composer.composer}`);
    await waitFor(() => expect(titleOf(again.container)).toBe(DRAFT));
  });

  /// The box asks for a description rather than a line, so it opens at three
  /// lines of one: the line height the page is set in, three times, plus the
  /// padding and the border the field itself takes up.
  it("opens the brief field three lines tall", () => {
    expect(rule(composerCss, ".box .field")).toContain(
      "min-height: calc(3 * 1.5rem + 1rem + 2px)",
    );
  });

  /// The box takes half a rem more padding either side and grows by the same,
  /// so what the padding was taken out of — the line the brief is written on —
  /// is the width it always was. Everything under the box is widened with it,
  /// which is what keeps *Start work* against the box's own edge.
  it("pays for its own padding rather than taking it out of the measure", () => {
    expect(rule(composerCss, ".box")).toContain("padding: 1rem 0.75rem 0.85rem");
    expect(rule(composerCss, ".composer > *")).toContain(
      "max-width: calc(var(--measure) + 1rem)",
    );
  });

  /// And a rem between the field and the row of options under it, the two of
  /// them being one box and two different questions.
  it("keeps a rem between the brief and the setup row", () => {
    expect(rule(setupCss, ".options")).toContain("margin-top: 1rem");
  });

  it("opens at a path of its own, on a cold load of it", async () => {
    theWorkbench();
    const { container } = mount(
      `/conversations/${OPEN.id}/events/${BRIEF.id}`,
    );

    const pane = await drawn(
      container,
      `.${shell.detailsPane} .${composer.composer}`,
    );

    // The whole of what setting a piece of work up is: the brief as a field,
    // everything that has to be settled, and the press.
    expect(
      (pane.querySelector("textarea") as HTMLTextAreaElement).value,
    ).toBe(BRIEF.markdown);
    await drawn(pane, `.${composer.box} .${setup.options}`);
    expect(pane.querySelector(`.${composer.startGrilling}`)).toBeTruthy();
  });

  /// The button was under the record, at the end of everything that had
  /// happened. It is under the setup now, and the timeline draws nothing of it:
  /// the pane is its only home.
  ///
  /// Asked over a record with something else on it, so that there is a timeline
  /// to have taken the button off — a Conversation whose record is the Brief
  /// alone draws no timeline pane at all.
  it("takes the start button off the foot of the timeline", async () => {
    theRecordedWith();
    const { container } = mount(COMPOSING);

    const start = await drawn(
      container,
      `.${shell.detailsPane} .${composer.startGrilling}`,
    );

    expect(
      container
        .querySelector(`.${shell.middlePane}`)!
        .querySelector(`.${composer.startGrilling}`),
    ).toBeNull();

    // Under the box everything there is to settle stands in, which is what it
    // is waiting on.
    expect(
      container
        .querySelector(`.${composer.box}`)!
        .compareDocumentPosition(start) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// The look: one box holding the brief and, along the inside of its bottom
  /// edge, the whole of the setup as a row of dropdowns — each a dimmed label
  /// over its value, the repo first and then the three roles.
  ///
  /// The label lives *inside* the handle on all four, which is what makes the
  /// whole two lines one thing to press and one rectangle to hover. So it is
  /// still what names the control, and says so through `aria-labelledby` on the
  /// three that are listboxes: a name read off the contents would carry the
  /// value into it.
  it("draws every option as a label over its value, inside the box", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const row = await drawn(container, `.${composer.box} > .${setup.options}`);

    await waitFor(() =>
      expect(
        [...row.querySelectorAll(`.${setup.optionLabel}`)].map(
          (label) => label.textContent,
        ),
      ).toEqual(["Repo", "Grilling", "Implementation", "Review"]),
    );

    // Over the value rather than beside it: the repo's label is the first line
    // of its own trigger.
    const repo = row.querySelector(`.${setup.repoOption} > button`)!;
    expect(
      repo
        .querySelector(`.${setup.optionLabel}`)!
        .compareDocumentPosition(repo.querySelector(`.${setup.optionValue}`)!) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    // And each role's is the first line of the listbox's own handle, naming it
    // without being read as part of what it is showing.
    for (const role of ["grilling", "implementation", "review"]) {
      const control = document.getElementById(`${role}-pairing`)!;
      const label = control.querySelector(`.${setup.optionLabel}`)!;

      expect(control.getAttribute("aria-labelledby")).toBe(label.id);
      expect(
        label.compareDocumentPosition(control.querySelector(`.${chrome.line}`)!) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }

    // Named by that label and by nothing else, which is what a combobox is
    // called: the value it is showing is announced separately.
    expect(picker("Grilling").id).toBe("grilling-pairing");
  });

  /// Every one of the four triggers is one rectangle around a label and a value:
  /// the pointer takes an edge around the pair of them, and the keyboard says
  /// where it is by lighting the label rather than by drawing a ring around a
  /// control that has given up every edge it had.
  it("hovers and focuses the whole handle rather than the value in it", () => {
    // The Repo trigger, which is a menu's button and so is painted here — the
    // three listboxes beside it take their hover from `picking.module.css`.
    expect(setupCss).toContain(".repoOption > button:not(:disabled):hover");
    expect(rule(setupCss, ".repoOption > button:not(:disabled):hover")).toContain(
      "border-color: var(--ink-soft)",
    );

    // And the keyboard, on all four: no ring, no accent border, and the dimmed
    // label brought up to the ordinary ink.
    const quiet = rule(
      setupCss,
      ".repoOption > button:focus-visible,\n" +
        ".optionPick > button:focus-visible,\n" +
        '.optionPick > button[aria-expanded="true"],\n' +
        ".repoSelectPick > button:focus-visible,\n" +
        '.repoSelectPick > button[aria-expanded="true"]',
    );
    expect(quiet).toContain("outline: none");
    expect(quiet).toContain("border-color: transparent");

    expect(
      rule(
        setupCss,
        ".repoOption > button:focus-visible .optionLabel,\n" +
          ".optionPick > button:focus-visible .optionLabel,\n" +
          ".repoSelectPick > button:focus-visible .optionLabel",
      ),
    ).toContain("color: var(--ink)");
  });

  /// The caret on each of them is the app's own chevron rather than whatever a
  /// reader's font draws for `▾`, and it stands beside both lines rather than
  /// on the value.
  it("draws a chevron beside the two lines of every trigger", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const row = await drawn(container, `.${composer.box} > .${setup.options}`);

    // Waited for the profiles, which is what the three pairing triggers are
    // drawn off.
    await waitFor(() => expect(picker("Grilling")).toBeTruthy());

    expect(
      row.querySelectorAll(`.${setup.repoOption} > button svg`),
    ).toHaveLength(1);
    expect(row.textContent).not.toContain("▾");

    for (const role of ["Grilling", "Implementation", "Review"]) {
      expect(picker(role).querySelector("svg")).toBeTruthy();
    }

    // Level with the pair of lines rather than with either: it says which way
    // the rows come down, and what they come down from is the whole handle.
    expect(rule(setupCss, ".optionArrow")).toContain("grid-row: 1 / 3");
  });

  /// The rows behind a pairing trigger are twice the width the trigger would
  /// give them, so a reading lands on one line rather than wrapping onto three.
  it("drops the pairing rows at twice the trigger's width", () => {
    const list = rule(setupCss, '.optionPick > [role="listbox"]');

    expect(list).toContain("min-width: 30rem");
    // And still capped against the window, which is what wins on a phone.
    expect(list).toContain("max-width: min(40rem, calc(100vw - 2.5rem))");
  });

  /// And the one control that is not part of the box: what happens to what is
  /// in it rather than something else to fill in.
  it("keeps the start button outside the box", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const pane = await drawn(container, `.${composer.composer}`);
    const start = await drawn(pane, `.${composer.startGrilling}`);

    expect(pane.querySelector(`.${composer.box}`)!.contains(start)).toBe(false);
  });

  /// An adopting draft's brief is the stage's own and comes down frozen, so
  /// there is nothing here to type into: the box is the rendering, and the
  /// adoption stands where the start button would.
  it("locks the box on an adopting draft, and adopts where it would start", async () => {
    const written: ConversationView = {
      ...ADOPTING,
      timeline: ADOPTING.timeline.map((event) =>
        "Brief" in event
          ? {
              Brief: {
                ...event.Brief,
                markdown: "# The stage\n",
                html: "<h1>The stage</h1>",
              },
            }
          : event,
      ),
    };
    theWorkbench(
      whenever(`/api/ui/conversations/${ADOPTING.id}`, json(written)),
    );
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    const pane = await drawn(
      container,
      `.${shell.detailsPane} .${composer.composer}`,
    );

    expect(pane.querySelector(`.${composer.written}`)!.innerHTML).toBe(
      "<h1>The stage</h1>",
    );
    expect(pane.querySelector("textarea")).toBeNull();
    expect(pane.querySelector(`.${adoption.adoption}`)).toBeTruthy();
    expect(pane.querySelector(`.${composer.startGrilling}`)).toBeNull();
  });

  /// And a round whose branch was cut long ago draws what is still the human's
  /// and nothing else: the branch and the base froze when the first round's
  /// work started, and the pairings are re-settled every time work starts.
  it("omits what froze when the branch was cut, and keeps the pairings", async () => {
    theWorkbenchWith({
      worktree: {
        path: "/var/lib/verkstead/worktrees/verkstead-open",
        missing: false,
      },
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    // The Repo option stays, because which repo the work is in is still a fact
    // worth reading — and its picker says for itself that it is settled. Every
    // control behind it is one the server would refuse, and what a control
    // cannot do it does not draw.
    expect(container.querySelector(`.${setup.repoOption}`)).toBeTruthy();

    fireEvent.click(
      await drawn<HTMLButtonElement>(container, `.${setup.repoOption} > button`),
    );
    await waitFor(() => screen.getByLabelText("Repo"));

    expect((screen.getByLabelText("Repo") as HTMLSelectElement).disabled).toBe(
      true,
    );
    expect(container.querySelector(`.${setup.companions}`)).toBeNull();
    expect(screen.queryByLabelText("Branch")).toBeNull();
    expect(screen.queryByLabelText("Base branch")).toBeNull();

    // And the press it is all for is still there.
    expect(container.querySelector(`.${composer.startGrilling}`)).toBeTruthy();
  });
});

/// The brief while the Conversation is drafting: a field that is always there
/// and saves itself, rather than a rendering with a way into a form. On the
/// composer, which is the details pane a Draft opens on — the record's own card
/// is the rendering of it, drafting or frozen.
describe("writing the brief", () => {
  const field = () => screen.getByLabelText("Brief") as HTMLTextAreaElement;

  /// Every textarea in the app trades the browser's focus ring for the accent
  /// border it already draws — two rectangles a pixel apart around a box the
  /// size of a paragraph is the same fact drawn twice. The composer's own has
  /// no border to light up, so the box around it takes the accent instead.
  it("says the focus in the border rather than in a ring", () => {
    expect(base).toContain(
      "textarea:focus-visible {\n  border-color: var(--accent);\n  outline: none;\n}",
    );
  });

  /// Where a save of the Brief goes.
  const WRITING = `/api/ui/conversations/${OPEN.id}/brief`;

  it("is a field on what was last written, with nothing to press to open it", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);
    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);

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
    await waitFor(() => composerHead());

    const growing = container.querySelector(
      `.${composer.composer} .${app.grow}`,
    )!;
    expect(growing.getAttribute("data-value")).toBe(BRIEF.markdown);

    fireEvent.input(field(), { target: { value: "# One\n\n# Two\n" } });
    expect(growing.getAttribute("data-value")).toBe("# One\n\n# Two\n");
  });

  it("saves what was typed when the field is left", async () => {
    const written = "# Rate limiting\n\nDecide where the counter lives.\n";
    const fetching = theWorkbench(json("Saved"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => composerHead());

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
    await waitFor(() => composerHead());

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
    await waitFor(() => composerHead());

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
    await waitFor(() => composerHead());

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
    await waitFor(() => composerHead());

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
    await waitFor(() => composerHead());

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
    await waitFor(() => composerHead());

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
    // something else about the Conversation changed — the repository renamed
    // in the settings — so the test can wait for it to have landed rather than
    // sleep until it has. What is asked here is what a read does to the field,
    // and it does the same whether or not anything came back different.
    //
    // The Repo's name rather than the branch, because this Conversation's
    // record is the Brief alone: there is no Timeline pane and so no header
    // titled by the branch, and what the composer says out loud about the
    // Conversation is the repository the Repo option is labelled with.
    const RENAMED: ConversationView = {
      ...OPEN,
      repo: { ...OPEN.repo, name: "renamed-away" },
    };
    let standing = OPEN;
    const fetching = serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(READING, () => json(standing)()),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);
    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);

    fireEvent.input(field(), { target: { value: "# Half a thought" } });

    standing = RENAMED;
    readAgain();
    await waitFor(() =>
      expect(
        container.querySelector(
          `.${setup.repoOption} .${setup.optionValue}`,
        )!.textContent,
      ).toContain(RENAMED.repo.name),
    );

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
/// it, drawn along the inside of the bottom edge of the box the brief is
/// written in — on the composer, which is where the brief is written.
describe("a conversation's setup", () => {
  /// The brief is what the box holds and the options are its bottom edge,
  /// because setting the work up and kicking it off are one act and both happen
  /// in the one box.
  it("stands inside the box, under the brief itself", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const row = await drawn(
      container,
      `.${shell.detailsPane} .${composer.box} > .${setup.options}`,
    );

    // Four options, the repo first and then the three roles.
    expect(row.querySelector(`.${setup.repoOption}`)).toBeTruthy();
    await waitFor(() =>
      expect(row.querySelectorAll(`.${setup.profileChoice}`)).toHaveLength(3),
    );
    expect(
      [...row.children].indexOf(row.querySelector(`.${setup.repoOption}`)!),
      "the repo is what everything after it is a fact about",
    ).toBe(0);

    // Under the words rather than over them: the brief is what the pane is
    // for, and while it is a draft the words are the field they are typed
    // into.
    const body = container.querySelector(`.${composer.composer} .${app.grow}`)!;
    expect(
      body.compareDocumentPosition(row) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// The branch, the base and the companion repos are all answers to *which
  /// code*, so they are one dropdown rather than four — and one flat panel
  /// rather than a menu walked a level at a time.
  it("puts everything about the repo behind the first option", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openComposer(container);

    // What the trigger reads: the repo, and how many others the work runs
    // alongside.
    const trigger = await drawn(container, `.${setup.repoOption} > button`);
    expect(trigger.textContent).toContain("Repo");
    expect(trigger.textContent).toContain(OPEN.repo.name);
    expect(OPEN.companions.length).toBe(1);
    expect(trigger.textContent).toContain("+1");

    // And nothing of it on the pane until it is pressed.
    expect(screen.queryByLabelText("Branch")).toBeNull();

    const panel = await openRepo(container);
    expect(panel.querySelector(`.${setup.branchName}`)).toBeTruthy();
    expect(panel.querySelector(`.${setup.baseBranch}`)).toBeTruthy();
    expect(panel.querySelector(`.${setup.addCompanion}`)).toBeTruthy();
    expect(panel.querySelector(`.${setup.companions}`)).toBeTruthy();
  });

  /// A Rust repository on a server with no sccache: the work will run, and
  /// every dependency in it will be compiled again. Said at the foot of the
  /// card, which is the last thing read before the work is started — and said
  /// as a note, because nothing here is broken and nothing is gated on it.
  it("warns under the box where compiles will not be cached", async () => {
    theWorkbenchWith({ compiles_uncached: true });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const pane = await drawn(container, `.${composer.composer}`);
    const warning = await drawn(pane, `.${setup.uncached}`);

    expect(warning.textContent).toMatch(/No sccache is installed/);
    expect(
      pane
        .querySelector(`.${composer.box}`)!
        .compareDocumentPosition(warning) & Node.DOCUMENT_POSITION_FOLLOWING,
      "under the box, which is everything there is to settle",
    ).toBeTruthy();
  });

  /// And nothing at all where the server has one — which is every conversation
  /// on a machine that is set up, so it must not be a line the card always
  /// carries.
  it("says nothing about sccache where the server has one", async () => {
    theWorkbenchWith({ compiles_uncached: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${setup.options}`);

    expect(container.querySelector(`.${setup.uncached}`)).toBeNull();
  });

  /// Branch, base and both pairings freeze server-side when grilling starts, so
  /// past that moment there is nothing here that could be changed — and nothing
  /// is drawn rather than drawn disabled.
  it("is gone entirely once the grilling has started", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);

    expect(container.querySelector(`.${setup.options}`)).toBeNull();
    expect(screen.queryByLabelText("Branch")).toBeNull();
    expect(screen.queryByLabelText("Base branch")).toBeNull();
    expect(screen.queryByLabelText("Grilling")).toBeNull();
  });

  /// What the conversation is attached to and where it has got to were three
  /// read-only lines in a pane that no longer exists. The record tells that
  /// story, so they are drawn nowhere at all — the Repo's *name*, which is the
  /// header's subtitle, being the one of the three that came back and a
  /// different fact from the path it is checked out at.
  it("shows the repo path, the worktree path and the state nowhere", async () => {
    theWorkbenchWith({
      state: "Grilling",
      worktree: { path: "/var/lib/verkstead/worktrees/verkstead-open", missing: false },
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    // The record itself, which is what tells the story: where the checkout is
    // stands on the Brief's own pane, under the configuration it was frozen
    // with, and nowhere else.
    const record = await drawn(container, `.${shell.middlePane}`);

    expect(record.textContent).not.toContain(OPEN.repo.path);
    expect(record.textContent).not.toContain(
      "/var/lib/verkstead/worktrees/verkstead-open",
    );
  });

  /// The branch keeps itself the way the brief above it does: there is nothing
  /// to press, and leaving the field is what sends what is in it.
  it("offers the branch name the server prefilled, and sends a new one", async () => {
    const fetching = theWorkbench(json("Renamed"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

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

  /// Until the human types one the name is Verkstead's, and a name nobody chose
  /// is drawn nowhere: the field stands empty under the placeholder that says
  /// what leaving it empty means, and the pane is headed by what the
  /// Conversation is — a draft.
  it("leaves the branch field empty where the name is Verkstead's own", async () => {
    theRecordedWith({ branch_named: false });
    const { container } = mount(COMPOSING);
    await openRepo(container);

    const field = screen.getByLabelText("Branch") as HTMLInputElement;
    expect(field.value).toBe("");
    expect(field.placeholder).toBe(AUTOMATIC);

    // The pane alone, the sidebar beside it being drawn from a list of its own
    // that this test did not touch.
    const pane = container.querySelector(`.${shell.middlePane}`)!;
    expect(pane.querySelector(`.${paneHead.head} .${timeline.paneTitle}`)!.textContent).toBe(DRAFT);
    expect(pane.textContent).not.toContain(OPEN.branch);
  });

  /// And the header goes on saying it once the work has started, for as long as
  /// the name is the first session's to replace. The card has gone by then, so
  /// there is no field left to say anything about — only the title.
  it("heads the pane Draft until the branch has been named", async () => {
    theRecordedWith({
      branch_named: false,
      naming: true,
      state: "Grilling",
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const pane = container.querySelector(`.${shell.middlePane}`)!;
    await waitFor(() =>
      expect(pane.querySelector(`.${paneHead.head} .${timeline.paneTitle}`)!.textContent).toBe(DRAFT),
    );
    expect(pane.textContent).not.toContain(OPEN.branch);
  });

  /// And says the branch the moment nobody is waiting on the name — the session
  /// renamed it, or ended and left it. Nothing reads Draft for ever.
  it("heads the pane with the branch once the naming is over", async () => {
    theRecordedWith({
      branch_named: false,
      naming: false,
      state: "Grilling",
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const pane = container.querySelector(`.${shell.middlePane}`)!;
    await waitFor(() =>
      expect(pane.querySelector(`.${paneHead.head} .${timeline.paneTitle}`)!.textContent).toBe(
        OPEN.branch,
      ),
    );
  });

  /// And emptying it hands the name back, which is a rename like any other: the
  /// server holds the one it prefilled, so there is nothing for the page to send
  /// but the empty field.
  it("hands the name back when the field is emptied", async () => {
    const fetching = theWorkbench(json("Renamed"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    const field = screen.getByLabelText("Branch") as HTMLInputElement;
    expect(field.value).toBe(OPEN.branch);

    fireEvent.input(field, { target: { value: "" } });
    fireEvent.blur(field);

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/branch`)).toEqual({
        branch: "",
      }),
    );
  });

  it("saves a typed branch name after a pause in the typing", async () => {
    const fetching = theWorkbench(json("Renamed"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);
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
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

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
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

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
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

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
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

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
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

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
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

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
    await openRepo(container);

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
    await openRepo(container);
    await waitFor(() => screen.getByLabelText("Base branch"));

    expect(
      container.querySelector(`.${setup.baseBranch} .${notices.note}`),
    ).toBeNull();
    expect(screen.queryByText(/branches from/)).toBeNull();
  });
});

/// Which repo the work is in at all: the first thing in the Repo panel, and the
/// one the branch, the base and the companions under it are facts about.
///
/// What follows a switch — the base back on the new repo's default, a companion
/// that has just become the conversation's own repo dropped — is the server's,
/// and `crates/store/tests/conversations.rs` is what says so. What is asked here
/// is that the press goes out, that the panel reads the record back, and that a
/// branch already cut settles the picker rather than hiding it.
describe("switching a draft's repo", () => {
  /// The picker, waited for: the repos arrive on a read of their own, and a
  /// `<select>` told to show an option it has not been given yet falls to the
  /// first one it has.
  async function repoPicker(container: ParentNode): Promise<HTMLSelectElement> {
    await openRepo(container);

    const picker = (await waitFor(() =>
      screen.getByLabelText("Repo"),
    )) as HTMLSelectElement;

    await waitFor(() => expect(picker.options.length).toBe(REPOS.length));

    return picker;
  }

  /// Every registered repo, the conversation's own among them and showing —
  /// nothing is filtered out, the way nothing is filtered out of the companion
  /// picker beside it.
  it("offers every registered repo, above the branch and the base", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const picker = await repoPicker(container);

    expect([...picker.options].map((option) => option.textContent)).toEqual(
      REPOS.map((repo) => repo.name),
    );
    expect(picker.value).toBe(String(OPEN.repo.id));

    // At the top of the panel, which is the order the panel is read in: the
    // repo, and then everything that is a fact about it.
    const panel = await drawn(container, `.${setup.repoOption} > [role="group"]`);
    const pick = panel.querySelector(`.${setup.repoPick}`)!;
    expect(
      pick.compareDocumentPosition(panel.querySelector(`.${setup.branchName}`)!) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("moves the work onto the repo that was picked", async () => {
    const fetching = theWorkbench(json("Switched"));
    const { container } = mount(`/conversations/${OPEN.id}`);

    const elsewhere = REPOS.find((repo) => repo.id !== OPEN.repo.id)!;
    fireEvent.change(await repoPicker(container), {
      target: { value: String(elsewhere.id) },
    });

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/repo`)).toEqual({
        repo_id: elsewhere.id,
      }),
    );
  });

  /// Said where the press was made, the panel not shutting on a refusal.
  it("says in the panel why a switch was refused", async () => {
    theWorkbench(json("NoSuchRepo"));
    const { container } = mount(`/conversations/${OPEN.id}`);

    const elsewhere = REPOS.find((repo) => repo.id !== OPEN.repo.id)!;
    fireEvent.change(await repoPicker(container), {
      target: { value: String(elsewhere.id) },
    });

    await waitFor(() =>
      expect(screen.getByText(REPO_SWITCH_REFUSAL.NoSuchRepo)).toBeTruthy(),
    );
  });

  /// A later round, steered onto work that is already built: the checkout is of
  /// one repository, so the server refuses the switch and the picker says so by
  /// being disabled. The branch and the base go entirely, as they always have —
  /// this is the one control in the panel drawn in a state it cannot act in,
  /// because which repo the work is in is still a fact worth reading.
  it("reads disabled once the branch has been cut", async () => {
    theWorkbenchWith({
      worktree: { path: "/var/lib/verkstead/worktrees/verkstead-open", missing: false },
    });
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    const picker = (await waitFor(() =>
      screen.getByLabelText("Repo"),
    )) as HTMLSelectElement;

    expect(picker.disabled).toBe(true);
    expect(screen.queryByLabelText("Branch")).toBeNull();
    expect(screen.queryByLabelText("Base branch")).toBeNull();
    expect(screen.queryByLabelText("Works alongside")).toBeNull();
  });

  /// And on a draft adopting a roadmap, which has no worktree at all: a roadmap
  /// is a file in the repo it was found in and only its name is kept, so work
  /// moved elsewhere would be adopting a name rather than a roadmap. The base
  /// and the companions are still the human's here — it is the repo alone that
  /// the roadmap settled.
  it("reads disabled on a draft adopting a roadmap", async () => {
    theWorkbench(
      whenever(`/api/ui/conversations/${ADOPTING.id}`, json(ADOPTING)),
    );
    const { container } = mount(`/conversations/${ADOPTING.id}`);
    await openRepo(container);

    const picker = (await waitFor(() =>
      screen.getByLabelText("Repo"),
    )) as HTMLSelectElement;

    expect(picker.disabled).toBe(true);
    expect(ADOPTING.worktree).toBeNull();
    expect(screen.getByLabelText("Base branch")).toBeTruthy();
  });
});
/// The other repositories a conversation works alongside: added from a picker
/// inside the Repo panel, and drawn as a row apiece under it.
///
/// Whether a repository may be added at all is the server's — its own repo and
/// one already there are refused over there, and the tests in
/// `crates/server/tests/conversations.rs` are what say so. What is asked here is
/// that the press goes out, that the row is drawn, and that a refusal is said in
/// words where it was made.
describe("a conversation's companion repos", () => {
  /// The control they are added from, inside the Repo panel with the branch and
  /// the base they belong beside.
  async function theAdder(container: ParentNode): Promise<HTMLSelectElement> {
    await openRepo(container);

    return (await waitFor(() =>
      screen.getByLabelText("Works alongside"),
    )) as HTMLSelectElement;
  }

  it("offers every registered repo under the invitation to add one", async () => {
    theWorkbenchWith({ companions: [] });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const adder = await theAdder(container);

    // The invitation first, which is what the control shows while nothing has
    // been picked — and then every registered repo, including this
    // conversation's own: what may be added is the server's to say, and a list
    // that quietly left one out would send the human hunting for a repo that is
    // registered.
    expect([...adder.options].map((option) => option.textContent)).toEqual([
      "Add a repo…",
      ...REPOS.map((repo) => repo.name),
    ]);
    expect(adder.value).toBe("");

    // And it is a flat panel rather than a menu walked a level at a time.
    expect(
      container.querySelector(`.${dropdown.drop} [role="menuitem"]`),
    ).toBeNull();
  });

  it("sends the repo that was pressed", async () => {
    const fetching = theWorkbenchWith({ companions: [] }, whenever(
      `/api/ui/conversations/${OPEN.id}/companions`,
      json("Added"),
      "POST",
    ));
    const { container } = mount(`/conversations/${OPEN.id}`);

    const adder = await theAdder(container);
    fireEvent.change(adder, { target: { value: String(REPOS[1]!.id) } });

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/companions`),
      ).toEqual({ repo_id: REPOS[1]!.id }),
    );

    // The panel stays where it is — the row appearing under the control is the
    // confirmation — and the control goes back to the invitation, what was
    // picked having been done rather than held.
    expect(container.querySelector(`.${dropdown.drop}`)).toBeTruthy();
    await waitFor(() => expect(adder.value).toBe(""));
  });

  /// A refusal is not an error: it is a sentence about the repository that was
  /// picked, and it is said under the control it was picked from — which is
  /// still open, because a panel that shut would take the only place it had
  /// left to be said.
  it("says a refusal where the pick was made, and stays open to say it", async () => {
    theWorkbenchWith({ companions: [] }, whenever(
      `/api/ui/conversations/${OPEN.id}/companions`,
      json("OwnRepo"),
      "POST",
    ));
    const { container } = mount(`/conversations/${OPEN.id}`);

    const adder = await theAdder(container);
    fireEvent.change(adder, { target: { value: String(OPEN.repo.id) } });

    await waitFor(() =>
      expect(
        container.querySelector(`.${setup.addCompanion} .${setup.failure}`)
          ?.textContent,
      ).toBe(COMPANION_REFUSAL.OwnRepo),
    );
    expect(container.querySelector(`.${dropdown.drop}`)).toBeTruthy();
  });

  /// A row apiece in the Repo panel, naming the repository.
  it("draws a row per companion under the control they were added from", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);
    const panel = await openRepo(container);

    const rows = await drawn(panel, `.${setup.companions}`);
    expect([...rows.querySelectorAll(`.${setup.companionName}`)].map(
      (name) => name.textContent,
    )).toEqual(OPEN.companions.map((companion) => companion.repo.name));

    // Under the control rather than over it: it is what they were added from.
    const adder = panel.querySelector(`.${setup.addCompanion}`)!;
    expect(
      adder.compareDocumentPosition(rows) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("takes one away when its × is pressed", async () => {
    const companion = OPEN.companions[0]!;
    const removing = `/api/ui/conversations/${OPEN.id}/companions/${companion.repo.id}/remove`;
    const fetching = theWorkbench(whenever(removing, json("Removed"), "POST"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    fireEvent.click(
      await waitFor(() =>
        screen.getByRole("button", { name: `Remove ${companion.repo.name}` }),
      ),
    );

    await waitFor(() => expect(writes(fetching, removing)).toBe(1));
  });

  it("says why a removal was refused, on the row it was refused for", async () => {
    const companion = OPEN.companions[0]!;
    const removing = `/api/ui/conversations/${OPEN.id}/companions/${companion.repo.id}/remove`;
    theWorkbench(whenever(removing, json("NotDrafting"), "POST"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    fireEvent.click(
      await waitFor(() =>
        screen.getByRole("button", { name: `Remove ${companion.repo.name}` }),
      ),
    );

    await waitFor(() =>
      expect(
        container.querySelector(`.${setup.companion} .${setup.failure}`)
          ?.textContent,
      ).toBe(COMPANION_REMOVAL_REFUSAL.NotDrafting),
    );
  });

  /// A conversation with none has nothing here to say, so nothing is drawn —
  /// not an empty list with a heading over it.
  it("draws nothing at all where there are none", async () => {
    theWorkbenchWith({ companions: [] });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const panel = await openRepo(container);

    expect(panel.querySelector(`.${setup.companions}`)).toBeNull();

    // And the way to add one is still there: having none is the state most
    // conversations are in rather than a thing to be locked out of.
    expect(panel.querySelector(`.${setup.addCompanion}`)).toBeTruthy();

    // The trigger says the repo alone, there being nothing to count.
    expect(
      container.querySelector(`.${setup.repoOption} > button`)!.textContent,
    ).not.toContain("+");
  });

  /// The rows freeze with the branch and the base, because the server freezes
  /// all three at the same moment: a row whose × comes back refused is worse
  /// than no row.
  it("goes with the branch and the base when the card freezes", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);

    expect(container.querySelector(`.${setup.companions}`)).toBeNull();
    expect(container.querySelector(`.${setup.repoOption}`)).toBeNull();
  });
});


/// What a companion row settles about the repository it names: where its
/// checkout comes off, whether the work may write to it, and — where it may —
/// what its branch is called.
///
/// Whether any of it may be changed at all is the server's, and the tests in
/// `crates/server/tests/conversations.rs` are what say so. What is asked here
/// is that each control draws what the record holds, that its save goes out,
/// and that a refusal is said on the row it was refused for.
describe("configuring a companion repo", () => {
  /// The fixture's one companion, which every test here alters.
  const ASKANCE = OPEN.companions[0]!;

  /// Where each of the three saves goes.
  const MODE = `/api/ui/conversations/${OPEN.id}/companions/${ASKANCE.repo.id}/mode`;
  const BASE = `/api/ui/conversations/${OPEN.id}/companions/${ASKANCE.repo.id}/base`;
  const NAMING = `/api/ui/conversations/${OPEN.id}/companions/${ASKANCE.repo.id}/branch`;

  /// The workbench with that companion altered — and both repositories'
  /// branches served, the conversation's row and the companion's each having a
  /// dropdown over a list of its own.
  function theCompanion(
    over: Partial<CompanionView>,
    ...answers: Parameters<typeof serving>
  ) {
    return theWorkbenchWith(
      { companions: [{ ...ASKANCE, ...over }] },
      whenever(`/api/ui/repos/${OPEN.repo.id}/branches`, json(BRANCHES)),
      whenever(
        `/api/ui/repos/${ASKANCE.repo.id}/branches`,
        json(COMPANION_BRANCHES),
      ),
      ...answers,
    );
  }

  /// What one of that row's controls is called: the words on the label, and
  /// the repository it belongs to, which the label carries where nobody sees
  /// it — a panel with two companions in it has two of every control.
  const named = (control: string) => `${control} for ${ASKANCE.repo.name}`;

  /// The branch field on the companion's row, which is told from the
  /// conversation's own by the repository its label names.
  function branchField(): HTMLInputElement {
    return screen.getByLabelText(named("Branch")) as HTMLInputElement;
  }

  /// The rule first and then the branches, as the conversation's own base
  /// dropdown offers them — but read off the companion's repository, which is a
  /// different repository with a default branch and a list of its own.
  it("offers the companion repo's own rule and then its own branches", async () => {
    theCompanion({});
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    const picker = (await waitFor(() =>
      screen.getByLabelText(named("Base")),
    )) as HTMLSelectElement;

    await waitFor(() => expect(picker.options).toHaveLength(3));
    expect([...picker.options].map((option) => option.value)).toEqual([
      "",
      ...COMPANION_BRANCHES,
    ]);
    expect(picker.options[0]!.textContent).toContain(
      ASKANCE.repo.default_branch,
    );
    expect(picker.options[0]!.textContent).not.toContain(
      OPEN.repo.default_branch,
    );
  });

  it("records the base branch that was picked, by name", async () => {
    const fetching = theCompanion({}, whenever(BASE, json("Recorded"), "POST"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    const picker = (await waitFor(() =>
      screen.getByLabelText(named("Base")),
    )) as HTMLSelectElement;
    await waitFor(() =>
      expect([...picker.options].map((option) => option.value)).toContain(
        "trunk",
      ),
    );

    fireEvent.change(picker, { target: { value: "trunk" } });

    await waitFor(() =>
      expect(sent(fetching, BASE)).toEqual({ branch: "trunk" }),
    );
  });

  /// The switch is what says whether the work may commit to the repository, and
  /// it is the one control on the row that changes what the row is: there is no
  /// branch to name until it is on.
  it("flips to read-write, and there is nothing to name until it is on", async () => {
    const fetching = theCompanion({}, whenever(MODE, json("Chosen"), "POST"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    const toggle = (await waitFor(() =>
      screen.getByLabelText(named("Read-write")),
    )) as HTMLInputElement;
    expect(toggle.checked).toBe(false);
    expect(
      container.querySelector(`#companion-${ASKANCE.repo.id}-branch`),
    ).toBeNull();

    fireEvent.click(toggle);

    await waitFor(() =>
      expect(sent(fetching, MODE)).toEqual({ mode: "ReadWrite" }),
    );
  });

  /// A read-write companion the human has never typed a name into is
  /// *mirroring*: the field is drawn prefilled with the conversation's own
  /// branch, so what they read is what they will get.
  it("prefills a mirroring branch with the conversation's own name", async () => {
    theCompanion({ mode: "ReadWrite", branch: "" });
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    await waitFor(() => expect(branchField().value).toBe(OPEN.branch));
  });

  /// And it follows that name: renaming the conversation's branch renames this
  /// one with it, for as long as nothing has been typed here.
  it("moves a mirroring branch with the conversation's own", async () => {
    theWorkbenchWith(
      {
        branch: "counter-in-redis",
        companions: [{ ...ASKANCE, mode: "ReadWrite", branch: "" }],
      },
      whenever(`/api/ui/repos/${OPEN.repo.id}/branches`, json(BRANCHES)),
      whenever(
        `/api/ui/repos/${ASKANCE.repo.id}/branches`,
        json(COMPANION_BRANCHES),
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    await waitFor(() => expect(branchField().value).toBe("counter-in-redis"));
  });

  /// A name that has been typed stands on its own and stops following.
  it("leaves a named branch where it is, whatever the conversation's is called", async () => {
    theCompanion({ mode: "ReadWrite", branch: "alongside" });
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    await waitFor(() => expect(branchField().value).toBe("alongside"));
    expect(branchField().value).not.toBe(OPEN.branch);
  });

  /// It keeps itself the way the conversation's own branch field does: there is
  /// nothing to press, and leaving the field is what sends what is in it.
  it("saves a typed branch name on the way out of the field", async () => {
    const fetching = theCompanion(
      { mode: "ReadWrite", branch: "" },
      whenever(NAMING, json("Renamed"), "POST"),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);
    await waitFor(() => branchField());

    expect(screen.queryByRole("button", { name: "Rename" })).toBeNull();
    fireEvent.input(branchField(), { target: { value: "alongside" } });
    fireEvent.blur(branchField());

    await waitFor(() =>
      expect(sent(fetching, NAMING)).toEqual({ branch: "alongside" }),
    );
  });

  /// And clearing it is going back to mirroring rather than a branch called
  /// nothing — so the record is asked to hold no name, and the field fills
  /// itself in again with what mirroring will now give.
  it("goes back to mirroring when the field is cleared", async () => {
    let held = "alongside";
    const fetching = serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/repos/${OPEN.repo.id}/branches`, json(BRANCHES)),
      whenever(
        `/api/ui/repos/${ASKANCE.repo.id}/branches`,
        json(COMPANION_BRANCHES),
      ),
      // The record as the save leaves it, so that what the field falls back to
      // is read rather than assumed.
      whenever(`/api/ui/conversations/${OPEN.id}`, () =>
        json({
          ...OPEN,
          companions: [{ ...ASKANCE, mode: "ReadWrite", branch: held }],
        })(),
      ),
      whenever(
        NAMING,
        () => {
          held = "";
          return json("Renamed")();
        },
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);
    await waitFor(() => expect(branchField().value).toBe("alongside"));

    fireEvent.input(branchField(), { target: { value: "" } });
    fireEvent.blur(branchField());

    await waitFor(() => expect(sent(fetching, NAMING)).toEqual({ branch: "" }));
    await waitFor(() => expect(branchField().value).toBe(OPEN.branch));
  });

  /// A refusal is a sentence about the row it was refused for, said on that
  /// row: the card draws one per companion, and a refusal said anywhere else
  /// would be about a repository the human has to guess at.
  it("says why a change was refused, on the row it was refused for", async () => {
    theCompanion({}, whenever(MODE, json("NotDrafting"), "POST"));
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);

    fireEvent.click(await waitFor(() => screen.getByLabelText(named("Read-write"))));

    await waitFor(() =>
      expect(
        container.querySelector(`.${setup.companion} .${setup.failure}`)
          ?.textContent,
      ).toBe(COMPANION_MODE_REFUSAL.NotDrafting),
    );
  });

  /// The same for the field, which is refused for a name git will not take as
  /// well as for a conversation that has moved on — and heals, for the reason
  /// the conversation's own branch field does: the keystroke after a refusal
  /// may well be what fixes it.
  it("says why a branch name was refused, and heals when it is valid", async () => {
    let outcome = "NotABranchName";
    const fetching = theCompanion(
      { mode: "ReadWrite", branch: "" },
      whenever(NAMING, () => json(outcome)(), "POST"),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);
    await openRepo(container);
    await waitFor(() => branchField());

    fireEvent.input(branchField(), { target: { value: "two..dots" } });
    fireEvent.blur(branchField());

    await waitFor(() =>
      expect(screen.getAllByText(COMPANION_BRANCH_REFUSAL.NotABranchName)),
    );

    outcome = "Renamed";
    fireEvent.input(branchField(), { target: { value: "two-dots" } });
    fireEvent.blur(branchField());

    await waitFor(() => expect(writes(fetching, NAMING)).toBe(2));
    await waitFor(() =>
      expect(
        screen.queryByText(COMPANION_BRANCH_REFUSAL.NotABranchName),
      ).toBeNull(),
    );
  });

  /// The three of them go with the branch and the base when the card freezes,
  /// because the server freezes every one of them at the same moment.
  it("goes with the rest of the setup when the card freezes", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);

    expect(screen.queryByLabelText(named("Base"))).toBeNull();
    expect(screen.queryByLabelText(named("Read-write"))).toBeNull();
  });
});

/// What one row of a pairing picker sends, as the picker writes it.
const pairing = (profile: ProfileEntry, model: string) =>
  `${profile.id}:${model}`;

/// And what every row of one reads as, in the order the fixture's profiles come
/// to.
///
/// Spelled out rather than composed from the fixture, because the composing is
/// the thing under test: the backend's name, the model's own name, and the
/// profile's after an em dash — said here because the fixture holds three Claude
/// Code accounts, so which of them a row is cannot be read off the model alone.
const READINGS = [
  "Claude Code Fable 5 — fable",
  "Claude Code Opus 5 — opus",
  "Claude Code Haiku 4.5 — opus",
  "Claude Code Sonnet 5 — sonnet",
];

/// And what the *closed* trigger in the setup row reads, which is every one of
/// those less the backend's name: the harness's mark is drawn beside the words
/// there, so the words that name the harness would be saying it twice. The rows
/// behind the trigger keep the whole reading — a list is read down.
const SHOWINGS = READINGS.map((reading) =>
  reading.replace("Claude Code ", ""),
);

/// Which row a Pairing the server has settled is, in the order the profiles come
/// to — so what a picker shows is checked against the row it came off rather
/// than against the composing being done again beside it.
const rowOf = (view: PairingView): number =>
  PROFILES.flatMap((profile) =>
    profile.models.map((model) => pairing(profile, model)),
  ).indexOf(pairing(view.profile, view.model!));

/// How one of them reads in a list of rows, and how the trigger it was picked on
/// says the same choice.
const readsAs = (view: PairingView): string => READINGS[rowOf(view)]!;
const showsAs = (view: PairingView): string => SHOWINGS[rowOf(view)]!;

/// The last thing a conversation settles before anything will run it: which
/// account and model grills, and which implements.
describe("a conversation's pairings", () => {
  /// A conversation showing neither choice, which is what a freshly started one
  /// looks like.
  const UNCHOSEN: ConversationView = {
    ...OPEN,
    grilling_pairing: "Nothing",
    implementation_pairing: null,
    review_pairing: "Nothing",
    ready_to_grill: false,
  };

  function withConversation(
    view: ConversationView,
    ...answers: Parameters<typeof serving>
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

  it("shows the pairings the conversation has chosen", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    // Separate choices, and in the fixture genuinely separate accounts: grill on
    // fable, implement on opus, review on sonnet.
    //
    // The fixture picks a Pairing for the grilling, which is one of that
    // picker's rows; the other says there is to be no grilling at all.
    const interviewing = under(OPEN.grilling_pairing)!;
    const reviewed = under(OPEN.review_pairing)!;

    // The trigger's own reading, which is the row's less the harness the mark
    // beside it is already drawing.
    expect(showing("Grilling")).toBe(showsAs(interviewing));
    expect(showing("Implementation")).toBe(
      showsAs(OPEN.implementation_pairing!),
    );
    expect(showing("Review")).toBe(showsAs(reviewed));
    expect(
      new Set([
        showing("Grilling"),
        showing("Implementation"),
        showing("Review"),
      ]).size,
    ).toBe(3);
  });

  /// One flat row per profile-and-model combination, read as the backend and the
  /// model — a profile listing two models is two rows, because a session runs
  /// one of them and the pick says which.
  ///
  /// And the profile's own name after each, because the fixture's three accounts
  /// are all Claude Code ones: with more than one of a backend saved, the name is
  /// the whole of what tells two rows on one model apart.
  it("offers every profile-and-model combination as one flat list", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    expect(offers("Implementation")).toEqual(READINGS);
  });

  /// And what a row sends is untouched by how it reads: the profile's id and the
  /// model's own, which is what a session is launched with.
  ///
  /// Every row rather than one, because the guarantee is that what was read off
  /// a row is what that row sends — one row proving it would leave the other
  /// three to a coincidence, and the reading is composed now rather than being
  /// the two ids it used to be.
  it("sends the profile and model of whichever row was read", async () => {
    const fetching = withConversation(UNCHOSEN, json("Chosen"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Implementation"));

    const wire = PROFILES.flatMap((profile) =>
      profile.models.map((model) => ({ profile_id: profile.id, model })),
    );

    for (const [index, reading] of READINGS.entries()) {
      // Pickable again: the control is disabled while a choice is in flight, so
      // a walk down the list waits for the one before it to land — which is the
      // same thing a hand at the card has to do.
      await waitFor(() => expect(picker("Implementation").disabled).toBe(false));
      pick("Implementation", reading);

      // The `index`th write to that path, this being the only test that makes
      // more than one of them.
      await waitFor(() =>
        expect(
          sent(
            fetching,
            `/api/ui/conversations/${OPEN.id}/implementation-pairing`,
            index,
          ),
        ).toEqual(wire[index]),
      );
    }
  });

  /// A Grok Build account beside the fixture's Claude Code ones, so that a
  /// picker has two harnesses in it: one account apiece, which is also the case
  /// where the reading says no profile name.
  const MIXED: ProfileEntry[] = [
    PROFILES[0]!,
    {
      account: { agent_type: "Grok", home: "/srv/accounts/grok" },
      broken: null,
      id: 9,
      models: ["grok-4.6"],
      name: "grok",
    },
  ];

  /// Each row under the mark of the harness it runs, which is what the pickers
  /// were drawn by hand for: a reader picks the account they mean out of a column
  /// by its shape before reading a word of any of them.
  it("draws every row under the mark of the harness it runs", async () => {
    withConversation(UNCHOSEN, whenever("/api/ui/profiles", json(MIXED)));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Implementation"));

    expect(offers("Implementation")).toEqual([
      "Claude Code Fable 5",
      "Grok 4.6",
    ]);
    expect(offered("Implementation").map(marked)).toEqual([
      art(claudeMarkFile),
      art(grokMarkFile),
    ]);
  });

  /// And the closed control draws the chosen row exactly as the list drew it: a
  /// control showing one thing and offering the same thing drawn differently is
  /// a control the eye has to check.
  it("draws the chosen pairing's mark on the closed control", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    expect(marked(picker("Grilling"))).toBe(art(claudeMarkFile));
    expect(marked(picker("Implementation"))).toBe(art(claudeMarkFile));
    expect(marked(picker("Review"))).toBe(art(claudeMarkFile));
  });

  /// The row that runs nothing is not an account, so there is no harness for a
  /// mark to be of: it draws its words and no element at all, rather than a gap
  /// where one would have been. And so does the picker sitting on it.
  it("draws the no-session row as words alone", async () => {
    withConversation({ ...UNCHOSEN, review_pairing: "Skipped" });
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Review"));

    expect(marked(offered("Grilling")[0]!)).toBeNull();
    expect(offers("Grilling")[0]).toBe("No grilling");
    expect(marked(picker("Review"))).toBeNull();
    expect(showing("Review")).toBe("No review");
  });

  /// A backend with one account saved says no name at all: there is nothing for
  /// it to tell apart, and the model is the whole of the choice.
  it("says only the backend and the model where a backend has one account", async () => {
    withConversation(
      UNCHOSEN,
      whenever("/api/ui/profiles", json([PROFILES[0]])),
    );
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    expect(offers("Implementation")).toEqual(["Claude Code Fable 5"]);
    // The placeholder is what the closed control says rather than a row of the
    // list: there is no unpicking here, so it is not something to go back to.
    expect(showing("Implementation")).toBe("Not chosen");
  });

  it("sends each choice on its own, to its own role", async () => {
    const fetching = withConversation(UNCHOSEN, json("Chosen"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    pick("Grilling", READINGS[0]!);
    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/grilling-pairing`),
      ).toEqual({
        pairing: {
          profile_id: PROFILES[0]!.id,
          model: PROFILES[0]!.models[0],
        },
      }),
    );

    // The second of a profile's models, which is the half of the choice a
    // profile alone could never have said.
    pick("Implementation", READINGS[2]!);
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

  /// Two of the pickers have a row that is no account at all, and it is one of
  /// the rows rather than a switch beside them: what runs this, and one of the
  /// answers is nobody. The implementation picker has none, there being no work
  /// without something building it.
  it("offers the no-session row on the grilling and review pickers alone", async () => {
    withConversation(UNCHOSEN);
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Review"));

    // Above the accounts, the row that says none of them will read this branch.
    expect(offers("Review")).toEqual(["No review", ...READINGS]);
    expect(offers("Grilling")).toEqual(["No grilling", ...READINGS]);
    expect(offers("Implementation")).toEqual(READINGS);

    // And nothing picked yet on any of them, which the closed control says
    // rather than offering it as a row.
    expect(showing("Review")).toBe("Not chosen");
    expect(showing("Grilling")).toBe("Not chosen");
    expect(showing("Implementation")).toBe("Not chosen");
  });

  /// And picking it sends a choice rather than the absence of one, exactly as
  /// the review row does: the brief goes straight to the work.
  it("sends no grilling as the choice it is", async () => {
    const fetching = withConversation(UNCHOSEN, json("Chosen"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    pick("Grilling", "No grilling");

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/grilling-pairing`),
      ).toEqual({ pairing: null }),
    );
  });

  /// And a picker already on it keeps it, the placeholder not being drawn over a
  /// settled choice.
  it("shows no grilling as what is chosen where it is", async () => {
    withConversation({ ...UNCHOSEN, grilling_pairing: "Skipped" });
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    expect(showing("Grilling")).toBe("No grilling");
  });

  /// And picking it sends a choice rather than the absence of one: an untouched
  /// picker and a picker moved to that row leave the same column empty, and only
  /// one of them lets the work start.
  it("sends no review as the choice it is", async () => {
    const fetching = withConversation(UNCHOSEN, json("Chosen"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Review"));

    pick("Review", "No review");

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/review-pairing`),
      ).toEqual({ pairing: null }),
    );
  });

  /// And picking an account sends the pairing under the same key, because it is
  /// the same press on the same picker.
  it("sends a review pairing under the same key", async () => {
    const fetching = withConversation(UNCHOSEN, json("Chosen"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Review"));

    pick("Review", READINGS[0]!);

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/review-pairing`),
      ).toEqual({
        pairing: {
          profile_id: PROFILES[0]!.id,
          model: PROFILES[0]!.models[0],
        },
      }),
    );
  });

  /// A picker already on that row keeps it: it is a settled choice, so the
  /// placeholder is not drawn over it the way it is over an empty one.
  it("shows no review as what is chosen where it is", async () => {
    withConversation({ ...UNCHOSEN, review_pairing: "Skipped" });
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Review"));

    expect(showing("Review")).toBe("No review");
  });

  /// A profile chosen before models were paired with them is half a choice: the
  /// picker draws it as none, and says so where it would have been shown.
  it("reads a profile with no model beside it as nothing chosen", async () => {
    withConversation({
      ...OPEN,
      grilling_pairing: { Under: { ...under(OPEN.grilling_pairing)!, model: null } },
      ready_to_grill: false,
    });
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => picker("Grilling"));
    expect(showing("Grilling")).toBe("Not chosen");
    await waitFor(() => screen.getByText(/was chosen before models were/));
  });

  /// Both pairings are fixed when grilling starts, and the refusal is the
  /// server's to make: the picker is drawn the same and says what came back.
  it("says a choice was refused once the grilling has started", async () => {
    withConversation(OPEN, json("NotDrafting"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => picker("Grilling"));

    pick("Grilling", READINGS[0]!);

    await waitFor(() =>
      screen.getByText(
        "The work has started, so who runs this conversation is settled.",
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

  /// The three stand in the option row beside the Repo, with no heading of
  /// their own: the role is written on each one, so a word over all three would
  /// be the row saying what its labels already say.
  it("draws the pickers as three options of the one row", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const row = await drawn(container, `.${setup.options}`);
    await waitFor(() =>
      expect(row.querySelectorAll(`.${setup.profileChoice}`)).toHaveLength(3),
    );
    expect(row.querySelector("h3")).toBeNull();
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
    await waitFor(() => picker("Grilling"));

    pick("Grilling", READINGS[0]!);

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
  /// Asked over a record with something on it beyond the Brief, which is what
  /// makes three levels to walk: a Conversation whose record is the one Event
  /// draws no Timeline at all, and the walk skips it — see *the one-event
  /// layout* below.
  it("starts on the conversations and walks in and back out", async () => {
    theRecordedWith();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    // Nothing picked, so the level being shown is the list.
    expect(frame(container).dataset.pane).toBe("conversations");

    fireEvent.click(screen.getByText(DRAFTING.branch));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("middle"),
    );

    // The third level is the open Event's own, and the landing opens the newest
    // thing on the record — so there is something to page into, and the way on
    // is offered.
    await waitFor(() => screen.getByRole("button", { name: "Details →" }));

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
    theRecordedWith();
    const { container, history } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    fireEvent.click(screen.getByText(DRAFTING.branch));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("middle"),
    );
    expect(history.get()).toBe(`/conversations/${DRAFTING.id}`);
    await drawn(container, `.${timeline.timeline}`);

    fireEvent.click(screen.getByRole("button", { name: "← Conversations" }));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("conversations"),
    );
    expect(history.get()).toBe("/");

    fireEvent.click(screen.getByText(DRAFTING.branch));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("middle"),
    );
  });

  /// So walking in to the third level is opening something, and the way forward
  /// is what stands afterwards: it is drawn for a selection rather than for a
  /// conversation.
  ///
  /// Waited on, because opening something is a navigation now — the pane the
  /// path names is drawn once the router has moved to it.
  it("walks on to the details by opening an event, and back out again", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    // The end of the record is open, the landing having picked it — and the
    // phone is still on the Timeline all the same: what the level follows is
    // the Conversation changing, and opening the newest thing changes none. So
    // the way forward is offered rather than taken.
    expect(frame(container).dataset.pane).toBe("middle");
    await waitFor(() => screen.getByRole("button", { name: "Details →" }));

    fireEvent.click(output);
    expect(frame(container).dataset.pane).toBe("details");

    // Waited for before its way back is taken: the landing had already opened a
    // pane on the way in, so a "← Timeline" was there to be picked up before the
    // router had moved to this one — and the button of a pane that has since
    // been replaced is a button attached to nothing.
    await drawn(container, `.${shell.detailsPane} .${outputPane.recordSwitch}`);

    fireEvent.click(screen.getByRole("button", { name: "← Timeline" }));
    expect(frame(container).dataset.pane).toBe("middle");

    // Still open, so the way forward is there to be taken again.
    fireEvent.click(screen.getByRole("button", { name: "Details →" }));
    expect(frame(container).dataset.pane).toBe("details");
  });

  /// Opening a Conversation is a navigation, and Back is a way of changing which
  /// one is open that never goes through a click handler.
  it("follows the URL rather than the button that changed it", async () => {
    theRecordedWith();
    const { container, history } = mount(`/conversations/${OPEN.id}`);
    await drawn(container, `.${timeline.timeline}`);

    expect(frame(container).dataset.pane).toBe("middle");

    history.set({ value: "/" });
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("conversations"),
    );
  });

  /// What `data-pane` means is the stylesheet's, and there is nothing to query
  /// it off: jsdom lays nothing out. So the rules themselves are what is read.
  it("is one pane at a time until the window is wide enough for more", () => {
    expect(shellCss).toContain(".panes > .pane {\n  display: none;\n}");
    expect(shellCss).toContain(
      '.panes[data-pane="conversations"] > .conversationsPane,\n' +
        '.panes[data-pane="middle"] > .middlePane,\n' +
        '.panes[data-pane="details"] > .detailsPane {\n' +
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
  /// paper fading to nothing, over the first rem of whatever is passing beneath
  /// — and only while something is passing, there being no gap below the block
  /// for a fade to hang in. What says so is `data-stuck`, which the block puts
  /// on itself off an observer of the rem of pane above it.
  it("fades the record out under whatever is stuck, and only then", () => {
    const fade =
      '  content: "";\n' +
      "  position: absolute;\n" +
      "  top: 100%;\n" +
      "  right: 0;\n" +
      "  left: 0;\n" +
      "  height: 1rem;\n" +
      "  background: linear-gradient(var(--paper), transparent);\n" +
      "  pointer-events: none;\n}";

    expect(shellCss).toContain(
      ".pane > .paneChrome[data-stuck]::after {\n" + fade,
    );
    // Nothing draws it otherwise: a pane at rest wears no paper over its first
    // line.
    expect(shellCss).not.toContain(".pane > .paneChrome::after");

    // And the block keeps no room under itself for one, the room between a
    // header and what follows it being the header's own — only the pane's own
    // padding, given back either side so the paper reaches the pane's edges.
    expect(shellCss).toContain("  margin: 0 -1.25rem;\n");
  });

  /// What says whether it is stuck: a pixel of pane above the block, watched.
  /// The bug this fixed: the conversations pane is a flex column, and one with
  /// a list too long for it shrinks whatever will shrink — which was this
  /// pixel, taken to nothing while the pixel it gives back stayed, so the whole
  /// column stood a pixel high and the top of the New conversation button was
  /// drawn under the header's paper.
  it("keeps the pixel it watches for the header being stuck", () => {
    expect(shellCss).toContain(
      ".pane > .paneEdge {\n" +
        "  flex: none;\n" +
        "  height: 1px;\n" +
        "  margin-bottom: -1px;\n}",
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

/// The same again with the session still being written to, which is what the
/// server pins the card on: the altered Event on the record *and* the same Event
/// at the head of the pinned list, exactly as the pinned block in
/// `crates/server/src/ui.rs` hands it over.
///
/// Composed here rather than in a fixture for the reason a running session is
/// altered here at all: nothing is writing into a fixture, so a fixture that
/// carried a pinned run would be a payload claiming a session that stopped in
/// 2026 is still going.
function theGrillingRunning(
  over: Partial<AgentOutputEvent>,
  ...answers: Parameters<typeof serving>
) {
  const running = { ...OUTPUT, ...over, running: true };

  const altered: TimelineEvent[] = GRILLING.timeline.map((event) =>
    "AgentOutput" in event ? { AgentOutput: running } : event,
  );

  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${GRILLING.id}`,
      json({
        ...GRILLING,
        timeline: altered,
        pinned: [{ AgentOutput: running }, ...GRILLING.pinned],
      }),
    ),
    whenever(TRANSCRIPT_OF_IT, json(SAID_NOTHING)),
    whenever(CAPTURE_OF_IT, json(CAPTURE)),
    whenever(SCREEN_OF_IT, json(SCREEN)),
    ...answers,
  );
}

/// The same conversation after a Cleanup has been through it: every card where
/// it was, and the session's own output taken out from under the one that opens
/// on it.
///
/// Which is exactly what the server hands over once a trim has run — the
/// Transcript has no lines left to read and the Capture has no chunks — so what
/// this stands up is the shape the pane really meets rather than a shape
/// invented for it.
function theTrimmed(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${GRILLING.id}`,
      json({ ...GRILLING, state: "Closed", archived: true, trimmed: true }),
    ),
    whenever(TRANSCRIPT_OF_IT, json(SAID_NOTHING)),
    whenever(CAPTURE_OF_IT, json({ ...CAPTURE, text: "" })),
    whenever(SCREEN_OF_IT, json(SCREEN)),
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


/// A second Event on the opened Conversation's record, and the Conversation
/// carrying it.
///
/// The fixture's own record is the Brief and nothing else, which is the frame
/// with no Timeline in it at all — see *the one-event layout* below. So what is
/// about the Timeline pane rather than about the composer is asked over this
/// instead: the same Conversation with one more Event on it, which is the whole
/// of what it takes for the three panes to stand again.
const ASIDE: TimelineEvent = {
  Notice: {
    id: 12,
    at: "2026-08-03T09:07:11.000Z",
    html: "<p>Verkstead had something to say about this one.</p>\n",
  },
};

const RECORDED: ConversationView = {
  ...OPEN,
  timeline: [...OPEN.timeline, ASIDE],
};

/// The workbench with that record open, and with whatever else about the
/// Conversation the test is asking after.
///
/// The second Event is the newest openable thing on the record, so opening the
/// Conversation by its own path lands on *its* pane rather than on the
/// composer. A test that wants both panes at once mounts the Brief's own path,
/// which is a cold load of the composer and leaves the landing nothing to do.
function theRecordedWith(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return theWorkbenchWith({ timeline: RECORDED.timeline, ...over }, ...answers);
}

/// And the Brief's own path on it, which is where a test that wants the
/// composer beside the record mounts.
const COMPOSING = `/conversations/${OPEN.id}/events/${BRIEF.id}`;

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
/// A Conversation whose record is the one Event, which is every Draft nothing
/// has happened to yet: there is nothing to read on such a Timeline — one card,
/// under a header saying what the sidebar row beside it already says — so the
/// pane is not drawn and the composer takes its column as well as its own.
describe("the one-event layout", () => {
  it("draws the conversations and the composer, and no timeline at all", async () => {
    expect(OPEN.timeline).toHaveLength(1);

    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);

    // The level is not drawn rather than drawn empty: there is no such pane in
    // the document, and the frame says so by the name it wears.
    expect(container.querySelector(`.${shell.conversationsPane}`)).toBeTruthy();
    expect(container.querySelector(`.${shell.middlePane}`)).toBeNull();
    expect(frame(container).classList.contains(shell.widened!)).toBe(true);

    // And nothing of what its head carried is anywhere else — no title, no
    // share, no status, no pins, and no record.
    expect(container.querySelector(`.${timeline.timeline}`)).toBeNull();
    expect(container.querySelector(`.${timeline.paneTitle}`)).toBeNull();
    expect(container.querySelector(`.${statusButton.status}`)).toBeNull();
    expect(container.querySelector(`.${timeline.pinned}`)).toBeNull();
    expect(screen.queryByRole("button", { name: "Share" })).toBeNull();
  });

  /// A second Event of any kind is the whole of what brings it back, and the
  /// widths it comes back at are the ones this device settled — which is the
  /// frame's own doing, and asked of it in `tests/sizing.test.tsx`.
  it("puts the timeline back when a second event lands", async () => {
    let standing: ConversationView = OPEN;
    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(READING, () => json(standing)()),
    );
    const { container, client } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);
    expect(container.querySelector(`.${shell.middlePane}`)).toBeNull();

    standing = RECORDED;
    await nudged(client);

    await drawn(container, `.${shell.middlePane} .${timeline.timeline}`);
    expect(frame(container).classList.contains(shell.widened!)).toBe(false);
  });

  /// And the walk on a narrow window skips the level that is not there, in
  /// either direction: opening the Conversation lands on the composer, and the
  /// way off the composer is the way out of the Conversation.
  it("lands on the brief pane, and back goes to the conversations", async () => {
    theWorkbench();
    const { container, history } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    fireEvent.click(screen.getByText(DRAFTING.branch));

    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("details"),
    );
    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);

    // Nothing to page forward into either: the way on is the timeline's, and
    // there is no timeline.
    expect(screen.queryByRole("button", { name: "Details →" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "← Conversations" }));

    await waitFor(() => expect(history.get()).toBe("/"));
    expect(frame(container).dataset.pane).toBe("conversations");
  });
});

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


/// Opening a Conversation is the human having looked at it, and the page says
/// so out loud: the news mark lives on the server so that every device agrees
/// about it, which means only the server can take it off.
///
/// A press of its own rather than something the read of the Conversation does
/// on the way past — a GET that wrote would spend the mark on a prefetch or a
/// retry, and what is being recorded is a person having looked.
describe("opening a conversation is looking at it", () => {
  /// The press for one Conversation, by the path it goes to.
  const seen = (id: number) => `/api/ui/conversations/${id}/seen`;

  /// The three, with the press answered as the server answers it: nothing at
  /// all.
  function theThreeSeen(...answers: Parameters<typeof serving>) {
    return theThree(
      ...[OPEN.id, SECOND.id, GRILLING.id].map((id) =>
        whenever(seen(id), () => Promise.resolve(new Response(null, { status: 204 })), "POST"),
      ),
      ...answers,
    );
  }

  it("says so when the URL names one", async () => {
    const fetching = theThreeSeen();
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => expect(writes(fetching, seen(OPEN.id))).toBe(1));
  });

  /// A card pressed in the sidebar and a URL typed into the bar are the same
  /// thing to the page — the click navigates, and the navigation is what this
  /// hangs off — which is why Back is covered by the same one line.
  it("says so again for the next one opened", async () => {
    const fetching = theThreeSeen();
    const { history } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => expect(writes(fetching, seen(OPEN.id))).toBe(1));

    history.set({ value: `/conversations/${GRILLING.id}` });

    await waitFor(() => expect(writes(fetching, seen(GRILLING.id))).toBe(1));
    expect(
      writes(fetching, seen(OPEN.id)),
      "and not again for the one it left",
    ).toBe(1);
  });

  it("says nothing on the bare workbench, there being nothing looked at", async () => {
    const fetching = theThreeSeen();
    const { container } = mount();

    await cards(container);

    expect(
      fetching.mock.calls.filter(([asked, init]) =>
        String(asked).endsWith("/seen") && init?.method === "POST",
      ),
    ).toEqual([]);
  });

  /// Nothing waits on it and nothing is done about a failure: the mark is a
  /// nudge to look rather than a record to keep, and the worst a lost press
  /// costs is a dot that comes off the next time the Conversation is opened.
  it("draws the conversation whether or not the press lands", async () => {
    theThree(
      whenever(seen(OPEN.id), json({ error: "no" }, 503), "POST"),
    );
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() =>
      expect((screen.getByLabelText("Brief") as HTMLTextAreaElement).value).toBe(
        BRIEF.markdown,
      ),
    );
  });
});

describe("starting the work", () => {
  it("offers the button under the timeline once the conversation is ready", async () => {
    theRecordedWith();
    const { container } = mount(COMPOSING);

    const start = await drawn(container, `.${composer.startGrilling} .${composer.start}`);

    expect(OPEN.ready_to_grill).toBe(true);
    expect(start.textContent).toContain("Start work");

    // Under the timeline, which is where the reason to press it is: at the end
    // of everything that has happened, under the brief it will freeze.
    expect(
      container.querySelector(`.${timeline.timeline}`)!.compareDocumentPosition(start) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// An unready conversation gets the button all the same, drawn inert. Not
  /// `disabled`: a disabled button is one no browser will hover, and hovering
  /// is what the explanation hangs off.
  it("draws the button inert rather than withholding it", async () => {
    theWorkbenchWith({ ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const start = await drawn<HTMLButtonElement>(
      container,
      `.${composer.startGrilling} .${composer.start}`,
    );

    expect(start.textContent).toContain("Start work");
    expect(start.classList).toContain(composer.inert);
    expect(start.getAttribute("aria-disabled")).toBe("true");
    expect(start.disabled).toBe(false);

    // And nothing said on the page: what is missing is the button's own
    // tooltip, and neither of the notes that used to stand in for the button
    // is there either.
    expect(screen.queryByText(/This needs a brief/)).toBeNull();
    expect(screen.queryByText(/the grilling can start/)).toBeNull();
  });

  it("answers a press on the inert button with nothing at all", async () => {
    const fetching = theWorkbenchWith(
      { ready_to_grill: false },
      whenever(
        `/api/ui/conversations/${OPEN.id}/grill`,
        json("Started" satisfies GrillingStarted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(await drawn(container, `.${composer.startGrilling} .${composer.start}`));

    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
    expect(screen.queryByText(/This needs a brief/)).toBeNull();
  });

  /// And what is missing is carried in the button's own tooltip, which is why
  /// it stays `aria-disabled` rather than becoming disabled: a disabled button
  /// is one a browser will not hover.
  it("carries what is missing in the button's tooltip", async () => {
    theWorkbenchWith({ ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const start = await drawn(container, `.${composer.startGrilling} .${composer.start}`);

    expect(start.getAttribute("title")).toBe(
      "This needs a brief, and every role picked and working.",
    );
  });

  /// And a conversation that is ready says nothing at all: what the press does
  /// is what the button already says.
  it("says nothing on a start that can be pressed", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const start = await drawn(container, `.${composer.startGrilling} .${composer.start}`);

    expect(start.getAttribute("title")).toBeNull();
    expect(screen.queryByText(/creates the branch and its worktree/)).toBeNull();
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

    fireEvent.click(await drawn(container, `.${composer.startGrilling} .${composer.start}`));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toEqual(
        {},
      ),
    );
  });

  /// Every refusal is its own sentence, because each of them is something
  /// different for the human to go and do.
  it.each([
    ["NoGrillingProfile", /Pick a grilling profile/],
    ["NoImplementationProfile", /Choose an implementation profile/],
    ["NoReviewProfile", /Pick a review profile/],
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

      fireEvent.click(await drawn(container, `.${composer.startGrilling} .${composer.start}`));

      await waitFor(() => screen.getByText(said));
    },
  );

  /// A conversation that has started has nothing to start, so there is nothing
  /// to draw — not a button that would be refused.
  it("offers nothing on a conversation that has already started", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() => screen.getByText("Draft → Grilling"));
    expect(container.querySelector(`.${composer.startGrilling}`)).toBeNull();
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

    // Read as which of the kinds the card is wearing rather than as its first
    // class: a pressable card is a `CardButton`, and what every card has is
    // written before the class the caller handed down.
    const KINDS = [
      timeline.brief,
      timeline.moved,
      timeline.agentOutput,
      timeline.questionSet,
    ];

    expect(
      [...container.querySelectorAll(`.${timeline.timelineEvent} > *`)].map(
        (event) => KINDS.find((kind) => event.classList.contains(kind!)),
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
  /// The design's summary: what it is, how far the conversation has got, and
  /// what it was run under. An hour of terminal output does not go in the middle
  /// pane, and neither does a line of it — reading output is the details pane's,
  /// which is what pressing the card opens.
  ///
  /// Turns rather than the lines it printed. A full-screen interface redraws
  /// itself with cursor moves rather than newlines, so a line count read 0 for
  /// every real session — and what a reader wanted from it was how much of a
  /// conversation there is to open, which is what a turn is.
  it("summarises as a turn count, and draws nothing the session printed", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(OUTPUT.turns).not.toBeNull();
    expect(output.querySelector(`.${timeline.turns}`)!.textContent).toBe(
      `${OUTPUT.turns} turns`,
    );

    // Neither the last thing it said nor the Capture behind it: both are the
    // details pane's, and only once one is opened.
    expect(OUTPUT.latest).not.toBe("");
    expect(output.textContent).not.toContain(OUTPUT.latest);
    expect(output.textContent).not.toContain("Reading the brief");
  });

  /// The head says what the card is, in the words every other card on this pane
  /// says its own kind in. What tells one run from another is the line under it.
  it("heads the card with the words Agent run", async () => {
    theGrillingOutput({
      agent_type: "Claude",
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(output.querySelector(`.${timeline.what}`)!.textContent).toBe(
      "Agent run",
    );
  });

  /// And the line under it is the shared reading of what the session was
  /// launched under, off the three facts it wrote down as it started.
  ///
  /// The account's name is said here because the saved list holds three Claude
  /// Code accounts and none of them is this one — a name the list no longer
  /// holds keeps its own, dropping it being a run attributed to whoever is left.
  it("says what the run was launched under on the line under it", async () => {
    theGrillingOutput({
      agent_type: "Claude",
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    await waitFor(() =>
      expect(output.querySelector(`.${timeline.ranAs}`)!.textContent).toBe(
        "Claude Code Fable 5 — Work",
      ),
    );
  });

  /// A session from before Verkstead wrote the backend down says the half it
  /// kept, with no backend guessed for it — which is every Agent run on every
  /// record made before this.
  it("leaves the backend out of a run that never recorded one", async () => {
    theGrillingOutput({
      agent_type: null,
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    await waitFor(() =>
      expect(output.querySelector(`.${timeline.ranAs}`)!.textContent).toBe(
        "Fable 5 — Work",
      ),
    );
  });

  /// And a session that recorded none of the three has nothing to say there, so
  /// the line is not drawn at all: no bare mark, and no empty row under the
  /// head. The card is still a card.
  it("draws no second line where the record kept nothing to name the run by", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(OUTPUT.profile).toBeNull();
    expect(output.querySelector(`.${timeline.what}`)!.textContent).toBe(
      "Agent run",
    );
    expect(output.querySelector(`.${timeline.ranAs}`)).toBeNull();
    expect(output.querySelector(`.${harnessMark.mark}`)).toBeNull();
  });

  /// The details pane is titled the same way, for the reason its summary line
  /// is: the card and the pane are one session read at two distances, and a
  /// pane titled otherwise would be two answers to who ran it.
  it("titles the details pane with the same reading", async () => {
    theGrillingOutput({
      agent_type: "Claude",
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const head = await drawn(container, `.${shell.detailsPane} .${paneHead.head} h1`);

    await waitFor(() =>
      expect(head.textContent).toBe("Claude Code Fable 5 — Work"),
    );
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

  /// And where a Cleanup has taken that record, the pane says so.
  ///
  /// The loss explaining itself. Trimmed, the Transcript comes back with no
  /// turns and the Capture with no bytes, which is the same nothing a session
  /// that never printed leaves — and drawn as that nothing it reads as a pane
  /// that has failed. The card above it is untouched: the summary the Timeline
  /// draws survived the trim, and this is the drill-down under it saying where
  /// the rest went.
  it("says the detail was trimmed rather than opening on nothing", async () => {
    theTrimmed();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const said = await drawn(container, `.${shell.detailsPane} .${outputPane.trimmed}`);

    expect(said.textContent).toContain("trimmed");
    expect(
      container.querySelector(`.${shell.detailsPane} .${outputPane.capture}`),
      "and nothing standing empty where the record was",
    ).toBeNull();

    // The summary that survived, still drawn over it: what is gone is the
    // drill-down rather than the card.
    const summary = await drawn(container, `.${shell.detailsPane} .${outputPane.captureSummary}`);
    expect(summary.querySelector(`.${outputPane.turns}`)!.textContent).toBe(
      `${OUTPUT.turns} turns`,
    );
  });

  /// The other half of the switch says the same thing, and asks the server for
  /// nothing: the grid is replayed from the bytes the trim took, so what a
  /// request would come back with is an empty terminal.
  it("says the same of the screen, and asks for no grid", async () => {
    const fetching = theTrimmed();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.trimmed}`);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${outputPane.screenTab}`));

    const said = await drawn(container, `.${shell.detailsPane} .${outputPane.trimmed}`);

    expect(said.textContent).toContain("trimmed");
    expect(askedFor(fetching, SCREEN_OF_IT)).toBe(0);
  });

  /// And a session that simply printed nothing says what it always said: the
  /// word is about a cleanup that happened, so a conversation no sweep has
  /// reached never sees it.
  it("still says a quiet session printed nothing where no cleanup ran", async () => {
    theGrilling(whenever(CAPTURE_OF_IT, json({ ...CAPTURE, text: "" })));
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    // Waited for rather than read once: the same notice says the pane is
    // loading while the two records are on their way, and what this is about is
    // the words it settles on.
    await waitFor(() =>
      expect(
        container.querySelector(`.${shell.detailsPane} .${notices.empty}`)
          ?.textContent,
      ).toBe("This session has printed nothing yet."),
    );

    expect(
      container.querySelector(`.${shell.detailsPane} .${outputPane.trimmed}`),
    ).toBeNull();
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
    // A whole line of a kind this version has never met folds in here too,
    // under the name the log gave it — see ADR 0006.
    expect(kept.textContent).toContain("atis-latch");

    // And not among the turns, which is what folding it away is for.
    expect(container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}`)).toHaveLength(
      rows(TRANSCRIPT.turns),
    );
  });

  /// ADR 0006's containment, at the end of the wire: the log's format belongs to
  /// somebody else, and one that has moved on should say so here rather than
  /// quietly emptying the pane. A block is where that is still drawn inline —
  /// it is part of what somebody said, so it stays in the turn it was said in,
  /// where a whole line of an unknown kind folds away with the bookkeeping.
  it("shows a block of a kind it does not know as the JSON it is", async () => {
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
    expect(output.classList).not.toContain(pressable.open);

    fireEvent.click(output);

    await waitFor(() => expect(output.classList).toContain(pressable.open));
    expect(output.getAttribute("aria-pressed")).toBe("true");
  });

  /// A running session is opened for what it is saying now, which is the end of
  /// the record rather than the beginning of it — and the pane goes on following
  /// as the session talks.
  ///
  /// What the ask lands on here is the window: which box scrolls is the
  /// stylesheet's answer to how wide the window is, and jsdom lays nothing out,
  /// so the walk up from the record finds no box that scrolls and the page
  /// itself is what moves. Where the pause and the resume are asked about is
  /// `following.test.ts`, which builds a box that scrolls.
  it("opens a running session's record at its end, and follows it down", async () => {
    const scrolled = vi.fn();
    vi.stubGlobal("scrollTo", scrolled);

    theGrillingOutput(
      { running: true },
      whenever(TRANSCRIPT_OF_IT, json(TRANSCRIPT)),
      whenever(REST_OF_IT, json(MORE)),
    );
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.prose}`);

    expect(scrolled).toHaveBeenCalled();
    const landed = scrolled.mock.calls.length;

    await client.invalidateQueries();

    // What the session has said since is drawn, and the view goes after it.
    await waitFor(() =>
      expect(container.querySelectorAll(`.${shell.detailsPane} .${outputPane.turn}`)).toHaveLength(
        rows(TRANSCRIPT.turns, MORE.turns),
      ),
    );
    expect(scrolled.mock.calls.length).toBeGreaterThan(landed);
  });

  /// And a session that has stopped talking is opened where every other document
  /// in this pane is: at the top. There is no end being written for the view to
  /// keep up with, and moving a reader off the first thing the session said
  /// would be the pane deciding where they meant to start.
  it("leaves a finished session's record where the reader arrives", async () => {
    const scrolled = vi.fn();
    vi.stubGlobal("scrollTo", scrolled);

    theSpeaking();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    // The Timeline's own following puts the record at its end on the way in,
    // and that is the pane beside this one: what is asked here is what the
    // output pane does once it is opened, so the count starts from there.
    await drawn(container, `.${timeline.timeline}`);
    const landed = scrolled.mock.calls.length;

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${outputPane.turn}.${outputPane.prose}`);

    expect(scrolled.mock.calls.length).toBe(landed);
  });
});

/// The brand mark of the harness, in front of the words that name the run.
///
/// The reading says who runs a session and the mark is what makes a column of
/// those readings scannable: a reader picks the Claude run out of five by its
/// shape before reading a word of any of them. So it is drawn everywhere the
/// reading is and JSX can put an `svg` — the card's second line, the pane it
/// opens, and the Brief's three pairing facts — and it is one component, so
/// those three cannot come to draw three different pictures of the one harness.
///
/// What is asserted is the drawing rather than a class: the four files are read
/// here exactly as lobehub published them, so a mark drawn from the wrong file
/// fails even though both files would carry the same class.
describe("the mark of the harness a session ran under", () => {
  /// Every harness, against the file its mark is drawn out of. A fifth backend
  /// arrives in this list because it arrives in the component's own record,
  /// which will not compile without a mark beside it.
  const MARKS = [
    ["Claude", claudeMarkFile],
    ["Codex", codexMarkFile],
    ["Grok", grokMarkFile],
    ["OpenCode", opencodeMarkFile],
  ] satisfies Array<[AgentType, string]>;

  /// The Agent run card on the record, on the line saying what the run was
  /// launched under, which is where a reader meets a run first.
  it.each(MARKS)(
    "draws %s's own mark on the card that names the run",
    async (harness, file) => {
      theGrillingOutput({ agent_type: harness, profile: "Work", model: null });
      const { container } = mount(`/conversations/${GRILLING.id}`);

      const head = await drawn(
        container,
        `.${timeline.timelineEvent} .${timeline.agentOutput} .${timeline.ranAs}`,
      );

      await waitFor(() => expect(marked(head)).toBe(art(file)));
    },
  );

  /// And the details pane the card opens carries the same one, for the reason it
  /// carries the same words: one session read at two distances.
  it("carries the same mark into the details pane", async () => {
    theGrillingOutput({
      agent_type: "OpenCode",
      profile: "Work",
      model: "minimax/minimax-m2.1",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    const head = await drawn(
      container,
      `.${shell.detailsPane} .${paneHead.head} h1`,
    );

    await waitFor(() => expect(marked(head)).toBe(art(opencodeMarkFile)));
    expect(head.textContent).toBe("OpenCode Minimax M2.1 — Work");
  });

  /// And the Brief's three pairing facts, which say what each role *will* run
  /// under. All three of the fixture's accounts are Claude Code ones, so all
  /// three rows wear the one mark — what is being asked is that the rows carry a
  /// mark at all, the four harnesses being covered on the card above.
  it("marks the backend on each of the Brief's pairing facts", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`),
    );

    const summary = await drawn(
      container,
      `.${shell.detailsPane} .${briefPane.configuration}`,
    );

    await waitFor(() =>
      expect(
        ["Grilling", "Implementation", "Review"].map((term) =>
          marked(
            [...summary.querySelectorAll(`.${briefPane.fact}`)].find(
              (fact) => fact.querySelector("dt")!.textContent === term,
            )!,
          ),
        ),
      ).toEqual([
        art(claudeMarkFile),
        art(claudeMarkFile),
        art(claudeMarkFile),
      ]),
    );
  });

  /// A run recorded before Verkstead wrote the harness down draws no mark, and
  /// no element either: the space in front of the words is the mark's own
  /// margin, so a card with nothing to draw has nothing to leave a gap.
  it("draws no mark, and no gap, where no harness was recorded", async () => {
    theGrillingOutput({
      agent_type: null,
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const head = await drawn(
      container,
      `.${timeline.timelineEvent} .${timeline.agentOutput} .${timeline.ranAs}`,
    );

    await waitFor(() => expect(head.textContent).toBe("Fable 5 — Work"));
    expect(head.querySelector(`.${harnessMark.mark}`)).toBeNull();
  });

  /// Claude Code's is lobehub's colour variant, which names its own fill and so
  /// stands in its own orange wherever it is drawn — colour where lobehub has it
  /// was the pick, and of these four it has it for Claude alone.
  it("keeps Claude Code's mark in its own colour", async () => {
    theGrillingOutput({ agent_type: "Claude", profile: "Work", model: null });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const head = await drawn(
      container,
      `.${timeline.timelineEvent} .${timeline.agentOutput} .${timeline.ranAs}`,
    );

    const path = await drawn(head, `.${harnessMark.mark} path`);

    expect(path.getAttribute("fill")).toBe("#D97757");
    expect(head.querySelector(`.${harnessMark.mark} svg`)!.getAttribute("fill"))
      .toBeNull();
  });

  /// And the other three follow the ink of whatever line they were put beside,
  /// the way an `Icon` does: the ordinary ink on a card's second line, and the
  /// heading's own in the pane it opens.
  it.each(MARKS.filter(([harness]) => harness !== "Claude"))(
    "leaves %s's mark in the ink around it",
    async (harness, _file) => {
      theGrillingOutput({ agent_type: harness, profile: "Work", model: null });
      const { container } = mount(`/conversations/${GRILLING.id}`);

      const head = await drawn(
        container,
        `.${timeline.timelineEvent} .${timeline.agentOutput} .${timeline.ranAs}`,
      );

      const svg = await drawn(head, `.${harnessMark.mark} svg`);

      expect(svg.getAttribute("fill")).toBe("currentColor");
      expect(svg.querySelector("path")!.getAttribute("fill")).toBeNull();
    },
  );

  /// The mark says nothing to a screen reader: the words beside it are the whole
  /// of what it means, and a mark read out as "Claude" in front of them would be
  /// the line saying itself twice.
  it("says nothing of itself to a reader who cannot see it", async () => {
    theGrillingOutput({ agent_type: "Codex", profile: "Work", model: null });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const mark = await drawn(
      container,
      `.${timeline.agentOutput} .${harnessMark.mark}`,
    );

    expect(mark.getAttribute("aria-hidden")).toBe("true");
    // Nor as a tooltip, which is what the `<title>` lobehub ships would have
    // drawn — taken out of every one of the four files as they were copied.
    expect(mark.querySelector("title")).toBeNull();
  });
});

/// And what is no longer held against the foot of the pane: a strip for the
/// session running now.
///
/// It carried the session's title and its liveness mark, and opened the same
/// details pane the card on the record opens — a way back to the one moving
/// thing on a record long enough to have scrolled it away. The status button at
/// the head of the pane says what is running now, in more words than the strip
/// ever did and where the eye lands, so what is left of the session is the one
/// card on the record.
describe("the foot of the timeline pane", () => {
  it("holds no strip for the session running now", async () => {
    theGrillingOutput({ running: true, idle: false });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    // The record's own card for the session is still there, and is the only
    // one: a second appearance of it was what the strip was.
    await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(container.querySelectorAll(`.${timeline.agentOutput}`)).toHaveLength(1);

    // And nothing in this pane wears the frame's name for something stuck to a
    // pane's bottom edge any more. The name itself outlived the strip by one
    // pane, which is why the search is this pane rather than the page: the
    // conversations keep their archived switch down there.
    expect(
      container.querySelector(`.${shell.middlePane} .${shell.paneFoot}`),
    ).toBeNull();
  });

  /// And the head of the pane says what the strip said, which is why it could
  /// go: the running session's own card, pinned while it runs and naming the
  /// run rather than marking it.
  it("says what is running at the head of the pane instead", async () => {
    theGrillingRunning({
      agent_type: "Claude",
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(
      container,
      `.${timeline.pinned} .${timeline.agentOutput}`,
    );

    await waitFor(() =>
      expect(card.querySelector(`.${timeline.ranAs}`)!.textContent).toBe(
        "Claude Code Fable 5 — Work",
      ),
    );
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
      ".panes > .detailsPane:has(.paneScreen) {\n" +
        "  flex-direction: column;\n" +
        "  height: 100dvh;\n" +
        "  padding-bottom: 1.25rem;\n" +
        "  overflow: hidden;\n" +
        "}",
    );

    // What is above the terminal keeps its size; the Screen takes the rest.
    expect(shellCss).toContain(
      ".panes > .detailsPane:has(.paneScreen) > :not(.paneScreen) {\n" +
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

    // And nothing asks for the state class by the word alone, here or anywhere
    // else: one standing on its own matches every element that carries it. The
    // record had a `.live` badge of its own — the words a waiting Question Set
    // said before the disc said it instead — and this is what kept the two
    // apart while both existed.
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
      (await drawn(container, `.${shell.middlePane} .${statusButton.standing}`))
        .textContent,
    ).not.toContain("Blocked");
  });
});

describe("closing a conversation", () => {
  /// Behind a menu at the head of the pane, because it throws a worktree away
  /// and the head of the pane is somewhere the cursor passes on the way to
  /// everything else.
  it("is not one click away", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${actions.conversationActions} > .${dropdown.trigger}`);

    // Closed, so nothing in it is on the page at all — which is the whole of
    // what standing a destructive action behind a menu means.
    expect(container.querySelector(`.${actions.close}`)).toBeNull();

    const menu = await openActions(container);
    expect(menu.querySelector(`.${actions.close}`)).toBeTruthy();
    expect(
      container.querySelector(`.${statusButton.status} .${actions.close}`),
    ).toBe(menu.querySelector(`.${actions.close}`));
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

  /// Closing is over, so the row is gone and archive stands in its place.
  /// Nothing says so in words: a menu whose rows have changed has already said
  /// it, and a line of prose about a press that is no longer offered would be
  /// the only thing in the card that was not a press.
  it("offers nothing to close on one that is closed already", async () => {
    theRecordedWith({ state: "Closed", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);

    await drawn(container, `.${actions.conversationActions} .${actions.archive}`);
    expect(container.querySelector(`.${actions.conversationActions} .${actions.close}`)).toBeNull();
    expect(screen.queryByText("This conversation has been closed.")).toBeNull();
  });

  /// A page drawn against a conversation that has since gone: the press is
  /// refused, and the refusal opens over the page rather than going to a
  /// console nobody has open.
  it("says over the page that the conversation has gone", async () => {
    theGrilling(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/close`,
        json("NoSuchConversation" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(await drawn(container, `.${actions.conversationActions} .${actions.close}`));

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toBe(CLOSE_REFUSAL.NoSuchConversation);
  });
});

/// Closing and archiving is the row under Close: the same press with the
/// archive already made, for a conversation there is nothing left to read on.
describe("closing and archiving a conversation", () => {
  it("stands under close, and goes with it when close goes", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);
    const offered = [...menu.querySelectorAll("button")].map(
      (button) => button.className,
    );
    expect(offered.slice(-2)).toEqual([actions.close, actions.closeAndArchive]);

    theRecordedWith({ state: "Closed", ready_to_grill: false });
    const closed = mount(`/conversations/${OPEN.id}`);

    await openActions(closed.container);
    await waitFor(() =>
      expect(closed.container.querySelector(`.${actions.archive}`)).toBeTruthy(),
    );
    expect(
      closed.container.querySelector(`.${actions.closeAndArchive}`),
    ).toBeNull();
  });

  it("posts to the conversation's own close-and-archive route", async () => {
    const fetching = theGrilling(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/close-and-archive`,
        json("Closed" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(
      await drawn(container, `.${actions.conversationActions} .${actions.closeAndArchive}`),
    );

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${GRILLING.id}/close-and-archive`),
      ).toEqual({}),
    );

    // And nothing went to the two routes it stands for: one press is one
    // request, which is what stops a dropped connection leaving it half made.
    expect(askedFor(fetching, `/api/ui/conversations/${GRILLING.id}/close`)).toBe(0);
    expect(askedFor(fetching, `/api/ui/conversations/${GRILLING.id}/archive`)).toBe(0);
  });

  /// It says both halves, because it does both: what cannot be undone first,
  /// and the reversible half after it.
  it("says it closes and that the record stays", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await waitFor(() =>
      screen.getByText(
        /The same, and take it off the conversations list. Its record stays/,
      ),
    );
  });

  it("says over the page that the conversation has gone", async () => {
    theGrilling(
      whenever(
        `/api/ui/conversations/${GRILLING.id}/close-and-archive`,
        json("NoSuchConversation" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);
    fireEvent.click(
      await drawn(container, `.${actions.conversationActions} .${actions.closeAndArchive}`),
    );

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toBe(CLOSE_REFUSAL.NoSuchConversation);
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
    theRecordedWith({ state: "Closed", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const menu = await openActions(container);
    await waitFor(() => expect(menu.querySelector(`.${actions.archive}`)).toBeTruthy());
    expect(menu.querySelector(`.${actions.close}`)).toBeNull();
  });

  it("posts to the conversation's own archive route", async () => {
    const fetching = theRecordedWith(
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
    theRecordedWith({ state: "Closed", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await openActions(container);

    await waitFor(() => screen.getByText(/Take it off the conversations list/));
    expect(screen.getByText(/Its record stays where it is/)).toBeTruthy();
  });

  /// A page drawn against a conversation that has since been steered back into
  /// the work: the press is refused, and the refusal opens over the page.
  it("says over the page that there is nothing to put away", async () => {
    theRecordedWith(
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

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toBe(ARCHIVE_REFUSAL.NotClosed);
  });
});

/// And the way back out of it, which stands in the same place on a conversation
/// that has already been put away: archiving is reversible, and this is the
/// reversal.
describe("unarchiving a conversation", () => {
  /// One row or the other, never both — the two say opposite things about the
  /// same conversation.
  it("stands where archive was, on one already put away", async () => {
    theRecordedWith({
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
    const fetching = theRecordedWith(
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
    theRecordedWith({
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
  /// since gone. Over the page, as every other refusal here goes.
  it("says over the page that the conversation is gone", async () => {
    theRecordedWith(
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

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toBe(UNARCHIVE_REFUSAL.NoSuchConversation);
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
    theRecordedWith({ state: "Done" });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${timeline.timeline}`);
    expect(container.querySelector(`.${composer.startGrilling}`)).toBeNull();

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
  /// The two stops sit in the same menu as the resume, the steer and the close,
  /// in the order of what each one costs: get going again, pause after this
  /// task, stop now, move the work somewhere else, end the conversation. Each
  /// says what it does, because *stop* and *force stop* are two words apart and
  /// hours of work apart — and each says it *inside* the row, so what the press
  /// is called and what it means are one thing to read and one thing to aim at.
  it("offers the four ways of stopping, each saying what it does", async () => {
    theGrillingStanding({ ready_to_stop: true, working: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);
    const offered = [...menu.querySelectorAll("button")].map(
      (button) => button.className,
    );

    expect(offered).toEqual([
      actions.resume,
      actions.stop,
      actions.forceStop,
      actions.steer,
      actions.close,
      actions.closeAndArchive,
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

  /// And the sentence is the row's own, rather than a line after it: everything
  /// the menu draws is inside one press or another, so there is nothing in it
  /// that reads as a row and is not one.
  it("carries every sentence inside the row it belongs to", async () => {
    theGrillingStanding({ ready_to_stop: true, working: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);

    for (const words of [
      "Stop after the current task until you resume.",
      "End any running task and stop immediately.",
      "Stop the run and move this conversation somewhere else.",
    ]) {
      const said = screen.getByText(words);
      expect(said.className, `"${words}" is drawn as the row's own note`).toBe(
        actions.says,
      );
      expect(said.closest("button"), `"${words}" is inside its row`).toBeTruthy();
    }

    // Nothing loose in the card: every element the menu holds is a row or
    // something inside one. Two shapes of row, because one of them is a file to
    // take away rather than a press — Share is a link, and reads as the rows
    // around it.
    expect(
      [...menu.children].every((row) =>
        ["BUTTON", "A"].includes(row.tagName),
      ),
    ).toBe(true);
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

  /// And the ordinary stop goes once it has been pressed: from there it is a
  /// decision the server has, waiting for the step the run is on to finish, and
  /// a row still offering it would answer a second press by doing what the
  /// first did. Force stop stays, being the escalation from there rather than
  /// the same press again.
  it("takes the stop off the menu once one has been asked for", async () => {
    theGrillingStanding({
      ready_to_stop: true,
      working: true,
      stop_asked: true,
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await drawn(
      container,
      `.${actions.conversationActions} .${actions.forceStop}`,
    );
    expect(
      container.querySelector(`.${actions.conversationActions} .${actions.stop}`),
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

  /// A stop that is still waiting for a task to finish is a press that landed,
  /// so the menu goes the way it does for one that stopped at once. Nothing on
  /// the timeline has changed yet — the session is still running, and the notice
  /// comes when it stops.
  it("shuts on a stop that is waiting for the task to finish", async () => {
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

    await waitFor(() =>
      expect(
        container.querySelector(`.${actions.conversationActions} > .${dropdown.drop}`),
      ).toBeNull(),
    );
  });

  /// And a press that was refused says so over the page. These are not presses
  /// that fail in ordinary use — every refusal is a page drawn against a
  /// conversation that has since moved, and the re-read that follows is what
  /// corrects it, by taking the row away — but the human made the press and is
  /// owed the sentence rather than a line in a console.
  ///
  /// Not in the menu: the card is drawn over the page and the menu has gone by
  /// the time there is anything to say.
  it("says a refused stop over the page rather than in the menu", async () => {
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

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toBe(STOP_REFUSAL.AlreadyStopped);
    expect(
      container.querySelector(`.${actions.conversationActions} .${notices.error}`),
    ).toBeNull();
  });
});

/// Where the share is published, which is the one press on this pane that
/// reaches outside the machine.
const PUBLISHING = `/api/ui/conversations/${GRILLING.id}/share/publish`;

/// And what comes back from it: the link the server composed, which is the
/// gist's id in the share viewer's fragment rather than the gist. Which page
/// composes it is settled over there — every Verkstead goes through the one
/// hosted copy — and what this side has to do is hand out whatever arrived.
const PUBLISHED = "https://tobico.github.io/verkstead/share-viewer.html#9f1";

/// And the gist itself, which travels beside it: where the file actually is,
/// and the only place a share can be deleted.
const GIST = "https://gist.github.com/tobico/9f1";

/// One published share, as the record carries it.
const SHARED = { url: PUBLISHED, gist: GIST, at: "2026-08-30T01:02:03Z" };

/// The share icon on the Timeline's header, which is the whole of the way into
/// the pane: no card opens it, sharing belonging to the Conversation rather
/// than to any moment on its record.
function shareIcon(container: ParentNode): HTMLButtonElement | null {
  return container.querySelector<HTMLButtonElement>(
    `.${shell.middlePane} .${button.iconButton}[aria-label="Share"]`,
  );
}

/// Press it, and wait for the pane it opens.
async function openShare(container: ParentNode): Promise<HTMLElement> {
  fireEvent.click(await drawn(container, `.${shell.middlePane} .${button.iconButton}[aria-label="Share"]`));
  return drawn(container, `.${shell.detailsPane} .${sharePane.doing}`);
}

describe("the way into the share pane", () => {
  /// Drawn the way the settings gear beside the Verkstead wordmark is, and for
  /// the same reason: it is another thing standing in a pane that is selected
  /// and opened into the pane beside it.
  it("stands on the timeline's header as an icon button", async () => {
    theGrillingStanding({});
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const icon = await drawn(
      container,
      `.${shell.middlePane} .${button.iconButton}[aria-label="Share"]`,
    );

    // The label is the whole of what a screen reader has: the shape says
    // nothing when it is read aloud.
    expect(icon.getAttribute("aria-label")).toBe("Share");
    expect(icon.querySelector("svg")).toBeTruthy();
  });

  /// And it reads as open while its pane is what the details are showing, which
  /// is what the open card in the sidebar says about itself, in the same fill.
  it("reads as open while the pane it opened is showing", async () => {
    theGrillingStanding({});
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const icon = await drawn(
      container,
      `.${shell.middlePane} .${button.iconButton}[aria-label="Share"]`,
    );
    expect(icon.getAttribute("aria-pressed")).toBe("false");
    expect(icon.className).not.toContain(button.open);

    fireEvent.click(icon);

    await waitFor(() =>
      expect(shareIcon(container)!.getAttribute("aria-pressed")).toBe("true"),
    );
    expect(shareIcon(container)!.className).toContain(button.open);
  });

  /// The press is two acts, exactly as pressing a card on the record is: the
  /// selection, and the walk into the details. Which is what makes it work on a
  /// narrow window, where the two panes are two levels.
  it("walks a narrow window into the details, and back out again", async () => {
    theGrillingStanding({});
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timeline}`);
    expect(frame(container).dataset.pane).toBe("middle");

    fireEvent.click(shareIcon(container)!);
    expect(frame(container).dataset.pane).toBe("details");

    await drawn(container, `.${shell.detailsPane} .${sharePane.doing}`);

    fireEvent.click(screen.getByRole("button", { name: "← Timeline" }));
    expect(frame(container).dataset.pane).toBe("middle");
  });

  /// And it can be linked to, like every other details pane: what is open is in
  /// the URL, so a cold load of this path opens on it.
  it("stands at a path of its own", async () => {
    theGrillingStanding({});
    const { container } = mount(`/conversations/${GRILLING.id}/share`);

    await drawn(container, `.${shell.detailsPane} .${sharePane.doing}`);
    await waitFor(() =>
      expect(shareIcon(container)!.getAttribute("aria-pressed")).toBe("true"),
    );
  });

  /// And the menus it came out of carry none of it. Four rows left both of them
  /// together — the pane's own status button and the sidebar's right-click are
  /// one set of rows — because not one of them did anything to the Conversation,
  /// which is what those menus are for.
  it("takes the share rows out of both menus", async () => {
    theGrillingStanding({ pinned: WRAPPING.pinned, shared: SHARED });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);

    for (const words of [
      "Download",
      "Publish",
      "Published share",
      "Share to pull request",
    ]) {
      expect(menu.textContent).not.toContain(words);
    }

    // Nothing that is not a press, either: the published-share row was a link
    // in a menu of buttons, and it goes with them.
    expect(menu.querySelector("a")).toBeNull();
  });
});

describe("the share pane", () => {
  /// What most visits to this pane are for: the link to send somebody, and the
  /// button that takes it. The URL is the server's own composition — the gist's
  /// id in the share viewer's fragment rather than the gist — so what is drawn
  /// is whatever the record came back carrying. See `link` in
  /// `crates/server/src/sharing.rs`.
  it("draws the viewer's link, the moment and the gist", async () => {
    theGrillingStanding({ shared: SHARED });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);

    const link = await drawn<HTMLAnchorElement>(
      container,
      `.${sharePane.link} a`,
    );
    expect(link.getAttribute("href")).toBe(PUBLISHED);

    expect(screen.getByText(/^Taken /)).toBeTruthy();

    // And the gist beside it, which is where the file is and the only place a
    // share can be deleted: nothing in Verkstead deletes one.
    const gist = await drawn<HTMLAnchorElement>(
      container,
      `.${sharePane.gist} a`,
    );
    expect(gist.getAttribute("href")).toBe(GIST);
    expect(
      screen.getByText(/the only place a share can be deleted/),
    ).toBeTruthy();
  });

  /// The copy button beside it, which is silent otherwise: a clipboard write
  /// changes nothing on the screen, and a press that looks like it did nothing
  /// gets pressed again.
  it("copies the viewer's link, and says it did", async () => {
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: (said: string) => {
          written.push(said);
          return Promise.resolve();
        },
      },
    });

    theGrillingStanding({ shared: SHARED });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);
    fireEvent.click(await drawn(container, `.${sharePane.copy}`));

    await waitFor(() => expect(written).toEqual([PUBLISHED]));
    await waitFor(() =>
      expect(container.querySelector(`.${sharePane.copy}`)!.textContent).toBe(
        "Copied",
      ),
    );
  });

  /// And a Conversation nobody has published one of says so plainly, rather
  /// than drawing a block of facts with nothing in it.
  it("says so plainly where nothing has been published", async () => {
    theGrillingStanding({});
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);

    expect(container.querySelector(`.${sharePane.published}`)).toBeNull();
    expect(
      screen.getByText(/This conversation has not been published\./),
    ).toBeTruthy();
  });
});

describe("publishing a share", () => {
  /// Two presses and one thing: the file to attach, and the same file put where
  /// a link reaches it. Both are offered on every conversation there is, because
  /// a share is the record as it stands and a record stands from the moment
  /// there is one.
  ///
  /// The download is a link rather than a press, because that is what a browser
  /// is for: the server answers as an attachment and names the file.
  it("offers publishing beside the download, with nothing to send", async () => {
    const fetching = theGrillingStanding(
      {},
      whenever(
        PUBLISHING,
        json({ Published: { share: SHARED } } satisfies SharePublished),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);

    const download = await drawn<HTMLAnchorElement>(
      container,
      `.${sharePane.download}`,
    );
    expect(download.getAttribute("href")).toBe(
      `/api/ui/conversations/${GRILLING.id}/share`,
    );
    expect(download.hasAttribute("download")).toBe(true);

    fireEvent.click(await drawn(container, `.${sharePane.publish}`));

    await waitFor(() => expect(sent(fetching, PUBLISHING)).toEqual({}));
  });

  /// And the moment itself carries the link, because it is worth reaching for
  /// while it is still what the human is thinking about.
  ///
  /// The link is the share viewer's rather than the gist's — composed by the
  /// server, so what this asks is that the toast opens what came back rather
  /// than something of its own: a toast pointing at the gist would hand
  /// somebody a page GitHub draws as source.
  it("puts the link it just made in the toast", async () => {
    theGrillingStanding(
      {},
      whenever(
        PUBLISHING,
        json({ Published: { share: SHARED } } satisfies SharePublished),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);
    fireEvent.click(await drawn(container, `.${sharePane.publish}`));

    const said = await waitFor(() =>
      screen.getByText("The share is published.", { exact: false }),
    );

    expect(said.closest(`.${toasts.toast}`)).toBeTruthy();
    expect(
      said.querySelector<HTMLAnchorElement>("a")!.getAttribute("href"),
    ).toBe(PUBLISHED);
  });

  /// And the press says what a second one would be: publishing again is a fresh
  /// snapshot rather than a second go at the same one.
  it("says publishing again where there is already a share", async () => {
    theGrillingStanding({ shared: SHARED });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);

    expect(
      (await drawn(container, `.${sharePane.publish}`)).textContent,
    ).toBe("Publish again");
  });

  /// The one press here whose failures are the human's to read. A token that
  /// cannot write gists is not a pane drawn against a conversation that moved:
  /// nothing moved, and a re-read would correct nothing — so what is wrong is
  /// said, with the way to the page it is fixed on inside the sentence.
  it("says which token trouble stopped it, and where to fix it", async () => {
    theGrillingStanding(
      {},
      whenever(PUBLISHING, json("NoGistScope" satisfies SharePublished), "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);
    fireEvent.click(await drawn(container, `.${sharePane.publish}`));

    const said = await waitFor(() =>
      screen.getByText("The saved GitHub token may not write gists.", {
        exact: false,
      }),
    );

    expect(said.closest(`.${toasts.toast}`)).toBeTruthy();
    expect(
      said.querySelector<HTMLAnchorElement>('a[href="/settings/github"]'),
    ).toBeTruthy();
  });

  /// And a Verkstead nobody has given a token is the other half of the same
  /// answer: there is nobody to publish as, and the settings page is where that
  /// is said.
  it("says so when there is no token to publish as", async () => {
    theGrillingStanding(
      {},
      whenever(PUBLISHING, json("NoToken" satisfies SharePublished), "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);
    fireEvent.click(await drawn(container, `.${sharePane.publish}`));

    await waitFor(() =>
      screen.getByText("Verkstead has no GitHub token to publish as.", {
        exact: false,
      }),
    );
  });
});

/// And where the whole of it is done in one press: the same publish, and a
/// comment on every pull request the conversation holds.
const SHARING_TO_PRS = `/api/ui/conversations/${GRILLING.id}/share/comment`;

/// One pull request the comment landed on, and one it did not — a conversation
/// that was worked in a companion repository ends on one each.
const ON_ITS_OWN = {
  number: 41,
  repo: null,
  url: "https://github.com/tobico/verkstead/pull/41#issuecomment-1",
};

const MISSED = {
  number: 7,
  repo: "verkstead-site",
  why: "`gh` said: Not Found (HTTP 404)",
};

describe("sharing a conversation to its pull requests", () => {
  /// The press that says something in front of other people, offered only where
  /// there is somewhere to say it: a conversation whose work is on no pull
  /// request has nowhere for this to go, and the pinned cards are what say so.
  it("is offered only where the work is on a pull request", async () => {
    theGrillingStanding({});
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const pane = await openShare(container);

    expect(pane.querySelector(`.${sharePane.comment}`)).toBeNull();
  });

  it("publishes and comments the link on every one of them", async () => {
    const fetching = theGrillingStanding(
      { pinned: WRAPPING.pinned },
      whenever(
        SHARING_TO_PRS,
        json({
          Commented: {
            share: SHARED,
            on: [ON_ITS_OWN, { ...MISSED, url: "https://github.com/x#1" }],
            missed: [],
          },
        } satisfies ShareCommented),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);
    fireEvent.click(await drawn(container, `.${sharePane.comment}`));

    await waitFor(() => expect(sent(fetching, SHARING_TO_PRS)).toEqual({}));

    // And where each of them went, named the way its card is: an unlabeled
    // number means the conversation's own repository.
    await waitFor(() =>
      screen.getByText("Commented on #41, #7 in verkstead-site."),
    );
  });

  /// A pull request the comment could not land on is named against the ones
  /// that worked. The share is published either way, so what the human needs is
  /// to be told where to paste the link themselves.
  it("names the pull request that missed out", async () => {
    theGrillingStanding(
      { pinned: WRAPPING.pinned },
      whenever(
        SHARING_TO_PRS,
        json({
          Commented: { share: SHARED, on: [ON_ITS_OWN], missed: [MISSED] },
        } satisfies ShareCommented),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);
    fireEvent.click(await drawn(container, `.${sharePane.comment}`));

    await waitFor(() =>
      screen.getByText(
        "Commented on #41. Nothing could be said on #7 in verkstead-site: " +
          "`gh` said: Not Found (HTTP 404)",
      ),
    );
  });

  /// A share that was never published says what the publish would have said:
  /// it is the same write to GitHub under the same token, and the settings page
  /// is where two of the three refusals are fixed.
  it("says which token trouble stopped it before anything was said", async () => {
    theGrillingStanding(
      { pinned: WRAPPING.pinned },
      whenever(
        SHARING_TO_PRS,
        json({
          NotPublished: { why: "NoGistScope" },
        } satisfies ShareCommented),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openShare(container);
    fireEvent.click(await drawn(container, `.${sharePane.comment}`));

    const said = await waitFor(() =>
      screen.getByText("The saved GitHub token may not write gists.", {
        exact: false,
      }),
    );

    expect(
      said.querySelector<HTMLAnchorElement>('a[href="/settings/github"]'),
    ).toBeTruthy();
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

  /// Following up is the same rule plus one: the work has to be on a pull
  /// request, and the pipeline has to have seen it through. A conversation still
  /// building has the ordinary ways of saying what to do next, so the target is
  /// drawn out there — and a steer is the only way into the state at all, which
  /// is why it is here rather than reachable some other way.
  it("offers following up only from done or wrapping up on a pull request", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: true, pinned: WRAPPING.pinned },
      whenever(STEERING, OVER_A_SESSION, "POST"),
    );
    const { container, unmount } = mount(`/conversations/${GRILLING.id}`);

    expect(targets(await openSteer(container))).toEqual([
      "Grilling",
      "Implementing",
      "Wrapping",
      "Done",
    ]);
    unmount();

    theGrillingStanding(
      {
        ready_to_stop: false,
        working: false,
        state: "Done",
        pinned: WRAPPING.pinned,
      },
      whenever(STEERING, OVER_NOTHING, "POST"),
    );
    const finished = mount(`/conversations/${GRILLING.id}`);

    expect(targets(await openSteer(finished.container))).toEqual([
      "Grilling",
      "Implementing",
      "Wrapping",
      "FollowUp",
      "Done",
    ]);
    expect(screen.getByText(/The pull request followed up on/)).toBeTruthy();
  });

  /// And the brief a follow-up is opened on is required whatever the record
  /// holds: nothing on the branch stands in for it, a follow-up being a thing
  /// the human wanted rather than a step of the run. So the field is what the
  /// target is, and the submit is held shut until it says something.
  it("requires the brief a follow-up is opened on, and sends it", async () => {
    const fetching = theGrillingStanding(
      {
        ready_to_stop: false,
        working: false,
        state: "Done",
        ready_to_continue: true,
        pinned: WRAPPING.pinned,
      },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(
        STEER_SUBMIT,
        json("Steered" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);

    expect(modal.querySelector("#steer-follow-up")).toBeNull();

    fireEvent.click(
      await drawn(modal, `.${steerModal.steerTarget} input[value="FollowUp"]`),
    );

    const press = (await drawn(
      modal,
      `.${steerModal.steerButtons} .${steerModal.steer}`,
    )) as HTMLButtonElement;

    await waitFor(() => expect(press.disabled).toBe(true));

    fireEvent.input(await drawn(modal, "#steer-follow-up"), {
      target: { value: "Does it count the 429s it sends?" },
    });

    await waitFor(() => expect(press.disabled).toBe(false));
    fireEvent.click(press);

    const building = GRILLING.implementation_pairing!;

    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toEqual({
        target: "FollowUp",
        interrupt: false,
        // Work being built, so it is the implementation pairing a follow-up
        // runs under.
        pairing: { profile_id: building.profile.id, model: building.model },
        // No round is being opened and no instruction written, so neither is
        // sent.
        brief: null,
        digest: false,
        // And nothing is being put in the sandbox either, which every target work
        // goes on in carries.
        added: [],
        upgraded: [],
        instruction: null,
        follow_up: "Does it count the 429s it sends?",
      }),
    );
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
        added: [],
        upgraded: [],
        instruction: "Note the window the count is against.",
        follow_up: null,
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

    await drawn(modal, "#steer-pairing");
    const interviewing = under(GRILLING.grilling_pairing)!;

    await waitFor(() =>
      expect(showing("Run it under")).toBe(readsAs(interviewing)),
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
      expect(showing("Run it under")).toBe(readsAs(building)),
    );

    pick("Run it under", READINGS[0]!);
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
        added: [],
        upgraded: [],
        instruction: null,
        follow_up: null,
      }),
    );
  });

  /// The modal's picker is the setup card's control in another place, so it draws
  /// what that one draws: the harness's mark in front of every reading, and in
  /// front of the one it is showing.
  it("marks the harness on every row of its own picker", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: false, pinned: WRAPPING.pinned },
      whenever(STEERING, OVER_NOTHING, "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);
    await drawn(modal, "#steer-pairing");

    expect(offers("Run it under")).toEqual(READINGS);
    expect(offered("Run it under").map(marked)).toEqual(
      READINGS.map(() => art(claudeMarkFile)),
    );
    expect(marked(picker("Run it under"))).toBe(art(claudeMarkFile));
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

    const own = under(GRILLING.grilling_pairing)!;

    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toEqual({
        target: "Grilling",
        interrupt: false,
        pairing: { profile_id: own.profile.id, model: own.model },
        brief: "# Retries\n\nThe backoff is wrong.\n",
        digest: true,
        added: [],
        upgraded: [],
        instruction: null,
        follow_up: null,
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
        // Nor a sandbox: nothing runs in done, so there is nothing a
        // companion could be for.
        added: [],
        upgraded: [],
        instruction: null,
        follow_up: null,
      }),
    );

    await waitFor(() =>
      expect(document.body.querySelector(`.${steerModal.steerConversation}`)).toBeNull(),
    );
  });

  /// Cancel is no press at all: the conversation stays where the click left it,
  /// stopped, with resume offered on it. That is accepted rather than a bug —
  /// the click is what froze the world while the human was composing.
  it("sends nothing when it is cancelled, and leaves resume offered", async () => {
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

    const press = await drawn(await openActions(container), `.${actions.resume}`);
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

  /// The companion section is sandbox setup rather than a payload of one state,
  /// so it is drawn under every target work goes on in — and not under done,
  /// where nothing runs and there is nothing a companion could be for.
  it("offers the repos to work alongside on every target work goes on in", async () => {
    theGrillingStanding(
      { ready_to_stop: true, working: false, ready_to_continue: true },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(`/api/ui/repos/${REPOS[0]!.id}/branches`, json(COMPANION_BRANCHES)),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);

    for (const target of ["Grilling", "Implementing"]) {
      fireEvent.click(
        await drawn(modal, `.${steerModal.steerTarget} input[value="${target}"]`),
      );

      const offered = await drawn(modal, `.${steerModal.steerAdding}`);

      expect(
        [...offered.querySelectorAll(`.${steerModal.steerAddName}`)].map(
          (row) => row.textContent,
        ),
      ).toEqual(["askance"]);
    }

    fireEvent.click(await drawn(modal, `.${steerModal.steerTarget} input[value="Done"]`));

    await waitFor(() =>
      expect(modal.querySelector(`.${steerModal.steerCompanions}`)).toBeNull(),
    );
  });

  /// And ticking one is what puts it in the submit, with everything a setup row
  /// would have said about it: how far in, off which branch, and under what
  /// name. The branch field opens only where there is a branch to name — a
  /// read-only companion is checked out detached.
  it("sends the repos ticked to go into the sandbox", async () => {
    const alongside = REPOS[0]!;
    const fetching = theGrillingStanding(
      { ready_to_stop: true, working: false, ready_to_continue: true },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(`/api/ui/repos/${alongside.id}/branches`, json(COMPANION_BRANCHES)),
      whenever(
        STEER_SUBMIT,
        json("Steered" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);

    // Nothing opens until the row is ticked: an untouched row says only that
    // the repository is registered.
    expect(modal.querySelector(`.${steerModal.steerAddConfig}`)).toBeNull();

    fireEvent.click(await drawn(modal, `.${steerModal.steerAddName} input`));

    const opened = await drawn(modal, `.${steerModal.steerAddConfig}`);

    // Read-only to begin with, which is the least a human has to say — and a
    // read-only checkout is detached, so there is no branch to name.
    expect(opened.querySelector(`.${steerModal.steerAddBranch}`)).toBeNull();

    fireEvent.click(await drawn(opened, "input[type='checkbox']"));

    const branch = (await drawn(
      opened,
      `#steer-companion-${alongside.id}-branch`,
    )) as HTMLInputElement;

    // Prefilled with the conversation's own branch, which is what mirroring
    // comes to: what the human reads is what they will get.
    expect(branch.value).toBe(GRILLING.branch);

    fireEvent.input(branch, { target: { value: "alongside" } });

    // The base dropdown is over the *companion's* own branches, which are read
    // when the row opens: picked once they are there, the way the setup card's
    // own is.
    const base = (await drawn(
      opened,
      `#steer-companion-${alongside.id}-base`,
    )) as HTMLSelectElement;

    await waitFor(() =>
      expect([...base.options].map((option) => option.value)).toContain(
        COMPANION_BRANCHES[0]!,
      ),
    );

    fireEvent.change(base, { target: { value: COMPANION_BRANCHES[0]! } });

    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.steer}`));

    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toMatchObject({
        added: [
          {
            repo_id: alongside.id,
            mode: "ReadWrite",
            base_ref: COMPANION_BRANCHES[0]!,
            branch: "alongside",
          },
        ],
      }),
    );
  });

  /// And a repo that came in read-only can be opened up on the row that says so,
  /// with the branch to cut beside the tick — the same question the setup card
  /// asked at draft time, asked at the one other moment it can be.
  ///
  /// One direction only: nothing here offers read-only and nothing offers
  /// removal, so a read-write row carries no control at all.
  it("opens a read-only repo up, and offers nothing on a read-write one", async () => {
    const askance = REPOS[0]!;
    const reading: CompanionView = {
      repo: askance,
      mode: "ReadOnly",
      base_ref: null,
      branch: "",
      worktree: { path: "/state/worktrees/askance-trunk", missing: false },
      base_commit: "c0ffee",
    };
    const fetching = theGrillingStanding(
      {
        ready_to_stop: true,
        working: false,
        ready_to_continue: true,
        companions: [reading],
      },
      whenever(STEERING, OVER_NOTHING, "POST"),
      whenever(
        STEER_SUBMIT,
        json("Steered" satisfies ConversationSteered),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);
    const row = await drawn(modal, `.${steerModal.steerAlong}`);

    // Nothing opens until the tick: the row says what the repo is and how far
    // into it the work reaches, and that is all.
    expect(row.querySelector(`.${steerModal.steerOpenBranch}`)).toBeNull();
    expect(row.textContent).toContain("read-only");

    fireEvent.click(await drawn(row, `.${steerModal.steerOpenUp} input`));

    // Ticked, the row says what it will be rather than what it was, and the
    // branch to cut opens under it.
    await waitFor(() => expect(row.textContent).toContain("read-write"));
    expect(row.textContent).not.toContain("read-only");

    const branch = (await drawn(
      row,
      `#steer-open-${askance.id}-branch`,
    )) as HTMLInputElement;

    // Prefilled with the conversation's own, which is what mirroring comes to.
    expect(branch.value).toBe(GRILLING.branch);

    fireEvent.input(branch, { target: { value: "alongside" } });
    fireEvent.click(await drawn(modal, `.${steerModal.steerButtons} .${steerModal.steer}`));

    // No mode on the wire, because there is one direction: a row that could
    // carry read-only would be a row that could take back what was given.
    await waitFor(() =>
      expect(sent(fetching, STEER_SUBMIT)).toMatchObject({
        added: [],
        upgraded: [{ repo_id: askance.id, branch: "alongside" }],
      }),
    );
  });

  /// And a read-write companion offers nothing at all: it is already as open as
  /// a repo gets, and there is no way back from it.
  it("offers no control on a companion that is read-write already", async () => {
    theGrillingStanding(
      {
        ready_to_stop: true,
        working: false,
        ready_to_continue: true,
        companions: [
          {
            repo: REPOS[0]!,
            mode: "ReadWrite",
            base_ref: null,
            branch: "",
            worktree: null,
            base_commit: null,
          },
        ],
      },
      whenever(STEERING, OVER_NOTHING, "POST"),
    );
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);
    const row = await drawn(modal, `.${steerModal.steerAlong}`);

    expect(row.querySelector(`.${steerModal.steerOpenUp}`)).toBeNull();
    expect(row.querySelector("input")).toBeNull();
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

  /// And the other press that stands above a move into wrapping is drawn as
  /// itself, which is the whole reason it is a kind of its own.
  ///
  /// A steer into wrapping carrying nothing written would otherwise be the same
  /// row and the same line — and the two are different acts: a steer opens the
  /// branch to be read again, and this leaves the review that carried the work
  /// to done standing. Which of them happened is what a record months old has to
  /// be readable back for.
  it("draws the resolve press as itself rather than as a steer", async () => {
    theGrillingStanding({
      state: "Wrapping",
      timeline: [
        ...GRILLING.timeline,
        { ResolveConflicts: { id: 9101, at: "2026-08-24T11:00:00Z" } },
        { Moved: { id: 9102, at: "2026-08-24T11:00:00Z", state: "Wrapping" } },
      ],
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const pressed = await drawn(
      container,
      `.${timeline.timelineEvent} > .${timeline.pressed}`,
    );

    expect(pressed.textContent).toBe("You asked for the conflict to be resolved");

    expect(
      container.querySelector(
        `.${timeline.timelineEvent} > .${timeline.steered}`,
      ),
      "and not as a steer, which would say the branch was read again",
    ).toBeNull();

    const moved = [
      ...container.querySelectorAll(
        `.${timeline.timelineEvent} > .${timeline.moved}`,
      ),
    ].map((line) => line.textContent);

    expect(moved.at(-1)).toBe("Grilling → Wrapping");
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
  /// recording it, said in the disc the sidebar's waiting card says it in.
  it("marks the one still waiting on the human", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.questionSet}`);
    const cards = [...container.querySelectorAll(`.${timeline.questionSet}`)];

    // The answered one, the one still waiting, the deferred one — which is
    // waiting too, the human being the one who has not answered either — and
    // the unreadable one, which is waiting on nobody, whatever the record says
    // about it, because nothing here can put its questions in front of anybody.
    expect(
      cards.map(
        (card) => card.querySelector(`.${marks.mark}.${marks.waiting}`) !== null,
      ),
    ).toEqual([false, true, true, false]);

    // The words are the disc's now rather than a badge's beside it, the card
    // having no label of its own to carry them.
    expect(screen.queryAllByText("waiting on you")).toHaveLength(0);
    expect(
      cards.map((card) =>
        card.querySelector(`.${marks.waiting}`)?.getAttribute("aria-label"),
      ),
    ).toEqual([undefined, "waiting on you", "waiting on you", undefined]);
  });

  /// At the right edge of the title's line, which is where every other mark on
  /// this page stands: the same class the session's ring is handed, so the two
  /// cannot drift apart. jsdom lays nothing out, so where that puts it is read
  /// off the rules.
  it("stands the disc at the edge of the head, centred on the line", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.questionSet}`);
    const disc = container.querySelector(
      `.${timeline.questionSet} .${timeline.eventHead} .${marks.waiting}`,
    )!;

    expect(disc.classList.contains(timeline.rowMark!)).toBe(true);
    expect(timelineCss).toContain(".rowMark {\n  margin-left: auto;\n}");
    // Against the head's own baseline alignment, so the disc sits on the middle
    // of the line rather than on the feet of the words.
    expect(marksCss).toContain("align-self: center;");
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
      if (!pane.querySelector("#preface")) {
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
      if (!pane.querySelector("#preface")) {
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

    // And it picks its shape from the pane rather than from the window: jsdom
    // lays nothing out, so the pane measures nought and the nav takes the
    // shape that asks nothing of a margin.
    expect(nav!.classList.contains(contents.roomy!)).toBe(false);
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
/// The diff is built from the one block of the answering set's own attached
/// Diff rather than written by hand: it is the same `DiffView`, rendered by the
/// same server-side renderer that a commit's diff goes through — which is the
/// whole reason a commit needs no diff machinery of its own. One repository's,
/// because a commit lands in one.
const COMMIT_PANE: CommitPane = {
  summary: null,
  diagrams: false,
  diff: readable(answeringSet).diff[0]!.diff,
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

  /// And the two counts take the app's own green and red, in both places a
  /// commit is drawn — the card here and the header of the pane it opens. They
  /// were a pair of hexes written twice: a green of their own, and a red that
  /// was very nearly unreadable against the dark scheme, the tokens being the
  /// only colours in the app that follow it.
  ///
  /// The stylesheet's, jsdom laying nothing out and computing no variable.
  it("colours the counts with the tokens the diff itself is coloured with", () => {
    for (const css of [timelineCss, commitPaneCss]) {
      expect(css).toContain("  color: var(--added);\n  font-weight: 600;");
      expect(css).toContain("  color: var(--removed);\n  font-weight: 600;");
    }

    // The pair they replaced, gone from both.
    expect(timelineCss).not.toContain("#1a7f37");
    expect(commitPaneCss).not.toContain("#1a7f37");
    expect(commitPaneCss).not.toContain("#b3382c");

    // And both tokens follow the scheme, which is the whole of what the change
    // buys: the hardcoded red never did.
    expect(base).toContain("--added: #2f7d4f;");
    expect(base).toContain("--removed: #b3382c;");
    expect(base).toContain("--added: #79c48f;");
    expect(base).toContain("--removed: #e0857a;");
  });

  /// The whole card, and nothing of what the commit said about itself: the
  /// Summary was clamped here under the counts and is the details pane's whole
  /// now, so the card is the line, the hash and the size of the change.
  ///
  /// Asserted as the markup, which is what says nothing else crept back in: a
  /// missing snippet reads the same as a snippet nobody wrote.
  it("draws the line and the counts, and nothing the commit said", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    const silent = COMMITS[0]!;
    const row = [
      ...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.commit}`),
    ].find((card) => card.querySelector(`.${timeline.subject}`)!.textContent === silent.subject)!;

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

    expect(COMMITS).toHaveLength(4);
    expect(subjects).toEqual(COMMITS.map((commit) => commit.subject));
  });

  /// Which repository a commit landed in, where that is not the conversation's
  /// own. The fixture's timeline carries both — two commits in the repo the work
  /// is in and one out of the companion — and only the companion's is labelled:
  /// an unlabelled card means the work's own repo.
  it("labels a commit that landed in a companion repo, and only that one", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    const labelled = COMMITS.filter((commit) => commit.repo !== null);
    expect(labelled).toHaveLength(1);

    const drawnLabels = [
      ...container.querySelectorAll(
        `.${timeline.timelineEvent} > .${timeline.commit} .${timeline.repo}`,
      ),
    ].map((it) => it.textContent);

    expect(drawnLabels).toEqual([labelled[0]!.repo]);
  });

  /// A merge is where a resolution session brought the base branch in and
  /// settled its conflicts. Its counts and its diff are the hunks the agent
  /// resolved, which is an ordinary small commit's worth — so the card says
  /// which it is, and every other card on the fixture's timeline says nothing.
  it("labels a merge, and only the merge", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    const merges = COMMITS.filter((commit) => commit.merge);
    expect(merges).toHaveLength(1);

    const labelled = [
      ...container.querySelectorAll(
        `.${timeline.timelineEvent} > .${timeline.commit}`,
      ),
    ].filter((card) => card.querySelector(`.${timeline.merge}`) !== null);

    expect(labelled).toHaveLength(1);
    expect(labelled[0]!.querySelector(`.${timeline.subject}`)!.textContent).toBe(
      merges[0]!.subject,
    );
    expect(labelled[0]!.querySelector(`.${timeline.merge}`)!.textContent).toBe(
      "Merge",
    );
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

    await waitFor(() => expect(frame(container).dataset.pane).toBe("middle"));
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
    expect(header.querySelector(`.${commitPane.repo}`)).toBeNull();
  });

  /// And which repository it landed in, where the pane is opened on a companion
  /// repo's commit: the same label the card carries, said again beside the hash,
  /// because the diff under it is that repository's rather than the work's own.
  it("says which repository a companion's commit came from", async () => {
    const companions = COMMITS.find((commit) => commit.repo !== null)!;

    theBuilding(
      {},
      whenever(
        `/api/ui/conversations/${BUILDING.id}/commit/${companions.id}`,
        json(COMMIT_PANE),
      ),
    );
    const { container } = mount(`/conversations/${BUILDING.id}`);

    await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`);

    fireEvent.click(
      [...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.commit}`)].find(
        (card) =>
          card.querySelector(`.${timeline.subject}`)!.textContent === companions.subject,
      )!,
    );

    const header = await drawn(container, `.${shell.detailsPane} .${commitPane.header}`);

    expect(header.querySelector(`.${commitPane.repo}`)!.textContent).toBe(companions.repo);
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

    const body = await drawn(container, `.${shell.detailsPane} #commit-message .${card.cardBody}`);

    expect(body.innerHTML).toBe("<p>A bucket per account.</p>");

    // In the box, under a heading of its own — the heading outside the box, as
    // a Preface's is.
    const message = body.closest(`.${card.card}`)!;

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

  /// And it is the *same* card, not a copy of one: the Message used to be its
  /// own five lines in `Commit.module.css` with a cap at the prose measure on
  /// top of them, so it sat narrow in a column a Preface spanned and a wide
  /// Diagram in it had nowhere to bleed to. One component now, and the pane has
  /// no box of its own left to disagree with.
  it("draws the message as the card a Preface is drawn as", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json(SUMMARISED)));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const message = await drawn(
      container,
      `.${shell.detailsPane} section#commit-message.${card.card}`,
    );

    expect(message.querySelector(`.${card.cardBody}`)).toBeTruthy();
    expect(commitPaneCss, "the pane keeps no box of its own").not.toContain(
      ".messageBody",
    );
  });

  /// What the card is, asserted off the stylesheet: jsdom computes no layout, and
  /// a media query it never evaluates is a rule no rendered page can be asked
  /// about.
  ///
  /// Two facts make the two look alike. The card spans the column — no measure on
  /// it, so the prose inside keeps the measure block by block and a table, a fence
  /// or a Diagram keeps the card. And it reserves the Gutter, which is what gives
  /// a wide block somewhere to bleed back into: `--bleed` is how far, and a
  /// Diagram takes all of it.
  it("spans the column and reserves the Gutter, in one place for both", () => {
    expect(cardCss, "a measure on the card would take the wide blocks down with the prose").not.toContain(
      "max-width",
    );
    expect(cardCss).toContain("--bleed: var(--gutter);");
    expect(cardCss).toContain("padding-left: calc(1rem + var(--gutter));");

    // And the Gutter it reserves is the one every page's column names, which is
    // the whole of the wiring: a details pane inherits it from there as a page
    // does, so the one rule above serves the Set page and this pane alike.
    expect(base).toContain("  main {\n    --gutter: 4.5rem;");
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

    const body = await drawn(container, `.${shell.detailsPane} #commit-message .${card.cardBody}`);

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

    /// The timeline row for one of the two, which is the only way to tell them
    /// apart on the record. Named for the row rather than for the card it is
    /// drawn as, because `card` here is the module a Message is boxed in.
    const commitCard = (subject: string) =>
      [...container.querySelectorAll(`.${timeline.timelineEvent} > .${timeline.commit}`)].find(
        (row) => row.querySelector(`.${timeline.subject}`)!.textContent === subject,
      )!;

    /// The message block once it is holding the commit that was clicked, rather
    /// than the one before it: which commit the pane is showing is exactly what
    /// this test is about, so waiting for the block alone would prove nothing.
    const showing = (words: string) =>
      waitFor(() => {
        const block = container.querySelector(`.${shell.detailsPane} #commit-message .${card.cardBody}`);
        if (!block?.textContent?.includes(words)) {
          throw new Error(`the pane is not showing ${words} yet`);
        }
        return block;
      });

    fireEvent.click(commitCard(COMMITS[0]!.subject));
    const first = await showing("A bucket per account.");
    await waitFor(() => expect(drawing).toHaveBeenCalledOnce());
    expect(drawing.mock.calls[0]![0]).toEqual({ root: first });

    fireEvent.click(commitCard(COMMITS[1]!.subject));
    const second = await showing("A queue per repository.");
    await waitFor(() => expect(drawing).toHaveBeenCalledTimes(2));
    expect(drawing.mock.calls[1]![0]).toEqual({ root: second });

    // Back to the first, which the cache still holds and hands back whole.
    fireEvent.click(commitCard(COMMITS[0]!.subject));
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
    await drawn(container, `.${shell.detailsPane} #commit-message .${card.cardBody}`);

    expect(drawing).not.toHaveBeenCalled();
  });

  /// The ordinary commit, and every commit recorded before summaries were kept:
  /// the pane is the header and the diff, exactly as it always was.
  it("draws nothing where the commit carried no message", async () => {
    theCommits();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));
    await drawn(container, `.${shell.detailsPane} .${diffSection.diffFiles}`);

    expect(container.querySelector(`.${shell.detailsPane} #commit-message .${card.cardBody}`)).toBeNull();
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
    expect(row.classList).not.toContain(pressable.open);

    fireEvent.click(row);

    await waitFor(() => expect(row.classList).toContain(pressable.open));
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

  /// And it stands above both of them in the pane, which is where the sidebar
  /// starts: the stylesheet pins the nav from where it sits in the flow, so one
  /// written under the Message would begin level with the diff and leave the
  /// margin beside the Message empty. The Set page puts its own in the same
  /// place — under the title, above everything it lists.
  it("starts the nav above the message it lists, not level with the diff", async () => {
    theBuilding({}, whenever(DIFF_OF_IT, json(SUMMARISED)));
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.timelineEvent} > .${timeline.commit}`));

    const nav = await drawn(container, `.${shell.detailsPane} nav.${contents.contents}`);
    const message = container.querySelector(`.${shell.detailsPane} #commit-message`)!;

    expect(
      nav.compareDocumentPosition(message) & Node.DOCUMENT_POSITION_FOLLOWING,
      "the nav should come before the message it lists",
    ).toBeTruthy();

    // And under the pane's own header, which is what says the commit is.
    const header = container.querySelector(`.${shell.detailsPane} .${commitPane.header}`)!;

    expect(
      header.compareDocumentPosition(nav) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
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
  /// on the record stays — it is a moment that happened — but nothing is drawn
  /// at it, the entry included: the record is a column with a rem between its
  /// entries, so an entry with nothing in it is two rems of blank paper where
  /// the backlog landed rather than nothing at all.
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

    // And no empty entry left behind where the card would have been: one entry
    // per event with something to draw, which is every event but this one.
    expect(
      container.querySelectorAll(`.${timeline.timelineEvent}`),
    ).toHaveLength(TASKED.timeline.length - 1);
  });

  /// Nothing pins or unpins one: the set is fixed, so there is no control for
  /// it. What the card does have is the one press its whole surface is — the
  /// documents its entries name, in the details pane.
  it("is one press and offers nothing else", async () => {
    theTasked();
    const { container } = mount(unopened(TASKED));

    const list = await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);

    expect(list.querySelectorAll("button")).toHaveLength(0);
    expect(list.textContent).not.toContain("Pin");
    expect(list.getAttribute("role")).toBe("button");
    expect(list.getAttribute("aria-pressed")).toBe("false");
  });

  /// And it *says* it is one, in the same three ways the copy on the record
  /// does. The card is drawn twice, so a rule that reached only the record
  /// would have the human press the card held above the pane and watch a
  /// different one light up. jsdom lays nothing out, so the rules themselves
  /// are what is read.
  it("reads as pressable wherever it is drawn", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);

    // The card carries what every pressable card in the app carries, and the
    // rules are that component's: unscoped by construction, so the pinned copy
    // and the copy on the record cannot come apart.
    expect(list.classList.contains(pressable.pressable!)).toBe(true);

    for (const rule of [
      ".pressable {\n  cursor: pointer;\n}",
      ".open {\n  background: var(--card);\n}",
      "  .pressable:hover {",
    ]) {
      expect(pressableCss).toContain(rule);
    }

    // And the record's own sheet has given up saying any of it a second time,
    // which is what would have split the two copies again.
    expect(timelineCss).not.toContain(".openable");
    for (const kind of ["agentOutput", "questionSet", "commit"]) {
      expect(timelineCss).not.toContain(`.timelineEvent > .${kind} {`);
      expect(timelineCss).not.toContain(`.timelineEvent > .${kind}.selected`);
    }
  });

  /// Pressing it marks it — both copies of it, because the two are one backlog
  /// and there is one details pane behind them.
  it("marks both copies of itself while its pane is open", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.taskList}`);

    fireEvent.click(list);

    await waitFor(() =>
      expect(
        container.querySelectorAll(
          `.${timeline.taskList}.${pressable.open}`,
        ),
      ).toHaveLength(2),
    );

    expect(list.getAttribute("aria-pressed")).toBe("true");
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
    // between them rather than against the record. And what settles it is the
    // menu's own layer: what it drops carries one and the pinned deck carries
    // none, so nothing about the header in between has to say anything.
    expect(menu.closest(`.${shell.paneChrome}`)).not.toBeNull();
    expect(menuCss).toContain(
      ".drop {\n" +
        "  position: absolute;\n" +
        "  top: calc(100% + 0.3rem);\n" +
        "  right: 0;\n" +
        "  z-index: 3;\n",
    );
    expect(timelineCss).not.toMatch(/\.carousel > \.deck \{[^}]*z-index/);
  });

  it("draws nothing at all where the worktree holds no backlog", async () => {
    // Every other fixture here is a conversation with no `.tasks/`, which is
    // the ordinary case: the server pins nothing and there is nothing to draw.
    expect(OPEN.pinned).toEqual([]);

    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${timeline.timeline}`);

    expect(container.querySelector(`.${timeline.pinned}`)).toBeNull();
    expect(container.querySelector(`.${timeline.taskList}`)).toBeNull();
  });
});

/// The backlog opened, as the details pane fetches it: one document per entry,
/// done or not — a task file stays in `.tasks/` until the feature is finished
/// with. The last entry names a document nobody wrote, which is the one way a
/// section comes back empty.
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
    done: task.done,
    html:
      task.number === "04"
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

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);

    const sections = [
      ...container.querySelectorAll(`.${shell.detailsPane} .${documents.section}`),
    ];

    expect(sections.map((section) => section.id)).toEqual(
      BACKLOG.tasks.map((task) => `task-${task.number}`),
    );
    expect(
      sections.map((section) => [
        section.querySelector(`.${documents.n}`)!.textContent,
        section.querySelector(`.${documents.what}`)!.textContent,
      ]),
    ).toEqual(BACKLOG.tasks.map((task) => [task.number, task.title]));

    // The Preface's own treatment: the heading outside the box, the rendered
    // markdown in it, put in the page as the server wrote it.
    const outstanding = sections[BACKLOG.tasks.findIndex((task) => !task.done)]!;
    const body = outstanding.querySelector(`.${documents.document}`)!;

    expect(body.classList).toContain("markdown");
    expect(body.querySelector("h2")!.textContent).toBe("What to build");
    expect(outstanding.querySelector("h2")!.closest(`.${documents.document}`)).toBeNull();
    expect(documentsCss).toContain(
      ".document,\n.missing {\n  padding: 1rem;\n  background: var(--card);",
    );

    expect(askedFor(fetching, THE_BACKLOG)).toBeGreaterThan(0);
  });

  /// A task document stays in `.tasks/` until the feature is finished with, so a
  /// done task has one like any other and the heading is where the done state
  /// goes — the roadmap pane's own arrangement, and the checkbox in `TODO.md` is
  /// what both are drawn from.
  it("marks the done tasks on their own headings, documents and all", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);

    const sections = [
      ...container.querySelectorAll(`.${shell.detailsPane} .${documents.section}`),
    ];

    expect(
      sections.map((section) => section.querySelector(`.${documents.mark}`)!.textContent),
    ).toEqual(BACKLOG.tasks.map((task) => (task.done ? "done" : "to do")));

    // And the done ones are drawn with their documents all the same.
    expect(
      sections
        .filter((_, at) => BACKLOG.tasks[at]!.done)
        .every((section) => section.querySelector(`.${documents.document}`) !== null),
    ).toBe(true);
  });

  /// The one thing a task has no document for is the list naming a file nobody
  /// wrote, which is the human's to fix and so is said in words.
  it("says so where the list names a document that is not there", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    const missing = await drawn(
      container,
      `.${shell.detailsPane} .${documents.missing}`,
    );

    expect(missing.textContent).toBe(
      "The list names a task document that is not there to read.",
    );
    expect(missing.closest(`.${documents.section}`)!.id).toBe("task-04");
  });

  /// The set page's own table of contents, one line per task: a done one is
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

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);

    const both = [...container.querySelectorAll(`.${timeline.taskList}`)];

    expect(both).toHaveLength(2);
    expect(both.every((card) => card.classList.contains(pressable.open!))).toBe(true);
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
      (await drawn(container, `.${shell.detailsPane} .${documents.feature}`)).textContent,
    ).toBe(BACKLOG.feature);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${paneHead.back}`));

    await waitFor(() => expect(frame(container).dataset.pane).toBe("middle"));
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
  /// above the record and again on it, with nothing to pin or unpin. What it
  /// does have is the one press its whole surface is — the briefs its stages
  /// name, in the details pane.
  it("is drawn above the record and is one press and nothing else", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, `.${timeline.pinned} .${timeline.stageList}`);

    expect(list.closest(`.${timeline.timeline}`)).toBeNull();
    expect(list.querySelectorAll("button")).toHaveLength(0);
    expect(list.textContent).not.toContain("Pin");
    expect(list.getAttribute("role")).toBe("button");
    expect(list.getAttribute("aria-pressed")).toBe("false");
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
  /// leaves the row with no card to draw at it — and no entry on the record
  /// either, for the backlog's reason: an empty one is a gap rather than
  /// nothing.
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

    expect(
      container.querySelectorAll(`.${timeline.timelineEvent}`),
    ).toHaveLength(STAGED.timeline.length - 1);
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

    // Titled as every other card on the record is, so a column of them reads
    // down the same left edge.
    expect(notice.querySelector("h2")!.textContent).toBe("Notice");

    // And quiet among them: the dim ink a size down is what keeps a fact about
    // the work from reading as loudly as the work.
    expect(timelineCss).toContain(
      "  font-size: 0.9rem;\n  color: var(--ink-soft);\n}",
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

/// A backlog of ten, the first `done` of them ticked: longer than the five a
/// card draws, which is the only thing the window shows up on.
function ofTen(done: number): TaskListEvent {
  return {
    feature: BACKLOG.feature,
    tasks: Array.from({ length: 10 }, (_, at) => ({
      number: `${at + 1}`.padStart(2, "0"),
      title: `Task ${at + 1}`,
      done: at < done,
    })),
  };
}

/// The same roadmap one level up, so the two cards can be read against each
/// other.
function stagesOfTen(done: number): StageListEvent {
  return {
    name: ROADMAP.name,
    title: ROADMAP.title,
    stages: ofTen(done).tasks.map((task) => ({
      number: task.number,
      title: task.title,
      done: task.done,
    })),
  };
}

/// That backlog in both of the places the card is drawn from — the pinned block
/// and the row on the record — because the window has to be the same in both.
function tenTasked(done: number): Partial<ConversationView> {
  return {
    pinned: [{ TaskList: ofTen(done) }],
    timeline: TASKED.timeline.map((event) =>
      "TaskList" in event
        ? { TaskList: { ...event.TaskList, list: ofTen(done) } }
        : event,
    ),
  };
}

/// What one card is showing: the numbers of the rows it drew, and whether there
/// is an ellipsis above them and below them.
function through(card: Element) {
  const rows = [...card.querySelectorAll("ol > li")];
  return {
    entries: rows
      .filter((row) => !row.classList.contains(timeline.more!))
      .map((row) => row.querySelector(`.${timeline.n}`)!.textContent),
    above: rows[0]?.classList.contains(timeline.more!) ?? false,
    below: rows[rows.length - 1]?.classList.contains(timeline.more!) ?? false,
  };
}

/// A card that stays the size of a phone's screen whatever the backlog behind
/// it is: five entries around the one being worked, and a mark at whichever end
/// the rest of them are at. The whole list is a press away, in the pane.
describe("a checklist longer than its card", () => {
  it("windows a backlog to five around the task being worked", async () => {
    for (const [done, entries] of [
      [0, ["01", "02", "03", "04", "05"]],
      [5, ["04", "05", "06", "07", "08"]],
      [9, ["06", "07", "08", "09", "10"]],
    ] as const) {
      theTasked(tenTasked(done));
      const { container, unmount } = mount(`/conversations/${TASKED.id}`);

      const card = await drawn(
        container,
        `.${timeline.pinned} .${timeline.taskList}`,
      );

      expect(through(card)).toEqual({
        entries,
        above: entries[0] !== "01",
        below: entries[4] !== "10",
      });

      unmount();
    }
  });

  /// The count the card cannot draw as an ellipsis: the glyph says the list
  /// goes on and the word says how far, for the reader that hears the row
  /// rather than sees it.
  it("says how many are hidden at each end, in words", async () => {
    theTasked(tenTasked(5));
    const { container } = mount(`/conversations/${TASKED.id}`);

    const card = await drawn(
      container,
      `.${timeline.pinned} .${timeline.taskList}`,
    );

    expect(
      [...card.querySelectorAll(`.${timeline.more}`)].map((row) => [
        row.querySelector("[aria-hidden]")!.textContent,
        row.querySelector(`.${timeline.state}`)!.textContent,
      ]),
    ).toEqual([
      ["…", "3 more"],
      ["…", "2 more"],
    ]);
  });

  /// The progress line is the one thing on the card that still knows the whole
  /// list — it is what the window is read against.
  it("still counts the whole backlog above the window", async () => {
    theTasked(tenTasked(5));
    const { container } = mount(`/conversations/${TASKED.id}`);

    const head = await drawn(
      container,
      `.${timeline.pinned} .${timeline.taskList} .${timeline.eventHead}`,
    );

    expect(head.querySelector(`.${timeline.progress}`)!.textContent).toBe(
      "5 of 10 done",
    );
  });

  /// One reading behind two cards: the copy on the record is the same card, so
  /// it is at the same place in the list.
  it("windows the copy on the record to the same five", async () => {
    theTasked(tenTasked(5));
    const { container } = mount(`/conversations/${TASKED.id}`);

    const card = await drawn(
      container,
      `.${timeline.timeline} .${timeline.taskList}`,
    );

    expect(through(card)).toEqual({
      entries: ["04", "05", "06", "07", "08"],
      above: true,
      below: true,
    });
  });

  /// A stage list outlives its own completion — the roadmap card stays on a
  /// conversation that has finished every stage of it — so there is no next
  /// entry to centre on and the end of the list is where the work got to.
  it("windows a roadmap the same way, and its finished one to the last five", async () => {
    theStaged({ pinned: [{ StageList: stagesOfTen(4) }] });
    const first = mount(`/conversations/${STAGED.id}`);

    expect(
      through(
        await drawn(
          first.container,
          `.${timeline.pinned} .${timeline.stageList}`,
        ),
      ),
    ).toEqual({
      entries: ["03", "04", "05", "06", "07"],
      above: true,
      below: true,
    });

    first.unmount();

    theStaged({ pinned: [{ StageList: stagesOfTen(10) }] });
    const { container } = mount(`/conversations/${STAGED.id}`);

    expect(
      through(
        await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
      ),
    ).toEqual({
      entries: ["06", "07", "08", "09", "10"],
      above: true,
      below: false,
    });
  });

  /// The four-entry fixtures either side of this are the ordinary case, and
  /// nothing about them changed: a list the card can hold is drawn whole, with
  /// nothing to say about what is missing.
  it("leaves a list that already fits alone", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const card = await drawn(
      container,
      `.${timeline.pinned} .${timeline.taskList}`,
    );

    expect(through(card)).toEqual({
      entries: BACKLOG.tasks.map((task) => task.number),
      above: false,
      below: false,
    });
    expect(card.querySelectorAll(`.${timeline.more}`)).toHaveLength(0);
  });

  /// The pane the card opens is where the whole list is read, so nothing there
  /// is windowed — that is the trade the card is making.
  it("draws every task of it in the details pane all the same", async () => {
    theTasked(tenTasked(5), whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);

    expect(
      container.querySelectorAll(
        `.${shell.detailsPane} .${documents.section}`,
      ),
    ).toHaveLength(BACKLOG_PANE.tasks.length);
  });
});

/// The roadmap opened, as the details pane fetches it: one brief per stage, done
/// or not — a stage's brief stays where it is for ever.
///
/// Written by hand rather than taken from a fixture, as the backlog pane's is:
/// the fixtures are of the conversation endpoint, and this is a pane's own
/// payload.
const ROADMAP_PANE: RoadmapPane = {
  name: ROADMAP.name,
  title: ROADMAP.title,
  diagrams: false,
  stages: ROADMAP.stages.map((stage) => ({
    number: stage.number,
    title: stage.title,
    done: stage.done,
    html:
      stage.number === "04"
        ? null
        : `<h1>${stage.number}. ${stage.title}</h1>\n<h2>What to build</h2>\n` +
          `<p>The ${stage.title.toLowerCase()} of it.</p>`,
  })),
};

/// Where the details pane fetches it from — the conversation and the roadmap's
/// own directory name, a worktree being allowed any number of roadmaps.
const THE_ROADMAP = `/api/ui/conversations/${STAGED.id}/roadmap/${ROADMAP.name}`;

describe("the stage list opened", () => {
  /// What the card is pressed for: the briefs its stages name, which is the one
  /// thing about a roadmap the card cannot show — the backlog pane one level up,
  /// drawn by the same component into the same boxed sections.
  it("draws every stage brief as its own boxed section, in the roadmap's order", async () => {
    const fetching = theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);

    const sections = [
      ...container.querySelectorAll(`.${shell.detailsPane} .${documents.section}`),
    ];

    expect(sections.map((section) => section.id)).toEqual(
      ROADMAP.stages.map((stage) => `stage-${stage.number}`),
    );
    expect(
      sections.map((section) => [
        section.querySelector(`.${documents.n}`)!.textContent,
        section.querySelector(`.${documents.what}`)!.textContent,
      ]),
    ).toEqual(ROADMAP.stages.map((stage) => [stage.number, stage.title]));

    // The Preface's own treatment, as the backlog pane draws it: the heading
    // outside the box, the rendered markdown in it.
    const body = sections[0]!.querySelector(`.${documents.document}`)!;

    expect(body.classList).toContain("markdown");
    expect(body.querySelector("h2")!.textContent).toBe("What to build");
    expect(sections[0]!.querySelector("h2")!.closest(`.${documents.document}`)).toBeNull();

    expect(askedFor(fetching, THE_ROADMAP)).toBeGreaterThan(0);
  });

  /// A stage's brief stays where it is for ever, so a done stage has a document
  /// like any other and the heading is where the done state goes — the backlog
  /// pane's own arrangement, one level up.
  it("marks the done stages on their own headings, briefs and all", async () => {
    theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);

    const sections = [
      ...container.querySelectorAll(`.${shell.detailsPane} .${documents.section}`),
    ];

    expect(
      sections.map((section) => section.querySelector(`.${documents.mark}`)!.textContent),
    ).toEqual(ROADMAP.stages.map((stage) => (stage.done ? "done" : "to do")));

    // And the done ones are drawn with their briefs all the same.
    expect(
      sections
        .filter((_, at) => ROADMAP.stages[at]!.done)
        .every((section) => section.querySelector(`.${documents.document}`) !== null),
    ).toBe(true);
  });

  /// The one thing a stage has no document for is a roadmap pointing at a brief
  /// nobody wrote, which is the human's to fix and so is said in words.
  it("says so where the roadmap names a brief that is not there", async () => {
    theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
    );

    const missing = await drawn(
      container,
      `.${shell.detailsPane} .${documents.missing}`,
    );

    expect(missing.textContent).toBe(
      "The roadmap names a brief that is not there to read.",
    );
    expect(missing.closest(`.${documents.section}`)!.id).toBe("stage-04");
  });

  /// The set page's own table of contents, one line per stage.
  it("offers a jump to each stage", async () => {
    theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
    );

    const nav = await drawn(container, `.${shell.detailsPane} .${contents.contents}`);
    const lines = [...nav.querySelectorAll(`.${contents.sections} > li a`)];

    expect(lines.map((line) => line.getAttribute("href"))).toEqual(
      ROADMAP.stages.map((stage) => `#stage-${stage.number}`),
    );
    expect(lines.map((line) => line.textContent)).toEqual(
      ROADMAP.stages.map((stage) => `${stage.number} ${stage.title}`),
    );
  });

  /// One roadmap in two places, so opening either opens the one pane and both
  /// read as selected while it is open — the backlog's own arrangement.
  it("opens from the row on the record as well as from the pinned card", async () => {
    theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.timeline} .${timeline.stageList}`),
    );

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);

    const both = [...container.querySelectorAll(`.${timeline.stageList}`)];

    expect(both).toHaveLength(2);
    expect(both.every((card) => card.classList.contains(pressable.open!))).toBe(true);
    expect(both.every((card) => card.getAttribute("aria-pressed") === "true")).toBe(true);
  });

  /// Titled for the card, and named for the roadmap under the header: which of
  /// a repository's roadmaps this is, is what the pane has to say for itself.
  it("is titled for the card, and walks back out to the record", async () => {
    theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
    );

    // Waited for by its title rather than by being a pane head at all: this
    // conversation opened on the end of its record, so a head was there before
    // the card was ever pressed and the change is which one it is.
    const head = await waitFor(() => {
      const found = container.querySelector<HTMLElement>(
        `.${shell.detailsPane} .${paneHead.head}`,
      );
      expect(found?.querySelector("h1")?.textContent).toBe("Roadmap");
      return found!;
    });

    expect(head.textContent).not.toContain("Close");
    expect(
      (await drawn(container, `.${shell.detailsPane} .${documents.feature}`)).textContent,
    ).toBe(ROADMAP.title);

    fireEvent.click(await drawn(container, `.${shell.detailsPane} .${paneHead.back}`));

    await waitFor(() => expect(frame(container).dataset.pane).toBe("middle"));
  });

  /// The server refuses cleanly where the worktree or the roadmap has gone, and
  /// the pane says what it was told rather than spinning.
  it("says what went wrong where there is no roadmap left to read", async () => {
    theStaged(
      {},
      whenever(
        THE_ROADMAP,
        json({ error: "there is no roadmap of that name on that Conversation" }, 404),
      ),
    );
    const { container } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
    );

    const line = await drawn(container, `.${shell.detailsPane} .${notices.error}`);

    expect(line.textContent).toContain(
      "there is no roadmap of that name on that Conversation",
    );
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

  /// The card is the press and holds no control of its own, which is what it
  /// has in common with every other card on the record: what gets the work
  /// going again is Resume, at the foot of the timeline.
  it("is the press itself, and holds nothing else to press", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const notice = await drawn(container, `.${timeline.timeline} .${timeline.notice}`);

    expect(notice.querySelector("button")).toBeNull();
    expect(notice.classList.contains(pressable.pressable!)).toBe(true);
    expect(notice.getAttribute("role")).toBe("button");
  });

  /// And what it opens is the whole of what the card cut off at a line: the
  /// reason the run stopped, and the terminal output under it.
  it("opens the whole of what it said in the details pane", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.timeline} .${timeline.notice}`),
    );

    const pane = await drawn(
      container,
      `.${shell.detailsPane} .${documentPane.document}`,
    );

    expect(
      container.querySelector(`.${shell.detailsPane} .${paneHead.head} h1`)!
        .textContent,
    ).toBe("Notice");
    expect(pane.textContent).toContain("the session exited with status 1");
    expect(pane.querySelectorAll("pre").length).toBe(2);
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
  /// One stopped shape: the notice saying what stopped and why, the status
  /// button saying the work is waiting on the human, and the one Resume in the
  /// menu it drops. The same three things a run stopped by a press draws — see
  /// the stop above.
  it("draws the card, the status and the row every stop draws", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    await drawn(container, `.${timeline.timeline} .${timeline.notice}`);

    const notice = [...container.querySelectorAll(`.${timeline.timeline} .${timeline.notice}`)].find(
      (drawn) => drawn.textContent!.includes("stopped"),
    );

    expect(notice!.textContent).toContain("Implementing the work");
    expect(notice!.textContent).toContain("opus");
    expect(notice!.textContent).toContain("is out of window");


    expect(
      (await drawn(container, `.${statusButton.standing}`)).textContent,
    ).toContain("Waiting on you");
    expect(
      (await drawn(await openActions(container), `.${actions.resume} .${actions.title}`))
        .textContent,
    ).toBe("Resume");
  });

  /// The one thing that tells this stop from any other is when the account
  /// comes back, and it is said on the resume row of the actions menu — the row
  /// the press is on, this being a stop that waits for the press *and* for an
  /// account. In the words the session printed them in, and in front of the
  /// sentence every resume row says, because *when* is the half the human does
  /// not already know.
  ///
  /// Nowhere on the status button, which says its status word either way: the
  /// button has one line and the work to say it about.
  it("says on the resume row when the account comes back", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    const resume = await drawn(await openActions(container), `.${actions.resume}`);

    expect(resume.querySelector(`.${actions.says}`)!.textContent).toBe(
      "Out of window until 3pm. Work out what should be running from where the work stands, and start it.",
    );

    // And nothing of it on the button that drops the menu.
    expect(
      (await drawn(container, `.${statusButton.status} .${statusButton.what}`))
        .textContent,
    ).not.toContain("Out of window");
  });

  /// A conversation stopped by a press carries no such words, which is the whole
  /// of the difference between the two: the row says what the press does and
  /// nothing about when, there being nothing to wait for but the press.
  it("is the only thing a conversation stopped by a press draws differently", async () => {
    expect(STOPPED.resets).toBeNull();

    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const resume = await drawn(await openActions(container), `.${actions.resume}`);

    expect(resume.querySelector(`.${actions.says}`)!.textContent).toBe(
      "Work out what should be running from where the work stands, and start it.",
    );
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

    fireEvent.click(
      await drawn(container, `.${statusButton.status} > .${dropdown.trigger}`),
    );

    await waitFor(() =>
      expect(
        fetching.mock.calls
          .map(([asked]) => String(asked))
          .filter((path) => path.includes("/pause/")),
      ).toEqual([]),
    );
  });

  /// The badge that pointed at the notice used to be the way to it, and the
  /// press that took over its place does something else: the status button
  /// opens what there is to *do* about the stop. The jump was dropped rather
  /// than moved — the notice is on the record, the record opens at its end, and
  /// a stop wrote nothing after the notice saying so.
  it("says the stop without sending anybody anywhere", async () => {
    thePaused();
    const { container } = mount(unopened(WAITING));

    fireEvent.click(
      await drawn(container, `.${statusButton.status} > .${dropdown.trigger}`),
    );

    await drawn(container, `.${statusButton.status} > .${dropdown.drop}`);
    expect(
      container.querySelector(`.${timeline.notice}.${pressable.open}`),
    ).toBeNull();
  });

  /// And it is marked where it stands as well, whether or not it is the one
  /// being read: the badge is how a stop is found on a long record, and what
  /// finds it is the edge in the colour that means stopped.
  it("marks the notice the run stopped at, apart from the one being read", async () => {
    thePaused();
    const { container } = mount(unopened(WAITING));

    const marked = await drawn(
      container,
      `.${timeline.timeline} .${timeline.notice}.${timeline.blocking}`,
    );

    expect(marked.textContent).toContain("Implementing the work");
    // Nothing is open, so the mark is not about being open.
    expect(marked.classList.contains(pressable.open!)).toBe(false);

    // And it is the last card on the record, which is what the stylesheet
    // leans on: a run that stopped wrote nothing after the notice saying so,
    // so the notice is found by being at the end rather than by any paint.
    const cards = [...container.querySelectorAll(`.${timeline.timelineEvent} > *`)];
    expect(cards.at(-1)).toBe(marked);
  });
});

/// The one place the Conversation pane says where the work stands: a one-line
/// button at the foot of the sticky block, and behind its press everything
/// there is to do about the Conversation.
///
/// It replaced five pieces of chrome that had each been put where there was
/// room for it — a Done/Closed word, a *Blocked on you* badge, a *Waiting on
/// checks* label and the ⋯ that hid the actions — so what is asked here is that
/// there is one of it, that it is where the eye lands, and that the press that
/// used to be a mark at the end of the header row is the whole button now.
describe("the status button", () => {
  /// Last in the block rather than first: the sticky area ends where this
  /// button ends, and a control along that edge is what says so.
  it("stands in the sticky chrome, under the pinned cards and last in it", async () => {
    theTasked();
    const { container } = mount(`/conversations/${TASKED.id}`);

    const button = await drawn(container, `.${statusButton.status}`);
    const chrome = button.parentElement!;

    expect(chrome.classList).toContain(shell.paneChrome);

    const inside = [...chrome.children];
    expect(inside.indexOf(chrome.querySelector(`.${paneHead.head}`)!)).toBeLessThan(
      inside.indexOf(chrome.querySelector(`.${timeline.pinned}`)!),
    );
    expect(
      inside.indexOf(chrome.querySelector(`.${timeline.pinned}`)!),
    ).toBeLessThan(inside.indexOf(button));
    expect(inside.indexOf(button)).toBe(inside.length - 1);
  });

  /// The press is the whole button rather than a mark at the end of a row,
  /// which is the point of the move: what there is to do about a Conversation
  /// is reached from the thing that says what it is doing.
  it("opens the conversation's actions when it is pressed", async () => {
    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(
      await drawn(container, `.${statusButton.status} > .${dropdown.trigger}`),
    );

    const menu = await drawn(
      container,
      `.${statusButton.status} > .${dropdown.drop}`,
    );

    expect(menu.querySelector(`.${actions.steer}`)).toBeTruthy();
  });

  /// And says so, in the mark every other thing that drops a menu says it in.
  it("carries the chevron that says it opens", async () => {
    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const mark = await drawn(
      container,
      `.${statusButton.status} > .${dropdown.trigger} .${statusButton.mark}`,
    );

    expect(mark.tagName).toBe("svg");
    // A mark rather than a word, and no part of what the button says.
    expect(mark.getAttribute("aria-hidden")).toBe("true");
  });

  it("leaves the pane no ⋯ of its own", async () => {
    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, `.${statusButton.status}`);

    expect(
      container.querySelector(`.${shell.middlePane} .${dropdown.mark}`),
    ).toBeNull();
  });

  /// One line and no second. What was running used to be said under the status
  /// word; it is the running session's own pinned card that says it now, in the
  /// same words, and the button says where the *work* stands and nothing else.
  it("says the status and the state and nothing under them", async () => {
    theGrillingOutput({
      running: true,
      agent_type: "Claude",
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const said = await drawn(container, `.${statusButton.status} .${statusButton.what}`);

    // The status line and nothing after it: no second line, and so no mark for
    // the harness that line used to carry.
    expect([...said.children]).toEqual([
      said.querySelector(`.${statusButton.standing}`),
    ]);
    expect(said.querySelector(`.${harnessMark.mark}`)).toBeNull();
    expect(said.textContent).not.toContain("Claude Code");
  });

  /// The out-of-window stop said its own sentence there, being the one stop a
  /// resume cannot clear on its own. Nothing of it is on the button now — the
  /// sentence is on the resume row of the menu this button drops, see *a run
  /// stopped because an account ran out of window* above.
  it("says nothing about a window the run is waiting on", async () => {
    thePaused();
    const { container } = mount(`/conversations/${WAITING.id}`);

    const said = await drawn(container, `.${statusButton.status} .${statusButton.what}`);

    expect(said.textContent).not.toContain("Out of window");
    expect(said.textContent).not.toContain("3pm");
  });
});

/// What the head of the pane says about a stop, which is one line of the status
/// button now rather than the two badges it used to be.
///
/// A stop that happened without the human is loud: something is waiting on
/// them, and the line is drawn in the accent because they are the only one who
/// can move it. A stop they pressed themselves is quiet — they were there, and
/// Verkstead reading them their own news in the accent would be shouting about
/// nothing.
describe("a conversation that has stopped", () => {
  it("says it is waiting on the human where the stop was not theirs", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Waiting on you");
    expect(line.state).toBe("Implementing");
    expect(line.attention).toBe(true);

    expect(STOPPED.blocked_on).toBe(SAID.id);
    expect(STOPPED.stopped_by_hand).toBe(false);
  });

  it("says stopped, and quietly, where the press was their own", async () => {
    theStopped({ stopped_by_hand: true, waiting: false });
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Stopped");
    expect(line.state).toBe("Implementing");
    expect(line.attention).toBe(false);

    // Which is a colour and not a second word: the accent is spent on the two
    // statuses that need somebody, and every other one is the text it is read
    // in.
    expect(statusButtonCss).toContain(
      ".status .attention .title,\n.status .attention .state {",
    );
  });

  /// Quiet is not the same as inert. There is one notice saying what stopped,
  /// and it is marked where it stands so that a long record still says where
  /// the run got to — the status button says *that* it stopped, and the mark
  /// says where.
  it("marks the notice that says what stopped, where it stands", async () => {
    theStopped({ stopped_by_hand: true, waiting: false });
    const { container } = mount(unopened(STOPPED));

    const marked = await drawn(
      container,
      `.${timeline.timeline} .${timeline.notice}.${timeline.blocking}`,
    );

    expect(marked.textContent).toContain(
      "The task in .tasks/03-commit-events.md",
    );
  });

  it("says nothing about a stop where nothing has stopped", async () => {
    expect(OPEN.blocked_on).toBeNull();

    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Draft");
    expect(line.attention).toBe(false);
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

/// And the pull request the same wrap-up opened in a read-write companion repo:
/// a repository of its own, a number of its own, and its name on the card.
///
/// Composed rather than a fixture of its own, because what is being read here is
/// how the pinned block draws more than one — the card itself is the same card
/// the server's own fixture carries.
const BESIDE_IT: PinnedEvent = {
  PullRequest: {
    id: OPENED.id + 1,
    at: "2026-08-21T08:32:11.000Z",
    number: 7,
    title: "Rate limiting",
    url: "https://github.com/tobico/askance/pull/7",
    repo: "askance",
    checks: null,
    merging: null,
  },
};

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
  checks: [
    { name: "Rust", how: "Passed", link: "https://github.com/tobico/verkstead/actions/runs/1/job/2" },
    // One GitHub gave no run for, which is a name and nothing to follow.
    { name: "buildkite", how: "Running", link: "" },
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

/// The same Conversation with its pull request's checks in a given state — or in
/// none, which is one nothing has asked GitHub about.
///
/// Both copies of the card are drawn from the pinned Event here: what the record
/// row holds is the same reading handed over twice, and the two are the one
/// component.
function whoseChecks(checks: CheckRollup | null): Partial<ConversationView> {
  return {
    pinned: WRAPPING.pinned.map((event) =>
      "PullRequest" in event
        ? { PullRequest: { ...event.PullRequest, checks } }
        : event,
    ),
  };
}

/// And the same Conversation with its pull request merging in a given state — or
/// in none, which is one nothing has asked GitHub about.
///
/// Both copies of the card again, and both drawn from the pinned Event for the
/// same reason: the record row holds the one reading handed over twice.
function whoseMerging(merging: Merging | null): Partial<ConversationView> {
  return {
    pinned: WRAPPING.pinned.map((event) =>
      "PullRequest" in event
        ? { PullRequest: { ...event.PullRequest, merging } }
        : event,
    ),
  };
}

/// The rule each rollup is drawn by, whose names are the words in lower case.
const LOWERCASED = {
  Passed: "passed",
  Running: "running",
  Failed: "failed",
} as const;

/// And the shape each is drawn as, named here rather than read off the
/// component: which Font Awesome icon means *passed* is a decision, and a test
/// that asked `Checks.tsx` which one it had chosen would agree with whatever it
/// answered.
const SHAPED = {
  Passed: faCheck,
  Running: faCircle,
  Failed: faXmark,
} as const;

describe("the pinned pull request", () => {
  it("says what it is called and what number it answers to", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(opened.textContent).toContain("Pull request");
    expect(opened.querySelector(`.${timeline.number}`)!.textContent).toBe(
      `#${OPENED.number}`,
    );
    expect(opened.querySelector(`.${timeline.pullRequestTitle}`)!.textContent).toBe(
      OPENED.title,
    );
  });

  /// One card, one target: nothing on it to press but itself, so there is no
  /// link out here. Merging is still the human's act and it still happens over
  /// there, and the details pane the card opens is what carries the way to it.
  it("carries no link of its own, and the pane it opens carries the way out", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(opened.querySelector("a")).toBeNull();
    expect(opened.textContent).not.toContain("On GitHub");

    fireEvent.click(opened);

    const where = await drawn<HTMLAnchorElement>(
      container,
      `.${shell.detailsPane} .${prPane.where} a`,
    );

    expect(where.href).toBe(OPENED.url);
  });

  /// The restructure took the card's button away, so the card itself is what
  /// answers for the keyboard and for anything reading the page aloud.
  it("stays a button for the keyboard and the screen reader", async () => {
    theWrapping();
    const { container } = mount(unopened(WRAPPING));

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(opened.getAttribute("role")).toBe("button");
    expect(opened.getAttribute("tabindex")).toBe("0");
    expect(opened.getAttribute("aria-pressed")).toBe("false");

    fireEvent.keyDown(opened, { key: "Enter" });

    await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);

    expect(
      container
        .querySelector(`.${timeline.pinned} .${timeline.pullRequest}`)!
        .getAttribute("aria-pressed"),
    ).toBe("true");
  });

  /// GitHub's own three, echoed here: a green tick, a red cross, and an empty
  /// ring for a suite that has not finished. Drawn rather than worded, so the
  /// icon is what carries the words for anything reading the page aloud.
  ///
  /// The shape is asserted as well as the rule that colours it, because the
  /// shape is Font Awesome's data now rather than something this repository
  /// draws: an icon swapped for the wrong one is a mark that says the opposite
  /// of what happened, and no rule anywhere would have changed.
  it("marks how the checks are, in the icon GitHub uses for it", async () => {
    for (const rollup of ["Passed", "Running", "Failed"] as const) {
      theWrapping(whoseChecks(rollup));
      const { container } = mount(`/conversations/${WRAPPING.id}`);

      const mark = await drawn(
        container,
        `.${timeline.pinned} .${timeline.pullRequest} .${checkMarks.checks}`,
      );

      expect(mark.classList.contains(checkMarks[LOWERCASED[rollup]]!)).toBe(true);
      expect(mark.getAttribute("role")).toBe("img");
      expect(mark.getAttribute("aria-label")).toBe(CHECKS_SPOKEN[rollup]);

      const shape = SHAPED[rollup];
      expect(mark.tagName.toLowerCase()).toBe("svg");
      expect(mark.getAttribute("viewBox")).toBe(
        `0 0 ${shape.icon[0]} ${shape.icon[1]}`,
      );
      expect(mark.querySelector("path")!.getAttribute("d")).toBe(shape.icon[4]);
    }
  });

  /// And nothing where nothing is known: a repository with no CI has passed
  /// nothing, and an icon guessing at it would be worse than no icon.
  it("draws no icon for a pull request nothing has asked about", async () => {
    theWrapping(whoseChecks(null));
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(opened.querySelector(`.${checkMarks.checks}`)).toBeNull();
  });

  /// And the mark beside them, which is about something else: whether the branch
  /// merges into its base at all. One shape and one meaning — a conflict — drawn
  /// in the palette's colour for work that has stopped.
  ///
  /// The shape is named here rather than read off the component, for the reason
  /// the three above are: an icon swapped for the wrong one is a mark that says
  /// something else, and no rule anywhere would have changed.
  it("marks a pull request that will not merge, and says so aloud", async () => {
    theWrapping(whoseMerging("Conflicting"));
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const mark = await drawn(
      container,
      `.${timeline.pinned} .${timeline.pullRequest} .${conflictMark.conflict}`,
    );

    expect(mark.getAttribute("role")).toBe("img");
    expect(mark.getAttribute("aria-label")).toBe(CONFLICTED);

    expect(mark.tagName.toLowerCase()).toBe("svg");
    expect(mark.getAttribute("viewBox")).toBe(
      `0 0 ${faCodeMerge.icon[0]} ${faCodeMerge.icon[1]}`,
    );
    expect(mark.querySelector("path")!.getAttribute("d")).toBe(
      faCodeMerge.icon[4],
    );
  });

  /// Beside the rollup rather than instead of it: the two are readings of
  /// different things, and a wrap-up whose suite is still running can conflict
  /// with its base at the same time.
  it("draws the conflict beside how the checks are", async () => {
    theWrapping({ ...whoseChecks("Running"), ...whoseMerging("Conflicting") });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(opened.querySelector(`.${conflictMark.conflict}`)).not.toBeNull();
    expect(opened.querySelector(`.${checkMarks.checks}`)).not.toBeNull();
  });

  /// And nothing for the other two readings, nor for the absence of one. A pull
  /// request that merges is doing what a pull request is supposed to do, and a
  /// GitHub still working the answer out has said nothing at all — so *merges*,
  /// *not worked out yet* and *nobody asked* are one card with no mark on it.
  it("draws no conflict where there is none to draw", async () => {
    for (const merging of ["Cleanly", null] as const) {
      theWrapping(whoseMerging(merging));
      const { container } = mount(`/conversations/${WRAPPING.id}`);

      const opened = await drawn(
        container,
        `.${timeline.pinned} .${timeline.pullRequest}`,
      );

      expect(opened.querySelector(`.${conflictMark.conflict}`)).toBeNull();
    }
  });

  /// And it goes when the conflict does, without the page being reloaded: the
  /// mark is drawn off the reading the Conversation carries, so a fresh one
  /// takes it away. Which is what a Nudge lands as — the server writes the new
  /// word down and tells the page, and the page re-reads.
  it("takes the mark away when a fresh reading says the conflict is gone", async () => {
    let merging: Merging = "Conflicting";

    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/conversations/${WRAPPING.id}`, () =>
        json({ ...WRAPPING, ...whoseMerging(merging) })(),
      ),
      whenever(WHAT_IS_ON_IT, json(CARRIED)),
    );

    const { container, client } = mount(`/conversations/${WRAPPING.id}`);

    await drawn(
      container,
      `.${timeline.pinned} .${timeline.pullRequest} .${conflictMark.conflict}`,
    );

    // The resolution session pushed, GitHub was asked again, and the server
    // Nudged on the strength of a word that changed.
    merging = "Cleanly";
    await nudged(client);

    await waitFor(() => {
      expect(
        container.querySelector(
          `.${timeline.pinned} .${timeline.pullRequest} .${conflictMark.conflict}`,
        ),
      ).toBeNull();
    });
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
    expect(listed.querySelector(`.${timeline.pullRequestTitle}`)!.textContent).toBe(
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
    fireEvent.click(listed);

    await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);

    expect(
      [...container.querySelectorAll(`.${timeline.pullRequest}`)].map((card) =>
        card.classList.contains(pressable.open!),
      ),
    ).toEqual([true, true]);
  });

  it("shows what is on it in the details pane, fetched rather than remembered", async () => {
    const fetching = theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    fireEvent.click(opened);

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

  /// The card above has one icon for a whole suite; this is where the human
  /// finds out which check it was and goes and reads the run. The same three
  /// marks, so the pane and the card are read in one alphabet.
  it("lists every check with its mark and the way to its run", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
    );

    const checks = await drawn(
      container,
      `.${shell.detailsPane} .${prPane.checks}`,
    );

    expect(
      [...checks.querySelectorAll(`.${prPane.ran} li`)].map((row) => [
        row.querySelector(`.${prPane.check}`)!.textContent,
        row.querySelector(`.${checkMarks.checks}`)!.getAttribute("aria-label"),
        row.querySelector(`.${prPane.check}`)!.getAttribute("href"),
      ]),
    ).toEqual([
      ["Rust", CHECKS_SAID.Passed, CARRIED.checks[0]!.link],
      // The one GitHub gave no run for is a name and nothing to follow, so it
      // is drawn as text rather than as a link to nowhere.
      ["buildkite", CHECKS_SAID.Running, null],
    ]);
  });

  /// And a repository with no CI says so quietly, the way the two lists beside
  /// it do: nothing ran, which is nothing to go and look at.
  it("says so quietly when nothing is running against it", async () => {
    theWrapping(
      {},
      whenever(WHAT_IS_ON_IT, json({ ...CARRIED, checks: [] })),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
    );

    const checks = await drawn(
      container,
      `.${shell.detailsPane} .${prPane.checks}`,
    );

    expect(checks.querySelector(`.${notices.empty}`)!.textContent).toBe(
      "Nothing is running against it.",
    );
  });

  /// And the conflict in words, which is what the pane has room for and the
  /// card's mark has not: the same recorded fact, off the same pinned Event.
  ///
  /// Only the conflict. A pull request that merges, one GitHub is still working
  /// the answer out about, and one nothing has asked about are all a pane with
  /// nothing to say about merging — the mark's own rule, drawn in the other
  /// alphabet.
  it("says the conflict in words, and says nothing where there is none", async () => {
    theWrapping(whoseMerging("Conflicting"));
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
    );

    const said = await drawn(
      container,
      `.${shell.detailsPane} .${prPane.conflict}`,
    );

    expect(said.textContent).toBe(IN_WORDS);

    for (const merging of ["Cleanly", null] as const) {
      theWrapping(whoseMerging(merging));
      const { container } = mount(`/conversations/${WRAPPING.id}`);

      fireEvent.click(
        await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
      );

      await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);

      expect(
        container.querySelector(`.${shell.detailsPane} .${prPane.conflict}`),
      ).toBeNull();
    }
  });

  /// And the one press that does something about it, on a conversation
  /// Verkstead has finished with.
  ///
  /// Which is the only state it is drawn in. A wrapping conversation has the
  /// watchers on it already and is resolving the conflict as fast as it can, so
  /// a button there would offer what is happening anyway; and a pull request
  /// that merges has nothing to resolve, which is the same rule the words above
  /// it are drawn by.
  it("offers the press only on a done conversation whose branch conflicts", async () => {
    theWrapping({ ...whoseMerging("Conflicting"), state: "Done" });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
    );

    const press = await drawn(
      container,
      `.${shell.detailsPane} .${prPane.resolve}`,
    );

    expect(press.textContent).toBe("Resolve conflicts");

    for (const over of [
      // The same conflict on a conversation the wrap-up is still running.
      { ...whoseMerging("Conflicting"), state: "Wrapping" as const },
      // And the same finished conversation with nothing to resolve — one that
      // merges, and one nothing has asked GitHub about.
      { ...whoseMerging("Cleanly"), state: "Done" as const },
      { ...whoseMerging(null), state: "Done" as const },
    ]) {
      theWrapping(over);
      const { container } = mount(`/conversations/${WRAPPING.id}`);

      fireEvent.click(
        await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
      );

      await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);

      expect(
        container.querySelector(`.${shell.detailsPane} .${prPane.resolve}`),
      ).toBeNull();
    }
  });

  /// The press posts to the conversation's own route with nothing in the body —
  /// which conversation it is is the whole of what it says, and which of its
  /// pull requests conflict is the server's own reading. And what it leaves
  /// behind is a conversation that has moved into a wrap-up, so the page is
  /// read again.
  it("posts to the conversation's own resolve route and reads it again", async () => {
    const route = `/api/ui/conversations/${WRAPPING.id}/resolve-conflicts`;
    const fetching = theWrapping(
      { ...whoseMerging("Conflicting"), state: "Done" },
      whenever(route, json("Resolving" satisfies Resolved), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
    );

    const before = askedFor(fetching, `/api/ui/conversations/${WRAPPING.id}`);

    fireEvent.click(
      await drawn(container, `.${shell.detailsPane} .${prPane.resolve}`),
    );

    await waitFor(() => expect(sent(fetching, route)).toEqual({}));

    await waitFor(() =>
      expect(
        askedFor(fetching, `/api/ui/conversations/${WRAPPING.id}`),
      ).toBeGreaterThan(before),
    );
  });

  /// And a press that was refused says which refusal it was. Two of them are
  /// the record having moved on under a page somebody left open — the
  /// conversation is not finished with any more, or the branch merges again —
  /// and neither is anything to go and do, which is why the re-read behind the
  /// sentence is as much of the answer. The other two are the checkout the
  /// resolution session would have worked in, which a conversation left finished
  /// with for weeks is the likeliest of any to have lost.
  it("says which refusal a press came back with", async () => {
    for (const outcome of [
      "NotDone",
      "NothingConflicts",
      "NowhereToWork",
      "WorktreeRefused",
    ] satisfies Resolved[]) {
      theWrapping(
        { ...whoseMerging("Conflicting"), state: "Done" },
        whenever(
          `/api/ui/conversations/${WRAPPING.id}/resolve-conflicts`,
          json(outcome),
          "POST",
        ),
      );
      const { container } = mount(`/conversations/${WRAPPING.id}`);

      fireEvent.click(
        await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`),
      );

      fireEvent.click(
        await drawn(container, `.${shell.detailsPane} .${prPane.resolve}`),
      );

      const said = await drawn(
        container,
        `.${shell.detailsPane} .${prPane.refused}`,
      );

      expect(said.textContent).toBe(RESOLVE_REFUSAL[outcome]);
    }
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
    fireEvent.click(opened);

    const error = await drawn(container, `.${shell.detailsPane} .${notices.error}`);

    expect(error.textContent).toContain("is not logged in");
  });

  /// Nothing is fetched until somebody opens it: reading this is an API call
  /// GitHub answers, and the conversation around it is read again on every
  /// Nudge about it.
  it("asks GitHub nothing until it is opened", async () => {
    const fetching = theWrapping();
    const { container } = mount(unopened(WRAPPING));

    await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);

    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBe(0);
  });

  /// A conversation ends on one pull request per repository it was worked in,
  /// so the pinned block draws every one of them rather than the last it finds.
  /// The work's own is unlabelled and a companion's carries its repository, by
  /// the rule a commit's label follows.
  it("draws every pull request, naming the ones in a companion repo", async () => {
    theWrapping({ pinned: [...WRAPPING.pinned, BESIDE_IT] });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const dots = await drawn(
      container,
      `.${timeline.pinned} .${timeline.carousel} > .${timeline.dots}`,
    );

    expect(
      [...dots.querySelectorAll("button")].map((dot) =>
        dot.getAttribute("aria-label"),
      ),
    ).toEqual(["Pull request", "Pull request in askance"]);

    const own = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    expect(own.querySelector(`.${timeline.repo}`)).toBeNull();

    fireEvent.click([...dots.querySelectorAll("button")][1]!);

    const companions = await drawn(
      container,
      `.${timeline.pinned} .${timeline.pullRequest} .${timeline.repo}`,
    );
    expect(companions.textContent).toBe("askance");
  });

  /// And opening a companion's asks the server about that pull request's own
  /// event, which is what says which repository to ask GitHub in.
  it("opens a companion's pull request against its own event", async () => {
    const fetching = theWrapping(
      { pinned: [...WRAPPING.pinned, BESIDE_IT] },
      whenever(
        `/api/ui/conversations/${WRAPPING.id}/pull-request/${BESIDE_IT.PullRequest.id}`,
        json(CARRIED),
      ),
    );
    const { container } = mount(unopened(WRAPPING));

    const dots = await drawn(
      container,
      `.${timeline.pinned} .${timeline.carousel} > .${timeline.dots}`,
    );
    fireEvent.click([...dots.querySelectorAll("button")][1]!);

    const companions = await drawn(
      container,
      `.${timeline.pinned} .${timeline.pullRequest} .${timeline.repo}`,
    );
    // The card itself, which is the whole of the press.
    fireEvent.click(companions.closest(`.${timeline.pullRequest}`)!);

    await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);

    expect(
      askedFor(
        fetching,
        `/api/ui/conversations/${WRAPPING.id}/pull-request/${BESIDE_IT.PullRequest.id}`,
      ),
    ).toBe(1);
    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBe(0);
  });
  });

/// A conversation with all three kinds pinned at once: the pull request it ended
/// on, the backlog it was built from, and the roadmap the branch wrote to.
///
/// In that order, which is the order the server hands them over in — see the
/// pinned block in `crates/server/src/ui.rs`.
///
/// Composed rather than a fixture of its own, because what is being read here is
/// how the timeline draws several pinned cards, and each of the three is already
/// a golden fixture the server wrote.
const ALL_THREE = [
  { PullRequest: OPENED },
  { TaskList: BACKLOG },
  { StageList: ROADMAP },
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
  /// The session running now leads the deck: it is what the Conversation is
  /// doing, where everything else pinned is something the work is against.
  it("leads with the running session where there is one", async () => {
    theWrapping({
      pinned: [{ AgentOutput: { ...OUTPUT, running: true } }, ...ALL_THREE],
    });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const dots = await drawn(container, `.${timeline.pinned} .${timeline.carousel} > .${timeline.dots}`);

    expect(
      [...dots.querySelectorAll("button")].map((dot) =>
        dot.getAttribute("aria-label"),
      ),
    ).toEqual(["Agent run", "Pull request", "Task list", "Roadmap"]);
  });

  /// And it is the same card the record holds, drawn a second time rather than
  /// moved: pressing either opens the one details pane, and both read as open
  /// while it is — the arrangement the pull request beside it already has.
  it("draws the running session's own card, opening the one pane", async () => {
    theGrillingRunning({
      agent_type: "Claude",
      profile: "Work",
      model: "claude-fable-5",
    });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, `.${timeline.pinned} .${timeline.agentOutput}`);

    expect(card.querySelector(`.${timeline.what}`)!.textContent).toBe(
      "Agent run",
    );
    await waitFor(() =>
      expect(card.querySelector(`.${timeline.ranAs}`)!.textContent).toBe(
        "Claude Code Fable 5 — Work",
      ),
    );

    fireEvent.click(card);

    // Both copies read as the open one, there being one session behind them.
    await waitFor(() =>
      expect(
        [...container.querySelectorAll(`.${timeline.agentOutput}`)].every(
          (drawn) => drawn.getAttribute("aria-pressed") === "true",
        ),
      ).toBe(true),
    );

    // And the pane behind it is that session's, titled by the same reading.
    const head = await drawn(container, `.${shell.detailsPane} .${paneHead.head} h1`);
    await waitFor(() =>
      expect(head.textContent).toBe("Claude Code Fable 5 — Work"),
    );
  });

  /// Nothing running is nothing pinned for it, which is what keeps a finished
  /// Conversation from carrying the last run it ever made for good.
  it("pins no session where nothing is writing into one", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timelineEvent} .${timeline.agentOutput}`);

    expect(
      container.querySelector(`.${timeline.pinned} .${timeline.agentOutput}`),
    ).toBeNull();
  });

  /// Which is why the deck holds its place by which card it is rather than by
  /// where it sits. The session leads the list and comes and goes with the run,
  /// so a reader who turned to the backlog would otherwise find the pull request
  /// under them the moment a session started, having pressed nothing — and the
  /// backlog again the moment it ended.
  it("keeps the reader on the card they turned to as a session comes and goes", async () => {
    let standing: ConversationView = { ...WRAPPING, pinned: ALL_THREE };

    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/conversations/${WRAPPING.id}`, () => json(standing)()),
      whenever(WHAT_IS_ON_IT, json(CARRIED)),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const dots = await drawn(
      container,
      `.${timeline.pinned} .${timeline.carousel} > .${timeline.dots}`,
    );
    const named = () =>
      dots
        .querySelector("button[aria-current='true']")!
        .getAttribute("aria-label");

    fireEvent.click(
      [...dots.querySelectorAll("button")].find(
        (dot) => dot.getAttribute("aria-label") === "Task list",
      )!,
    );
    await waitFor(() => expect(named()).toBe("Task list"));

    // A session starts, and goes in at the head of the list.
    standing = {
      ...WRAPPING,
      pinned: [{ AgentOutput: { ...OUTPUT, running: true } }, ...ALL_THREE],
    };
    readAgain();
    await waitFor(() => expect(dots.querySelectorAll("button").length).toBe(4));
    expect(named()).toBe("Task list");

    // And ends, taking it out from under the rest again.
    standing = { ...WRAPPING, pinned: ALL_THREE };
    readAgain();
    await waitFor(() => expect(dots.querySelectorAll("button").length).toBe(3));
    expect(named()).toBe("Task list");
  });

  it("shows one of several pinned cards at a time", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(
      pinned.querySelectorAll(`.${timeline.taskList}, .${timeline.stageList}, .${timeline.pullRequest}`),
    ).toHaveLength(1);
    expect(pinned.querySelector(`.${timeline.pullRequest}`)).not.toBeNull();
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
      "Pull request",
      "Task list",
      "Roadmap",
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
      expect(container.querySelector(`.${timeline.pinned} .${timeline.stageList}`)).not.toBeNull(),
    );
    // The card it turned off is held in the deck while the slide runs, and gone
    // once it has.
    await waitFor(() =>
      expect(container.querySelector(`.${timeline.pinned} .${timeline.pullRequest}`)).toBeNull(),
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
      expect(carousel.querySelector(`.${timeline.taskList}`)).not.toBeNull(),
    );

    // Back past the front, which is the far end of the list.
    fireEvent.click(carousel.querySelector(`.${timeline.step}.${timeline.back}`)!);
    fireEvent.click(carousel.querySelector(`.${timeline.step}.${timeline.back}`)!);
    await waitFor(() =>
      expect(carousel.querySelector(`.${timeline.stageList}`)).not.toBeNull(),
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

  /// Each arrow straddles the edge of the card it turns the deck across: half
  /// its own width outside, which puts its centre on the edge line. The room is
  /// the pane's own padding, and no card is padded for them — the cards are
  /// borderless at rest, so an arrow lying inside one had nothing to lie
  /// against and the inset it was given read as a card indented for nothing.
  it("straddles each card's edge with its arrow, and pads no card for them", () => {
    const [, hovering] = timelineCss.split("@media (hover: hover) {");

    // Half of the 1.75rem circle above, which is what centres it on the edge.
    expect(hovering).toContain("    width: 1.75rem;");
    expect(hovering).toContain("  .deck > .back {\n    left: -0.875rem;\n  }");
    expect(hovering).toContain("  .deck > .on {\n    right: -0.875rem;\n  }");

    expect(timelineCss).not.toContain("padding-inline");
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
    expect(leaving?.querySelector(`.${timeline.pullRequest}`)).not.toBeNull();

    const arriving = deck.querySelector(`.${timeline.arriving}`);
    expect(arriving?.className).toContain(timeline.onward);
    expect(arriving?.querySelector(`.${timeline.taskList}`)).not.toBeNull();

    // And back the other way, which the pair say too.
    await waitFor(() => expect(deck.querySelector(`.${timeline.leaving}`)).toBeNull());
    fireEvent.click(deck.querySelector(`.${timeline.step}.${timeline.back}`)!);

    expect(deck.querySelector(`.${timeline.leaving}`)?.className).toContain(
      timeline.backward,
    );

    await waitFor(() => expect(deck.querySelector(`.${timeline.leaving}`)).toBeNull());
    expect(deck.querySelector(`.${timeline.arriving}`)).toBeNull();
    expect(deck.querySelector(`.${timeline.pullRequest}`)).not.toBeNull();
  });

  it("turns the card on a swipe across it", async () => {
    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const deck = await drawn(container, `.${timeline.pinned} .${timeline.carousel} > .${timeline.deck}`);

    // Leftwards is onwards, the way a page turns.
    swipe(deck, 200, 200 - SWIPE);
    await waitFor(() =>
      expect(deck.querySelector(`.${timeline.taskList}`)).not.toBeNull(),
    );

    swipe(deck, 200, 200 + SWIPE);
    await waitFor(() =>
      expect(deck.querySelector(`.${timeline.pullRequest}`)).not.toBeNull(),
    );

    // A press that slid a little is still a press, and turns nothing.
    await waitFor(() => expect(deck.querySelector(`.${timeline.leaving}`)).toBeNull());
    swipe(deck, 200, 200 - (SWIPE - 1));
    expect(deck.querySelector(`.${timeline.taskList}`)).toBeNull();
  });

  /// Which card the reader is put in front of: the one the work has stopped on,
  /// which is what they opened the conversation to deal with. Only a pull
  /// request can be one, so it only picks a card out where there is more than
  /// one of those — a companion's over the work's own here.
  it("fronts the card the work is blocked on", async () => {
    theWrapping({
      pinned: [...ALL_THREE, BESIDE_IT],
      blocked_on: BESIDE_IT.PullRequest.id,
    });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const repo = await drawn(
      container,
      `.${timeline.pinned} .${timeline.pullRequest} .${timeline.repo}`,
    );

    expect(repo.textContent).toBe("askance");
  });

  /// And with nothing stopping it, the fixed order — which is the order the
  /// server hands them over in, the pull request leading it as the one of the
  /// three with anything on it to answer.
  it("otherwise fronts the first, which is the pull request", async () => {
    expect(WRAPPING.blocked_on).toBeNull();

    theWrapping({ pinned: ALL_THREE });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(pinned.querySelector(`.${timeline.pullRequest}`)).not.toBeNull();
  });

  /// And with no pull request to be first, the task list — the order is the
  /// server's, and what is behind the pull request is the order the work goes
  /// through the two lists in.
  it("fronts the task list where there is no pull request before it", async () => {
    theWrapping({ pinned: ALL_THREE.slice(1) });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const pinned = await drawn(container, `.${timeline.pinned}`);

    expect(pinned.querySelector(`.${timeline.taskList}`)).not.toBeNull();
    expect(pinned.querySelector(`.${timeline.stageList}`)).toBeNull();
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
    await drawn(container, `.${timeline.pinned} .${timeline.stageList}`);
    fireEvent.click(dots.querySelectorAll("button")[0]!);

    const opened = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    fireEvent.click(opened);

    await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);
    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBeGreaterThan(0);
  });
});

/// A wrap-up narrowed to its checks: the review answered, nothing said on the
/// pull request left unaddressed, and nothing running in the worktree.
///
/// A condition of wrapping rather than a state, so the server hands it over as
/// a flag beside the lifecycle and the page draws it beside the branch. It is a
/// label and not a control: the checks are GitHub's to finish, so there is
/// nothing to press and nowhere to go.
describe("a wrap-up waiting on its checks", () => {
  /// Nothing to resume alongside it: a wrap-up down to its checks is not a run
  /// that has stopped, and the status the button draws for one is what stands
  /// above this condition in the order.
  it("says so where the conversation is named", async () => {
    theWrapping({ waiting_on_checks: true, ready_to_resume: false });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Waiting on checks");
    expect(line.state).toBe("Wrapping");

    // A condition to read rather than one to do something about, so it is not
    // in the accent: the checks are GitHub's to finish.
    expect(line.attention).toBe(false);
  });

  it("says nothing where the wrap-up is still waiting on more than that", async () => {
    expect(WRAPPING.waiting_on_checks).toBe(false);

    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    expect((await standing(container)).word).not.toBe("Waiting on checks");
  });

  /// The sidebar draws no state in words at all — see the card's own marks — so
  /// where the condition is said there is the label the row is read aloud by,
  /// in place of the lifecycle word rather than beside it.
  it("is what the sidebar row says in place of its state", async () => {
    theSidebar(
      { state: "Wrapping", working: false, waiting: false },
      { state: "Wrapping", working: false, waiting: false, waiting_on_checks: true },
    );
    const { container } = mount();

    const [plain, narrowed] = await cards(container);

    expect(plain!.querySelector("button")!.getAttribute("aria-label")).toContain(
      "Wrapping",
    );
    expect(
      narrowed!.querySelector("button")!.getAttribute("aria-label"),
    ).toContain("Waiting on checks");
    expect(
      narrowed!.querySelector("button")!.getAttribute("aria-label"),
    ).not.toContain("Wrapping");
  });
});

/// And the other end of the ladder, where the word is the state itself rather
/// than a condition of one.
///
/// Done and Closed are where a conversation stops, and neither is somewhere a
/// status applies: nothing is supposed to be driving one, and the word for
/// where it got to *is* the state. So the line collapses to that word on its
/// own — a status and a state saying the same thing twice, with a colour
/// between them, would be the button reading itself out.
describe("a conversation that has ended", () => {
  it("says Done where the work reached the end of the ladder", async () => {
    theRecordedWith({ state: "Done" });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Done");
    expect(line.state).toBeNull();
    expect(line.attention).toBe(false);
  });

  it("says Closed where the work stopped wherever it was", async () => {
    theRecordedWith({ state: "Closed", ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    expect((await standing(container)).word).toBe("Closed");
  });

  /// And the one thing a Cleanup leaves on the page: the word for what it took,
  /// beside the word for where the work got to.
  ///
  /// A word rather than a banner, and only ever about a trim that has already
  /// happened — nothing anywhere says one is coming.
  it("names a trimmed conversation Trimmed beside its state", async () => {
    theRecordedWith({
      state: "Closed",
      ready_to_grill: false,
      archived: true,
      trimmed: true,
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Closed");
    expect(line.trimmed).toBe("Trimmed");
  });

  /// And says nothing of the kind on one no sweep has reached, which is nearly
  /// every conversation there is: the line is exactly what it was.
  it("says nothing about trimming on one no cleanup has been through", async () => {
    theRecordedWith({
      state: "Closed",
      ready_to_grill: false,
      archived: true,
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Closed");
    expect(line.trimmed).toBeNull();
  });

  /// And a Draft is the third of them, for the other half of the same reason:
  /// nothing is supposed to be driving one either, so there is no status to
  /// say beside the word.
  it("says Draft on one nothing has started", async () => {
    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Draft");
    expect(line.state).toBeNull();
  });

  /// The states on the way up are where a status is worth saying, and they say
  /// it beside the state rather than instead of it.
  it("says the state beside a status while the work is still going", async () => {
    for (const state of ["Grilling", "Implementing", "Wrapping", "FollowUp"] as const) {
      theRecordedWith({ state, ready_to_resume: true });
      const { container, unmount } = mount(`/conversations/${OPEN.id}`);

      const line = await standing(container);

      expect(line.word).toBe("Stopped");
      expect(line.state).toBe(STATE[state]);

      unmount();
    }
  });

  /// An ended conversation may still be holding a stop, and the word for where
  /// it got to is what the line says about it: there is nothing left for
  /// anybody to resume, so nothing is waiting on them.
  it("says where the work got to over anything it stopped on", async () => {
    theStopped({ state: "Done" });
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const line = await standing(container);

    expect(line.word).toBe("Done");
    expect(line.state).toBeNull();
    expect(line.attention).toBe(false);
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

/// The card a refused press opens over the page, or nothing where nothing has
/// been refused. Found on the body rather than in the container, a `dialog`
/// being drawn in the top layer.
function refusal(): HTMLElement | null {
  return document.body.querySelector<HTMLElement>(`.${actions.refused}`);
}

/// Getting Verkstead driving again, which is a row of the conversation's
/// actions menu like every other: above the stops, because it is the one *go*
/// among them.
///
/// It was a block at the foot of the timeline, with its own heading, its own
/// note and its own refusal lines. What put it in the menu is what put every
/// other control there — the status button says nothing is driving this, and
/// what there is to do about that is behind its press — and the sidebar's
/// right-click gets it for nothing, both menus being the one set of rows.
describe("the resume row", () => {
  /// Drawn on the server's word alone. What drives a conversation is a register
  /// of running tasks, which lives in the server — a page working it out from
  /// the state and the session it can see would be a second opinion about a
  /// question only one side can answer.
  it("is the first row where nothing is driving the conversation", async () => {
    theWrapping({ ready_to_resume: true });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const menu = await openActions(container);
    const resume = await drawn(menu, `.${actions.resume}`);

    expect(resume.querySelector(`.${actions.title}`)!.textContent).toBe("Resume");
    // First, over the stops: everything under it ends the work or moves it.
    expect([...menu.querySelectorAll("button")][0]).toBe(resume);
  });

  /// And gone where something is. There is nothing to start again, and a row
  /// offering to would be one that could only refuse.
  it("goes where something is driving it already", async () => {
    theWrapping({ ready_to_resume: false });
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const menu = await openActions(container);
    await drawn(menu, `.${actions.steer}`);

    expect(menu.querySelector(`.${actions.resume}`)).toBeNull();
  });

  /// Nothing goes with the press. What should be running is recomputed from
  /// where the work now stands, which is the whole reason there is one row
  /// rather than a choice of them.
  it("sends the press with nothing on it", async () => {
    const fetching = theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json("Resumed" as Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.resume}`));

    await waitFor(() => expect(sent(fetching, RESUMING)).toEqual({}));
    // Nothing was refused, so nothing is drawn over the page and the menu goes.
    await waitFor(() => expect(refusal()).toBeNull());
  });

  /// A press that found nothing to start says so over the page. This is the
  /// whole of what resume is for: a conversation nothing is driving, and the
  /// reason nothing is.
  it("says in words that there was nothing to start", async () => {
    theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json("NothingToWork" as Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.resume}`));

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toBe(RESUME_REFUSAL.NothingToWork);
    expect(said.textContent).toContain("no backlog left");
  });

  /// And a second press on a conversation the first one got going is refused as
  /// driven, which is the same press arriving twice rather than a mistake.
  it("says in words that something is driving it now", async () => {
    theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json("AlreadyDriven" as Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.resume}`));

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toBe(RESUME_REFUSAL.AlreadyDriven);
  });
});

/// What a press that did nothing comes to, whichever row was pressed: the
/// refusal's own sentence, drawn over the page.
///
/// This menu used to answer a refusal with a `console.error` and leave the rows
/// where they were, on the grounds that every refusal it had was a page drawn
/// against a conversation that had moved and the re-read behind the press was
/// the correction. Resume is what changed that: its refusals are the whole of
/// what the row is for, and there was never a way to tell those sentences from
/// the rest.
describe("what a refused press says", () => {
  /// A row that is not resume, to say that the card is the menu's answer rather
  /// than the resume row's: a close on a conversation the server says is gone.
  it("opens the refusal's sentence over the page, whichever row was pressed", async () => {
    theWrapping(
      {},
      whenever(
        `/api/ui/conversations/${WRAPPING.id}/close`,
        json("NoSuchConversation" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.close}`));

    const card = await drawn(document.body, `.${actions.refused}`);

    expect(card.querySelector(`.${actions.refusedWhy}`)!.textContent).toBe(
      CLOSE_REFUSAL.NoSuchConversation,
    );
    // A heading that says what the card is, and one way out of it: nothing is
    // being decided here, the press having already been refused.
    expect(card.querySelector(`.${actions.refusedTitle}`)!.textContent).toBe(
      "Nothing happened",
    );
    expect(card.querySelectorAll(`.${actions.refusedOut} button`)).toHaveLength(1);
  });

  /// The menu goes on the way: a dropdown left hanging behind a card drawn over
  /// the page is a menu nobody can see to close.
  it("takes the menu back as the card comes up", async () => {
    theWrapping(
      {},
      whenever(
        `/api/ui/conversations/${WRAPPING.id}/close`,
        json("NoSuchConversation" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.close}`));

    await drawn(document.body, `.${actions.refused}`);
    expect(
      container.querySelector(`.${actions.conversationActions} > .${dropdown.drop}`),
    ).toBeNull();
  });

  /// And takes the focus back to the status button with it. The row that was
  /// pressed goes when the menu does, so a shut that left the focus where it
  /// was would put it on the document body — and a card drawn over the page
  /// hands the focus back to whatever had it when it opened, which would send
  /// somebody working by keyboard to the top of the document once they had read
  /// the sentence.
  it("leaves the focus on the button the press came from", async () => {
    theWrapping(
      {},
      whenever(
        `/api/ui/conversations/${WRAPPING.id}/close`,
        json("NoSuchConversation" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.close}`));

    await drawn(document.body, `.${actions.refused}`);
    expect(document.activeElement).toBe(
      container.querySelector(`.${statusButton.status} > .${dropdown.trigger}`),
    );
  });

  /// And the one way out takes it back, leaving the page as it was.
  it("goes when the one way out is pressed", async () => {
    theWrapping(
      {},
      whenever(
        `/api/ui/conversations/${WRAPPING.id}/close`,
        json("NoSuchConversation" satisfies ConversationClosed),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.close}`));

    const card = await drawn(document.body, `.${actions.refused}`);
    fireEvent.click(card.querySelector(`.${actions.refusedOut} button`)!);

    await waitFor(() => expect(refusal()).toBeNull());
  });

  /// A request that never came back is said the same way. Nothing about it is a
  /// refusal the server named, so the sentence is the page's own — but a press
  /// that did nothing is a press that did nothing, and the human is owed the
  /// same answer either way.
  it("says so when the request itself fell over", async () => {
    theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json({ error: "the server is not answering" }, 503), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.resume}`));

    const said = await drawn(document.body, `.${actions.refused} .${actions.refusedWhy}`);

    expect(said.textContent).toContain("could not be resumed");
    expect(said.textContent).toContain("the server is not answering");
  });

  /// And a press that landed says nothing at all. There is no card, because
  /// there is nothing that did not happen.
  it("says nothing where the press landed", async () => {
    theWrapping(
      { ready_to_resume: true },
      whenever(RESUMING, json("Resumed" as Resumed), "POST"),
    );
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    fireEvent.click(await drawn(await openActions(container), `.${actions.resume}`));

    await waitFor(() =>
      expect(
        container.querySelector(`.${actions.conversationActions} > .${dropdown.drop}`),
      ).toBeNull(),
    );
    expect(refusal()).toBeNull();
  });
});

/// The three documents on a timeline: the frozen brief, the handoff the grilling
/// wrote, and the instruction a steer sent a session off with.
///
/// Each of them is as long as whoever wrote it made it, so the card shows the
/// first three lines and ends in an ellipsis, and the whole of it is a press
/// away. Where the third line falls is a fact about a laid-out box and jsdom has
/// no layout, so the clamp itself is asserted off the stylesheet — the way a
/// drawn diagram's rules are — and what is asked here is that each document
/// wears it and that pressing the card opens the whole.
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

    expect(brief.querySelector(`.${timeline.clamp}.${timeline.briefBody}`)).toBeTruthy();

    fireEvent.click(brief);

    // Its own pane rather than the one the handoff and the instruction share,
    // because it carries the configuration summary under the markdown.
    const opened = await drawn(details(), `.${briefPane.brief}`);

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

    expect(handoff.querySelector(`.${timeline.clamp}.${timeline.handoffBody}`)).toBeTruthy();

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
    expect(handoff.classList.contains(pressable.pressable!)).toBe(true);

    fireEvent.keyDown(handoff, { key: "Enter" });

    await drawn(details(), `.${documentPane.document}`);

    await waitFor(() =>
      expect(handoff.classList.contains(pressable.open!)).toBe(true),
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

  /// The brief while it is still a draft is the same card as the frozen one: no
  /// field on the record, the rendering in the same clamp, and the press that
  /// opens it. What it opens is the composer rather than the read-only pane —
  /// the whole of the document, and under it the setup and the start.
  it("clamps the drafting brief too, and opens the composer", async () => {
    theRecordedWith();
    const { container } = mount(`/conversations/${OPEN.id}`);

    const brief = await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`);

    expect(brief.querySelector(`.${timeline.clamp}.${timeline.briefBody}`)).toBeTruthy();
    expect(brief.querySelector("textarea")).toBeNull();
    expect(brief.getAttribute("role")).toBe("button");
    expect(brief.classList.contains(pressable.pressable!)).toBe(true);

    fireEvent.click(brief);

    const opened = await drawn(details(), `.${composer.composer}`);

    // Titled by the branch the work will be done on, which is what a draft is
    // called everywhere else it is named — the frozen Brief's own pane above is
    // the one still called after the document.
    expect(details().querySelector("h1")!.textContent).toBe(titled(OPEN));
    // The markdown in the field rather than the rendering: what the pane opens
    // on is the document as it is written.
    expect(
      (opened.querySelector("textarea") as HTMLTextAreaElement).value,
    ).toBe(BRIEF.markdown);
    expect(details().querySelector(`.${timeline.clamp}`)).toBeNull();
  });

  /// A notice is a sentence rather than a document, so its card is cut at one
  /// line where a document's is cut at three. The same kind of cut either way,
  /// which it was not: a document was clamped to a height under a gradient, and
  /// a fade at the foot of the sticky block read as the block itself trailing
  /// off. The whole of it is still a press away — see the notice's own pane
  /// below.
  it("cuts a notice off at a line where a document is cut at three", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const notice = await drawn(container, `.${timeline.timelineEvent} > .${timeline.notice}`);

    expect(notice.querySelector(`.${timeline.clamp}`)).toBeNull();
    expect(notice.querySelector(`.${timeline.noticeBody}`)).toBeTruthy();

    // How far each of them is cut is the stylesheet's, and jsdom lays nothing
    // out.
    expect(timelineCss).toContain("-webkit-line-clamp: 1;");
    expect(timelineCss).toContain("line-clamp: 1;");
    expect(timelineCss).toContain("-webkit-line-clamp: 3;");
    expect(timelineCss).toContain("line-clamp: 3;");
  });
});

/// What the conversation was configured with, under the brief that started it.
///
/// The setup card goes when the brief freezes, so this is the only place the
/// rest of a conversation's life says what it was set up with — a read-write
/// companion surfaces later through its commits and its pull request, and a
/// read-only one never does. The worktree directories and the two pairings are
/// as unfindable, which is why they are here too.
describe("the configuration on the brief's pane", () => {
  /// The details pane, and the summary it has drawn.
  const details = () => screen.getByLabelText("Details");
  const summary = () => details().querySelector(`.${briefPane.configuration}`);

  /// Every term the summary lays out against what it says, which is how the
  /// pane is read: down the terms, past the ones it was not opened for.
  function facts(list: Element): Record<string, string> {
    return Object.fromEntries(
      [...list.querySelectorAll(`.${briefPane.fact}`)].map((fact) => [
        fact.querySelector("dt")!.textContent,
        fact.querySelector("dd")!.textContent,
      ]),
    );
  }

  /// The conversation's own facts, which are the first list on the pane — a
  /// companion's are inside the block that names it.
  function configuration(): Record<string, string> {
    return facts(summary()!.querySelector(`.${briefPane.facts}`)!);
  }

  /// Every companion block, in the order the conversation carries them.
  function companions(): Record<string, string>[] {
    return [...summary()!.querySelectorAll(`.${briefPane.companion}`)].map(
      (block) => ({
        Repo: block.querySelector(`.${briefPane.companionName}`)!.textContent!,
        ...facts(block.querySelector(`.${briefPane.facts}`)!),
      }),
    );
  }

  /// The brief opened, whichever conversation is being served.
  async function openBrief(conversation: ConversationView): Promise<void> {
    const { container } = mount(`/conversations/${conversation.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.timelineEvent} > .${timeline.brief}`),
    );

    await drawn(details(), `.${briefPane.configuration}`);
  }

  /// A read-only companion left on the rule, and a read-write one left
  /// mirroring: the two shapes a companion row can freeze in, which no fixture
  /// holds — a golden file is one payload, and these are a conversation
  /// configured two ways at once.
  const READING: CompanionView = {
    repo: {
      id: 2,
      name: "askance",
      path: "/srv/repos/askance",
      default_branch: "trunk",
    },
    mode: "ReadOnly",
    base_ref: null,
    branch: "",
    worktree: {
      path: "/var/lib/verkstead/worktrees/askance-trunk",
      missing: false,
    },
    base_commit: "8b1c3d5e76f32b11a0c4d1e8f5b3a97c2d0e4f6a",
  };

  const WRITING: CompanionView = {
    repo: {
      id: 3,
      name: "tobico-skills",
      path: "/srv/repos/tobico-skills",
      default_branch: "main",
    },
    mode: "ReadWrite",
    base_ref: "main",
    branch: "",
    worktree: {
      path: `/var/lib/verkstead/worktrees/tobico-skills-${GRILLING.branch}`,
      missing: false,
    },
    base_commit: "0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e76f32b11a",
  };

  it("says the repo, the branch, the base commit, the worktree and the pairings", async () => {
    theGrilling();
    await openBrief(GRILLING);

    // Under the brief itself rather than over it: the document is what the pane
    // is titled for, and this is what follows it.
    const panes = [...details().children];
    const markdown = panes.findIndex((pane) =>
      pane.classList.contains(briefPane.brief!),
    );
    expect(markdown).toBeGreaterThan(-1);
    expect(
      panes.findIndex((pane) =>
        pane.classList.contains(briefPane.configuration!),
      ),
    ).toBeGreaterThan(markdown);

    expect(configuration()).toEqual({
      Repo: GRILLING.repo.name,
      Branch: GRILLING.branch,
      // The commit, abbreviated the way every other commit on the page is.
      Base: GRILLING.base_commit!.slice(0, ABBREVIATED),
      Worktree: GRILLING.worktree!.path,
      // The shared reading, which is what every picker of a pairing says too:
      // the backend, the model, and the account's name after an em dash — said
      // here because all three of the fixture's accounts are Claude Code ones.
      Grilling: "Claude Code Fable 5 — fable",
      Implementation: "Claude Code Opus 5 — opus",
      Review: "Claude Code Sonnet 5 — sonnet",
    });
  });

  /// A conversation nobody is to review says so, rather than reading as one
  /// whose review pairing was never picked: what the human picked is a choice,
  /// and the pane says what it was.
  it("says no review where that is what was picked", async () => {
    theGrillingStanding({ review_pairing: "Skipped" });
    await openBrief(GRILLING);

    expect(configuration().Review).toBe("No review.");
    expect(
      configuration().Implementation,
      "and the roles beside it read as they always did",
    ).toBe("Claude Code Opus 5 — opus");
  });

  /// And the same one role along: a conversation whose brief went straight to
  /// the work says so, rather than reading as one whose grilling pairing was
  /// never picked.
  it("says no grilling where that is what was picked", async () => {
    theGrillingStanding({ grilling_pairing: "Skipped" });
    await openBrief(GRILLING);

    expect(configuration().Grilling).toBe("No grilling.");
    expect(
      configuration().Review,
      "and the roles beside it read as they always did",
    ).toBe("Claude Code Sonnet 5 — sonnet");
  });

  /// A profile chosen before models were paired beside them is half a choice,
  /// and the pane says the half there is: the backend and the account, with no
  /// model invented for it. The name is said whatever the list holds — with no
  /// model there is nothing else to tell one account from another.
  it("says the half there is where a pairing named no model", async () => {
    theGrillingStanding({
      implementation_pairing: {
        ...GRILLING.implementation_pairing!,
        model: null,
      },
    });
    await openBrief(GRILLING);

    expect(configuration().Implementation).toBe("Claude Code — opus");
  });

  /// And the account's name is dropped where its backend has one account saved:
  /// there is nothing for the name to tell apart, and the model is the whole of
  /// what ran the work.
  ///
  /// The grilling's account is not one of them any more — this is the list as it
  /// stands, and a Pairing carries the Profile it was settled with — so its name
  /// stays: dropping it would read as the account that happens to be left.
  it("says only the backend and the model where a backend has one account", async () => {
    theGrillingStanding({}, whenever("/api/ui/profiles", json([PROFILES[1]])));
    await openBrief(GRILLING);

    // Awaited, because the name is what the pane says until the list has been
    // read: saying it can never misattribute a run, and dropping it can.
    await waitFor(() =>
      expect(configuration().Implementation).toBe("Claude Code Opus 5"),
    );
    expect(configuration().Grilling).toBe("Claude Code Fable 5 — fable");
  });

  it("lists each companion with its mode, its branch and its directory", async () => {
    theGrillingStanding({ companions: [READING, WRITING] });
    await openBrief(GRILLING);

    expect(companions()).toEqual([
      {
        Repo: "askance",
        Access: "Read-only",
        // No branch, because a read-only companion is checked out detached —
        // and the commit it is detached at rather than the branch that was
        // picked, which is the same honesty the base row above is written for:
        // a name that has moved since would say the checkout is somewhere it
        // is not. Abbreviated the way every other commit on the page is.
        "Detached at": READING.base_commit!.slice(0, ABBREVIATED),
        Worktree: READING.worktree!.path,
      },
      {
        Repo: "tobico-skills",
        Access: "Read-write",
        // Left mirroring, which is the conversation's own branch name — the
        // record holds nothing, and this is what nothing resolves to.
        Branch: GRILLING.branch,
        Worktree: WRITING.worktree!.path,
      },
    ]);
  });

  /// A checkout made before Verkstead kept the commit has only the name to fall
  /// back on — the base that was picked, or that repo's own default branch
  /// where nothing was.
  it("falls back to the base's name where no commit was recorded", async () => {
    theGrillingStanding({
      companions: [{ ...READING, base_commit: null }],
    });
    await openBrief(GRILLING);

    expect(companions()[0]!["Detached at"]).toBe(READING.repo.default_branch);
  });

  it("summarises a conversation with no companions all the same", async () => {
    theGrilling();
    await openBrief(GRILLING);

    expect(GRILLING.companions).toHaveLength(0);
    expect(Object.keys(configuration())).toContain("Worktree");
    // No heading over an empty list: a section saying a conversation has no
    // companions would read as something having gone missing.
    expect(summary()!.querySelector(`.${briefPane.companions}`)).toBeNull();
  });

  /// A directory somebody deleted by hand is a conversation with a problem,
  /// said where the directory is said rather than left for whatever next tries
  /// to work in it to fall over on.
  it("says which checkout is gone from disk", async () => {
    theGrillingStanding({
      worktree: { ...GRILLING.worktree!, missing: true },
    });
    await openBrief(GRILLING);

    expect(configuration().Worktree).toContain(GRILLING.worktree!.path);
    expect(summary()!.querySelector(`.${briefPane.gone}`)!.textContent).toBe(
      "gone from disk",
    );
  });

  /// The pane reports the configuration; the setup card is still the only place
  /// any of it is changed. So there is nothing on it to press, and a
  /// conversation past drafting has nothing here it could press anyway.
  it("changes nothing", async () => {
    theGrillingStanding({ companions: [READING, WRITING] });
    await openBrief(GRILLING);

    expect(
      summary()!.querySelectorAll("button, input, select, textarea, a"),
    ).toHaveLength(0);
  });

  /// The other two documents are unchanged by it: they are read in the plain
  /// pane they have always been read in, and neither has a configuration to
  /// carry.
  it("leaves the handoff and the instruction panes alone", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.timelineEvent} > .${timeline.handoff}`),
    );

    await drawn(details(), `.${documentPane.document}`);

    expect(details().querySelector(`.${briefPane.configuration}`)).toBeNull();
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

  /// Three lines, counted as line boxes rather than measured as a height: the
  /// count in the stylesheet and the one the module exports are the same number
  /// written in two languages, and this is where they are held to each other.
  it("shows three lines of it and cuts the rest with an ellipsis", () => {
    const clamp = block(".clamp");

    expect(CLAMPED_LINES).toBe(3);
    expect(clamp).toContain(`-webkit-line-clamp: ${CLAMPED_LINES}`);
    expect(clamp).toContain(`line-clamp: ${CLAMPED_LINES}`);
    expect(clamp).toContain("overflow: hidden");

    // The ellipsis comes with the clamp, so there is nothing to measure and
    // nothing to draw over the last line: a document that fits shows whole, and
    // one that does not ends in the mark that says so.
    expect(clamp).not.toContain("max-height");
  });

  /// Nothing fades. The cut was a gradient over the last line, drawn where a
  /// `ResizeObserver` had found the document went on — a soft edge at the foot
  /// of the sticky block, where the eye needs a hard one to tell the block's own
  /// end from a card trailing off.
  it("draws no fade, and keeps no card colour for one", () => {
    expect(timelineCss).not.toContain("linear-gradient");
    expect(timelineCss).not.toContain(".clamp.cut");
    expect(timelineCss).not.toContain(".clamp::after");

    // And the variable the fade read the card's own colour through, which had
    // no other reader.
    expect(pressableCss).not.toContain("--ground");
  });
});

/// Where a details pane stands, as a path of its own under the Conversation it
/// belongs to.
///
/// The arithmetic behind these paths is `pathing.test.ts`, which holds it true
/// as a value; what is asked here is what the page does with it — that a card
/// press writes the path, that a path drawn cold opens the pane, and which of
/// the two kinds of navigation grows the history stack.
describe("the path a details pane stands at", () => {
  /// A press on any of the three kinds of card — an Event, the backlog, a
  /// roadmap — leaves the page at the path that pane stands at.
  it("writes the path of whatever card is pressed", async () => {
    theGrilling();
    const { container, history } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));

    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${OUTPUT.id}`,
      ),
    );
  });

  it("writes the backlog's path by the word, there being one per conversation", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container, history } = mount(`/conversations/${TASKED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.taskList}`),
    );

    await waitFor(() =>
      expect(history.get()).toBe(`/conversations/${TASKED.id}/backlog`),
    );
  });

  it("writes a roadmap's path by its own directory name", async () => {
    theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container, history } = mount(`/conversations/${STAGED.id}`);

    fireEvent.click(
      await drawn(container, `.${timeline.pinned} .${timeline.stageList}`),
    );

    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${STAGED.id}/roadmaps/${ROADMAP.name}`,
      ),
    );
  });

  /// Which is the whole point of putting it there: the pane is drawn from the
  /// path, so a reload or a link somebody kept opens what it names.
  it("opens the event a path names on a cold load", async () => {
    theSpeaking();
    const { container } = mount(
      `/conversations/${GRILLING.id}/events/${OUTPUT.id}`,
    );

    await drawn(container, `.${shell.detailsPane} .${outputPane.turn}`);
  });

  it("opens the backlog a path names on a cold load", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container } = mount(`/conversations/${TASKED.id}/backlog`);

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);
  });

  it("opens the roadmap a path names on a cold load", async () => {
    theStaged({}, whenever(THE_ROADMAP, json(ROADMAP_PANE)));
    const { container } = mount(
      `/conversations/${STAGED.id}/roadmaps/${ROADMAP.name}`,
    );

    await drawn(container, `.${shell.detailsPane} .${documents.section}`);
  });

  /// And a path naming something the Conversation has not got leaves the pane
  /// empty, exactly as a stale selection did: the URL is a record of what was
  /// picked rather than a promise that it is still there.
  it("leaves the pane empty where the path names no such event", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}/events/99999`);

    await drawn(container, `.${timeline.timeline}`);

    expect(
      container.querySelector(`.${shell.detailsPane} .${paneHead.head}`),
    ).toBeNull();
  });

  /// A phone shows one level at a time, and the level a cold load lands on is
  /// the one the path names — the details, rather than the Timeline the human
  /// would have to walk forward from a second time.
  it("lands a narrow window on the details pane it was opened at", async () => {
    theSpeaking();
    const { container } = mount(
      `/conversations/${GRILLING.id}/events/${OUTPUT.id}`,
    );

    await waitFor(() => expect(frame(container).dataset.pane).toBe("details"));

    // And the walk back out of it is the pane's own, as it is from a pane
    // opened by pressing a card.
    fireEvent.click(
      await drawn(container, `.${shell.detailsPane} .${paneHead.back}`),
    );
    expect(frame(container).dataset.pane).toBe("middle");
  });

  /// Entering a Conversation is the page changing, so it pushes: Back from
  /// inside one leaves it for the list it was entered from.
  it("pushes entering a conversation, so back leaves it", async () => {
    theWorkbench();
    const { container, history } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    fireEvent.click(screen.getByText(DRAFTING.branch));
    await waitFor(() =>
      expect(history.get()).toBe(`/conversations/${DRAFTING.id}`),
    );

    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
    await waitFor(() =>
      expect(frame(container).dataset.pane).toBe("conversations"),
    );
  });

  /// Opening a details pane is a place in that page rather than a page, so it
  /// replaces: however many the human walks between, Back leaves the
  /// Conversation rather than retracing them one at a time.
  it("replaces between details, so back leaves the conversation whole", async () => {
    theGrilling();
    const { container, history } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${OUTPUT.id}`,
      ),
    );

    fireEvent.click(await drawn(container, `.${timeline.brief}`));
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${briefOf(GRILLING).id}`,
      ),
    );

    // Two details walked between, and one step out of the Conversation: the
    // entry the second press wrote is the entry the first one wrote.
    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// What the nesting under the Conversation's route is for: the middle pane
  /// stands through a change of detail, so nothing it is holding — a Brief half
  /// typed into above all — goes when a card is pressed.
  it("leaves the timeline pane standing while the detail changes", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const before = await drawn(container, `.${shell.middlePane} .${timeline.timeline}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.${shell.detailsPane} .${paneHead.head}`);

    expect(container.querySelector(`.${shell.middlePane} .${timeline.timeline}`)).toBe(
      before,
    );
  });
});

/// Opening a Conversation is asking where the work got to, so the page finishes
/// the walk the sidebar started: the newest thing on the record that has a pane
/// behind it is selected, and the URL is rewritten to its path.
///
/// The sidebar cannot do it itself — its list says a Conversation moved and
/// nothing about what moved — so the card navigates to the Conversation as it
/// always did and this lands once the Timeline has arrived.
///
/// Which thing on a record that is, is `pathing.test.ts`: the arithmetic over a
/// Timeline, held true against these same golden fixtures. What is asked here is
/// what the page does with the answer.
describe("landing on the end of the record", () => {
  it("opens the newest thing on the record a card is pressed on", async () => {
    theThree();
    const { container, history } = mount();
    await waitFor(() => screen.getByText(GRILLING.branch));

    fireEvent.click(screen.getByText(GRILLING.branch));

    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${UNREADABLE_SET.id}`,
      ),
    );
    // And the pane opens on it, rather than the path naming something the page
    // has not got round to drawing.
    await drawn(container, `.${shell.detailsPane} .${paneHead.head}`);
  });

  /// A record very often ends on something with nothing behind it — every step
  /// of the ladder writes a move, and a wrap-up ends on a manual task — so what
  /// is picked is the last *openable* thing rather than the last thing.
  it("skips past the events with nothing to open", async () => {
    theWrapping();
    const { container, history } = mount(`/conversations/${WRAPPING.id}`);

    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${WRAPPING.id}/events/${OPENED.id}`,
      ),
    );
    await drawn(container, `.${shell.detailsPane} .${prPane.commits}`);
  });

  /// And where the newest openable thing is one of the two lists, it is opened
  /// by the word its card is named by: neither has an Event of its own.
  it("opens a list by its word where that is the newest thing", async () => {
    theTasked({}, whenever(THE_BACKLOG, json(BACKLOG_PANE)));
    const { container, history } = mount(`/conversations/${TASKED.id}`);

    await waitFor(() =>
      expect(history.get()).toBe(`/conversations/${TASKED.id}/backlog`),
    );
    await drawn(container, `.${shell.detailsPane} .${documents.section}`);
  });

  /// Every Conversation starts on a Brief, so a Draft with nothing else on its
  /// record lands on the composer that Brief is written in — which is where the
  /// human is going next in any case.
  it("lands a draft on the composer its brief is written in", async () => {
    theWorkbench();
    const { container, history } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${OPEN.id}/events/${BRIEF.id}`,
      ),
    );
    await drawn(container, `.${shell.detailsPane} .${composer.composer}`);
  });

  /// And a path that already names a pane is a cold load of that pane — a
  /// reload, or a link somebody kept — so it keeps the selection it was opened
  /// at rather than being moved on to the end of the record.
  it("leaves a cold load of a details pane where it was opened", async () => {
    theGrilling();
    const { container, history } = mount(
      `/conversations/${GRILLING.id}/events/${briefOf(GRILLING).id}`,
    );

    await drawn(container, `.${shell.detailsPane} .${briefPane.brief}`);

    expect(history.get()).toBe(
      `/conversations/${GRILLING.id}/events/${briefOf(GRILLING).id}`,
    );
  });

  /// The landing replaces rather than pushes, as every other change of detail
  /// does: entering the Conversation is the one entry it wrote, so Back leaves
  /// it rather than stepping back through the pane it landed on.
  it("lands by replacing, so back leaves the conversation", async () => {
    theThree();
    const { history } = mount();
    await waitFor(() => screen.getByText(GRILLING.branch));

    fireEvent.click(screen.getByText(GRILLING.branch));
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${UNREADABLE_SET.id}`,
      ),
    );

    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// And a phone stays on the Timeline. What decides which level a narrow window
  /// shows is the Conversation changing, and landing on the end of a record
  /// changes none — so the newest thing is marked open with the details a tap
  /// away, rather than the human being carried past the record they opened.
  it("lands a phone on the timeline rather than in the details", async () => {
    theGrilling();
    const { container, history } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${UNREADABLE_SET.id}`,
      ),
    );

    expect(frame(container).dataset.pane).toBe("middle");
    await waitFor(() => screen.getByRole("button", { name: "Details →" }));
  });
});

/// One more Event for a record to grow by, which is what a running session does
/// to the Timeline being read. A notice, that being the shortest thing Verkstead
/// ever puts on one.
const LANDED: TimelineEvent = {
  Notice: {
    id: 99,
    at: "2026-08-03T09:07:11.000Z",
    html: "<p>The session stopped.</p>\n",
  },
};

/// The Timeline read from the bottom, which is where the work got to.
///
/// The following itself is `following.test.ts` — landing at the end, the pause
/// when the human scrolls up, the resume when they come back down — asked of a
/// box whose height is a number, jsdom laying nothing out to measure. What is
/// asked here is that this pane is following at all: it opens at the end of the
/// record, and goes after each Event that lands on it.
describe("the timeline following its bottom", () => {
  it("opens the record at its end", async () => {
    const scrolled = vi.fn();
    vi.stubGlobal("scrollTo", scrolled);

    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timeline}`);

    expect(scrolled).toHaveBeenCalled();
  });

  it("goes after each event that lands on the record", async () => {
    const scrolled = vi.fn();
    vi.stubGlobal("scrollTo", scrolled);

    /// The record as the server has it, which grows under the page the way a
    /// running session's does.
    let record: TimelineEvent[] = GRILLING.timeline;

    theGrilling(
      whenever(`/api/ui/conversations/${GRILLING.id}`, () =>
        json({ ...GRILLING, timeline: record })(),
      ),
    );
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, `.${timeline.timeline}`);
    const landed = scrolled.mock.calls.length;
    expect(landed).toBeGreaterThan(0);

    record = [...GRILLING.timeline, LANDED];
    await nudged(client);

    await waitFor(() =>
      expect(
        container.querySelectorAll(`.${timeline.timelineEvent}`),
      ).toHaveLength(record.length),
    );
    expect(scrolled.mock.calls.length).toBeGreaterThan(landed);
  });
});

/// The selection read the same way: a Conversation opened at the end of its
/// record goes on standing there, so what the details pane holds is the newest
/// thing on the Timeline rather than the newest thing as of the moment it was
/// opened.
///
/// Both ways in are the same walk — a press on a sidebar row, and the redirect a
/// Conversation is started or drafted with, each of them a navigation to the
/// Conversation with no pane named — so what is asked here of the press is true
/// of the redirect, and mounting at a Conversation's own path is that redirect
/// arriving.
///
/// Nothing is drawn for it. The mode's whole effect is which card is open, so
/// what these read is the URL: what is open is the URL's, and a test reading the
/// pane instead would be asking the same question through the pane's own
/// spinners.
describe("the selection following the end of the record", () => {
  /// Two more Events for a record to grow by, in the order a session lands them.
  /// Notices, that being the shortest thing Verkstead ever puts on one, and each
  /// with a pane behind it — which is what the selection moves on to.
  const SAID: NoticeEvent = {
    id: 981,
    at: "2026-08-03T09:07:11.000Z",
    html: "<p>The session started.</p>\n",
  };
  const SAID_NEXT: NoticeEvent = {
    id: 982,
    at: "2026-08-03T09:09:23.000Z",
    html: "<p>The session stopped.</p>\n",
  };

  /// The three Conversations, with the grilling one's record growing the way a
  /// running session grows it: the endpoint answers with whatever the record
  /// holds at the moment it is asked, so landing an Event and nudging the page
  /// is a session having done something.
  ///
  /// What comes back lands them: it waits for the page to have read the record
  /// again and drawn what grew, so an assertion after it is about the page the
  /// re-read left rather than the page mid-flight.
  function growing() {
    let record: TimelineEvent[] = GRILLING.timeline;

    theThree(
      whenever(`/api/ui/conversations/${GRILLING.id}`, () =>
        json({ ...GRILLING, timeline: record })(),
      ),
    );

    return async (
      container: ParentNode,
      client: Parameters<typeof nudged>[0],
      ...landed: TimelineEvent[]
    ) => {
      record = [...record, ...landed];
      await nudged(client);

      await waitFor(() =>
        expect(
          container.querySelectorAll(`.${timeline.timelineEvent}`),
        ).toHaveLength(record.length),
      );
    };
  }

  /// Where the grilling conversation's record ends when it is opened, which is
  /// the Set this build cannot read.
  const END = `/conversations/${GRILLING.id}/events/${UNREADABLE_SET.id}`;

  /// A press on the grilling conversation's row in the sidebar, found by the id
  /// its card carries rather than by the branch name on it: the name is on the
  /// Timeline's own header as well once the Conversation is open, and a press by
  /// name would be two things by then.
  async function opened(container: ParentNode): Promise<void> {
    const rows = await cards(container);
    const row = rows.find((row) => row.dataset.id === String(GRILLING.id));
    if (row === undefined) {
      throw new Error("the sidebar should be holding the grilling conversation");
    }

    fireEvent.click(row.querySelector("button")!);
  }

  it("opens each event that lands on a record opened at its end", async () => {
    const lands = growing();
    const { container, history, client } = mount();

    await opened(container);
    await waitFor(() => expect(history.get()).toBe(END));

    await lands(container, client, { Notice: SAID });
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${SAID.id}`,
      ),
    );

    // And again, the mode being where the Timeline stands rather than one
    // advance it was owed.
    await lands(container, client, { Notice: SAID_NEXT });
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${SAID_NEXT.id}`,
      ),
    );
  });

  /// The redirect a started or drafted Conversation is carried on by, which is a
  /// navigation to the Conversation with no pane named — the same arrival as the
  /// press above, made by the composer rather than by the sidebar.
  it("advances a conversation arrived at by its own path", async () => {
    const lands = growing();
    const { container, history, client } = mount(
      `/conversations/${GRILLING.id}`,
    );

    await waitFor(() => expect(history.get()).toBe(END));

    await lands(container, client, { Notice: SAID });
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${SAID.id}`,
      ),
    );
  });

  /// And picking a card is the human saying what they want to be looking at,
  /// which is the whole of what ends the mode.
  it("stays on the card the human pressed once they have pressed one", async () => {
    const lands = growing();
    const { container, history, client } = mount(
      `/conversations/${GRILLING.id}`,
    );

    await waitFor(() => expect(history.get()).toBe(END));

    const at = `/conversations/${GRILLING.id}/events/${OUTPUT.id}`;
    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await waitFor(() => expect(history.get()).toBe(at));

    await lands(container, client, { Notice: SAID });
    expect(history.get()).toBe(at);
  });

  /// A cold load of a details pane keeps the selection it was opened at, and
  /// goes on keeping it: a link somebody kept is a request to be shown that
  /// thing, which the next Event landing does not answer.
  it("leaves a pane opened by its own link where it was opened", async () => {
    const at = `/conversations/${GRILLING.id}/events/${briefOf(GRILLING).id}`;
    const lands = growing();
    const { container, history, client } = mount(at);

    await drawn(container, `.${shell.detailsPane} .${briefPane.brief}`);

    await lands(container, client, { Notice: SAID });
    expect(history.get()).toBe(at);
  });

  /// And the mode is off until it is arrived at again, rather than off for good:
  /// pressing the row a second time is the same arrival as pressing it the
  /// first, and the Timeline is following again from the end it lands on.
  it("follows again when the conversation is opened again", async () => {
    const lands = growing();
    const { container, history, client } = mount();

    await opened(container);
    await waitFor(() => expect(history.get()).toBe(END));

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${OUTPUT.id}`,
      ),
    );

    await opened(container);
    await waitFor(() => expect(history.get()).toBe(END));

    await lands(container, client, { Notice: SAID });
    await waitFor(() =>
      expect(history.get()).toBe(
        `/conversations/${GRILLING.id}/events/${SAID.id}`,
      ),
    );
  });
});

/// A Verkstead with no session to run: the state where a session would start,
/// and no press that would start one.
///
/// The server says so on every conversation it sends — a Windows build has no
/// terminal for an agent to work in — and this is the whole of what the page
/// does about it. Asked here rather than on a Windows machine, which is the
/// point of the fact being a value: the fixture says the server runs none, and
/// what the page draws is drawn on whatever is running these tests.
describe("a Verkstead that runs no sessions", () => {
  /// The drafting conversation as such a server would send it.
  const ABSENT: Partial<ConversationView> = { sessions: "NotOnWindowsYet" };

  it("says so where the work would be started, and offers no press", async () => {
    theWorkbenchWith(ABSENT);
    const { container } = mount(`/conversations/${OPEN.id}`);

    const under = await drawn(container, `.${composer.startGrilling}`);

    expect(under.textContent).toContain(NO_SESSIONS);
    expect(
      under.querySelector(`.${composer.start}`),
      "and no Start work to press, ready or not",
    ).toBeNull();
  });

  /// And the press beside it, on the page a conversation started from the
  /// abandoned-roadmaps notice opens on: the same sentence, in the place the
  /// same press would have been.
  it("says the same where a stage would be adopted, and offers no press", async () => {
    theWorkbench(
      whenever(
        `/api/ui/conversations/${ADOPTING.id}`,
        json({ ...ADOPTING, ...ABSENT }),
      ),
    );
    const { container } = mount(`/conversations/${ADOPTING.id}`);

    const panel = await drawn(container, `.${adoption.adoption}`);

    expect(panel.textContent).toContain(NO_SESSIONS);
    expect(screen.queryByRole("button", { name: "Adopt" })).toBeNull();
  });

  /// And Resume goes off the menu, on a conversation the server still says
  /// there is driving to start again for: what the press works out is which
  /// session should be running, and there are none.
  it("offers no resume, on a conversation the server says is ready for one", async () => {
    theGrillingStanding(ABSENT);
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);
    await drawn(menu, `.${actions.close}`);

    expect(GRILLING.ready_to_resume).toBe(true);
    expect(menu.querySelector(`.${actions.resume}`)).toBeNull();
    expect(
      menu.querySelector(`.${actions.steer}`),
      "and the steer row stays: done is still somewhere to steer into",
    ).toBeTruthy();
  });

  /// The steer modal is the one place where some of what it offers still works.
  /// Four of the five targets run a session and are held shut with the reason
  /// above them; done runs nothing, and is the move this build can still make.
  it("holds the steer shut on every target that runs, and says why", async () => {
    theGrillingStanding(ABSENT, whenever(STEERING, OVER_NOTHING, "POST"));
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const modal = await openSteer(container);
    const press = (await drawn(
      modal,
      `.${steerModal.steerButtons} .${steerModal.steer}`,
    )) as HTMLButtonElement;

    for (const target of ["Grilling", "Implementing"]) {
      fireEvent.click(
        await drawn(modal, `.${steerModal.steerTarget} input[value="${target}"]`),
      );

      await waitFor(() => expect(press.disabled).toBe(true));
      expect(modal.textContent).toContain(NO_SESSIONS);
    }

    fireEvent.click(
      await drawn(modal, `.${steerModal.steerTarget} input[value="Done"]`),
    );

    await waitFor(() => expect(press.disabled).toBe(false));
    expect(
      modal.textContent,
      "and nothing is said about sessions where none would run",
    ).not.toContain(NO_SESSIONS);
  });

  /// And every press that could still be made and refused says one thing.
  ///
  /// Five ways into a session and one answer: which press asked for one is not
  /// something the human has to be told about, and five wordings of it would be
  /// five people's guesses at the same sentence.
  it("says one thing, whichever press asked for a session", () => {
    expect(ADOPT_REFUSAL.NotOnWindowsYet).toBe(NO_SESSIONS);
    expect(RESUME_REFUSAL.NotOnWindowsYet).toBe(NO_SESSIONS);
    expect(STEER_REFUSAL.NotOnWindowsYet).toBe(NO_SESSIONS);
    expect(RESOLVE_REFUSAL.NotOnWindowsYet).toBe(NO_SESSIONS);
  });

  /// And a start that was refused all the same — the page is only as fresh as
  /// its last read, so a conversation that was drawn before the server said
  /// anything about sessions still has a button to press.
  it("answers a start that was refused with the same sentence", async () => {
    theWorkbenchWith(
      {},
      whenever(
        `/api/ui/conversations/${OPEN.id}/grill`,
        json("NotOnWindowsYet" satisfies GrillingStarted),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    fireEvent.click(
      await drawn(container, `.${composer.startGrilling} .${composer.start}`),
    );

    await waitFor(() => screen.getByText(NO_SESSIONS));
  });
});
