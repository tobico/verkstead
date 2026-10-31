//! The direction chooser on the Set page: the control injected onto a Set whose
//! agent closed with a proposal, and the record it becomes once the Set is
//! answered.
//!
//! Both Sets come out of `tests/fixtures/set-proposing.json` and
//! `set-proposed.json`, which `cargo test` writes from the real
//! `/api/ui/sets/{id}` — so the chooser is drawn from the proposal the server
//! really sends, and the pick this page puts on the wire is checked against the
//! Response the store really recorded.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The page's own vocabulary, both ways: the hashed names to query the page by,
// and the source to read the Gutter's own rules off, jsdom laying nothing out
// to read them from.
import app from "../src/App.module.css";
import sheet from "../src/set/Sheet.module.css";
import stylesheet from "../src/set/Sheet.module.css?raw";
import type {
  AdditionView,
  ClusterDevice,
  Direction,
  SetView,
  Submitted,
} from "../src/api/types";
import pill from "../src/Attaching.module.css";
import contents from "../src/set/Contents.module.css";
import submitting from "../src/set/Answering.module.css";
import { draftKey } from "../src/set/filling";
import { answering, sent, texts } from "./reading";
import { json, readable, whenever } from "./serving";
import { slidPress } from "./sliding";
import proposed from "./fixtures/set-proposed.json" with { type: "json" };
import proposing from "./fixtures/set-proposing.json" with { type: "json" };
import waiting from "./fixtures/set-answering.json" with { type: "json" };
import wanting from "./fixtures/set-wanting.json" with { type: "json" };

/// The renderer is a page's own doing and none of these fixtures has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

const PROPOSING = readable(proposing);
const PROPOSED = readable(proposed);

/// The same closing Set wanting Windows, for the ConPTY stage.
const WANTING = readable(wanting);

