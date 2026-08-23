//! The direction chooser on the Set page: the control injected onto a Set whose
//! agent closed with a proposal, and the record it becomes once the Set is
//! answered.
//!
//! Both Sets come out of `tests/fixtures/set-proposing.json` and
//! `set-proposed.json`, which `cargo test` writes from the real
//! `/api/ui/sets/{id}` — so the chooser is drawn from the proposal the server
//! really sends, and the pick this page puts on the wire is checked against the
//! Response the store really recorded.

import { fireEvent, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Direction, SetView, Submitted } from "../src/api/types";
import { draftKey } from "../src/set/sheet";
import { answering, sent, texts } from "./reading";
import { json } from "./serving";
import proposed from "./fixtures/set-proposed.json" with { type: "json" };
import proposing from "./fixtures/set-proposing.json" with { type: "json" };
import waiting from "./fixtures/set-answering.json" with { type: "json" };

/// The renderer is a page's own doing and none of these fixtures has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

const PROPOSING = proposing as SetView;
const PROPOSED = proposed as SetView;

/// An ordinary Set, which carries no proposal and so has no chooser on it.
const ORDINARY = waiting as SetView;

/// The Response the answered fixture carries: what the server stored and handed
/// back when this same Set was picked on.
const DECIDED =
  "Answered" in PROPOSED.standing ? PROPOSED.standing.Answered.response : null;

const submitted = (outcome: Submitted) => json(outcome);

