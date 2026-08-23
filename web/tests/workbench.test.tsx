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
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  AbandonedRepo,
  Adopted,
  AgentOutputEvent,
  BriefEvent,
  Capture,
  CommitDiff,
  ConversationAborted,
  ConversationEntry,
  ConversationView,
  GrillingStarted,
  ManualTaskStarted,
  ProfileEntry,
  PullRequestDetails,
  RemedySettled,
  Screen,
  Shown,
  SetView,
  Submitted,
  TimelineEvent,
  TranscriptView,
} from "../src/api/types";
import stylesheet from "../src/main.css?raw";
import { ADOPT_REFUSAL } from "../src/workbench/Adoption";
import { MANUAL_TASK_REFUSAL } from "../src/workbench/Timeline";
import {
  OPEN,
  PROFILES,
  REPOS,
  SIDEBAR,
  drawn,
  mount,
  nudged,
  theWorkbench,
} from "./bench";
import { askedFor, json, serving, whenever } from "./serving";
import abandoned from "./fixtures/abandoned-roadmaps.json" with { type: "json" };
import adopting from "./fixtures/conversation-adopting.json" with { type: "json" };
import building from "./fixtures/conversation-building.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import interrupted from "./fixtures/conversation-interrupted.json" with { type: "json" };
import answeredSet from "./fixtures/set-answered.json" with { type: "json" };
import answeringSet from "./fixtures/set-answering.json" with { type: "json" };
import roadmap from "./fixtures/conversation-roadmap.json" with { type: "json" };
import tasks from "./fixtures/conversation-tasks.json" with { type: "json" };
import capture from "./fixtures/capture.json" with { type: "json" };
import transcript from "./fixtures/transcript.json" with { type: "json" };
import more from "./fixtures/transcript-more.json" with { type: "json" };
import screenOfIt from "./fixtures/screen.json" with { type: "json" };
import wrapping from "./fixtures/conversation-wrapping.json" with { type: "json" };

/// The renderer is a page's own doing and neither Set fixture has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

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
  // One test drives the brief field's typing pause on a clock of its own.
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

/// Open the conversation's action menu, the way a click on its summary does.
///
/// `details` opens itself natively, which jsdom does not do for a synthetic
/// click — so the state is set and the toggle it would have fired is fired.
async function openActions(container: ParentNode): Promise<void> {
  const menu = await drawn<HTMLDetailsElement>(
    container,
    ".conversation-actions",
  );

  menu.open = true;
  fireEvent(menu, new Event("toggle"));
}

/// The body the page put on the wire when it wrote to `path`.
///
/// By the request rather than by being the last thing sent: writing anything
/// here is followed by reading the Conversation back, so the last call is
/// ordinarily the read.
function sent(
  fetching: ReturnType<typeof serving>,
  path: string,
): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  );
  expect(written, `expected the page to have written to ${path}`).toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

/// How many times the page wrote to `path`, for the tests about *when* a save
/// goes out rather than what was in it.
function writes(fetching: ReturnType<typeof serving>, path: string): number {
  return fetching.mock.calls.filter(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  ).length;
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
  /// Repos and the Agent Profiles were folded onto the settings page.
  it("reaches the rest of Verkstead from the sidebar", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    const elsewhere = container.querySelector(".elsewhere")!;
    expect(
      [...elsewhere.querySelectorAll("a")].map((to) => to.getAttribute("href")),
    ).toEqual(["/settings"]);
  });

  it("says where to go when there is no repo to start one against", async () => {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/repos", json([])),
    );
    const { container } = mount();

    await waitFor(() => screen.getByText(/No repos are registered yet/));
    expect(container.querySelector(".start-conversation")).toBeNull();
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

  it("shows a dot on a conversation waiting on the human", async () => {
    theSidebar({ state: "Grilling", working: false, waiting: true });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.querySelector(".mark.waiting")).toBeTruthy();
    expect(card!.querySelector(".mark.working")).toBeNull();
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
  });

  it("draws a draft as a draft, and marks nothing on it", async () => {
    theSidebar({ state: "Draft", working: false, waiting: false });
    const { container } = mount();

    const [card] = await cards(container);

    expect(card!.classList.contains("draft")).toBe(true);
    expect(card!.querySelector(".mark")).toBeNull();

    // What "draft" means is the stylesheet's, and jsdom lays nothing out.
    expect(stylesheet).toContain(
      ".conversation-row.draft button {\n  border-style: dotted;\n}",
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

describe("starting a conversation", () => {
  it("sends the repo that was picked, and opens what came back", async () => {
    const fetching = theWorkbench(json({ Started: { id: OPEN.id } }));
    const { history } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    fireEvent.change(screen.getByLabelText(/new conversation in/i), {
      target: { value: String(REPOS[1]!.id) },
    });
    fireEvent.click(screen.getByRole("button", { name: "Start" }));

    // Straight into it: what the human does next is write the brief.
    await waitFor(() =>
      expect(history.get()).toBe(`/conversations/${OPEN.id}`),
    );
    expect(sent(fetching, "/api/ui/conversations")).toEqual({
      repo_id: REPOS[1]!.id,
    });
  });

  it("offers the first repo without anything being picked", async () => {
    const fetching = theWorkbench(json({ Started: { id: OPEN.id } }));
    mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    fireEvent.click(screen.getByRole("button", { name: "Start" }));

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations")).toEqual({
        repo_id: REPOS[0]!.id,
      }),
    );
  });
});