/// An ordinary Set, which carries no proposal and so has no chooser on it.
const ORDINARY = readable(waiting);

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

    expect(texts(page, `.${sheet.directionPick} .${sheet.directionName}`)).toEqual([
      "Implement inline",
      "Break into a task list",
      "Stage a roadmap",
    ]);
  });

  it("marks the agent's recommendation without picking it", async () => {
    const { page } = await answering(PROPOSING);

    const recommended = page.querySelectorAll(`.${sheet.directionPick} .${sheet.recommended}`);
    expect(recommended).toHaveLength(1);
    expect(recommended[0]!.querySelector(`.${sheet.directionName}`)!.textContent).toBe(
      "Break into a task list",
    );
    expect(
      recommended[0]!.querySelector(`.${sheet.star}`),
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

    const rationale = page.querySelector(`.${sheet.directionPick} .${sheet.ask} .${sheet.text}`);
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
  it("asks it the way a question is asked, labelled Final", async () => {
    const { page } = await answering(PROPOSING);

    const label = page.querySelector(`.${sheet.directionCard} .${sheet.ask} .${sheet.text} .${sheet.label}`);
    expect(label, "expected the ask's label on the chooser").toBeTruthy();
    expect(label!.textContent).toBe("Final");
  });

  /// The house pattern every other section on the page keeps: the name of the
  /// section outside, what it holds in a card. Which is what makes the chooser
  /// read as one more question under a heading of its own, rather than as a
  /// piece of furniture that arrived from somewhere else.
  it("is a headed section holding one question-like card", async () => {
    const { page } = await answering(PROPOSING);

    const heading = page.querySelector(
      `.${sheet.directionPick} > h2.${app.sectionHeading}`,
    );
    expect(heading, "expected the section's heading over the card").toBeTruthy();
    expect(heading!.textContent).toBe("Direction");

    const card = page.querySelector(`.${sheet.directionPick} > .${sheet.directionCard}`);
    expect(card, "expected the card under the heading").toBeTruthy();
    for (const part of [
      `.${sheet.ask} .${sheet.text} .${sheet.label}`,
      `.${sheet.directions}`,
      `.${sheet.semantics}`,
    ]) {
      expect(
        card!.querySelector(part),
        `${part} belongs inside the card, as a question's parts do`,
      ).toBeTruthy();
    }
  });

  it("says what picking one does, so the Preface does not have to", async () => {
    const { page } = await answering(PROPOSING);

    const said = page.querySelector(`.${sheet.directionPick} .${sheet.semantics}`)!.textContent!;
    expect(said).toContain("accepts the proposal");
    expect(said, "and how to disagree, which is the whole way back").toContain(
      "sends it back",
    );
  });

  it("says which platform the work wants and why, under the directions", async () => {
    const { page } = await answering(WANTING);

    const line = page.querySelector(`.${sheet.directionPick} .${sheet.wants}`);
    expect(line, "a proposal wanting a platform says so").toBeTruthy();
    expect(line!.textContent).toBe(
      "This work wants Windows for the ConPTY stage and its cfg(windows) tests.",
    );
    expect(
      line!.querySelector("code")?.textContent,
      "the reason is the agent's markdown, rendered",
    ).toBe("cfg(windows)");
  });

  it("says nothing about platforms where the proposal wants none", async () => {
    const { page } = await answering(PROPOSING);

    expect(page.querySelector(`.${sheet.directionPick}`)).toBeTruthy();
    expect(page.querySelector(`.${sheet.wants}`)).toBeNull();
  });

  it("is not drawn at all on an ordinary Set", async () => {
    const { page } = await answering(ORDINARY);

    expect(page.querySelector(`.${sheet.directionPick}`)).toBeNull();
  });

  it("is named in the table of contents, under the Questions", async () => {
    const { page } = await answering(PROPOSING);

    const nav = page.querySelector(`nav.${contents.contents}`)!;
    expect(
      [...nav.querySelectorAll(`a.${contents.link}`)].map((link) =>
        link.getAttribute("href"),
      ),
    ).toEqual(["#preface", "#questions", "#q9", "#direction", "#postscript"]);
  });
});

describe("picking a direction", () => {
  /// A press on a direction's name that slid before it was let go — see
  /// [`slidPress`](./sliding.ts), which says what is staged and why.
  function slidName(page: ParentNode, direction: Direction): void {
    slidPress(
      offered(page, direction)
        .closest("label")!
        .querySelector(`.${sheet.directionName}`)!,
    );
  }

  it("picks on a press that slid over the name, not only on a still one", async () => {
    const { page } = await answering(PROPOSING);

    slidName(page, "roadmap");
    expect(offered(page, "roadmap").checked).toBe(true);

    // And once: a gesture counted twice would pick the direction and un-pick it
    // again, so this press would be picking rather than clearing.
    slidName(page, "roadmap");
    expect(offered(page, "roadmap").checked).toBe(false);
  });

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
    fireEvent.click(page.querySelector(`.${submitting.submit} button`)!);

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

    fireEvent.click(page.querySelector(`.${submitting.submit} button`)!);

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

    const chosen = page.querySelectorAll(`.${sheet.directionPick} .${sheet.chosen}`);
    expect(chosen).toHaveLength(1);
    expect(chosen[0]!.querySelector(`.${sheet.directionName}`)!.textContent).toBe(
      "Implement inline",
    );

    const recommended = page.querySelectorAll(`.${sheet.directionPick} .${sheet.recommended}`);
    expect(
      recommended[0]!.querySelector(`.${sheet.directionName}`)!.textContent,
      "the ★ still says what was argued for, which was not what was picked",
    ).toBe("Break into a task list");
  });

  /// Asked as a question and read back as one: the record and the chooser draw
  /// the same ask, out of the same component.
  it("reads back as the question it was asked as", async () => {
    const { page } = await answering(PROPOSED);

    const label = page.querySelector(`.${sheet.directionCard} .${sheet.ask} .${sheet.text} .${sheet.label}`);
    expect(label, "expected the ask's label on the record").toBeTruthy();
    expect(label!.textContent).toBe("Final");
  });

  /// Headed and carded on the record exactly as on the chooser: the two are one
  /// section read at two moments, and a record that dropped the heading would be
  /// a different page from the one that was answered.
  it("keeps the heading and the card on the settled record", async () => {
    const { page } = await answering(PROPOSED);

    const heading = page.querySelector(
      `.${sheet.directionPick} > h2.${app.sectionHeading}`,
    );
    expect(heading, "expected the heading over the record").toBeTruthy();
    expect(heading!.textContent).toBe("Direction");
    expect(
      page.querySelector(`.${sheet.directionPick} > .${sheet.directionCard} .${sheet.directions}`),
      "and the three inside the card, as they were picked from",
    ).toBeTruthy();
  });

  it("keeps the directions that were turned down", async () => {
    const { page } = await answering(PROPOSED);

    expect(
      texts(page, `.${sheet.directionPick} .${sheet.directionName}`),
      "what was turned down is half of what the decision was",
    ).toEqual([
      "Implement inline",
      "Break into a task list",
      "Stage a roadmap",
    ]);
    expect(
      page.querySelectorAll(`.${sheet.directionPick} input[type="radio"]`),
      "and there is nothing left to press: a Set is answered once",
    ).toHaveLength(0);
  });
});

