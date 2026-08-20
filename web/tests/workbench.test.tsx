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
  ConversationEntry,
  ConversationView,
  RepoEntry,
} from "../src/api/types";
import stylesheet from "../src/main.css?raw";
import { Workbench } from "../src/workbench/Workbench";
import { json, serving, whenever } from "./serving";
import conversation from "./fixtures/conversation.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };

const SIDEBAR = conversations as ConversationEntry[];
const OPEN = conversation as ConversationView;
const REPOS = repos as RepoEntry[];

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

/// The two lists every pane of the workbench is drawn over, in whatever order a
/// page happens to ask for them.
function theWorkbench(...answers: Array<() => Promise<Response>>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever(`/api/ui/conversations/${OPEN.id}`, json(OPEN)),
    ...answers,
  );
}

/// The frame, which is what says which level a narrow window is showing.
function frame(container: ParentNode): HTMLElement {
  return container.querySelector(".workbench")!;
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
