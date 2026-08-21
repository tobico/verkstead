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
  AgentOutputEvent,
  CommitDiff,
  ConversationAborted,
  ConversationEntry,
  ConversationView,
  GrillingStarted,
  ProfileEntry,
  RepoEntry,
  SetView,
  Submitted,
  TimelineEvent,
  Transcript,
} from "../src/api/types";
import stylesheet from "../src/main.css?raw";
import { Workbench } from "../src/workbench/Workbench";
import { askedFor, json, serving, whenever } from "./serving";
import conversation from "./fixtures/conversation.json" with { type: "json" };
import direction from "./fixtures/conversation-direction.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import answeredSet from "./fixtures/set-answered.json" with { type: "json" };
import answeringSet from "./fixtures/set-answering.json" with { type: "json" };
import transcript from "./fixtures/transcript.json" with { type: "json" };

/// The renderer is a page's own doing and neither Set fixture has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

const SIDEBAR = conversations as ConversationEntry[];
const OPEN = conversation as ConversationView;
const REPOS = repos as RepoEntry[];
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
});

/// The workbench on its own routes, so the Conversation it reads is the one the
/// URL names — and so that opening one is a navigation, which is what it is in
/// the app.
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
  };
}

/// The lists every pane of the workbench is drawn over, in whatever order a page
/// happens to ask for them.
function theWorkbench(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
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

  it("says of each conversation which repo it is in and where it has got to", async () => {
    theWorkbench();
    mount();

    const row = (await waitFor(() => screen.getByText(DRAFTING.branch))).closest(
      "li",
    )!;

    expect(row.querySelector(".repo")!.textContent).toBe(DRAFTING.repo);
    expect(row.querySelector(".state")!.textContent).toBe(DRAFTING.state);
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
const TRANSCRIPT = transcript as Transcript;

/// The session's output on the grilling conversation's timeline.
const OUTPUT = (() => {
  const found = GRILLING.timeline.find((event) => "AgentOutput" in event);
  if (!found || !("AgentOutput" in found)) {
    throw new Error("the fixture should hold a session's output");
  }
  return found.AgentOutput;
})();

/// Where the details pane fetches the whole of it from.
const TRANSCRIPT_OF_IT = `/api/ui/conversations/${GRILLING.id}/transcript/${OUTPUT.id}`;

/// The workbench with the grilling conversation open instead of the drafting
/// one.
function theGrilling(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
    whenever(TRANSCRIPT_OF_IT, json(TRANSCRIPT)),
    ...answers,
  );
}

/// The same, with the session's output Event altered — for the states no
/// fixture holds, a running session being one: a fixture is a payload rather
/// than a moment, and one that said it was running would have a page drawing a
/// spinner over something that has not moved since 2026.
function theGrillingOutput(over: Partial<AgentOutputEvent>) {
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
    whenever(TRANSCRIPT_OF_IT, json(TRANSCRIPT)),
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

    // Nothing of the transcript itself: it is fetched by the pane that shows
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

  /// A session that has ended is a conversation with a transcript, not one with
  /// an agent in it — and the fixture is exactly that.
  it("says nothing about running when the session has ended", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    const output = await drawn(container, ".timeline-event .agent-output");

    expect(OUTPUT.running).toBe(false);
    expect(output.classList).not.toContain("running");
    expect(output.querySelector(".live")).toBeNull();
  });

  it("shows the whole transcript in the details pane, byte for byte", async () => {
    const fetching = theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));

    const shown = await drawn(container, ".details-pane .transcript");

    expect(shown.textContent).toBe(TRANSCRIPT.text);
    expect(askedFor(fetching, TRANSCRIPT_OF_IT)).toBeGreaterThan(0);
  });

  /// The transcript belongs to the pane, and what the conversation is comes
  /// back when it is closed.
  it("goes back to the conversation's details when it is closed", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(await drawn(container, ".agent-output"));
    await drawn(container, ".details-pane .transcript");

    fireEvent.click(await drawn(container, ".details-pane .close-event"));

    await drawn(container, ".details-pane .conversation-facts");
    expect(container.querySelector(".details-pane .transcript")).toBeNull();
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
/// over, with the agent's proposal on it and the choice still open.
const DIRECTING = direction as ConversationView;

/// The workbench with that conversation open.
function theDirecting(
  over: Partial<ConversationView> = {},
  ...answers: Parameters<typeof serving>
) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(
      `/api/ui/conversations/${DIRECTING.id}`,
      json({ ...DIRECTING, ...over }),
    ),
    ...answers,
  );
}

