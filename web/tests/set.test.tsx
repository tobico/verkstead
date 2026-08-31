//! The Set page in its reading half: the Set's own material, and — once it has
//! settled — the record of what was decided.
//!
//! Every Set here comes out of `tests/fixtures/`, which `cargo test` writes from
//! the real `/api/ui/sets/{id}`: the markdown, the flattened Options and the
//! Diagram flag are the server's own answers rather than a mock's agreement with
//! this file. The three fixtures are one Set in each of its three standings,
//! plus a fourth carrying a Diagram.
//!
//! The answering form is its own task and is not drawn yet. The Diff and the table
//! of contents are drawn here too, and asked about in `diff.test.tsx` and
//! `contents.test.tsx` — this file is the record itself.

import { cleanup, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Response as Decided } from "../src/api/types";
// The page's own vocabulary, and the four components that keep names of their
// own beside it.
import app from "../src/App.module.css";
// The card a Preface and a commit's Message are both drawn as.
import card from "../src/Card.module.css";
import contents from "../src/set/Contents.module.css";
import closing from "../src/set/Postscript.module.css";
import sheet from "../src/set/Sheet.module.css";
import standing from "../src/set/Standing.module.css";
import illegible from "../src/set/Unreadable.module.css";
import paneHead from "../src/workbench/PaneHead.module.css";
import {
  mount,
  reading,
  texts,
  withHeading,
  withPostscript,
  withTable,
  unreadably,
} from "./reading";
import { json, readable, reads, serving, unreadable } from "./serving";
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };
import locked from "./fixtures/set-locked.json" with { type: "json" };
import diagram from "./fixtures/set-diagram.json" with { type: "json" };
import unreadableSet from "./fixtures/set-unreadable.json" with { type: "json" };

/// The renderer, which is a page's own doing rather than this page's: what is
/// asked here is whether it was reached for at all, and never what it drew —
/// that is `diagrams.test.ts`.
const drawing = vi.hoisted(() => vi.fn(() => () => {}));
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: drawing }));

const WAITING = readable(answering);
const ANSWERED = readable(answered);
const LOCKED = readable(locked);
const DIAGRAMMED = readable(diagram);

/// And the one no standing at all: a stored body this build cannot read, which
/// is a record to be looked at rather than a Set to be answered.
const UNREADABLE = unreadable(unreadableSet);

/// When the two settled fixtures were settled, as the page words it — pinned
/// by the test that writes them, and pinned far enough back that the wording
/// is the date, which no run's own clock can move.
const SETTLED = "2025-08-03";

/// The same settling to the minute — the tooltip behind the words.
const SETTLED_STAMP = "2025-08-03 09:07 UTC";

/// The label at the head of one question, found among the Questions rather than
/// anywhere on the page: the table of contents lists a Question by its label too,
/// so `Q3` names two things in the document and only one of them is a question.
function named(page: ParentNode, label: string): HTMLElement {
  const found = [...page.querySelectorAll<HTMLElement>(`.${sheet.ask} .${sheet.text} > .${sheet.label}`)].find(
    (head) => head.textContent === label,
  );
  expect(found, `expected the question ${label}`).toBeTruthy();
  return found as HTMLElement;
}

/// The row drawing this Option, found by words that are only in it, so a test can
/// ask what that Option was marked with.
///
/// The words may be inside the markup the server rendered — a code span, an
/// emphasis — so what is matched is whichever element holds them and the row is
/// found from there.
function optionRow(text: string | RegExp): HTMLElement {
  const row = screen.getByText(text, { exact: false }).closest(`li.${sheet.option}`);
  expect(row, `expected the Option ${text} in a row of its own`).toBeTruthy();
  return row as HTMLElement;
}

afterEach(() => {
  vi.unstubAllGlobals();
  drawing.mockClear();
});