/// The Gutter the page's cards reserve at a wide window, and the card's place in
/// it — asserted off the stylesheet, because jsdom computes no layout and a
/// media query it never evaluates is a rule no rendered page can be asked about.
///
/// Two rules make the alignment: the one that reserves the Gutter inside the
/// card, and the one that hangs a label out into it. The label used to hang off
/// `.direction-pick` — the section, which reserves nothing — so at a wide window
/// it hung outside the column entirely. Both name the card now.
describe("the card in the wide-window Gutter", () => {
  it("reserves the Gutter like the page's other cards", () => {
    expect(
      declares(
        ".directionCard,\n  .question",
      ),
    ).toContain("--bleed: var(--gutter);");
  });

  it("hangs the End label in the Gutter the card reserved", () => {
    expect(stylesheet).toContain(".directionCard > .ask > .text > .label {");
    expect(
      stylesheet,
      "and no longer off the section, which reserves no Gutter of its own",
    ).not.toContain(".directionPick > .ask > .text > .label");
  });
});

/// What one rule declares, read off the stylesheet by the selector list that
/// carries it.
function declares(selectors: string): string {
  const at = stylesheet.indexOf(`\n  ${selectors} {\n`);
  expect(
    at,
    `expected the stylesheet to hold \`${selectors}\``,
  ).toBeGreaterThan(-1);
  return stylesheet.slice(at, stylesheet.indexOf("\n  }", at));
}

/// One device of the cluster, as the server places it.
function member(
  name: string,
  os: string,
  platform: ClusterDevice["platform"],
  standing: ClusterDevice["standing"],
): ClusterDevice {
  return {
    device: {
      device: name.padEnd(32, "0"),
      fingerprint: "AA:BB",
      name,
      os,
      addresses: [],
      kept_by: null,
      version: null,
      install: null,
    },
    platform,
    standing,
  };
}

const HERE = member("tower", "Linux", "linux", "WorkIsHere");
const NUC = member("nuc-win", "Windows", "windows", "Tickable");
const LAPTOP = member("the-laptop", "Linux (WSL)", "linux", "Tickable");

/// A Set placed on a cluster of those, with what was picked on it.
function clustered(
  set: SetView,
  cluster: ClusterDevice[],
  additions: AdditionView[] = [],
): SetView {
  return { ...set, additions, addable: { ...set.addable, cluster } };
}

/// The *May be transferred to* group on the chooser, where there is one.
function placing(page: ParentNode): HTMLElement | null {
  return page.querySelector<HTMLElement>(
    `.${sheet.directionPick} [role="group"]`,
  );
}

/// The ticks in it, by the device each names.
function ticks(page: ParentNode): Array<[string, boolean]> {
  return [
    ...(placing(page)?.querySelectorAll<HTMLInputElement>(
      'input[type="checkbox"]',
    ) ?? []),
  ].map((tick) => [tick.closest("label")!.textContent!.trim(), tick.checked]);
}

/// The POSTs to one path, with their bodies.
function writes(
  fetching: Awaited<ReturnType<typeof answering>>["fetching"],
  path: string,
): unknown[] {
  return fetching.mock.calls
    .filter(([asked, init]) => String(asked) === path && init?.method === "POST")
    .map(([, init]) => JSON.parse(String(init?.body ?? "{}")) as unknown);
}