describe("the abandoned roadmaps notice", () => {
  /// The whole of it: one notice per Repo, its roadmaps inside, each named with
  /// the stage that would be started.
  it("names the repo, its roadmaps and the stage each one is up to", async () => {
    theWorkbench(whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)));
    const { container } = mount();

    const notices = await waitFor(() => {
      const drawn = container.querySelectorAll(".abandoned-notice");
      expect(drawn.length).toBe(ABANDONED.length);
      return drawn;
    });

    const notice = notices[0]!;
    expect(notice.textContent).toContain(ABANDONED[0]!.repo);

    const roadmaps = notice.querySelectorAll("li");
    expect(roadmaps.length).toBe(ABANDONED[0]!.roadmaps.length);

    for (const [n, roadmap] of ABANDONED[0]!.roadmaps.entries()) {
      const said = roadmaps[n]!.textContent!;
      expect(said).toContain(roadmap.name);
      expect(said).toContain(roadmap.stage);
      expect(said).toContain(roadmap.stage_title);
    }
  });

  /// Under the box that starts a conversation, which is what it is an
  /// alternative to — and above the conversations themselves, which are the work
  /// already under way.
  it("is drawn under the new conversation box", async () => {
    theWorkbench(whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)));
    const { container } = mount();

    const notice = await drawn(container, ".abandoned");
    const box = container.querySelector(".start-conversation")!;

    expect(box.compareDocumentPosition(notice)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  /// Nothing to adopt is nothing to say. A Repo whose roadmaps are all
  /// complete, mid-flight or broken contributes no notice at all, and a
  /// workbench with none draws no heading over an empty list.
  it("says nothing when there is nothing to adopt", async () => {
    theWorkbench();
    const { container } = mount();

    await waitFor(() => screen.getByText(DRAFTING.branch));

    expect(container.querySelector(".abandoned")).toBeNull();
  });

  /// Read again with everything else the page is showing, because the server
  /// reads it off the repositories every time it is asked: a roadmap somebody
  /// has since picked up stops being on the list, and the notice goes with it.
  it("is read again when the page looks again", async () => {
    const fetching = theWorkbench(
      whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)),
    );
    const { container } = mount();
    await drawn(container, ".abandoned-notice");

    const before = askedFor(fetching, "/api/ui/abandoned-roadmaps");
    readAgain();

    await waitFor(() =>
      expect(askedFor(fetching, "/api/ui/abandoned-roadmaps")).toBeGreaterThan(
        before,
      ),
    );
  });

  /// Clicking a roadmap starts a conversation to adopt it with, and goes
  /// straight into it — which is where both profiles and the base commit are
  /// fixed, and where adopting is pressed.
  ///
  /// What goes out is the repo and the roadmap and nothing else: which stage is
  /// next is the roadmap's own answer at the commit the conversation ends up
  /// branching from, and the page reads it back there.
  it("starts a conversation to adopt the roadmap that was clicked", async () => {
    const fetching = theWorkbench(
      whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)),
      whenever(
        "/api/ui/adoptions",
        json({ Started: { id: OPEN.id } }),
        "POST",
      ),
    );
    const { container, history } = mount();

    const notice = await drawn(container, ".abandoned-notice");
    const roadmaps = notice.querySelectorAll<HTMLButtonElement>("li button");
    fireEvent.click(roadmaps[1]!);

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
    expect(screen.getByLabelText("Base commit")).toBeTruthy();
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

  /// The stage is the server's reading at the base commit, so a base recorded
  /// is a page read again — and what it names then is the stage that is next
  /// *there*.
  it("names the stage again when the base commit changes", async () => {
    const elsewhere: ConversationView = {
      ...ADOPTING,
      base_commit: "9a1c3e5b7d90f2468ace13579bdf02468ace1357",
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

    fireEvent.input(await waitFor(() => screen.getByLabelText("Base commit")), {
      target: { value: elsewhere.base_commit },
    });
    fireEvent.click(screen.getByRole("button", { name: "Record" }));

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
  /// The field, and the word beside the heading about what became of what is
  /// in it.
  const field = () => screen.getByLabelText("Brief") as HTMLTextAreaElement;
  const indicator = (container: ParentNode) =>
    container.querySelector(".brief-standing")?.textContent ?? "";

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

    // The field stays where it is, and says the record has what is in it.
    await waitFor(() => expect(indicator(container)).toBe("Saved"));
    expect(field().value).toBe(written);
  });

  it("saves what was typed after a pause in the typing", async () => {
    const fetching = theWorkbench(json("Saved"));
    const { container } = mount(`/conversations/${OPEN.id}`);
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
    expect(indicator(container)).toBe("Not saved yet");

    await vi.advanceTimersByTimeAsync(2_000);

    // One save, of the whole of what was typed rather than of the first half.
    expect(writes(fetching, WRITING)).toBe(1);
    expect(sent(fetching, WRITING)).toEqual({ markdown: "# Half a thought" });
    await waitFor(() => expect(indicator(container)).toBe("Saved"));
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
    expect(setup.querySelector(".base-commit")).toBeTruthy();
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
    expect(screen.queryByLabelText("Base commit")).toBeNull();
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

  it("offers the branch name the server prefilled, and sends a new one", async () => {
    const fetching = theWorkbench(json("Renamed"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Branch"));

    const field = screen.getByLabelText("Branch") as HTMLInputElement;
    expect(field.value).toBe(OPEN.branch);

    fireEvent.input(field, { target: { value: "counter-in-redis" } });
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/branch`),
      ).toEqual({ branch: "counter-in-redis" }),
    );
  });

  it("says why a branch name was refused, in words", async () => {
    theWorkbench(json("NotABranchName"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Branch"));

    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "two..dots" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));

    await waitFor(() => screen.getByText(/will not take that as a branch name/i));
  });

  it("shows the base commit that was recorded, and sends a new one", async () => {
    const fetching = theWorkbench(json("Recorded"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Base commit"));

    const field = screen.getByLabelText("Base commit") as HTMLInputElement;
    expect(field.value).toBe(OPEN.base_commit);

    fireEvent.input(field, { target: { value: "v0.1.0" } });
    fireEvent.click(screen.getByRole("button", { name: "Record" }));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/base`)).toEqual({
        commit: "v0.1.0",
      }),
    );
  });

  /// Emptying the field is the override taken away, not a commit called
  /// nothing — and what it goes back to is the rule, which the pane says in
  /// words because an empty field cannot.
  it("takes the override away when the field is emptied", async () => {
    const fetching = theWorkbench(json("Recorded"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Base commit"));

    fireEvent.input(screen.getByLabelText("Base commit"), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Record" }));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/base`)).toEqual({
        commit: null,
      }),
    );
  });

  it("names the branch an unpinned conversation will start from", async () => {
    const rule: ConversationView = { ...OPEN, base_commit: null };
    serving(
      whenever("/api/ui/conversations", json(SIDEBAR)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever(`/api/ui/conversations/${OPEN.id}`, json(rule)),
    );
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByLabelText("Base commit"));

    expect((screen.getByLabelText("Base commit") as HTMLInputElement).value).toBe(
      "",
    );
    expect(container.querySelector(".base-commit .note")!.textContent).toContain(
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
  /// answer is the server's rather than a count of the two fields.
  it("says whether the conversation is ready to grill", async () => {
    withConversation(UNCHOSEN);
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByText(/Not ready to grill/));
    expect(container.querySelector(".readiness")!.classList).not.toContain(
      "ready",
    );

    // The fixture's own conversation has both, and the server says so.
    expect(OPEN.ready_to_grill).toBe(true);
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
    mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByText("Its config file is gone."));
    await waitFor(() => screen.getByText(/Not ready to grill/));
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

/// Where the workbench hands this conversation's keyboard back.
const HANDING_BACK = `/api/ui/conversations/${GRILLING.id}/hand-back`;

/// The same conversation with the human at its session's keyboard: `running`
/// says whether the session is still going, because a Hold outlives one.
///
/// A fixture has no Hold in it and never will — a Hold is a fact about a running
/// server rather than a payload, and it is nowhere on the Timeline.
function theHeld(running: boolean, ...answers: Parameters<typeof serving>) {
  const altered: TimelineEvent[] = GRILLING.timeline.map((event) =>
    "AgentOutput" in event
      ? { AgentOutput: { ...event.AgentOutput, running } }
      : event,
  );

  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${GRILLING.id}`,
      json({
        ...GRILLING,
        timeline: altered,
        held: OUTPUT.id,
        blocked_on: OUTPUT.id,
      }),
    ),
    whenever(TRANSCRIPT_OF_IT, json(SAID_NOTHING)),
    whenever(CAPTURE_OF_IT, json(CAPTURE)),
    whenever(SCREEN_OF_IT, json(SCREEN)),
    // Last, so a test can hold one of those paths to an answer of its own: a
    // later answer for a path replaces the earlier.
    ...answers,
  );
}

/// The workbench with the opened conversation altered, for the states no fixture
/// holds — a refusal from the server, a worktree that has gone.
function theWorkbenchWith(
  over: Partial<ConversationView>,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(`/api/ui/conversations/${OPEN.id}`, json({ ...OPEN, ...over })),
    ...answers,
  );
}


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

  it("says what is missing instead of offering a dead button", async () => {
    theWorkbenchWith({ ready_to_grill: false });
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByText(/the grilling can start/));
    expect(container.querySelector(".start-grilling .start")).toBeNull();
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

    await waitFor(() => screen.getByText("Started grilling"));
    expect(container.querySelector(".start-grilling")).toBeNull();
  });
});

describe("a move on the timeline", () => {
  it("draws the state the conversation moved to, as something that happened", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const moved = await drawn(container, ".timeline-event .moved");

    expect(moved.textContent).toBe("Started grilling");
    expect(moved.classList).toContain("grilling");
  });

  /// The brief stays the first event and everything after it follows in the
  /// order it happened, which is also reading order.
  it("comes after the brief it followed", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() => screen.getByText("Started grilling"));

    expect(
      [...container.querySelectorAll(".timeline-event > *")].map(
        (event) => event.className.split(" ")[0],
      ),
    ).toEqual([
      "brief",
      "moved",
      "agent-output",
      // The two Sets that session put to the human, in the order it asked
      // them: the answered one, and the one still waiting.
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
    const answered = await drawn(container, ".details-pane .turn.tool-result");

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
      ".details-pane .turn.tool-use details",
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
      TRANSCRIPT.turns.length,
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
        TRANSCRIPT.turns.length + MORE.turns.length,
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
        WHOLE_AGAIN.turns.length,
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

    // Watching, and saying what typing into it would cost: the first keystroke
    // takes the Hold, and a pane that let one be typed without saying so would
    // be one that stopped Verkstead by surprise.
    const said = await drawn(container, ".details-pane .screen .read-only");
    expect(said.textContent).toContain("Watching");
    expect(said.textContent).toContain("hand it back");

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

describe("taking a live session's keyboard", () => {
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
  function said(
    socket: Attached,
    kind: "Typed" | "Moused" | "Resized",
  ): unknown[] {
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

    await waitFor(() => expect(said(socket, "Typed")).toEqual(["\r"]));
    expect(grid.textContent).toBe(before);
  });

  /// A paste is the keyboard too. It arrives at the terminal as an event of its
  /// own rather than as a keypress, and what it carries is exactly what somebody
  /// meant to put into the session — so it goes up as typing, and takes the Hold
  /// the way a keystroke does.
  it("sends a paste as typing", async () => {
    const { container, socket } = await watching();

    const typing = await drawn<HTMLTextAreaElement>(
      container,
      ".details-pane .screen .xterm-helper-textarea",
    );

    fireEvent.paste(typing, {
      clipboardData: { getData: () => "cargo test" },
    });

    await waitFor(() => expect(said(socket, "Typed")).toEqual(["cargo test"]));
    expect(said(socket, "Moused")).toEqual([]);
  });

  /// And the mouse is not. A session whose interface tracks it has the terminal
  /// report every move, click and scroll down the same callback a keystroke
  /// comes out of — so a cursor crossing a live Screen would take the Hold, and
  /// silently stop Verkstead ending anything, if the two were not told apart.
  ///
  /// Told apart by what the human touched rather than by the bytes: nothing
  /// about a mouse report distinguishes it from an arrow key on the wire. What
  /// the wheel is turned into here is one of them.
  it("sends what the mouse did as the mouse, which takes nothing", async () => {
    const { container, socket } = await watching();

    const grid = await drawn(container, ".details-pane .screen .xterm-screen");

    fireEvent.wheel(grid, { deltaY: 120 });

    await waitFor(() => expect(said(socket, "Moused")).not.toEqual([]));

    // And nothing of it went up as typing, which is the whole of the claim: the
    // server takes the Hold on the one kind and never on the other.
    expect(said(socket, "Typed")).toEqual([]);
  });

  /// A Hold is the server's answer rather than this page's memory of having
  /// typed: one Hold is one Conversation's, and a phone that took it is one this
  /// window has to be able to see and end.
  it("says who has the keyboard, and hands it back on one press", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);

    const fetching = theHeld(
      true,
      whenever(HANDING_BACK, json("HandedBack"), "POST"),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    // Opened on the Screen rather than the Transcript, because that is what the
    // badge points at and where the press is.
    const socket = await attached();
    socket.says(PAINTED);

    const holding = await drawn(container, ".details-pane .screen .holding");
    expect(holding.textContent).toContain("You have the keyboard");

    fireEvent.click(await drawn(container, ".details-pane .hand-back"));

    await waitFor(() =>
      expect(
        fetching.mock.calls.filter(
          ([asked, init]) =>
            String(asked) === HANDING_BACK && init?.method === "POST",
        ),
      ).toHaveLength(1),
    );
  });

  /// And a session that exited while held still has the press. That is exactly
  /// the case that is waiting to be judged: nothing about the run moves until
  /// the keyboard goes back, so a pane that dropped the control once the session
  /// went would be one the human could not get out of.
  it("keeps the hand-back on a session that has ended", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);

    theHeld(false, whenever(HANDING_BACK, json("HandedBack"), "POST"));

    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const holding = await drawn(container, ".details-pane .screen .holding");
    expect(holding.textContent).toContain("the session has exited");
    expect(await drawn(container, ".details-pane .hand-back")).toBeTruthy();

    // Its Screen is the one it last stood on, fetched: there is nothing left to
    // attach to, held or not.
    expect(Attached.opened).toHaveLength(0);
  });

  /// The badge in the header says the work has stopped and points at the session
  /// holding it up, which is the same badge an Interruption draws — a Hold is
  /// the other thing the human can be blocked on, and the one that is nowhere on
  /// the Timeline.
  it("carries blocked on you while the hold lasts", async () => {
    Attached.opened = [];
    vi.stubGlobal("WebSocket", Attached);
    theHeld(true);

    const { container } = mount(`/conversations/${GRILLING.id}`);

    const badge = await drawn(container, ".timeline-pane .blocked");
    expect(badge.textContent).toContain("Blocked on you");
  });
});

describe("aborting a conversation", () => {
  /// Behind a menu on the header, because it throws a worktree away and the
  /// header is somewhere the cursor passes on the way to everything else.
  it("is not one click away", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await drawn<HTMLDetailsElement>(
      container,
      ".conversation-actions",
    );

    // Closed, so nothing in it can be reached without opening it first — which
    // is the whole of what standing a destructive action behind a menu means.
    expect(menu.open).toBe(false);
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

  /// What the human is owed before throwing a worktree away: what goes, and
  /// what stays.
  it("says the branch survives it", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await openActions(container);

    await waitFor(() => screen.getByText(/Removes the worktree/));
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

/// The two Question Sets the grilling conversation's session put to the human:
/// one answered, and one still waiting. Both are needed, because what a row
/// draws turns on which — and it is the waiting one the human is offered a sheet
/// for.
const ASKED = (() => {
  const found = GRILLING.timeline.flatMap((event) =>
    "QuestionSet" in event ? [event.QuestionSet] : [],
  );
  if (found.length !== 2) {
    throw new Error("the fixture should hold an answered Set and a waiting one");
  }
  return found;
})();

const ANSWERED_SET = ASKED.find((asked) => "Answered" in asked.standing)!;
const WAITING_SET = ASKED.find((asked) => "Waiting" in asked.standing)!;

/// The whole document behind each, which is what the details pane fetches. The
/// two Set fixtures are the same shapes read back from the same endpoint — the
/// standing is what decides whether the pane draws a sheet or a record, and
/// these are the two.
const DOCUMENT = answeredSet as SetView;
const SHEET = answeringSet as SetView;

/// The workbench with the grilling conversation open and both of its Sets
/// answerable, which is what the details pane fetches when one is opened.
function theGrillingSets(...answers: Parameters<typeof serving>) {
  return theGrilling(
    whenever(`/api/ui/sets/${ANSWERED_SET.set_id}`, json(DOCUMENT)),
    whenever(`/api/ui/sets/${WAITING_SET.set_id}`, json(SHEET)),
    ...answers,
  );
}

/// The rows of one Question Set's summary, as the three columns the design gives
/// it: the number, the question, and what became of it.
function summarised(card: ParentNode): string[][] {
  return [...card.querySelectorAll(".asked tr")].map((row) =>
    [...row.querySelectorAll("td")].map((cell) => cell.textContent ?? ""),
  );
}

describe("a question set on the timeline", () => {
  it("is summarised as the table of number, question and answer", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const card = await drawn(container, ".question-set");

    expect(summarised(card)).toEqual(
      ANSWERED_SET.rows.map((row) => [
        row.name,
        row.question,
        // A question the human left open — and the Heading, which was never
        // asked. The row says so rather than leaving a blank, because a blank
        // on a settled Set would read as an Answer of nothing.
        row.answer === "" ? "unanswered" : row.answer,
      ]),
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

    expect(cards.map((card) => card.classList.contains("waiting"))).toEqual([
      false,
      true,
    ]);
    expect(screen.getByText("waiting on you")).toBeTruthy();
  });

  /// A column of blanks would read as a Set that was answered with nothing.
  it("draws no answers at all on one nothing has been decided about", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    await drawn(container, ".question-set");
    const waiting = [...container.querySelectorAll(".question-set")][1]!;

    expect(
      summarised(waiting).map(([, , answer]) => answer),
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

  /// The table of contents is a description of a column the whole window wide,
  /// and a details pane is a column beside two others.
  it("leaves the page's own table of contents to the page", async () => {
    theGrillingSets();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".question-set"));

    const pane = screen.getByLabelText("Details");
    await waitFor(() => {
      if (!pane.querySelector(".preface")) {
        throw new Error("the document has not been drawn");
      }
    });

    expect(pane.querySelector(".contents")).toBeNull();
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

    expect(moved).toEqual(["Started grilling", "Started implementing"]);
  });

  it("draws the handoff the grilling wrote as the document it is", async () => {
    theBuilding();
    const { container } = mount(`/conversations/${BUILDING.id}`);

    const handoff = await drawn(container, ".timeline-event > .handoff");

    expect(handoff.querySelector("h2")?.textContent).toBe("Handoff");
    expect(handoff.querySelector(".markdown")?.innerHTML).toContain(
      "<h1>Pausing on a usage limit</h1>",
    );

    // Nothing to press and nothing to edit: it is the agent's account of a
    // conversation that is over.
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

/// One commit's diff, as the details pane fetches it.
///
/// The payload is built from the answering set's own attached diff rather than
/// written by hand: it is the same `DiffView`, rendered by the same server-side
/// renderer that a commit's diff goes through — which is the whole reason a
/// commit needs no diff machinery of its own.
const COMMIT_DIFF: CommitDiff = { diff: (answeringSet as SetView).diff };

/// Where the details pane fetches it from.
const DIFF_OF_IT = `/api/ui/conversations/${BUILDING.id}/commit/${COMMITS[0]!.id}`;

/// The workbench with that conversation open and its commits' diffs to hand.
function theCommits(...answers: Parameters<typeof serving>) {
  return theBuilding({}, whenever(DIFF_OF_IT, json(COMMIT_DIFF)), ...answers);
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
      COMMIT_DIFF.diff!.paths[0],
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

    const summary = await drawn(container, ".details-pane .commit-summary");

    expect(summary.textContent).toContain(COMMITS[0]!.subject);
    expect(summary.textContent).toContain(COMMITS[0]!.sha.slice(0, 7));
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

    // In words as well as in a class, so a row read aloud says it too.
    expect(rows.map((row) => row.querySelector(".state")!.textContent)).toEqual(
      BACKLOG.tasks.map((task) => (task.done ? "done" : "to do")),
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

    const menu = await drawn(container, ".conversation-actions .menu");

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

  it("says which stages are checked", async () => {
    theStaged();
    const { container } = mount(`/conversations/${STAGED.id}`);

    const list = await drawn(container, ".pinned .stage-list");
    const rows = [...list.querySelectorAll(".stages li")];

    expect(rows.map((row) => row.classList.contains("done"))).toEqual(
      ROADMAP.stages.map((stage) => stage.done),
    );

    // In words as well as in a class, as a task's row is.
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

/// A conversation whose run has stopped, and the interruption it stopped at.
const STOPPED = interrupted as ConversationView;

const HALTED = (() => {
  const event = STOPPED.timeline.find((entry) => "Interruption" in entry);
  if (!event || !("Interruption" in event)) {
    throw new Error("the fixture should carry an interruption");
  }
  return event.Interruption;
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

/// Where the human answers a run that stopped.
const REMEDY_PATH = `/api/ui/conversations/${STOPPED.id}/interruption/${HALTED.id}`;

describe("an interruption on the timeline", () => {
  it("says which step failed and how it ended", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");

    expect(stopped.querySelector(".what")!.textContent).toBe(HALTED.what);
    expect(stopped.querySelector(".how")!.textContent).toBe(HALTED.how);
  });

  /// The three remedies are on the event itself. Roadrunner asked this over
  /// askance because nobody was at its terminal; here the timeline is where the
  /// human looks, so the question is put where they are already looking.
  it("offers all three remedies on the event", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");

    expect(
      [...stopped.querySelectorAll(".remedy")].map((it) => it.textContent),
    ).toEqual(["Retry", "Take over manually", "Abort the run"]);
  });

  /// Nothing any of the three does touches the repo, and the human is told so
  /// before they choose rather than after.
  it("says the repo is left as the session left it", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");

    expect(stopped.textContent).toContain(
      "the repo is left exactly as the session left it",
    );
  });

  it("sends the remedy with whatever was written alongside it", async () => {
    const fetching = theStopped(
      {},
      whenever(REMEDY_PATH, json("Settled" satisfies RemedySettled), "POST"),
    );
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");

    const note = stopped.querySelector("textarea")!;
    fireEvent.input(note, {
      target: { value: "try again but leave the migration alone" },
    });

    const retry = [...stopped.querySelectorAll(".remedy")].find(
      (it) => it.textContent === "Retry",
    )!;
    fireEvent.click(retry);

    await waitFor(() =>
      expect(sent(fetching, REMEDY_PATH)).toEqual({
        remedy: "Retry",
        note: "try again but leave the migration alone",
      }),
    );

    // And nothing is said about a remedy that worked: the event reading back
    // settled is what says it.
    expect(stopped.querySelector(".error")).toBeNull();
  });

  /// The record is what a timeline is: a run that was retried and stopped again
  /// has both stops on it, each saying what was decided.
  it("shows what was chosen once it has been settled, and stops asking", async () => {
    theStopped({
      blocked_on: null,
      timeline: STOPPED.timeline.map((entry) =>
        "Interruption" in entry
          ? {
              Interruption: {
                ...entry.Interruption,
                settled: {
                  remedy: "TakeOver" as const,
                  note: "I'll finish this one myself",
                  at: "2026-08-03T10:14:02.000Z",
                },
              },
            }
          : entry,
      ),
    });
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");

    expect(stopped.querySelector(".settled")!.textContent).toContain(
      "Take over manually",
    );
    expect(stopped.querySelector(".settled .note")!.textContent).toBe(
      "I'll finish this one myself",
    );
    expect(stopped.querySelector(".remedies")).toBeNull();
    expect(stopped.classList.contains("open")).toBe(false);
  });

  /// Answered from another device, or by a second press. Not an error, and said
  /// in words rather than retried.
  it("says so when it was already answered", async () => {
    theStopped(
      {},
      whenever(
        REMEDY_PATH,
        json("AlreadySettled" satisfies RemedySettled),
        "POST",
      ),
    );
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");

    fireEvent.click(stopped.querySelector(".remedy")!);

    await waitFor(() =>
      expect(stopped.querySelector(".error")!.textContent).toContain(
        "The first choice stands",
      ),
    );
  });
});

describe("a conversation blocked on the human", () => {
  it("says so where the conversation is named", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const badge = await drawn(container, ".pane-head .blocked");

    expect(badge.textContent).toBe("Blocked on you");
  });

  /// A timeline is long by the time a run gets far enough to stop, so the badge
  /// goes to the event that stopped it rather than leaving the human to hunt.
  it("opens the event it is blocked on", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const badge = await drawn<HTMLButtonElement>(container, ".blocked");
    fireEvent.click(badge);

    const evidence = await drawn(container, ".details-pane .evidence");
    expect(evidence).toBeTruthy();
    expect(frame(container).dataset.pane).toBe("details");
  });

  it("draws no badge where nothing is stopping the work", async () => {
    expect(OPEN.blocked_on).toBeNull();

    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await drawn(container, ".timeline");

    expect(container.querySelector(".blocked")).toBeNull();
  });
});

describe("an interruption's evidence", () => {
  /// It rides on the event rather than being fetched, unlike a Capture or a
  /// diff: it is what the remedies are chosen against, and a pane that had to
  /// fetch it could draw the buttons before it could say what they were for.
  it("shows the worktree and the session's last words without another request", async () => {
    const fetching = theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");
    fireEvent.click(stopped.querySelector(".open-event")!);

    const pane = await drawn(container, ".details-pane .interruption-summary");
    expect(pane.textContent).toContain(HALTED.what);

    const status = await drawn(container, ".details-pane .git-status");
    expect(status.textContent).toBe(HALTED.git_status);

    const tail = await drawn(container, ".details-pane .tail");
    expect(tail.textContent).toBe(HALTED.tail);

    // Nothing was asked for it: every request the page made is a list or the
    // conversation itself.
    expect(
      fetching.mock.calls.map(([asked]) => String(asked)),
    ).not.toContain(`${REMEDY_PATH}`);
    expect(
      fetching.mock.calls
        .map(([asked]) => String(asked))
        .filter((path) => path.includes("/interruption")),
    ).toEqual([]);
  });

  /// Neither `git status` nor a terminal's last words are markdown, and the
  /// columns are the whole of what makes a status readable.
  it("draws both preformatted", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");
    fireEvent.click(stopped.querySelector(".open-event")!);

    const status = await drawn(container, ".details-pane .git-status");
    expect(status.tagName).toBe("PRE");

    const tail = await drawn(container, ".details-pane .tail");
    expect(tail.tagName).toBe("PRE");
  });

  /// Read at the moment the run stopped and kept, because both move on — a
  /// worktree is a directory the human also has.
  it("says the whole capture is elsewhere", async () => {
    theStopped();
    const { container } = mount(`/conversations/${STOPPED.id}`);

    const stopped = await drawn(container, ".timeline .interruption");
    fireEvent.click(stopped.querySelector(".open-event")!);

    const pane = await drawn(container, ".details-pane .evidence");
    expect(pane.closest(".details-pane")!.textContent).toContain(
      "The whole capture is the session's own event",
    );
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
    expect(moves.at(-1)).toBe("Moved to wrapping up");
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

  /// Read-only, and nothing to open: what its session went on to do arrives as
  /// the events any work arrives as, under this one.
  it("asks the human for nothing", async () => {
    theWrapping();
    const { container } = mount(`/conversations/${WRAPPING.id}`);

    const asked = await drawn(container, ".timeline-event > .manual-task");

    expect(asked.querySelectorAll("button")).toHaveLength(0);
    expect(asked.querySelectorAll("textarea")).toHaveLength(0);
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