describe("reading a Set", () => {
  it("asks the server for the Set the URL names", async () => {
    const fetching = serving(json(reads(WAITING)));
    mount("7");

    await waitFor(() => screen.getByText(WAITING.title));
    expect(fetching).toHaveBeenCalledWith(
      "/api/ui/sets/7",
      expect.anything(),
    );
  });

  it("shows where the ask came from", async () => {
    const page = await reading(WAITING);

    expect(page.querySelector("h1")!.textContent).toBe(WAITING.title);
    expect(page.querySelector(`.${sheet.meta} .${sheet.project}`)!.textContent).toBe(
      WAITING.project,
    );
    expect(page.querySelector(`.${sheet.meta} .${sheet.branch}`)!.textContent).toBe(
      WAITING.branch,
    );
  });

  it("shows no provenance at all for an ask from outside a repo", async () => {
    const page = await reading({ ...WAITING, project: null, branch: null });

    expect(page.querySelector("h1")!.textContent).toBe(WAITING.title);
    // The line itself stands — how the Set stands lives at its far end — but
    // it says nothing about where the ask came from.
    expect(page.querySelector(`.${sheet.meta} .${sheet.project}`)).toBeNull();
    expect(page.querySelector(`.${sheet.meta} .${sheet.branch}`)).toBeNull();
  });

  it("puts the Preface in as the server rendered it", async () => {
    const page = await reading(WAITING);

    const preface = page.querySelector(`section#preface .${card.cardBody}`)!;
    expect(preface.className).toContain("markdown");
    expect(preface.innerHTML).toContain("<code>POST /v1/messages</code>");
    expect(preface.innerHTML).toContain(
      "<li>one client sent 40k requests in a minute</li>",
    );
  });

  /// Drawn as the shared card, which is the same component a commit's Message
  /// is drawn as: the heading outside the box and the markdown inside it. Two
  /// copies of one box in two stylesheets is how the two came to look unalike,
  /// so what is asked here is that the Preface is that component and not a copy
  /// of it.
  it("draws the Preface as the shared card, headed outside the box", async () => {
    const page = await reading(WAITING);

    const section = page.querySelector(`section#preface.${card.card}`)!;

    expect(section.querySelector("h2")!.textContent).toBe("Preface");
    expect(section.querySelector("h2")!.nextElementSibling).toBe(
      section.querySelector(`.${card.cardBody}`),
    );
  });

  it("shows no Preface section for a Set with no Preface", async () => {
    const page = await reading({ ...WAITING, preface_html: null });

    expect(page.querySelector("#preface")).toBeNull();
  });

  it("draws every Question and Sub-question in the order they were asked", async () => {
    const page = await reading(WAITING);

    expect(texts(page, `.${sheet.ask} .${sheet.label}`)).toEqual([
      "Q1",
      "Q2",
      "Q2a",
      "Q2b",
      "Q3",
    ]);
    // One level of nesting, and the Sub-questions under the Question that asked
    // them.
    const nested = page.querySelector(`#q2 .${sheet.subquestions}`)!;
    expect(texts(nested as HTMLElement, `.${sheet.ask} .${sheet.label}`)).toEqual(["Q2a", "Q2b"]);
  });

  it("offers every Option of every question", async () => {
    const page = await reading(DIAGRAMMED);

    expect(texts(page, `.${sheet.option} .${sheet.optionText}`)).toEqual([
      "In-process, per instance.",
      "In Redis, shared across instances.",
      "A bare 429.",
      "A 429 plus RateLimit headers.",
      "The exact number of seconds.",
      "A rounded number.",
    ]);
    // Selecting is by number, so every row carries the Option's own.
    expect(texts(page, `.${sheet.option} .${sheet.n}`)).toEqual(["1", "2", "1", "2", "1", "2"]);
  });

  it("offers nothing on a question that has no Options", async () => {
    const page = await reading(WAITING);

    // Q2b and Q3 offer nothing to choose between, so they get no list of
    // Options at all — just their text.
    for (const bare of ["Q2b", "Q3"]) {
      const ask = named(page, bare).closest(`.${sheet.ask}`)!;
      expect(
        ask.querySelector(`.${sheet.options}`),
        `${bare} offers no Options, so it should have no list of them`,
      ).toBeNull();
    }
  });

  it("puts a Question's markdown in as the server rendered it", async () => {
    const page = await reading(WAITING);
    const markup = page.innerHTML;

    expect(markup).toContain("<li>in-process, per instance</li>");
    expect(markup).toContain("<code>redis</code>");
    expect(markup).toContain("<td>Retry-After</td>");
    expect(
      markup,
      "nothing may reach the page as raw markup",
    ).not.toContain("| --- |");

    // The fenced block arrives as one, already highlighted: the browser gets
    // neither a markdown parser nor a syntax highlighter, so the tokens are the
    // server's own.
    const fenced = page.querySelector("#q2 .markdown pre")!;
    expect(fenced.textContent).toContain("fn allowance() -> u32 { 600 }");
    expect(fenced.querySelector(".tok-storage")!.textContent).toBe("fn");
  });

  it("keeps a Question's label at the head of its rendered text", async () => {
    const page = await reading(WAITING);

    for (const label of ["Q1", "Q2a"]) {
      const text = named(page, label).closest(`.${sheet.text}`)!;
      expect(text.firstElementChild!.className).toBe(sheet.label);
      expect(text.lastElementChild!.className).toContain("markdown");
    }
  });

  it("puts an Option's markdown in as the server rendered it", async () => {
    const page = await reading(WAITING);

    expect(optionRow("Counter::local").innerHTML).toContain(
      "<code>Counter::local</code>",
    );
    expect(page.innerHTML).toContain("<strong>Redis</strong>");
    // An Option is one line beside its number, so the server flattened
    // anything blockier on the way here.
    expect(page.innerHTML).not.toContain("<li>no headers</li>");
    expect(optionRow("A bare 429.").textContent).toContain("no headers");
  });

  it("marks the Recommendation, and only the one", async () => {
    const page = await reading(WAITING);

    expect(page.querySelectorAll(`.${sheet.option} .${sheet.star}`)).toHaveLength(1);
    // The emphasis the agent put on it, rather than the word anywhere: `redis`
    // is also a code span in Q1's own text.
    expect(optionRow(/^Redis$/).className).toBe(`${sheet.option} ${sheet.recommended}`);
    expect(optionRow("Counter::local").className).toBe(sheet.option);
  });

  it("names and anchors the Questions, and every Question in them", async () => {
    const page = await reading(WAITING);

    const heading = page.querySelector("h2#questions")!;
    expect(heading.className).toBe(app.sectionHeading);
    expect(heading.textContent).toBe("Questions");

    for (const id of ["q1", "q2", "q3"]) {
      expect(page.querySelector(`#${id}`), `expected #${id}`).toBeTruthy();
    }
    expect(
      page.querySelector("#q2a"),
      "a Sub-question scrolls with its parent and needs no anchor of its own",
    ).toBeNull();

    // The anchor sits on the Question it names.
    expect(page.querySelector("#q3")!.textContent).toContain(
      "Anything I should know before starting?",
    );
  });

  /// However it was refused, and in the server's own words: the pane draws
  /// what it was told rather than a sentence of its own. A Set that is not
  /// there reads like any other read that did not land — the "No such Set."
  /// of its own went with the page it was the whole of.
  it("shows the server's own wording when the Set cannot be read", async () => {
    serving(json({ error: "there is no Question Set 404" }, 404));
    mount("404");

    await waitFor(() => screen.getByText(/there is no Question Set 404/));

    cleanup();
    serving(json({ error: "the Question Set could not be read" }, 500));
    mount();

    await waitFor(() => screen.getByText(/the Question Set could not be read/));
  });
});