describe("choosing a direction", () => {
  it("draws the chooser once the grilling has proposed wrapping up", async () => {
    theDirecting();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const chooser = await drawn(container, ".direction-chooser");

    expect(DIRECTING.state).toBe("Direction");
    expect(chooser.textContent).toContain("How should this be built?");
  });

  it("does not draw it while the conversation is still grilling", async () => {
    theGrilling();
    const { container } = mount(`/conversations/${GRILLING.id}`);

    // The timeline is up, so the pane has drawn what it was handed.
    await drawn(container, ".timeline");

    expect(GRILLING.state).toBe("Grilling");
    expect(container.querySelector(".direction-chooser")).toBeNull();
  });

  it("shows the agent's reasoning, as the server rendered it", async () => {
    theDirecting();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const proposal = await drawn(container, ".direction-chooser .proposal");

    // Straight out of the fixture: the server has the markdown parser, and this
    // side only puts the HTML in the page.
    expect(DIRECTING.proposal).toBeTruthy();
    expect(proposal.innerHTML).toBe(DIRECTING.proposal!.rationale_html);
    expect(proposal.querySelector("strong")).toBeTruthy();
  });

  it("marks the recommended direction without choosing it", async () => {
    theDirecting();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const marked = await drawn(container, ".directions .recommended");

    expect(DIRECTING.proposal!.direction).toBe("task-list");
    expect(marked.textContent).toContain("Break into a task list");
    expect(marked.querySelector(".mark")!.textContent).toContain("Recommended");

    // Marked and not chosen: nothing is settled until the human presses one.
    expect(DIRECTING.direction).toBeNull();
    expect(container.querySelector(".directions .chosen")).toBeNull();
    expect(container.querySelector(".chosen-note")).toBeNull();
  });

  it("offers a staged roadmap disabled, and says it is coming later", async () => {
    theDirecting();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    await drawn(container, ".direction-chooser");

    const buttons = [
      ...container.querySelectorAll<HTMLButtonElement>(".directions .direction"),
    ];
    const labelled = buttons.map((button) => button.textContent);

    // All three, so the shape of the decision is visible — and only two of them
    // are pressable.
    expect(labelled).toEqual([
      "Implement inline",
      "Break into a task list",
      "Stage a roadmap",
    ]);
    expect(buttons.map((button) => button.disabled)).toEqual([
      false,
      false,
      true,
    ]);

    const roadmap = buttons[2]!.closest("li")!;
    expect(roadmap.textContent).toContain("Arriving in a later stage");
  });

  it("sends the direction the human pressed", async () => {
    const fetching = theDirecting({}, json("Chosen"));
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const buttons = await drawn(container, ".directions");
    fireEvent.click(buttons.querySelectorAll(".direction")[0]!);

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${DIRECTING.id}/direction`),
      ).toEqual({ direction: "inline" }),
    );
  });

  // A conversation still in this chooser with a direction on it is one whose
  // session never started: both runnable directions take it past the chooser
  // altogether.
  it("says what was chosen, where nothing started off it", async () => {
    theDirecting({ direction: "task-list" });
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const note = await drawn(container, ".direction-chooser .chosen-note");

    expect(note.textContent).toContain("break into a task list");
    expect(note.textContent).toContain("Nothing started off it");
    expect(container.querySelector(".directions .chosen")).toBeTruthy();
  });

  it("says that both runnable directions start as soon as they are chosen", async () => {
    theDirecting();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const directions = await drawn(container, ".directions");
    const [inline, taskList] = directions.querySelectorAll("li");

    expect(inline!.textContent).toContain("Starts as soon as you choose it");
    expect(taskList!.textContent).toContain("Starts as soon as you choose it");
  });

  it("draws the handoff the grilling wrote as the document it is", async () => {
    theDirecting();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const handoff = await drawn(container, ".timeline-event > .handoff");

    expect(handoff.querySelector("h2")?.textContent).toBe("Handoff");
    expect(handoff.querySelector(".markdown")?.innerHTML).toContain(
      "<h1>Pausing on a usage limit</h1>",
    );

    // Nothing to press and nothing to edit: it is the agent's account of a
    // conversation that is over.
    expect(handoff.querySelector("button")).toBeNull();
  });

  it("says in words why a direction was refused", async () => {
    const fetching = theDirecting({}, json("NotChoosing"));
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const buttons = await drawn(container, ".directions");
    fireEvent.click(buttons.querySelectorAll(".direction")[0]!);

    await waitFor(() =>
      screen.getByText(/not choosing a direction/i),
    );
    expect(askedFor(fetching, `/api/ui/conversations/${DIRECTING.id}`)).toBeGreaterThan(1);
  });

  it("draws the choice on the timeline as the line it is", async () => {
    const chosen: TimelineEvent[] = [
      ...DIRECTING.timeline,
      { Directed: { id: 99, at: "2026-08-03T11:45:00.000Z", direction: "task-list" } },
    ];
    theDirecting({ timeline: chosen, direction: "task-list" });
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const line = await drawn(container, ".timeline-event > .directed");

    expect(line.textContent).toContain("Chose to break into a task list");
  });

  it("says so where the grilling left no recommendation at all", async () => {
    theDirecting({ proposal: null });
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const chooser = await drawn(container, ".direction-chooser");

    expect(chooser.textContent).toContain("left no recommendation");
    expect(container.querySelector(".directions .recommended")).toBeNull();

    // The buttons are still there: the choice is the human's whether or not
    // anything recommended one.
    expect(container.querySelectorAll(".directions .direction")).toHaveLength(3);
  });
});

describe("disagreeing with a proposal", () => {
  it("draws no chooser while the grilling is still running", async () => {
    // What a refused proposal leaves behind: the set is on the timeline and
    // answered, the conversation is still grilling, and nothing was proposed
    // *in force* — the server only lifts an accepted one onto the view.
    theDirecting({ state: "Grilling", proposal: null });
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    await drawn(container, ".timeline");

    expect(container.querySelector(".direction-chooser")).toBeNull();
    expect(container.querySelector(".directions")).toBeNull();
  });
});

/// The commits on that conversation's timeline: what a session leaves behind
/// besides its output.
const COMMITS = DIRECTING.timeline.flatMap((event) =>
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
const DIFF_OF_IT = `/api/ui/conversations/${DIRECTING.id}/commit/${COMMITS[0]!.id}`;

/// The workbench with that conversation open and its commits' diffs to hand.
function theCommits(...answers: Parameters<typeof serving>) {
  return theDirecting({}, whenever(DIFF_OF_IT, json(COMMIT_DIFF)), ...answers);
}

describe("a commit on the timeline", () => {
  it("summarises as what it was called and how much it moved", async () => {
    theCommits();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

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
    const { container } = mount(`/conversations/${DIRECTING.id}`);

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
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const row = await drawn(container, ".timeline-event > .commit");

    expect(row.querySelectorAll("button")).toHaveLength(0);
    expect(row.textContent).not.toContain("Approve");
  });

  it("shows that commit's diff in the details pane, as the server rendered it", async () => {
    const fetching = theCommits();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

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
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const summary = await drawn(container, ".details-pane .commit-summary");

    expect(summary.textContent).toContain(COMMITS[0]!.subject);
    expect(summary.textContent).toContain(COMMITS[0]!.sha.slice(0, 7));
  });

  it("says so plainly when the commit changed no files", async () => {
    theDirecting({}, whenever(DIFF_OF_IT, json({ diff: null })));
    const { container } = mount(`/conversations/${DIRECTING.id}`);

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
    theDirecting(
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
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    fireEvent.click(await drawn(container, ".timeline-event > .commit"));

    const error = await drawn(container, ".details-pane .error");

    expect(error.textContent).toContain(
      "there is no such commit on that Conversation",
    );
  });

  /// The event that is open is the one the timeline says is open, so a narrow
  /// window walking back out can see which it came from.
  it("marks the commit the details pane is showing", async () => {
    theCommits();
    const { container } = mount(`/conversations/${DIRECTING.id}`);

    const row = await drawn(container, ".timeline-event > .commit");
    expect(row.classList).not.toContain("selected");

    fireEvent.click(row);

    await waitFor(() => expect(row.classList).toContain("selected"));
    expect(row.getAttribute("aria-pressed")).toBe("true");
  });
});