describe("the ticks under the chooser", () => {
  it("draws the wanted platform's devices first, with the reason, then the others", async () => {
    const { page } = await answering(clustered(WANTING, [HERE, LAPTOP, NUC]));

    const group = placing(page);
    expect(group, "a clustered Verkstead draws ticks").toBeTruthy();
    expect(screen.getByRole("group", { name: "May be transferred to" })).toBe(
      group,
    );
    expect(ticks(page)).toEqual([
      ["nuc-win", false],
      ["the-laptop", false],
    ]);

    const wanted = group!.querySelector(`.${sheet.wanted}`)!;
    expect(wanted.querySelector(`.${sheet.wants}`)!.textContent).toBe(
      "Windows for the ConPTY stage and its cfg(windows) tests",
    );
    expect(wanted.textContent).toContain("nuc-win");
    expect(
      page.querySelector(`.${sheet.directionPick} > div > p.${sheet.wants}`),
      "and not the line of prose besides",
    ).toBeNull();
  });

  it("says where the cluster has no device of the wanted platform, with nothing to tick", async () => {
    const { page } = await answering(clustered(WANTING, [HERE, LAPTOP]));

    const wanted = placing(page)!.querySelector(`.${sheet.wanted}`)!;
    expect(wanted.querySelector('input[type="checkbox"]')).toBeNull();
    expect(wanted.querySelector(`.${sheet.placed}`)!.textContent).toBe(
      "No Windows device is linked to this one, so the work will run where it is.",
    );
    expect(ticks(page)).toEqual([["the-laptop", false]]);
  });

  it("names a wanted device that is permitted already, or where the work is, with no tick", async () => {
    const permitted = await answering(
      clustered(WANTING, [
        HERE,
        { ...NUC, standing: "Permitted" },
      ]),
    );
    expect(ticks(permitted.page)).toEqual([]);
    expect(
      placing(permitted.page)!.querySelector(`.${sheet.placed}`)!.textContent,
    ).toBe("The work may already be transferred to nuc-win.");

    const here = await answering(
      clustered(WANTING, [
        { ...NUC, standing: "WorkIsHere" },
        { ...HERE, standing: "Permitted" },
      ]),
    );
    expect(ticks(here.page)).toEqual([]);
    expect(
      placing(here.page)!.querySelector(`.${sheet.placed}`)!.textContent,
    ).toBe("The work is on nuc-win.");
  });

  it("draws only the line of prose in no cluster", async () => {
    const { page } = await answering(WANTING);

    expect(WANTING.addable.cluster).toBeNull();
    expect(placing(page)).toBeNull();
    expect(page.querySelector('input[type="checkbox"]')).toBeNull();
    expect(page.querySelector(`.${sheet.wants}`)!.textContent).toBe(
      "This work wants Windows for the ConPTY stage and its cfg(windows) tests.",
    );
  });

  it("draws the chooser as before where the proposal wants nothing", async () => {
    const { page } = await answering(clustered(PROPOSING, [HERE, NUC]));

    expect(placing(page)).toBeNull();
    expect(page.querySelector(`.${sheet.wants}`)).toBeNull();
  });

  it("records a tick under no label, as the comment box's menu does", async () => {
    const path = `/api/ui/sets/${WANTING.id}/additions`;
    const { page, fetching } = await answering(
      clustered(WANTING, [HERE, NUC]),
      whenever(path, json("Answered"), "POST"),
    );

    fireEvent.click(
      placing(page)!.querySelector<HTMLInputElement>('input[type="checkbox"]')!,
    );

    await waitFor(() =>
      expect(writes(fetching, path)).toEqual([
        { label: null, pick: { Device: { device: NUC.device.device } } },
      ]),
    );
    await waitFor(() =>
      expect(placing(page)!.textContent).toContain(
        "This question set has been answered, so what it adds is settled.",
      ),
    );
  });

  it("draws a device picked at the comment box as ticked, and not again as its chip", async () => {
    const remove = `/api/ui/sets/${WANTING.id}/additions/5/remove`;
    const { page, fetching } = await answering(
      clustered(
        WANTING,
        [HERE, NUC, LAPTOP],
        [
          {
            id: 5,
            label: null,
            added: { Device: { device: NUC.device.device, name: "nuc-win" } },
            state: "Pending",
          },
        ],
      ),
      whenever(remove, json("Removed"), "POST"),
    );

    expect(ticks(page)).toEqual([
      ["nuc-win", true],
      ["the-laptop", false],
    ]);
    expect(
      screen.queryByRole("list", { name: "Attached to the question set" })
        ?.querySelectorAll(`.${pill.server}`).length ?? 0,
      "no chip for what the chooser draws",
    ).toBe(0);

    fireEvent.click(
      placing(page)!.querySelector<HTMLInputElement>('input[type="checkbox"]')!,
    );
    await waitFor(() => expect(writes(fetching, remove)).toEqual([{}]));
  });
});