describe("the record of a settled Set", () => {
  it("shows what was chosen apart from what was recommended", async () => {
    await reading(ANSWERED);

    // Q1: Option 1 was chosen, and it is Option 2 that carries the ★. The class
    // is what the outline hangs off and the word is what a reader who cannot see
    // one is told; both have to be on it, and neither on the other Option.
    const chosen = optionRow("Counter::local");
    expect(chosen.className).toBe(`${sheet.option} ${sheet.chosen}`);
    expect(chosen.querySelector(`.${sheet.chose}`)!.textContent).toBe("chosen");
    expect(chosen.querySelector(`.${sheet.star}`)).toBeNull();

    const recommended = optionRow(/^Redis$/);
    expect(recommended.className).toBe(`${sheet.option} ${sheet.recommended}`);
    expect(recommended.querySelector(`.${sheet.star}`)).toBeTruthy();
    expect(
      recommended.querySelector(`.${sheet.chose}`),
      "the Recommendation was not taken, and the page must not read as if it was",
    ).toBeNull();

    // Every Option is kept, chosen or not: what was turned down is half of what
    // the decision was.
    expect(optionRow("A bare 429.")).toBeTruthy();
    expect(optionRow("The exact number of seconds.")).toBeTruthy();
  });

  it("shows what was written", async () => {
    const page = await reading(ANSWERED);

    expect(texts(page, `.${sheet.answerText}`)).toEqual([
      "Your thoughtsand document them in the changelog",
      "Your answerkeep them short",
    ]);
  });

  it("reads a Heading as the words over its Sub-questions and never as one left open", async () => {
    const page = await reading(withHeading(ANSWERED));

    const heading = page.querySelector(`.${sheet.ask}.${sheet.heading}`)!;
    expect(heading, "expected the Heading drawn").toBeTruthy();
    expect(heading.querySelector(`.${sheet.label}`)!.textContent).toBe("Q2");

    // The fixture's Response still carries an entry naming Q2 — it was answered
    // before Headings existed. Nothing is drawn from it: a Question that asked
    // nothing cannot have been left open, and saying so would report a decision
    // nobody was ever asked to make.
    expect(heading.querySelector(`.${sheet.unanswered}`)).toBeNull();
    expect(heading.querySelector(`.${sheet.answerText}`)).toBeNull();
    expect(heading.querySelector(`.${sheet.options}`)).toBeNull();
  });

  it("says of a question that went back open that it went back unanswered", async () => {
    const page = await reading(ANSWERED);

    // Q2a and Q3 went back open, and both are still drawn: an Unanswered
    // question is part of what the agent was told, not an omission.
    expect(page.querySelector("#q2a, #q3")).toBeTruthy();
    expect(page.innerHTML).toContain("What should Retry-After say?");
    expect(texts(page, `.${sheet.unanswered}`)).toEqual([
      "Unanswered — the agent was told this one is still open.",
      "Unanswered — the agent was told this one is still open.",
    ]);
  });

  it("says what was said about the Set as a whole, and when it was answered", async () => {
    const page = await reading(ANSWERED);

    expect(page.querySelector(`.${sheet.answeredAt}`)!.textContent).toBe(
      `Answered ${SETTLED}`,
    );
    // The exact minute rides behind the words, as the tooltip.
    expect(page.querySelector(`.${sheet.answeredAt}`)!.getAttribute("title")).toBe(
      SETTLED_STAMP,
    );
    const comment = page.querySelector(`section.${sheet.setComment}.${sheet.decided}`)!;
    expect(comment.querySelector(`.${sheet.comment}`)!.textContent).toBe(
      "Do the in-process one first; we can move it later.",
    );
  });

  it("heads the closing section for what it holds, and anchors it for the nav", async () => {
    const withOne = await reading(withPostscript(ANSWERED));
    const section = withOne.querySelector(`section.${closing.postscript}`)!;

    expect(section.id, "the id the table of contents jumps to").toBe(
      "postscript",
    );
    expect(section.querySelector(`h2.${app.sectionHeading}`)!.textContent).toBe(
      "Postscript",
    );

    // With no Postscript there is only the box, and the heading says so rather
    // than naming something the agent never wrote.
    const without = await reading(ANSWERED);
    expect(
      without.querySelector(`section.${closing.postscript} h2`)!.textContent,
    ).toBe("Comment");
  });

  it("closes an answered Set with the Postscript, wrapped around what was said about it", async () => {
    const page = await reading(withPostscript(ANSWERED));

    const postscript = page.querySelector(`section.${closing.postscript}`)!;
    expect(postscript, "expected the Postscript drawn").toBeTruthy();
    const body = postscript.querySelector(`.${closing.postscriptBody}`)!;
    expect(body.className).toContain("markdown");
    expect(body.innerHTML).toContain("<code>ops/export</code>");

    // Nested exactly as it is on the sheet, so the record reads the way the
    // page it was filled in on did.
    const comment = postscript.querySelector(`.${sheet.setComment}.${sheet.decided}`)!;
    expect(comment, "expected the comment inside the Postscript").toBeTruthy();
    expect(body.nextElementSibling).toBe(comment);
  });

  it("closes a Set the human said nothing about with it just the same", async () => {
    // Locked unanswered: there is no Response behind it, so there is no
    // comment either — and the Postscript belongs above where one would have
    // been, because it is the agent's own closing word rather than part of the
    // answer.
    const page = await reading(withPostscript(LOCKED));

    const postscript = page.querySelector(`section.${closing.postscript}`)!;
    expect(postscript, "expected the Postscript drawn").toBeTruthy();
    expect(postscript.previousElementSibling!.className).toContain(sheet.questions!);
    expect(page.querySelector(`.${sheet.setComment}`)).toBeNull();
  });

  it("draws no card at all for a settled Set with nothing to close it", async () => {
    // Locked unanswered and without a Postscript: no closing word from either
    // side, so there is nothing for a card to hold and none is drawn.
    const page = await reading(LOCKED);

    expect(page.querySelector(`.${closing.postscript}`)).toBeNull();
    expect(page.querySelector(`.${sheet.setComment}`)).toBeNull();
  });

  it("keeps the card for a Set commented on without a Postscript", async () => {
    const page = await reading(ANSWERED);

    const postscript = page.querySelector(`section.${closing.postscript}`)!;
    expect(postscript, "the comment is still read in a card").toBeTruthy();
    expect(postscript.querySelector(`.${closing.postscriptBody}`)).toBeNull();
    expect(postscript.querySelector(`.${sheet.setComment}.${sheet.decided}`)).toBeTruthy();
  });

  it("offers nothing to press", async () => {
    const page = await reading(ANSWERED);

    // A Set is answered once, so there is nothing here to act on it with.
    expect(page.querySelector("input")).toBeNull();
    expect(page.querySelector("textarea")).toBeNull();
    // Two buttons, and neither of them acts on the Set: the way back out of the
    // pane, and the nav's bar, which is a way around the record. Counted rather
    // than excused, so a button that does act on the Set still fails this.
    expect(texts(page, "button")).toHaveLength(2);
    expect(page.querySelector(`.${paneHead.back}`)).toBeTruthy();
    expect(page.querySelector(`.${contents.bar}`)).toBeTruthy();
    expect(page.querySelector(`.${sheet.questions}`)!.className).toContain(sheet.decided!);
  });

  it("is read for what was asked as well as for what was decided", async () => {
    for (const settled of [ANSWERED, LOCKED]) {
      const page = await reading(settled);

      expect(page.innerHTML).toContain("<li>in-process, per instance</li>");
      expect(page.innerHTML).toContain("<code>redis</code>");
      expect(page.innerHTML).toContain("<td>Retry-After</td>");
      expect(page.querySelector(`.${card.cardBody}`)).toBeTruthy();
    }
  });

  it("reads a Response that resolved nothing as a counter-question", async () => {
    const nothing: Decided = {
      answers: ["Q1", "Q2", "Q2a", "Q2b", "Q3"].map((label) => ({
        label,
        unanswered: true,
      })),
      comment: "Neither, really — why not cache it upstream?",
    };
    const page = await reading({
      ...ANSWERED,
      standing: { Answered: { submitted_at: "2026-08-03T09:07:11.000Z", response: nothing } },
    });

    // A Response that resolved nothing is still a Response, and has to read as
    // one rather than as a page whose Answers failed to arrive.
    expect(page.querySelector(`.${sheet.counterQuestion}`)!.textContent).toContain(
      "The comment below is the whole Response",
    );
    expect(page.querySelectorAll(`.${sheet.unanswered}`)).toHaveLength(5);
    expect(page.querySelector(`.${sheet.setComment} .${sheet.comment}`)!.textContent).toBe(
      nothing.comment,
    );
  });

  /// The one variant of this notice that says nothing worth reading: every
  /// question already reads Unanswered and there is no comment for the line to
  /// account for, so it only repeats the page back at whoever is on it. Its two
  /// siblings stay — the counter-question above, and the locked-unanswered
  /// line below — because each of those says something the rows do not.
  it("says nothing at the head of a Set answered in silence", async () => {
    const silent: Decided = {
      answers: ["Q1", "Q2", "Q2a", "Q2b", "Q3"].map((label) => ({
        label,
        unanswered: true,
      })),
      comment: null,
    };
    const page = await reading({
      ...ANSWERED,
      standing: { Answered: { submitted_at: "2026-08-03T09:07:11.000Z", response: silent } },
    });

    expect(page.querySelector(`.${sheet.counterQuestion}`)).toBeNull();
    expect(
      page.querySelectorAll(`.${sheet.unanswered}`),
      "the rows are the whole of the account, and they were always there",
    ).toHaveLength(5);
    expect(page.querySelector(`.${sheet.setComment}`)).toBeNull();
  });

  it("reads a Set closed unanswered as a record with no Response behind it", async () => {
    const page = await reading(LOCKED);

    expect(page.querySelector(`.${sheet.lockedAt}`)!.textContent).toBe(
      `Locked unanswered ${SETTLED}`,
    );
    expect(page.querySelector(`.${sheet.answeredAt}`)).toBeNull();
    expect(page.querySelector(`.${sheet.counterQuestion}`)!.textContent).toContain(
      "This Set was locked unanswered",
    );

    // Nothing was decided, and only a Response can leave a question open — so
    // no Option is marked and no question claims the agent was told anything.
    expect(page.querySelector(`.${sheet.option}.${sheet.chosen}`)).toBeNull();
    expect(page.querySelectorAll(`.${sheet.unanswered}`)).toHaveLength(0);
    // The Recommendation is still the agent's, and still marked.
    expect(page.querySelectorAll(`.${sheet.option} .${sheet.star}`)).toHaveLength(1);
  });

  it("leads back to the Timeline the Set was asked on", async () => {
    // The same way out however the Set stands: settled or waiting, it is an
    // Event on one Timeline and there is nowhere else for reading it to lead.
    // The pane's own way out, drawn by the header the sheet hands in: where
    // the Set had a page of its own there was a link, and a pane has this.
    for (const set of [WAITING, ANSWERED, LOCKED]) {
      const page = await reading(set);
      expect(page.querySelector(`.${paneHead.back}`)!.textContent).toBe(
        "← Timeline",
      );
    }
  });

  it("names the Preface and the Questions by headings on every standing", async () => {
    for (const set of [WAITING, ANSWERED, LOCKED]) {
      const page = await reading(set);

      // Named so a jump from the table of contents lands somewhere the reader can
      // see they have arrived at. The Diff is named the same way when there is one
      // — and it is the waiting fixture that carries one.
      //
      // The section closing the page is named for what it holds: none of these
      // fixtures has a Postscript, so it is the box, and it is there on the Set
      // still waiting to be filled in and on the one that came back with a
      // comment. The Set locked unanswered has neither and so ends at the
      // Questions.
      const closes =
        "Waiting" in set.standing ||
        ("Answered" in set.standing &&
          (set.standing.Answered.response.comment ?? "") !== "");

      expect(texts(page, `h2.${app.sectionHeading}`)).toEqual([
        "Preface",
        ...(set.diff.length === 0 ? [] : ["Diff"]),
        "Questions",
        ...(closes ? ["Comment"] : []),
      ]);
      for (const id of ["preface", "questions", "q1"]) {
        expect(page.querySelector(`#${id}`), `expected #${id}`).toBeTruthy();
      }
    }
  });
});