/// The radio for one of the three directions.
function offered(page: ParentNode, direction: Direction): HTMLInputElement {
  const radio = page.querySelector<HTMLInputElement>(
    `input[name="direction"][value="${direction}"]`,
  );
  expect(radio, `expected the ${direction} choice`).toBeTruthy();
  return radio!;
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("the chooser on a Set that carries a proposal", () => {
  it("offers all three directions, whichever one was recommended", async () => {
    const { page } = await answering(PROPOSING);

    expect(texts(page, ".direction-pick .direction-name")).toEqual([
      "Implement inline",
      "Break into a task list",
      "Stage a roadmap",
    ]);
  });

  it("marks the agent's recommendation without picking it", async () => {
    const { page } = await answering(PROPOSING);

    const recommended = page.querySelectorAll(".direction-pick .recommended");
    expect(recommended).toHaveLength(1);
    expect(recommended[0]!.querySelector(".direction-name")!.textContent).toBe(
      "Break into a task list",
    );
    expect(
      recommended[0]!.querySelector(".star"),
      "marked with the ★ an Option's Recommendation carries",
    ).toBeTruthy();

    for (const direction of ["inline", "task-list", "roadmap"] as Direction[]) {
      expect(
        offered(page, direction).checked,
        `nothing is picked until the human picks it, and ${direction} is no exception`,
      ).toBe(false);
    }
  });

  it("draws the agent's reasoning beside the choices", async () => {
    const { page } = await answering(PROPOSING);

    const rationale = page.querySelector(".direction-pick .ask .text");
    expect(rationale, "expected the rationale on the page").toBeTruthy();
    expect(rationale!.textContent).toContain("Five changes that barely touch");
    expect(
      rationale!.querySelector("strong"),
      "and rendered, like every other piece of agent markdown",
    ).toBeTruthy();
  });

  /// The one ask on the page that is not one of the Questions, and it is asked
  /// the way they are: the label floated in the accent beside the words, with
  /// the human's own word for the move in place of a question number.
  it("asks it the way a question is asked, labelled End", async () => {
    const { page } = await answering(PROPOSING);

    const label = page.querySelector(".direction-pick .ask .text .label");
    expect(label, "expected the ask's label on the chooser").toBeTruthy();
    expect(label!.textContent).toBe("End");
    expect(
      page.querySelector(".direction-pick h2"),
      "and no heading over it, because a question carries none",
    ).toBeNull();
  });

  it("says what picking one does, so the Preface does not have to", async () => {
    const { page } = await answering(PROPOSING);

    const said = page.querySelector(".direction-pick .semantics")!.textContent!;
    expect(said).toContain("accepts the proposal");
    expect(said, "and how to disagree, which is the whole way back").toContain(
      "sends it back",
    );
  });

  it("is not drawn at all on an ordinary Set", async () => {
    const { page } = await answering(ORDINARY);

    expect(page.querySelector(".direction-pick")).toBeNull();
  });

  it("is named in the table of contents, under the Questions", async () => {
    const { page } = await answering(PROPOSING);

    const nav = page.querySelector("nav.contents")!;
    expect(
      [...nav.querySelectorAll("a.contents-link")].map((link) =>
        link.getAttribute("href"),
      ),
    ).toEqual(["#preface", "#questions", "#q9", "#direction", "#postscript"]);
  });
});

describe("picking a direction", () => {
  it("picks on a click and clears on a second, as an Option does", async () => {
    const { page } = await answering(PROPOSING);

    fireEvent.click(offered(page, "roadmap"));
    expect(offered(page, "roadmap").checked).toBe(true);

    fireEvent.click(offered(page, "roadmap"));
    expect(
      offered(page, "roadmap").checked,
      "a second click on the picked direction un-picks it, which is how a mind is changed",
    ).toBe(false);
  });

  it("moves on an arrow key without ever clearing", async () => {
    const { page } = await answering(PROPOSING);

    fireEvent.click(offered(page, "inline"));
    // What an arrow key does to a radio group: it selects, and fires a change
    // without ever firing a click.
    fireEvent.change(offered(page, "task-list"));

    expect(offered(page, "task-list").checked).toBe(true);
    expect(offered(page, "inline").checked).toBe(false);
  });

  it("is kept in the draft, and is a draft on its own", async () => {
    const { page } = await answering(PROPOSING);
    const key = draftKey(PROPOSING.id);

    fireEvent.click(offered(page, "inline"));

    await waitFor(() => expect(localStorage.getItem(key)).toBeTruthy());
    expect(
      JSON.parse(localStorage.getItem(key)!).direction,
      "a pick with nothing said beside it is still worth coming back to",
    ).toBe("inline");
  });

  it("sends the pick as the Response's own field, not as an Answer", async () => {
    const { page, fetching } = await answering(
      PROPOSING,
      submitted("Accepted"),
    );

    fireEvent.click(offered(page, "inline"));
    fireEvent.click(page.querySelector(".submit button")!);

    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    expect(
      sent(fetching),
      "which is the Response the store recorded when this Set was really answered",
    ).toEqual(DECIDED);
  });

  it("sends no direction at all where nothing was picked", async () => {
    const { page, fetching } = await answering(
      PROPOSING,
      submitted("Accepted"),
    );

    fireEvent.click(page.querySelector(".submit button")!);

    // The warning stands between the human and a Set with an offered choice
    // left open — this Set has none, so the Response goes straight out.
    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    expect(
      (sent(fetching) as { direction?: Direction }).direction,
      "no pick is the proposal sent back, and it travels as nothing rather than as a word",
    ).toBeUndefined();
  });
});

describe("the record a picked-on Set becomes", () => {
  it("marks what was chosen apart from what was recommended", async () => {
    const { page } = await answering(PROPOSED);

    const chosen = page.querySelectorAll(".direction-pick .chosen");
    expect(chosen).toHaveLength(1);
    expect(chosen[0]!.querySelector(".direction-name")!.textContent).toBe(
      "Implement inline",
    );

    const recommended = page.querySelectorAll(".direction-pick .recommended");
    expect(
      recommended[0]!.querySelector(".direction-name")!.textContent,
      "the ★ still says what was argued for, which was not what was picked",
    ).toBe("Break into a task list");
  });

  /// Asked as a question and read back as one: the record and the chooser draw
  /// the same ask, out of the same component.
  it("reads back as the question it was asked as", async () => {
    const { page } = await answering(PROPOSED);

    const label = page.querySelector(".direction-pick .ask .text .label");
    expect(label, "expected the ask's label on the record").toBeTruthy();
    expect(label!.textContent).toBe("End");
  });

  it("keeps the directions that were turned down", async () => {
    const { page } = await answering(PROPOSED);

    expect(
      texts(page, ".direction-pick .direction-name"),
      "what was turned down is half of what the decision was",
    ).toEqual([
      "Implement inline",
      "Break into a task list",
      "Stage a roadmap",
    ]);
    expect(
      page.querySelectorAll('.direction-pick input[type="radio"]'),
      "and there is nothing left to press: a Set is answered once",
    ).toHaveLength(0);
  });
});
