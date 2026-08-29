//! A shared conversation, drawn from the file it travels in.
//!
//! `tests/fixtures/share.json` is a golden fixture like the workbench's: `cargo
//! test` asks the real endpoint and writes the file, so what these assertions
//! read is the record the server actually composes. Which is also what makes
//! this worth asserting on this side — the curation is the server's and is
//! proved over there, and what is proved here is that the page drawn from it
//! reads as a record and offers nothing to do about it.
//!
//! Nothing is stubbed, and that is the subject rather than a convenience. A
//! share opens off a disk with no server anywhere: `fetch` is left exactly as
//! the environment has it, so a component that reached for the network would
//! fail here the way it would fail in the recipient's browser.

import { cleanup, render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";

import type {
  SetReading,
  SetView,
  SharedConversation,
  TimelineEvent,
} from "../src/api/types";
import { drawDiagrams } from "../src/set/diagrams";
import { Share } from "../src/share/Share";
import alongside from "./fixtures/set-alongside.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };
import followingUp from "./fixtures/set-following-up.json" with { type: "json" };
import locked from "./fixtures/set-locked.json" with { type: "json" };
import shared from "./fixtures/share.json" with { type: "json" };

const SHARED = shared as unknown as SharedConversation;

/// The Set fixtures the standalone page is tested from, which stand in here for
/// the sheets a share carries: what has to be drawn is every part of a Set, and
/// the share fixture's own is whichever one its Conversation ended on.
const readable = (reading: unknown): SetView =>
  (reading as SetReading as Extract<SetReading, { Set: unknown }>).Set;

const ANSWERED = readable(answered);
const ANSWERING = readable(answering);
const LOCKED = readable(locked);
const ALONGSIDE = readable(alongside);
const FOLLOWING_UP = readable(followingUp);

/// What `import("mermaid")` means inside a share, which is the module the share
/// build aliases the package to. Imported through a name so that the alias, and
/// not the package, is what these tests reach.
const carried = () => import("../src/share/mermaid");

/// The same record with its timeline replaced, for the tests about one shape of
/// it.
function holding(timeline: TimelineEvent[]): SharedConversation {
  return {
    ...SHARED,
    conversation: { ...SHARED.conversation, timeline },
  };
}

/// The record this file's fixture carries, by kind.
function kinds(shared: SharedConversation): string[] {
  return shared.conversation.timeline.map(
    (event) => Object.keys(event)[0] ?? "",
  );
}

describe("a shared conversation", () => {
  it("draws the record it was given, titled for its branch", async () => {
    render(() => <Share shared={SHARED} />);

    // The timeline pane, named as the workbench names it.
    expect(await screen.findByLabelText("Timeline")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "usage-limits" })).toBeTruthy();

    // And what is on it: the brief, and every commit the branch carried.
    expect(screen.getByText("Brief")).toBeTruthy();
    expect(
      screen.getByText("feat: read the account's own limit error"),
    ).toBeTruthy();
  });

  /// The curation is the server's — see `crates/server/tests/sharing.rs` — and
  /// this is the half of it a reader sees: what the fixture carries is what the
  /// page has to draw, and the kinds that never board leave no trace.
  it("carries the asks and the commits, and nothing of the sessions", () => {
    const held = kinds(SHARED);

    expect(held).toContain("Brief");
    expect(held).toContain("QuestionSet");
    expect(held).toContain("Commit");

    for (const left of ["AgentOutput", "Handoff", "Notice", "PullRequest"]) {
      expect(held).not.toContain(left);
    }

    render(() => <Share shared={SHARED} />);

    // Nothing marks the gap. The record between the brief and the first commit
    // held a session's output and a handoff, and the page says neither.
    expect(screen.queryByText(/handoff/i)).toBeNull();
    expect(screen.queryByText(/output/i)).toBeNull();
  });

  /// The whole of what a share is: a record to read. Every control the
  /// workbench hangs off a conversation is about doing something to it, and
  /// there is nobody on the other end of this file to do it.
  ///
  /// Asked of a record that says every one of them applies. The server clears
  /// those fields on the way out — `crates/server/tests/sharing.rs` is where
  /// that is proved — and this is the other half of the same guarantee: the
  /// page would not draw them even if a record arrived saying it could.
  it("offers nothing to press but the cards themselves", async () => {
    const { container } = render(() => (
      <Share
        shared={{
          ...SHARED,
          conversation: {
            ...SHARED.conversation,
            state: "Draft",
            ready_to_grill: true,
            ready_to_resume: true,
            ready_to_stop: true,
            working: true,
          },
        }}
      />
    ));
    await screen.findByLabelText("Timeline");

    // The ⋯ that carries stop, steer and close, and the two blocks under the
    // record that say what happens next.
    expect(
      screen.queryByRole("button", { name: "Conversation actions" }),
    ).toBeNull();
    expect(screen.queryByText("Start work")).toBeNull();
    expect(screen.queryByText("Resume")).toBeNull();

    // And nothing anywhere on the page that is a row of a menu or a field to
    // fill in: a shared brief is the document it froze as, whatever state the
    // record says the conversation is in.
    expect(container.querySelector('[role="menuitem"]')).toBeNull();
    expect(container.querySelector("textarea")).toBeNull();
    expect(container.querySelector("input")).toBeNull();
  });

  /// There is no list to pick from and no way back to one: a share is one piece
  /// of work and nothing around it.
  it("has no conversations pane and no way back to one", async () => {
    render(() => <Share shared={SHARED} />);
    await screen.findByLabelText("Timeline");

    expect(screen.queryByLabelText("Conversations")).toBeNull();
    expect(screen.queryByText("← Conversations")).toBeNull();
  });

  /// Opening a share lands on the end of the record, which is where the work
  /// got to — the same landing the workbench makes when a conversation is
  /// opened.
  it("opens on the last thing that has a pane behind it", async () => {
    render(() => <Share shared={SHARED} />);

    const details = await screen.findByLabelText("Details");
    // The fixture ends on its companion's commit, and a commit's pane is the
    // one this build's share does not carry the diff for yet.
    await waitFor(() =>
      expect(details.querySelector("h1")?.textContent).toBe("Commit"),
    );
  });

  /// A brief opens as the document it is, out of the record the file carries —
  /// no fetch, because there is nowhere to fetch from.
  it("opens the brief as the document it froze as", async () => {
    const brief = SHARED.conversation.timeline.find(
      (event): event is Extract<TimelineEvent, { Brief: unknown }> =>
        "Brief" in event,
    );
    expect(brief).toBeTruthy();

    render(() => <Share shared={holding([brief!])} />);

    const details = await screen.findByLabelText("Details");
    await waitFor(() =>
      expect(details.querySelector("h1")?.textContent).toBe("Brief"),
    );
    expect(
      details.textContent?.includes("Pausing when an account runs out of window"),
    ).toBe(true);
  });

  /// The template before anything is written into it, and any file whose slot
  /// somebody has emptied: a page that says what it is holding rather than a
  /// blank one.
  it("says so where it is carrying no conversation", () => {
    render(() => <Share shared={null} />);

    expect(screen.getByText(/not carrying a conversation/)).toBeTruthy();
  });
});