describe("the record of a question whose Options were declared as a table", () => {
  /// The Answer Table drawn for the question named `label`.
  ///
  /// Found from the question's own label rather than from a radio the way the
  /// sheet's tests find it: there are no radios here, which is half of what
  /// makes this the record.
  function table(page: ParentNode, label: string): HTMLTableElement {
    const drawn = named(page, label)
      .closest(`.${sheet.ask}`)
      ?.querySelector(`table.${sheet.answerTable}`);
    expect(drawn, `expected an Answer Table on ${label}`).toBeTruthy();
    return drawn as HTMLTableElement;
  }

  /// The row of Option `n`. The rows are the Options in the order the agent
  /// offered them, so the position is the number — which the row says itself,
  /// and which the assertions read back.
  function row(page: ParentNode, label: string, n: number): HTMLTableRowElement {
    const rows = table(page, label).querySelectorAll<HTMLTableRowElement>(
      "tbody tr",
    );
    expect(rows[n - 1], `expected Option ${n} of ${label}`).toBeTruthy();
    return rows[n - 1]!;
  }

  it("draws the table the sheet drew, with nothing left to fill in", async () => {
    const page = await reading(withTable(ANSWERED));

    // The same columns in the same order as on the sheet: empty over the
    // number, **Option** over the Option's own text, the agent's axes, and an
    // empty header over the ★.
    expect(texts(table(page, "Q1"), "thead th")).toEqual([
      "",
      "Option",
      "Latency",
      "ops cost",
      "",
    ]);
    // The row that was not chosen, which is the shape of a row and nothing
    // else: its number, its text, its cells, and the ★ column it left empty.
    expect(texts(row(page, "Q1", 2), "td")).toEqual([
      "2",
      "In Redis, shared across instances.",
      "A hop",
      "A box to run",
      "★",
    ]);
    // Inline markup survives in a cell as it does in an Option's own text.
    expect(row(page, "Q1", 1).querySelector("td:nth-child(3)")!.innerHTML).toContain(
      "<code>ms</code>",
    );

    // Read rather than filled in: a record is not a sheet, and there is nothing
    // on it to press.
    expect(table(page, "Q1").querySelectorAll("input")).toHaveLength(0);
  });

  it("marks the row that was chosen, visually and in as many words", async () => {
    const page = await reading(withTable(ANSWERED));

    // Q1 was answered with Option 1, and it is Option 2 that carries the ★ —
    // the class is what the treatment hangs off and the word is what a reader
    // who cannot see one is told.
    const chosen = row(page, "Q1", 1);
    expect(chosen.className).toContain(sheet.chosen!);
    expect(chosen.querySelector(`.${sheet.chose}`)!.textContent).toBe("chosen");
    expect(chosen.querySelector(`.${sheet.star}`)).toBeNull();

    const recommended = row(page, "Q1", 2);
    expect(recommended.className).toContain(sheet.recommended!);
    expect(recommended.querySelector(`.${sheet.star}`)).toBeTruthy();
    expect(
      recommended.querySelector(`.${sheet.chose}`),
      "the Recommendation was not taken, and the row must not read as if it was",
    ).toBeNull();

    // Every row is kept: what was turned down is half of what the decision was.
    expect(table(page, "Q1").querySelectorAll("tbody tr")).toHaveLength(2);
  });

  it("heads no ★ column on a question the agent recommended nothing on", async () => {
    const page = await reading(withTable(ANSWERED));

    expect(texts(table(page, "Q2a"), "thead th")).toEqual([
      "",
      "Option",
      "Precision",
    ]);
    expect(texts(row(page, "Q2a", 1), "td")).toEqual([
      "1",
      "The exact number of seconds.",
      "Exact",
    ]);
  });

  it("says of a table-mode question that went back open that it did", async () => {
    const page = await reading(withTable(ANSWERED));

    // Q2a is a table and was left Unanswered: the table changes how its Options
    // are shown, not how the outcome is said.
    const asked = named(page, "Q2a").closest(`.${sheet.ask}`)!;
    expect(asked.querySelector(`tr.${sheet.chosen}`)).toBeNull();
    expect(asked.querySelector(`.${sheet.chose}`)).toBeNull();
    expect(asked.querySelector(`.${sheet.unanswered}`)!.textContent).toBe(
      "Unanswered — the agent was told this one is still open.",
    );
  });

  it("marks no row on a Set locked unanswered, under the same account", async () => {
    const page = await reading(withTable(LOCKED));

    // The table is drawn, because what was asked is still worth reading — and
    // nothing on it is marked chosen, because nobody chose.
    expect(table(page, "Q1").querySelectorAll("tbody tr")).toHaveLength(2);
    expect(page.querySelector(`tr.${sheet.chosen}`)).toBeNull();
    expect(page.querySelector(`.${sheet.chose}`)).toBeNull();
    expect(page.querySelectorAll(`.${sheet.unanswered}`)).toHaveLength(0);

    // The head-of-page account is the one it has always been.
    expect(page.querySelector(`.${sheet.counterQuestion}`)!.textContent).toContain(
      "This Set was locked unanswered",
    );
    // The Recommendation is still the agent's, and still marked on its row.
    expect(row(page, "Q1", 2).querySelector(`.${sheet.star}`)).toBeTruthy();
  });

  it("leaves a question that declared no axes the list it always was", async () => {
    const page = await reading(withTable(ANSWERED));

    // Q2 declared none, so nothing about how its record reads moved.
    const asked = named(page, "Q2").closest(`.${sheet.ask}`)!;
    expect(asked.querySelector(`ul.${sheet.options}`), "expected Q2 still a list").toBeTruthy();
    expect(page.querySelectorAll(`ul.${sheet.options}`)).toHaveLength(1);
  });
});

