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

import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  AbandonedRepo,
  Adopted,
  AgentOutputEvent,
  Capture,
  CommitDiff,
  ConversationAborted,
  ConversationEntry,
  ConversationView,
  GrillingStarted,
  ProfileEntry,
  PullRequestDetails,
  RemedySettled,
  RepoEntry,
  SetView,
  Submitted,
  TimelineEvent,
  TranscriptView,
} from "../src/api/types";
import stylesheet from "../src/main.css?raw";
import { ADOPT_REFUSAL } from "../src/workbench/Adoption";
import { Workbench } from "../src/workbench/Workbench";
import { askedFor, json, serving, whenever } from "./serving";
import abandoned from "./fixtures/abandoned-roadmaps.json" with { type: "json" };
import adopting from "./fixtures/conversation-adopting.json" with { type: "json" };
import conversation from "./fixtures/conversation.json" with { type: "json" };
import building from "./fixtures/conversation-building.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import interrupted from "./fixtures/conversation-interrupted.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import answeredSet from "./fixtures/set-answered.json" with { type: "json" };
import answeringSet from "./fixtures/set-answering.json" with { type: "json" };
import roadmap from "./fixtures/conversation-roadmap.json" with { type: "json" };
import tasks from "./fixtures/conversation-tasks.json" with { type: "json" };
import capture from "./fixtures/capture.json" with { type: "json" };
import transcript from "./fixtures/transcript.json" with { type: "json" };
import wrapping from "./fixtures/conversation-wrapping.json" with { type: "json" };

/// The renderer is a page's own doing and neither Set fixture has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

const SIDEBAR = conversations as ConversationEntry[];
const OPEN = conversation as ConversationView;
const REPOS = repos as RepoEntry[];
const ABANDONED = abandoned as AbandonedRepo[];

/// The conversation that clicking one of those roadmaps made: a draft adopting
/// `mvp`, on the page shaped for adopting.
const ADOPTING = adopting as ConversationView;
const PROFILES = profiles as ProfileEntry[];

/// The one the fixture opens, which is the second row of the sidebar.
const DRAFTING = SIDEBAR.find((entry) => entry.id === OPEN.id)!;

/// The Brief on the opened Conversation's Timeline.
const BRIEF = (() => {
  const first = OPEN.timeline[0]!;
  if (!("Brief" in first)) {
    throw new Error("the fixture's first Event should be the Brief");
  }
  return first.Brief;
})();

afterEach(() => {
  vi.unstubAllGlobals();
  // The state is the instance's own, over the one every other test reads off
  // the prototype — so it goes when the test does.
  delete (document as { visibilityState?: DocumentVisibilityState })
    .visibilityState;
});

/// The read of the open Conversation, which is the one worth counting: the page
/// fetches three other things around it.
const READING = `/api/ui/conversations/${OPEN.id}`;

/// The page reading everything it is showing again, provoked the way coming back
/// to the app provokes it — the cheapest of the three ways it happens, the other
/// two being the ten-second poll and a Nudge. What is asked here is what a read
/// does to the page, and all three do the same one.
function readAgain(): void {
  for (const state of ["hidden", "visible"] as const) {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => state,
    });
    document.dispatchEvent(new Event("visibilitychange", { bubbles: true }));
  }
}

/// The workbench on its own routes, so the Conversation it reads is the one the
/// URL names — and so that opening one is a navigation, which is what it is in
/// the app.
///
/// The client comes back with the render: `invalidateQueries()` on it is
/// exactly what a Nudge does to the page — see `lookAgain` in `src/nudge.ts` —
/// so a test about one needs no fake `EventSource`.
function mount(at = "/") {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real page is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  const history = createMemoryHistory();
  history.set({ value: at });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/" component={Workbench} />
          <Route path="/conversations/:id" component={Workbench} />
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
    client,
  };
}

/// The lists every pane of the workbench is drawn over, in whatever order a page
/// happens to ask for them.
function theWorkbench(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever("/api/ui/abandoned-roadmaps", json([])),
    whenever(`/api/ui/conversations/${OPEN.id}`, json(OPEN)),
    ...answers,
  );
}

/// The frame, which is what says which level a narrow window is showing.
function frame(container: ParentNode): HTMLElement {
  return container.querySelector(".workbench")!;
}