/// A share whose one Question Set carries `sheet`, opened on that Set.
///
/// The fixture's own Set is the thin one its Conversation happened to end on,
/// and what a shared sheet has to draw is every part of one — so the Set
/// fixtures the standalone page is tested from stand in here, put on the record
/// under the id its row opens.
function showing(sheet: SetView): SharedConversation {
  const asked = SHARED.conversation.timeline.find(
    (event): event is Extract<TimelineEvent, { QuestionSet: unknown }> =>
      "QuestionSet" in event,
  );
  expect(asked).toBeTruthy();

  return {
    ...holding([asked!]),
    sets: [{ ...sheet, id: asked!.QuestionSet.set_id }],
  };
}

/// The details pane of a share, once whatever it opened on has drawn.
async function opened(shared: SharedConversation): Promise<HTMLElement> {
  render(() => <Share shared={shared} />);

  const details = await screen.findByLabelText("Details");
  await waitFor(() => expect(details.querySelector("h1")).toBeTruthy());

  return details;
}

describe("a question set in a share", () => {
  /// The whole sheet out of the file, drawn by the component the workbench
  /// opens a Set with — there is nothing to fetch and nothing that tries.
  it("opens as the whole document it was asked as", async () => {
    const details = await opened(showing(ANSWERED));

    // Its own heading, its Preface, and its Questions with every Option that
    // was offered — what was turned down is half of what a decision was.
    expect(details.querySelector("h1")?.textContent).toBe(
      "Rate limiting for the public API",
    );
    expect(details.textContent).toContain("has no rate limit");
    expect(details.textContent).toContain("In-process, per instance");
    expect(details.textContent).toContain("shared across instances");

    // What was decided, and the word about the Set as a whole.
    expect(details.textContent).toContain("and document them in the changelog");
    expect(details.textContent).toContain(
      "Do the in-process one first; we can move it later.",
    );
  });

  /// The Diff a Set was asked over is the evidence it was decided against, so
  /// it boards with the sheet and is read here exactly as it is read there.
  it("carries the worktree diff it was decided against", async () => {
    const details = await opened(showing(ALONGSIDE));

    expect(details.querySelector("section#diff")).toBeTruthy();
    expect(details.querySelectorAll("[id^='diff-']").length).toBeGreaterThan(0);
  });

  /// And whatever the agent closed with, which sits where the human's own
  /// closing word would be on a Set that got one.
  it("carries the postscript the agent closed with", async () => {
    const details = await opened(showing(FOLLOWING_UP));

    expect(details.textContent).toContain(
      "Both are pushed as separate commits",
    );
  });

  /// The whole of what a read-only sheet is: the record, whatever the Set's
  /// standing was when the share was taken. A form on a page with no server
  /// behind it would be an offer nobody could take up.
  it.each([
    ["answered", ANSWERED],
    ["still open", ANSWERING],
    ["locked unanswered", LOCKED],
  ])("draws a %s set as a record with no form", async (_how, sheet) => {
    const details = await opened(showing(sheet));

    // The Questions are there to read.
    expect(details.querySelector("h2#questions")).toBeTruthy();

    // And nothing to answer them with: no Option to pick, no box to type in
    // and no submit. Nor the menu behind the standing badge, whose one row is
    // the only irreversible act in the whole UI.
    //
    // The word-wrap switch beside the Diff heading is not one of these and
    // stays: it is how the reader wants a patch drawn on their own device, and
    // it does nothing to the record at all.
    expect(details.querySelector('input[type="radio"]')).toBeNull();
    expect(details.querySelector('input[type="text"]')).toBeNull();
    expect(details.querySelector("textarea")).toBeNull();
    expect(details.querySelector('[role="menuitem"]')).toBeNull();
    expect(screen.queryByRole("button", { name: /submit/i })).toBeNull();
    expect(screen.queryByText("Lock unanswered")).toBeNull();
  });

  /// A Set nobody had got to yet says so, rather than reading as a decision
  /// whose Answers failed to arrive — and it is not confused with a Set that
  /// was closed for good, which is the opposite thing.
  it("says a still-open set was still open, not that it was locked", async () => {
    const open = await opened(showing(ANSWERING));

    expect(open.textContent).toContain("still open when this record was made");
    expect(open.textContent).not.toContain("locked unanswered");

    // The badge stays — how it stood is part of what was shared — and it is a
    // word rather than something to press.
    expect(screen.getByText("agent waiting").tagName).toBe("SPAN");

    cleanup();

    const locked = await opened(showing(LOCKED));
    expect(locked.textContent).toContain("This Set was locked unanswered");
    expect(locked.textContent).not.toContain("still open when this record");
  });
});