describe("the client-side renderer", () => {
  it("is reached for only by a Set that has a Diagram on it", async () => {
    const page = await reading(DIAGRAMMED);

    expect(drawing).toHaveBeenCalledOnce();
    // What it draws over, and what a reader is left with if it never draws: the
    // source block the markdown renderer already wrote.
    expect(page.querySelector("pre.mermaid")!.textContent).toContain(
      "graph LR;",
    );
  });

  it("is not reached for by a Set without one", async () => {
    // Fences and tables and code spans throughout, and not one Diagram: this is
    // what almost every Set looks like, and it pays nothing.
    await reading(WAITING);

    expect(drawing).not.toHaveBeenCalled();
  });
});

describe("a Set this build cannot read", () => {
  it("says so, with the stored body under it and the way back out", async () => {
    const page = await unreadably(UNREADABLE);

    expect(page.querySelector(`.${illegible.unreadableBadge}`)!.textContent).toBe(
      "cannot be read",
    );
    // Serde's own sentence, which names the field that has left the schema.
    expect(page.querySelector(`.${illegible.unreadableWhy}`)!.textContent).toContain(
      "accepted_by",
    );
    // And the record itself, byte for byte: it is what was asked, and the whole
    // of what there is left to show of it.
    expect(page.querySelector(`.${illegible.storedJson}`)!.textContent).toBe(
      UNREADABLE.body,
    );
    expect(page.querySelector(`.${paneHead.back}`)!.textContent).toBe(
      "← Timeline",
    );
  });

  it("offers nothing to fill in and nothing to press", async () => {
    const page = await unreadably(UNREADABLE);

    // No sheet, because a Response is checked against Questions nobody here can
    // read; and no standing menu, so there is no locking behind it either.
    expect(page.querySelector(`.${sheet.questions}`)).toBeNull();
    expect(page.querySelector(`.${standing.standing}`)).toBeNull();
    // The one button is the way back out of the pane, which is the pane's
    // rather than the record's: there is nothing here to press about the Set.
    expect(texts(page, "button")).toEqual(["← Timeline"]);
  });
});
