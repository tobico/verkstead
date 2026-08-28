//! The Nothing-else option on a follow-up's Sets: the control the closing
//! section carries when, and only when, the Set was asked while its
//! Conversation is in Follow-up.
//!
//! Both Sets come out of `tests/fixtures/set-following-up.json` and
//! `set-answering.json`, which `cargo test` writes from the real
//! `/api/ui/sets/{id}` — so what decides whether the option is drawn is the
//! payload the server really sends, and the mark this page puts on the wire is
//! checked against the Response the schema really describes.
//!
//! The other half of the arrangement is not on this side of the wire at all:
//! the mark comes off the Response in the store, so the agent is handed the
//! same bytes either way. `crates/store/tests/endings.rs` is where that is
//! asked.

import { fireEvent, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Response as Decided, Submitted } from "../src/api/types";
import submitting from "../src/set/Answering.module.css";
import postscript from "../src/set/Postscript.module.css";
import sheet from "../src/set/Sheet.module.css";
import { draftKey } from "../src/set/sheet";
import { answering, sent } from "./reading";
import { json, readable } from "./serving";
import following from "./fixtures/set-following-up.json" with { type: "json" };
import waiting from "./fixtures/set-answering.json" with { type: "json" };

/// The renderer is a page's own doing and neither fixture has a Diagram; mocked
/// so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

/// A round of a follow-up, which is the one kind of Set the option is drawn on.
const FOLLOWING_UP = readable(following);

/// An ordinary Set, asked from a Conversation that is building rather than
/// following up.
const ORDINARY = readable(waiting);

const submitted = (outcome: Submitted) => json(outcome);

/// The option itself.
function option(page: ParentNode): HTMLInputElement {
  const box = page.querySelector<HTMLInputElement>(
    `.${sheet.ending} input[name="nothing-else"]`,
  );
  expect(box, "expected the Nothing else option").toBeTruthy();
  return box!;
}

/// Answer the round's one multiple-choice question, so a submit goes straight
/// out rather than through the warning about choices left open.
function answer(page: ParentNode) {
  fireEvent.click(page.querySelector('input[name="Q1-option"][value="1"]')!);
}

function submit(page: ParentNode) {
  fireEvent.click(page.querySelector(`.${submitting.submit} button`)!);
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("the option on a follow-up's Set", () => {
  it("is drawn in the closing section, inside the Postscript's card", async () => {
    const { page } = await answering(FOLLOWING_UP);

    const card = page.querySelector(`#postscript .${postscript.postscriptCard}`);
    expect(card, "expected the closing section's card").toBeTruthy();
    expect(
      card!.querySelector(`.${sheet.ending}`),
      "the option closes the Set beside the comment box, not somewhere of its own",
    ).toBeTruthy();
    expect(
      page.querySelector(`.${sheet.ending} .${sheet.endingName}`)!.textContent,
    ).toBe("Nothing else");
  });

  it("starts unticked, so nothing ends a follow-up unasked", async () => {
    const { page } = await answering(FOLLOWING_UP);

    expect(option(page).checked).toBe(false);
  });

  it("says what ticking it does", async () => {
    const { page } = await answering(FOLLOWING_UP);

    const said = page.querySelector(`.${sheet.ending} .${sheet.semantics}`)!
      .textContent!;
    expect(said).toContain("nothing more you want from this follow-up");
    expect(
      said,
      "and that what they wrote still goes back, which is what makes it safe to tick",
    ).toContain("still goes back");
  });

  it("is not drawn at all on an ordinary Set", async () => {
    const { page } = await answering(ORDINARY);

    expect(
      page.querySelector(`.${sheet.ending}`),
      "every state but Follow-up draws the closing section without it",
    ).toBeNull();
    expect(
      page.querySelector(`#postscript`),
      "which is the same closing section, minus the option",
    ).toBeTruthy();
  });
});

describe("ticking it", () => {
  it("ticks on a click and clears on a second", async () => {
    const { page } = await answering(FOLLOWING_UP);

    fireEvent.click(option(page));
    expect(option(page).checked).toBe(true);

    fireEvent.click(option(page));
    expect(
      option(page).checked,
      "a follow-up ended by a mis-tap has to be un-endable",
    ).toBe(false);
  });

  it("is kept in the draft, and is a draft on its own", async () => {
    const { page } = await answering(FOLLOWING_UP);
    const key = draftKey(FOLLOWING_UP.id);

    fireEvent.click(option(page));

    await waitFor(() => expect(localStorage.getItem(key)).toBeTruthy());
    expect(
      JSON.parse(localStorage.getItem(key)!).nothing_else,
      "a tick with nothing said beside it is still worth coming back to",
    ).toBe(true);
  });

  it("sends the mark as the Response's own field, not as an Answer", async () => {
    const { page, fetching } = await answering(
      FOLLOWING_UP,
      submitted("Accepted"),
    );

    answer(page);
    fireEvent.click(option(page));
    submit(page);

    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    expect(sent(fetching)).toEqual({
      answers: [
        { label: "Q1", selected: 1 },
        { label: "Q2", unanswered: true },
      ],
      nothing_else: true,
    } satisfies Decided);
  });

  it("sends no mark at all where it was left alone", async () => {
    const { page, fetching } = await answering(
      FOLLOWING_UP,
      submitted("Accepted"),
    );

    answer(page);
    submit(page);

    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    expect(
      (sent(fetching) as Decided).nothing_else,
      "an untouched option is the follow-up carrying on, and travels as nothing",
    ).toBeUndefined();
  });
});