/// Wait for an element to be drawn, by selector.
///
/// Throwing rather than returning null when it is not there yet, because that is
/// what `waitFor` retries on — a callback that hands back null has answered, and
/// the wait ends on the first try with nothing.
function drawn<T extends Element>(
  container: ParentNode,
  selector: string,
): Promise<T> {
  return waitFor(() => {
    const found = container.querySelector<T>(selector);
    if (!found) {
      throw new Error(`nothing matching ${selector} has been drawn`);
    }
    return found;
  });
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
  /// workbench has the root.
  it("reaches the rest of Verkstead from the sidebar", async () => {
    theWorkbench();
    const { container } = mount();
    await waitFor(() => screen.getByText(DRAFTING.branch));

    const elsewhere = container.querySelector(".elsewhere")!;
    expect(
      [...elsewhere.querySelectorAll("a")].map((to) => to.getAttribute("href")),
    ).toEqual(["/repos", "/profiles"]);
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
      "/repos",
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
        branch: "quiet",
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
      "quiet, verkstead, Done",
    ]);
  });

  /// The spinner is motion, and motion is something to be able to turn off.
  it("holds the spinner still where motion is unwelcome", () => {
    expect(stylesheet).toContain(
      "@media (prefers-reduced-motion: reduce) {\n" +
        "  .conversation-row .mark.working {\n" +
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
    fireEvent.click(screen.getByRole("button", { name: "Details →" }));

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

    expect(container.querySelector(".edit-brief")).toBeNull();
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

    fireEvent.click(screen.getByRole("button", { name: "Details →" }));
    fireEvent.input(screen.getByLabelText("Base commit"), {
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

  it("draws the brief inline, as the server rendered it", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    // The server's own HTML, put in the page: the browser has no markdown
    // parser and never needed one.
    const body = container.querySelector(".brief-body")!;
    expect(body.innerHTML).toBe(BRIEF.html);
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

describe("writing the brief", () => {
  /// Open the field, which is what the Edit button is for.
  function edit() {
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
  }

  it("opens the field on what was last written", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    edit();

    // The markdown, not the HTML: the source travels beside the rendering for
    // exactly this, so the field needs no parser to fill itself in.
    expect((screen.getByLabelText("Brief") as HTMLTextAreaElement).value).toBe(
      BRIEF.markdown,
    );
  });

  it("sends what was typed, and reads the conversation back", async () => {
    const written = "# Rate limiting\n\nDecide where the counter lives.\n";
    const fetching = theWorkbench(json("Saved"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    edit();
    fireEvent.input(screen.getByLabelText("Brief"), {
      target: { value: written },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/brief`),
      ).toEqual({ markdown: written }),
    );

    // The field is spent, and what is read is what the server has.
    await waitFor(() => expect(screen.queryByLabelText("Brief")).toBeNull());
  });

  it("keeps what was written when the server refuses it", async () => {
    const fetching = theWorkbench(json("NotDrafting"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    edit();
    fireEvent.input(screen.getByLabelText("Brief"), {
      target: { value: "# Too late\n" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/frozen when grilling started/i));
    // The draft is the only copy of what was written, so it stands.
    expect((screen.getByLabelText("Brief") as HTMLTextAreaElement).value).toBe(
      "# Too late\n",
    );
    expect(fetching).toHaveBeenCalled();
  });

  /// The page reads its Conversation again every ten seconds, on every Nudge and
  /// on every return to it, and a field being typed into has to live through all
  /// of them: a Brief half written is the only copy of it there is, and one that
  /// went every time the world was read again could never be finished.
  it("lives through the page reading the conversation again", async () => {
    // The read that lands while the field is open comes back with something
    // else about the Conversation changed — the branch renamed from another
    // device — so the test can wait for it to have landed rather than sleep
    // until it has. What is asked here is what a read does to the field, and it
    // does the same whether or not anything came back different.
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

    edit();
    fireEvent.input(screen.getByLabelText("Brief"), {
      target: { value: "# Half a thought" },
    });

    standing = RENAMED;
    readAgain();
    await waitFor(() => screen.getByRole("heading", { name: RENAMED.branch }));

    // The read landed, and the field is still open on what was typed into it.
    expect((screen.getByLabelText("Brief") as HTMLTextAreaElement).value).toBe(
      "# Half a thought",
    );
    expect(askedFor(fetching, READING)).toBeGreaterThan(1);
  });

  it("puts the field away again on Cancel", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));

    edit();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByLabelText("Brief")).toBeNull();
  });
});

describe("a conversation's details", () => {
  it("shows the repo the work is in, without offering to change it", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => screen.getByRole("heading", { name: "Details" }));

    const facts = container.querySelector(".conversation-facts")!;
    expect(facts.querySelector(".repo")!.textContent).toBe(OPEN.repo.name);
    expect(facts.querySelector(".path")!.textContent).toBe(OPEN.repo.path);
    expect(facts.querySelector(".state")!.textContent).toBe(OPEN.state);
  });

  it("offers the branch name the server prefilled, and sends a new one", async () => {
    const fetching = theWorkbench(json("Renamed"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Details" }));

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
    await waitFor(() => screen.getByRole("heading", { name: "Details" }));

    fireEvent.input(screen.getByLabelText("Branch"), {
      target: { value: "two..dots" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));

    await waitFor(() => screen.getByText(/will not take that as a branch name/i));
  });

  it("shows the base commit that was recorded, and sends a new one", async () => {
    const fetching = theWorkbench(json("Recorded"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByRole("heading", { name: "Details" }));

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
    await waitFor(() => screen.getByRole("heading", { name: "Details" }));

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

    await waitFor(() => screen.getByRole("heading", { name: "Details" }));

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
describe("a conversation's agent profiles", () => {
  /// A conversation showing neither choice, which is what a freshly started one
  /// looks like.
  const UNCHOSEN: ConversationView = {
    ...OPEN,
    grilling_profile: null,
    implementation_profile: null,
    ready_to_grill: false,
  };

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

  it("shows the two profiles the conversation has chosen", async () => {
    theWorkbench();
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    const grilling = screen.getByLabelText("Grilling") as HTMLSelectElement;
    const implementing = screen.getByLabelText(
      "Implementation",
    ) as HTMLSelectElement;

    // Separate choices, and in the fixture genuinely separate accounts: grill on
    // fable, implement on opus.
    expect(grilling.value).toBe(String(OPEN.grilling_profile!.id));
    expect(implementing.value).toBe(String(OPEN.implementation_profile!.id));
    expect(grilling.value).not.toBe(implementing.value);
  });

  it("sends each choice on its own, to its own role", async () => {
    const fetching = withConversation(UNCHOSEN, json("Chosen"));
    mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));

    fireEvent.change(screen.getByLabelText("Grilling"), {
      target: { value: String(PROFILES[0]!.id) },
    });
    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/grilling-profile`),
      ).toEqual({ profile_id: PROFILES[0]!.id }),
    );

    fireEvent.change(screen.getByLabelText("Implementation"), {
      target: { value: String(PROFILES[1]!.id) },
    });
    await waitFor(() =>
      expect(
        sent(
          fetching,
          `/api/ui/conversations/${OPEN.id}/implementation-profile`,
        ),
      ).toEqual({ profile_id: PROFILES[1]!.id }),
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
      implementation_profile: {
        ...OPEN.implementation_profile!,
        broken: "ConfigMissing",
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
      target: { value: String(PROFILES[0]!.id) },
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
    expect(screen.getByText("add one").getAttribute("href")).toBe("/profiles");
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

    // The Timeline itself is a fetch behind the URL, and the way on to the
    // details is drawn with it.
    await waitFor(() => screen.getByRole("heading", { name: "Brief" }));
    fireEvent.click(screen.getByRole("button", { name: "Details →" }));
    expect(frame(container).dataset.pane).toBe("details");

    fireEvent.click(screen.getByRole("button", { name: "← Timeline" }));
    expect(frame(container).dataset.pane).toBe("timeline");

    fireEvent.click(screen.getByRole("button", { name: "← Conversations" }));
    expect(frame(container).dataset.pane).toBe("conversations");
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
});

/// The other shape the middle pane draws: a conversation that has been started,
/// with a move on its timeline and a worktree to say where the work is going on.
const GRILLING = grilling as ConversationView;
const CAPTURE = capture as Capture;
const TRANSCRIPT = transcript as TranscriptView;

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

/// A session that kept no record of its own conversation, which is what the
/// server says for every stub agent and every backend that writes no log — and
/// what sends the pane to the Capture.
const SAID_NOTHING: TranscriptView = { turns: [], bookkeeping: [] };

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
  /// The design's summary: a line count and the last thing the agent said. An
  /// hour of terminal output does not go in the middle pane.
  it("summarises as a line count and the latest statement", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(output.querySelector(".lines")!.textContent).toBe(
      `${OUTPUT.lines} lines`,
    );
    expect(output.querySelector(".latest")!.textContent).toBe(OUTPUT.latest);

    // Nothing of the Capture itself: it is fetched by the pane that shows
    // it, and only once one is opened.
    expect(output.textContent).not.toContain("Reading the brief");
  });

  it("says so while the session is still running", async () => {
    theGrillingOutput({ running: true });
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(output.classList).toContain("running");
    expect(output.querySelector(".live")!.textContent).toBe("running");
  });

  /// A session that has ended is a conversation with a Capture, not one with
  /// an agent in it — and the fixture is exactly that.
  it("says nothing about running when the session has ended", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(OUTPUT.running).toBe(false);
    expect(output.classList).not.toContain("running");
    expect(output.querySelector(".live")).toBeNull();
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
    // The transcript again, one turn longer: a session still talking.
    const GROWN: TranscriptView = {
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

    let grown = false;
    // Running, so the Transcript is live and read again on every Nudge —
    // which is exactly when a fold has to hold.
    theGrillingOutput(
      { running: true },
      whenever(TRANSCRIPT_OF_IT, () => json(grown ? GROWN : TRANSCRIPT)()),
    );
    const { container, client } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    const reasoning = await drawn<HTMLDetailsElement>(
      container,
      ".details-pane .turn.reasoning details",
    );
    reasoning.open = true;

    grown = true;
    await client.invalidateQueries();

    // The new turn has been drawn under the fold…
    await waitFor(() =>
      expect(container.querySelectorAll(".details-pane .turn")).toHaveLength(
        GROWN.turns.length,
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

  /// The Capture belongs to the pane, and what the conversation is comes
  /// back when it is closed.
  it("goes back to the conversation's details when it is closed", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    await drawn(container, ".details-pane .capture");

    fireEvent.click(await drawn(container, ".details-pane .close-event"));

    await drawn(container, ".details-pane .conversation-facts");
    expect(container.querySelector(".details-pane .capture")).toBeNull();
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

describe("a conversation's worktree", () => {
  it("says where the work is being done", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const path = await drawn(container, ".conversation-worktree .path");

    expect(path.textContent).toBe(GRILLING.worktree!.path);
    expect(path.classList).not.toContain("missing");
  });

  /// A worktree deleted by hand should read as a conversation with a problem
  /// while the human is looking at it, rather than as an obscure failure from
  /// whatever next tries to work in it.
  it("says so when the directory has gone from under it", async () => {
    theWorkbenchWith({
      state: "Grilling",
      ready_to_grill: false,
      worktree: {
        path: "/var/lib/verkstead/worktrees/verkstead-gone",
        missing: true,
      },
    });
    const { container } = mount(`/conversations/${OPEN.id}`);

    const path = await drawn(container, ".conversation-worktree .path");

    expect(path.classList).toContain("missing");
    expect(screen.getByText(/This directory is gone/)).toBeTruthy();
  });

  it("says nothing at all before there is one", async () => {
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);

    // Waited on the details pane itself rather than on the branch, which by now
    // names both the sidebar row and the timeline's heading.
    await drawn(container, ".conversation-profiles");

    expect(OPEN.worktree).toBeNull();
    expect(container.querySelector(".conversation-worktree")).toBeNull();
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

  /// What `position: sticky` means is the stylesheet's, and jsdom lays nothing
  /// out — so the rule itself is what is read, as the panes' own is.
  it("stays in view while the record scrolls past it", () => {
    expect(stylesheet).toContain(".pinned {\n  position: sticky;\n  top: 0;");
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

  /// Nothing is fetched until somebody opens it: reading this is an API call,
  /// and the conversation is read again every ten seconds.
  it("asks GitHub nothing until it is opened", async () => {
    const fetching = theWrapping();
    mount(`/conversations/${WRAPPING.id}`);

    await waitFor(() => screen.getByRole("heading", { name: "Details" }));

    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBe(0);
  });
});
