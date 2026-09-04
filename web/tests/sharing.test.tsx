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

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";

// The column every page of the app sits in, which is what a share has to sit in
// too — see the test that holds the two together.
import { Shell } from "../src/App";
import type {
  CommitEvent,
  SetReading,
  SetView,
  SharedCommit,
  SharedConversation,
  TimelineEvent,
} from "../src/api/types";
import { drawDiagrams } from "../src/set/diagrams";
import { Share } from "../src/share/Share";
// What the summary under a Brief says of a branch nobody has named.
import { UNNAMED } from "../src/workbench/Brief";
// The header a commit's pane draws itself with: the subject, the repository it
// landed in where that is worth saying, and how much it moved.
import commitStyles from "../src/workbench/Commit.module.css";
// And the cards the record is walked by.
import timeline from "../src/workbench/Timeline.module.css";
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

    // The timeline pane, named as the workbench names it: the branch, and the
    // Repo understated beside it. A share keeps the Repo's name and loses its
    // path — the name is what the record calls the work's repository — so the
    // header of a share reads exactly as the workbench's own does.
    expect(await screen.findByLabelText("Timeline")).toBeTruthy();
    expect(
      screen.getByRole("heading", { name: "usage-limits verkstead" }),
    ).toBeTruthy();

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

    // And neither of the two icons on the Timeline's own header: one offers a
    // reader a publish of somebody else's conversation, and the other a shell
    // in a sandbox on somebody else's machine.
    expect(screen.queryByRole("button", { name: "Share" })).toBeNull();
    expect(
      container.querySelector('button[aria-label^="Terminal"]'),
    ).toBeNull();

    // And nothing anywhere on the page that is a row of a menu or a field to
    // fill in: a shared brief is the document it froze as, whatever state the
    // record says the conversation is in.
    expect(container.querySelector('[role="menuitem"]')).toBeNull();
    expect(container.querySelector("textarea")).toBeNull();
    expect(container.querySelector("input")).toBeNull();
  });

  /// The share sits in the app's own column, which is what makes the two
  /// documents one rendering rather than two that resemble each other.
  ///
  /// The widths every card and the Diff's gutter are built out of — `--measure`,
  /// and `--gutter` and `--bleed` above the breakpoint — are named on `main` in
  /// `styles/base.css`, and the exception a frame of panes makes to the column
  /// is named on the shell's class in `App.module.css`. A share that dropped
  /// either would draw the same components with every padding built on those
  /// names collapsed to nothing, which is what this happened to.
  ///
  /// Asked against the real `Shell` rather than against a class name written out
  /// here: what is being held is that the two elements are the same element, so
  /// the assertion has to fail when the app's shell changes and the share's does
  /// not.
  it("draws its record in the same column the app draws every page in", async () => {
    const app = render(() => (
      <Shell>
        <p>Any page at all.</p>
      </Shell>
    ));
    const shell = app.container.querySelector("main");
    expect(shell).toBeTruthy();
    expect(shell!.className).toBeTruthy();

    cleanup();

    const { container } = render(() => <Share shared={SHARED} />);
    await screen.findByLabelText("Timeline");

    const column = container.querySelector("main");
    expect(column).toBeTruthy();
    expect(column!.className).toBe(shell!.className);

    // And the frame stands inside it, which is what the shell's exception for a
    // page of panes is looking for.
    expect(column!.querySelector("[data-pane]")).toBeTruthy();
  });

  /// Including the file that is carrying nothing: the app wraps every page it
  /// draws, and what a share says when its slot is empty is a page.
  it("draws the column even where it is carrying no conversation", () => {
    const { container } = render(() => <Share shared={null} />);

    const column = container.querySelector("main");
    expect(column).toBeTruthy();
    expect(column!.textContent).toContain("not carrying a conversation");
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
    // The fixture ends on its companion's commit.
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

  /// A share of work nobody has named yet says what will become of the branch
  /// rather than the name Verkstead invented for the row.
  ///
  /// The one place that reading is still drawn: in the workbench a Conversation
  /// with a name still to be chosen is a Draft, and a Draft's Brief opens the
  /// composer rather than this pane.
  it("reports a branch nobody has named as one still to be chosen", async () => {
    const brief = SHARED.conversation.timeline.find(
      (event): event is Extract<TimelineEvent, { Brief: unknown }> =>
        "Brief" in event,
    );
    expect(brief).toBeTruthy();

    const unnamed = holding([brief!]);

    render(() => (
      <Share
        shared={{
          ...unnamed,
          conversation: {
            ...unnamed.conversation,
            state: "Draft",
            branch_named: false,
          },
        }}
      />
    ));

    const details = await screen.findByLabelText("Details");
    await waitFor(() =>
      expect(details.querySelector("h1")?.textContent).toBe("Brief"),
    );

    const branch = [...details.querySelectorAll("div")].find(
      (fact) => fact.querySelector("dt")?.textContent === "Branch",
    );

    expect(branch?.querySelector("dd")?.textContent).toBe(UNNAMED);
    expect(details.textContent).not.toContain(
      SHARED.conversation.branch,
    );
  });

  /// And says nothing under it about the machine the work was done on.
  ///
  /// The Configuration under a Brief is two halves: what the work is against,
  /// which a reader is owed, and where it sits on somebody's disk and which
  /// account wrote it, which is the human's own arrangement. A share is emailed
  /// about and attached to pull requests, so the second half is neither drawn
  /// here nor carried in the file — see `shared` in
  /// `crates/render/src/sharing.rs`.
  it("says nothing about the machine the work was done on", async () => {
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

    const facts = () =>
      [...details.querySelectorAll("dt")].map((term) => term.textContent);

    // What the work is against, which is the whole of what a reader is owed
    // about how it was set up.
    expect(facts()).toContain("Repo");
    expect(facts()).toContain("Branch");
    expect(facts()).toContain("Base");

    // And nothing about where any of it lives or who ran it.
    expect(facts()).not.toContain("Worktree");
    expect(facts()).not.toContain("Grilling");
    expect(facts()).not.toContain("Implementation");
    expect(facts()).not.toContain("Review");

    // Not by name, and not by anything a path or an account would have put on
    // the page.
    expect(details.textContent).not.toContain("/var/lib/verkstead");
    expect(details.textContent).not.toContain("/srv/");
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

/// The commits the fixture's record carries, in its own order.
const COMMITS: CommitEvent[] = SHARED.conversation.timeline.flatMap((event) =>
  "Commit" in event ? [event.Commit] : [],
);

/// One commit's pane as the server renders one: the Message, and a diff with a
/// fold per file.
///
/// Built out of a Set's own attached Diff rather than written by hand, exactly
/// as the workbench's own commit tests build one: it is the same `DiffView` off
/// the same server-side renderer, which is the whole reason a commit needs no
/// diff machinery of its own. The fixture's commits carry no diff — the Repos
/// behind them are paths nothing is at, so git had nothing to say — and what a
/// read one draws is what these are for.
const PANE: SharedCommit["pane"] = {
  summary: "<p>A bucket per account.</p>\n",
  diagrams: false,
  diff: ALONGSIDE.diff[0]!.diff,
};

/// A share opened on one of its commits, carrying `carried` as that commit's
/// pane.
///
/// The record cut down to the one commit, so that what the share opens on is
/// the commit under test — a share opens at the end of its own record, which is
/// how the workbench opens a Conversation.
function showingCommit(
  carried: Omit<SharedCommit, "id">,
  which = 0,
): SharedConversation {
  const landed = COMMITS[which]!;
  const event = SHARED.conversation.timeline.find(
    (event): event is Extract<TimelineEvent, { Commit: unknown }> =>
      "Commit" in event && event.Commit.id === landed.id,
  );
  expect(event).toBeTruthy();

  return { ...holding([event!]), commits: [{ ...carried, id: landed.id }] };
}

describe("a commit in a share", () => {
  /// The whole pane out of the file, drawn by the component the workbench opens
  /// a commit with — there is nothing to fetch and nothing that tries.
  it("opens as the message and the whole diff it landed", async () => {
    const details = await opened(showingCommit({ pane: PANE, held: true }));
    const commit = COMMITS[0]!;

    expect(details.querySelector("h1")?.textContent).toBe("Commit");

    // What the commit was, off the record's own card: the subject, the hash
    // shortened for reading, and how much of the repository it moved.
    expect(details.querySelector(`.${commitStyles.subject}`)?.textContent).toBe(
      commit.subject,
    );
    expect(details.querySelector(`.${commitStyles.sha}`)?.textContent).toBe(
      commit.sha.slice(0, 7),
    );
    expect(details.querySelector(`.${commitStyles.added}`)?.textContent).toBe(
      `+${commit.insertions}`,
    );

    // The Message it wrote about itself, above the diff.
    expect(details.querySelector("#commit-message")).toBeTruthy();
    expect(details.textContent).toContain("A bucket per account.");

    // And the diff itself, folded per file, with the way through it down the
    // margin.
    expect(details.querySelector("section#commit-diff")).toBeTruthy();
    expect(
      details.querySelectorAll("section#commit-diff [id^='diff-']").length,
    ).toBe(PANE.diff!.paths.length);
    expect(details.textContent).toContain("Diff");
  });

  /// Which repository a commit landed in, where that is not the conversation's
  /// own — the same label the card carries, drawn by the same pane. The
  /// fixture's last commit is the companion's and its first is not.
  it("labels a commit out of a companion repo, and only that one", async () => {
    const companion = COMMITS.findIndex((commit) => commit.repo !== null);
    expect(companion).toBeGreaterThan(-1);

    const labelled = await opened(
      showingCommit({ pane: PANE, held: true }, companion),
    );
    expect(labelled.querySelector(`.${commitStyles.repo}`)?.textContent).toBe(
      COMMITS[companion]!.repo,
    );

    cleanup();

    const own = COMMITS.findIndex((commit) => commit.repo === null);
    const unlabelled = await opened(showingCommit({ pane: PANE, held: true }, own));
    expect(unlabelled.querySelector(`.${commitStyles.repo}`)).toBeNull();
  });

  /// A commit git no longer had when the share was taken. What the store kept
  /// still reads — the card, and the commit's own account of itself — and the
  /// pane says where the diff went rather than leaving the reader to wonder.
  it("says where the diff went for a commit the repository had lost", async () => {
    const details = await opened(
      showingCommit({ pane: { ...PANE, diff: null }, held: false }),
    );

    expect(details.textContent).toContain("no longer had this commit");
    expect(details.textContent).toContain("A bucket per account.");

    // And it is not confused with the other thing an absent diff can mean.
    expect(details.textContent).not.toContain("changed no files");
    expect(details.querySelector("section#commit-diff")).toBeNull();
  });

  /// The other thing an absent diff can mean: a merge, or an empty commit. The
  /// pane says what the workbench's says, because it is the same pane.
  it("says a commit that changed nothing changed nothing", async () => {
    const details = await opened(
      showingCommit({ pane: { ...PANE, diff: null }, held: true }),
    );

    expect(details.textContent).toContain("changed no files");
    expect(details.textContent).not.toContain("no longer had this commit");
  });

  /// A record whose commit has no pane beside it, which is a file written by
  /// something that disagrees with this build: the pane says so rather than
  /// drawing an empty document.
  it("says so where the file is carrying no pane for it", async () => {
    const landed = COMMITS[0]!;
    const event = SHARED.conversation.timeline.find(
      (event): event is Extract<TimelineEvent, { Commit: unknown }> =>
        "Commit" in event && event.Commit.id === landed.id,
    );

    const details = await opened({ ...holding([event!]), commits: [] });

    expect(details.textContent).toContain("not carrying the pane");
  });

  /// A branch of any length is read by walking it, which is the thing a file
  /// carrying every diff at once exists for: each card opens its own pane, and
  /// the one before it goes.
  ///
  /// Worth its own test because nothing here is keyed — opening a second commit
  /// assigns into the component the first one built, message, diff and all.
  it("walks from one commit on the record to the next", async () => {
    const { container } = render(() => (
      <Share
        shared={{
          ...SHARED,
          commits: COMMITS.map((commit, n) => ({
            id: commit.id,
            pane: { ...PANE, summary: `<p>What step ${n} did.</p>\n` },
            held: true,
          })),
        }}
      />
    ));

    const details = await screen.findByLabelText("Details");
    const cards = container.querySelectorAll(
      `.${timeline.timelineEvent} > .${timeline.commit}`,
    );
    expect(cards.length).toBe(COMMITS.length);

    for (const [n, commit] of COMMITS.entries()) {
      fireEvent.click(cards[n]!);

      await waitFor(() =>
        expect(
          details.querySelector(`.${commitStyles.subject}`)?.textContent,
        ).toBe(commit.subject),
      );

      // The whole pane follows, not just the header: this commit's own account
      // of itself, and a diff to read under it.
      expect(details.textContent).toContain(`What step ${n} did.`);
      expect(details.querySelector("section#commit-diff")).toBeTruthy();
    }
  });

  /// A patch is read, not answered. The word-wrap switch beside the Diff
  /// heading stays — it is how this reader wants a diff drawn on their own
  /// device — and there is nothing else to press.
  it("offers the way back and the wrap switch, and nothing else", async () => {
    const details = await opened(showingCommit({ pane: PANE, held: true }));

    expect(screen.getByLabelText("Word wrap")).toBeTruthy();
    expect(details.querySelector('[role="menuitem"]')).toBeNull();
    expect(details.querySelector("textarea")).toBeNull();
    expect(details.textContent).not.toContain("Close");
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