describe("the diagram renderer a share carries", () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).verksteadMermaid;
  });

  /// The seam the share build is aliased across: `import("mermaid")` in a share
  /// means the library the document is already holding, and the drawing reaches
  /// it the way it reaches any other bundle.
  it("draws from the library the document is holding", async () => {
    const drawn: string[] = [];

    (window as unknown as Record<string, unknown>).verksteadMermaid = {
      initialize: () => {},
      render: (_id: string, text: string) => {
        drawn.push(text);
        return Promise.resolve({ svg: "<svg></svg>" });
      },
    };

    document.body.innerHTML = `<pre class="mermaid">graph LR;\n  a--&gt;b;\n</pre>`;

    drawDiagrams({ bundle: () => carried().then((module) => module.default) });

    await waitFor(() =>
      expect(document.querySelectorAll("div.diagram")).toHaveLength(1),
    );
    expect(drawn).toEqual(["graph LR;\n  a-->b;\n"]);

    document.body.innerHTML = "";
  });

  /// And a share that is not carrying one leaves every Diagram as the source
  /// the agent wrote, which is a readable page rather than a broken one. It is
  /// unreachable — a Set with no Diagram never asks — so this is what a build
  /// that got the slot wrong degrades to.
  it("refuses where the document is holding none", async () => {
    document.body.innerHTML = `<pre class="mermaid">graph LR;\n  a--&gt;b;\n</pre>`;

    drawDiagrams({ bundle: () => carried().then((module) => module.default) });

    await waitFor(() => expect(document.querySelector("pre.mermaid")).toBeTruthy());
    expect(document.querySelector("div.diagram")).toBeNull();

    document.body.innerHTML = "";
  });
});
