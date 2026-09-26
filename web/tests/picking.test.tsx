//! The listbox the app draws for itself: what the keyboard does to it, what a
//! screen reader is told about it, and the mark each row carries.
//!
//! A native `<select>` arrives with all of this and a control drawn out of
//! ordinary elements arrives with none of it, so the whole of what was given
//! back is asserted here rather than trusted: the workbench is answered from a
//! phone and from a keyboard as readily as from a mouse, and every choice this
//! control stands in for settles something about somebody's work — who runs it,
//! and which repository it is in.
//!
//! Driven straight rather than through a page — what is asked is the control's
//! own, so no query, no card and no modal is in the way of the answer. Where a
//! page's own picker is the subject, the test is with that page:
//! `workbench.test.tsx` for the four pairing pickers and the Repo,
//! `profiles.test.tsx` for the profile form's harness type, and
//! `surviving.test.tsx` for what a re-read leaves of a choice.

import { fireEvent, render, screen } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AgentType } from "../src/agents";
import { Listbox } from "../src/picking";
import styles from "../src/picking.module.css";
import css from "../src/picking.module.css?raw";
import claudeMarkFile from "../src/marks/claude-color.svg?raw";
import grokMarkFile from "../src/marks/grok.svg?raw";
import { art, marked } from "./marking";
import {
  actionRows,
  actions,
  expanded,
  offered,
  opened,
  pick,
  picker,
  press,
  rows,
  showing,
} from "./pickers";

/// Every source under `src/`, for the one question about the control that is not
/// about the control: which modules draw it.
const SOURCES = import.meta.glob("../src/**/*.tsx", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/// Three rows: two accounts with marks of their own, and one row that is no
/// account at all — which is the shape the review picker has.
const ROWS: { value: string; label: string; mark: AgentType | null }[] = [
  { value: ":", label: "No review", mark: null },
  { value: "1:claude-fable-5", label: "Claude Code Fable 5", mark: "Claude" },
  { value: "2:grok-4.6", label: "Grok 4.6", mark: "Grok" },
];

/// The control, with the choice held where a caller would hold it.
function picking(
  at = "",
  options: typeof ROWS = ROWS,
  disabled = false,
): { chosen: () => string } {
  const [chosen, setChosen] = createSignal(at);

  render(() => (
    <>
      <label for="under">Run it under</label>
      <Listbox
        id="under"
        options={options}
        value={(row) => row.value}
        label={(row) => row.label}
        mark={(row) => row.mark}
        chosen={chosen()}
        pick={setChosen}
        disabled={disabled}
      />
    </>
  ));

  return { chosen };
}

/// The same control with rows at its foot that press rather than pick, which is
/// what the Repo dropdown draws: two of them, behind a rule.
function pressing(at = ""): {
  chosen: () => string;
  pressed: () => string[];
} {
  const [chosen, setChosen] = createSignal(at);
  const [pressed, setPressed] = createSignal<string[]>([]);

  const acts = (label: string) => () =>
    setPressed((was) => [...was, label]);

  render(() => (
    <>
      <label for="under">Run it under</label>
      <Listbox
        id="under"
        options={ROWS}
        value={(row) => row.value}
        label={(row) => row.label}
        chosen={chosen()}
        pick={setChosen}
        actions={[
          { label: "Create repo", press: acts("Create repo") },
          { label: "Open repo", press: acts("Open repo") },
        ]}
      />
    </>
  ));

  return { chosen, pressed };
}

/// The one picker every test here drives.
const UNDER = "Run it under";

/// And the control itself, for the attributes a screen reader reads it by.
const control = () => picker(UNDER);

describe("what the listbox says it is", () => {
  /// The label reaching the control is why it is a `button` rather than a `div`
  /// with a role: only a labelable element is what a `<label for=…>` names, and
  /// every caller but the composer's Repo row labels its picker that way — that
  /// one names itself from inside its handle, which is `heading` on the control.
  it("is reached by the label that names it", () => {
    picking();

    expect(screen.getByLabelText(UNDER).tagName).toBe("BUTTON");
    expect(screen.getByLabelText(UNDER).id).toBe("under");
  });

  it("reads as a combobox with a list under it", () => {
    picking();

    expect(control().getAttribute("role")).toBe("combobox");
    expect(control().getAttribute("aria-haspopup")).toBe("listbox");
    // Said in both states rather than only when it is open: a combobox that
    // stopped saying it would stop being announced as one.
    expect(control().getAttribute("aria-expanded")).toBe("false");
  });

  /// What it points at while the rows are down, and that it points at nothing
  /// while they are not: an `aria-controls` naming an element the page does not
  /// hold is a reader sent nowhere.
  it("names the list it drops, and only while it is down", () => {
    picking();

    expect(control().getAttribute("aria-controls")).toBeNull();

    const list = opened(UNDER);

    expect(control().getAttribute("aria-controls")).toBe(list.id);
    expect(list.getAttribute("role")).toBe("listbox");
  });

  it("draws every row as an option, with the choice marked as selected", () => {
    picking("2:grok-4.6");

    expect(offered(UNDER).map((row) => row.getAttribute("role"))).toEqual([
      "option",
      "option",
      "option",
    ]);
    expect(offered(UNDER).map((row) => row.getAttribute("aria-selected"))).toEqual(
      ["false", "false", "true"],
    );
  });

  /// The row the keyboard is on, named on the control rather than focused: the
  /// focus stays where the label is, which is what lets a reader hear the
  /// control's own name and the row it is over.
  it("names the row the keyboard is on", () => {
    picking("2:grok-4.6");
    opened(UNDER);

    expect(control().getAttribute("aria-activedescendant")).toBe(
      offered(UNDER)[2]!.id,
    );

    fireEvent.keyDown(control(), { key: "Home" });

    expect(control().getAttribute("aria-activedescendant")).toBe(
      offered(UNDER)[0]!.id,
    );
  });
});

describe("driving the listbox by hand", () => {
  it("drops the rows on a press, and takes them back on the next", () => {
    picking();

    fireEvent.click(control());
    expect(expanded(UNDER)).toBe(true);

    fireEvent.click(control());
    expect(expanded(UNDER)).toBe(false);
  });

  it("picks the row that was pressed", () => {
    const { chosen } = picking();

    pick(UNDER, "Grok 4.6");

    expect(chosen()).toBe("2:grok-4.6");
    expect(showing(UNDER)).toBe("Grok 4.6");
    // And the rows go with the press: the choice is made, and a list left down
    // over it would be an invitation to make it again.
    expect(expanded(UNDER)).toBe(false);
  });

  /// A press away from the rows is the other way out of every dropdown in the
  /// app, and it picks nothing: a stray press that chose an account is not a
  /// small thing on the card this stands on.
  it("takes the rows back on a press away from them, picking nothing", () => {
    const { chosen } = picking("1:claude-fable-5");
    opened(UNDER);

    fireEvent.click(document.querySelector(`.${styles.backdrop}`)!);

    expect(expanded(UNDER)).toBe(false);
    expect(chosen()).toBe("1:claude-fable-5");
  });

  it("drops nothing at all while it is disabled", () => {
    picking("1:claude-fable-5", ROWS, true);

    fireEvent.click(control());

    expect(expanded(UNDER)).toBe(false);
    expect(control().disabled).toBe(true);
  });
});

describe("driving the listbox by keyboard", () => {
  it.each(["Enter", " ", "ArrowDown", "ArrowUp"])(
    "drops the rows on %s",
    (key) => {
      picking();

      fireEvent.keyDown(control(), { key });

      expect(expanded(UNDER)).toBe(true);
    },
  );

  /// The walk starts on the choice rather than at the top: somebody who has
  /// answered this picker once is asking to move from where they are.
  it("starts the walk on the row that is the choice", () => {
    const { chosen } = picking("2:grok-4.6");

    fireEvent.keyDown(control(), { key: "ArrowDown" });
    fireEvent.keyDown(control(), { key: "Enter" });

    expect(chosen()).toBe("2:grok-4.6");
  });

  it("walks the rows with the arrows and picks with Enter", () => {
    const { chosen } = picking();

    fireEvent.keyDown(control(), { key: "ArrowDown" });
    fireEvent.keyDown(control(), { key: "ArrowDown" });
    fireEvent.keyDown(control(), { key: "Enter" });

    expect(chosen()).toBe("1:claude-fable-5");
    expect(showing(UNDER)).toBe("Claude Code Fable 5");
  });

  /// Neither end of the list wraps round: an arrow held down walks to the end
  /// and waits there, which is what a native dropdown does.
  it("stops at either end rather than wrapping round", () => {
    const { chosen } = picking();

    fireEvent.keyDown(control(), { key: "ArrowDown" });
    for (const _ of ROWS) fireEvent.keyDown(control(), { key: "ArrowUp" });

    expect(control().getAttribute("aria-activedescendant")).toBe(
      offered(UNDER)[0]!.id,
    );

    for (const _ of ROWS) fireEvent.keyDown(control(), { key: "ArrowDown" });
    fireEvent.keyDown(control(), { key: "Enter" });

    expect(chosen()).toBe("2:grok-4.6");
  });

  it("goes to either end on Home and End", () => {
    const { chosen } = picking();

    fireEvent.keyDown(control(), { key: "ArrowDown" });
    fireEvent.keyDown(control(), { key: "End" });
    fireEvent.keyDown(control(), { key: "Enter" });

    expect(chosen()).toBe("2:grok-4.6");
  });

  it("closes on Escape, picking nothing", () => {
    const { chosen } = picking();

    fireEvent.keyDown(control(), { key: "ArrowDown" });
    fireEvent.keyDown(control(), { key: "End" });
    fireEvent.keyDown(control(), { key: "Escape" });

    expect(expanded(UNDER)).toBe(false);
    expect(chosen()).toBe("");
  });

  /// Tab is the hand leaving the control, so the rows go and the browser is left
  /// to move the focus: a dropdown that swallowed Tab would trap a keyboard on
  /// one field of the form.
  it("takes the rows back on Tab without swallowing it", () => {
    const { chosen } = picking();
    opened(UNDER);

    const tabbed = fireEvent.keyDown(control(), { key: "Tab" });

    expect(expanded(UNDER)).toBe(false);
    expect(chosen()).toBe("");
    expect(tabbed).toBe(true);
  });
});

describe("what a row of the listbox draws", () => {
  it("draws each account's own mark beside its reading", () => {
    picking();

    expect(offered(UNDER).map(marked)).toEqual([
      // The row that is no account draws no mark at all — no element, so no gap
      // where one would have been.
      null,
      art(claudeMarkFile),
      art(grokMarkFile),
    ]);
    expect(rows(UNDER)).toEqual([
      "No review",
      "Claude Code Fable 5",
      "Grok 4.6",
    ]);
  });

  it("draws the chosen row on the closed control the way the list drew it", () => {
    picking("2:grok-4.6");

    expect(marked(control())).toBe(art(grokMarkFile));
    expect(showing(UNDER)).toBe("Grok 4.6");
  });

  /// A row of the caller's own that sends the empty string is a choice rather
  /// than the absence of one, so no placeholder is drawn over it — which is the
  /// state the base-branch dropdown's rule row is in.
  it("draws no placeholder where a row of its own sends nothing", () => {
    const offering = [
      { value: "", label: "Automatically select", mark: null },
      ...ROWS.slice(1),
    ];
    const { chosen } = picking("", offering);

    expect(showing(UNDER)).toBe("Automatically select");

    pick(UNDER, "Claude Code Fable 5");
    pick(UNDER, "Automatically select");

    // And picking it is a pick like any other, rather than an unpicking.
    expect(chosen()).toBe("");
    expect(showing(UNDER)).toBe("Automatically select");
  });

  /// A finger's target rather than a pointer's: the workbench is answered from a
  /// phone, which is what the native control was kept for until now.
  it("gives every row a height a finger can hit", () => {
    expect(css).toContain("min-height: 2.75rem;");
  });

  /// The rows are a popover, so they come down over the same wash a menu comes
  /// down over — 20%, the figure `Menu.module.css` carries.
  it("washes the page under the rows, as a menu does", () => {
    expect(css).toMatch(
      /\.backdrop \{[^}]*background: rgb\(0 0 0 \/ 20%\);/,
    );
    // And the one set of rows that is not a popover keeps a clear one — the
    // browse field's, where the typing goes on while they are down.
    expect(css).toMatch(/\.backdrop\.clear \{[^}]*background: none;/);

    // As does every set of rows inside the modal, where there is no page behind
    // them to wash: the dialog is in the top layer with its own backdrop over
    // the page already, and this one — fixed, and painted above everything in
    // the card — would dim the card being filled in rather than anything
    // behind it.
    expect(css).toMatch(/dialog \.backdrop \{[^}]*background: none;/);
  });

  /// The caret is the app's own shape rather than whatever the reader's font
  /// has for `▾` — the same chevron the status button draws.
  it("draws the app's chevron rather than a character", () => {
    picking();

    expect(control().querySelector("svg")).toBeTruthy();
    expect(control().textContent).not.toContain("▾");
  });
});

/// The trigger with its label inside the handle, which is what the composer's
/// setup row draws: one rectangle around the label and the value, a press on
/// either opening the rows, and the label still what the control is called.
describe("the listbox with its label in the handle", () => {
  /// Named by the label rather than by everything inside the button: a combobox
  /// says what it is called and what it is showing as two different things, and
  /// a name read off the contents would run the two together.
  it("is named by the label it draws inside itself", () => {
    render(() => (
      <Listbox
        id="review-pairing"
        heading={{ words: "Review" }}
        options={ROWS}
        value={(row) => row.value}
        label={(row) => row.label}
        mark={(row) => row.mark}
        chosen="2:grok-4.6"
        pick={() => {}}
      />
    ));

    const control = picker("Review");

    expect(control.id).toBe("review-pairing");
    expect(control.getAttribute("aria-labelledby")).toBe(
      "review-pairing-label",
    );
    expect(document.getElementById("review-pairing-label")!.textContent).toBe(
      "Review",
    );
    // The value is still the value, said where every other control says it.
    expect(showing("Review")).toBe("Grok 4.6");

    // And out of the contents the value is read from, so that the name is not
    // read into it as well: a node named directly by `aria-labelledby` is
    // still read for the name however it is hidden, which is what leaves this
    // control called "Review" and showing "Grok 4.6" rather than called
    // "Review" and showing "Review Grok 4.6".
    expect(
      document.getElementById("review-pairing-label")!.getAttribute("aria-hidden"),
    ).toBe("true");
  });

  /// And a press on the label is a press on the trigger, the label being part
  /// of it: what the hover rectangle surrounds is what the press reaches.
  it("drops the rows on a press on the label", () => {
    render(() => (
      <Listbox
        id="review-pairing"
        heading={{ words: "Review" }}
        options={ROWS}
        value={(row) => row.value}
        label={(row) => row.label}
        chosen=""
        pick={() => {}}
      />
    ));

    fireEvent.click(document.getElementById("review-pairing-label")!);

    expect(expanded("Review")).toBe(true);
  });

  /// Two readings of one choice: the rows say the whole of it, and the trigger
  /// says the shorter one the caller hands in.
  it("reads the closed control shorter than the row it came off", () => {
    render(() => (
      <Listbox
        id="review-pairing"
        heading={{ words: "Review" }}
        options={ROWS}
        value={(row) => row.value}
        label={(row) => row.label}
        closed={(row) => row.label.replace("Claude Code ", "")}
        chosen="1:claude-fable-5"
        pick={() => {}}
      />
    ));

    expect(showing("Review")).toBe("Fable 5");
    expect(rows("Review")).toEqual([
      "No review",
      "Claude Code Fable 5",
      "Grok 4.6",
    ]);
  });

  /// And the words a control says where nothing is chosen are the caller's
  /// where the caller has better ones: the composer's Repo control is an
  /// invitation rather than a record of a choice not made.
  it("says the caller's own placeholder", () => {
    render(() => (
      <Listbox
        id="conversation-repo"
        heading={{ words: "Repo" }}
        options={[]}
        value={(row: { value: string }) => row.value}
        label={(row: { value: string }) => row.value}
        nothing="Select"
        chosen=""
        pick={() => {}}
      />
    ));

    expect(showing("Repo")).toBe("Select");
  });
});

/// The last thing a native dropdown kept for itself: its popup is the browser's
/// own and goes wherever it fits on the screen, and these rows are an element of
/// the page. They are put on the screen all the same — they are `position:
/// fixed`, so nothing between them and the window clips them — which leaves the
/// window as the one edge they can fall past, and where they go as something to
/// be measured off the control's own box.
///
/// Measured rather than styled, which is why it is asserted through the
/// measurements: jsdom lays nothing out, so the boxes are the ones this file
/// hands back and the answer is the arithmetic over them.
describe("which way the rows come down", () => {
  /// A window of a known height, and boxes for the two elements that decide it:
  /// the control wherever it is put, and the rows however tall they are.
  ///
  /// `clip` is the box of everything else on the page — the pane or the card
  /// these stand inside. It decided which way the rows hung until the rows were
  /// made fixed, and it is still laid here so that the one test about it is
  /// asking something real.
  ///
  /// `across` is where the control's left edge is and `wide` how wide the rows
  /// stand, which is the control's own width unless a caller has asked for more
  /// — the compose page's pairing lists do, in `Setup.module.css`.
  function laid(at: {
    control: number;
    rows: number;
    across?: number;
    wide?: number;
    clip?: { top: number; bottom: number };
  }): void {
    const clip = at.clip ?? { top: 0, bottom: 1000 };
    const across = at.across ?? 24;

    window.innerHeight = 1000;
    window.innerWidth = 1000;

    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
      function (this: Element): DOMRect {
        if (this.classList.contains(styles.drop!)) {
          return {
            top: 0,
            bottom: at.rows,
            height: at.rows,
            width: at.wide ?? 300,
          } as DOMRect;
        }

        if (this.tagName === "BUTTON") {
          return {
            top: at.control,
            bottom: at.control + 40,
            height: 40,
            left: across,
            width: 300,
          } as DOMRect;
        }

        return { top: clip.top, bottom: clip.bottom } as DOMRect;
      },
    );
  }

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /// Which is the ordinary way round, and stays it wherever the rows fit.
  it("comes down under the control where there is room for it", () => {
    laid({ control: 100, rows: 250 });
    picking();

    expect(opened(UNDER).classList.contains(styles.above!)).toBe(false);
  });

  it("comes down over the control where there is not", () => {
    laid({ control: 800, rows: 250 });
    picking();

    expect(opened(UNDER).classList.contains(styles.above!)).toBe(true);
  });

  /// Against the window rather than against the box the control stands in: the
  /// rows are fixed, so the card capped at `80vh` and the pane that scrolls its
  /// own content clip nothing, and rows that fit on the screen are rows there is
  /// room for. Which is the whole of what being fixed bought — a browse near the
  /// foot of the Open repo card had most of its list taken away by the card.
  it("measures against the window, not the box the control stands in", () => {
    laid({ control: 400, rows: 250, clip: { top: 200, bottom: 500 } });
    picking();

    expect(opened(UNDER).classList.contains(styles.above!)).toBe(false);
  });

  /// And where they go is the control's own box, read off the window: a fixed
  /// element has no anchor to hang from, so the anchor is measured instead.
  it("puts them at the control's left edge, as wide as it is", () => {
    laid({ control: 100, rows: 250 });
    picking();

    const rows = opened(UNDER);

    expect(rows.style.left).toBe("24px");
    expect(rows.style.width).toBe("300px");
    // Under the control, which is where the gap in `picking.module.css` is taken
    // off — the margin rather than this measure, so that the look stays in the
    // sheet.
    expect(rows.style.top).toBe("140px");
    expect(rows.style.bottom).toBe("");
  });

  /// And off the other edge where they hang over it, which is the same
  /// coordinate said from the bottom of the window.
  it("puts them off the control's top edge where they hang over it", () => {
    laid({ control: 800, rows: 250 });
    picking();

    const rows = opened(UNDER);

    expect(rows.style.bottom).toBe("200px");
    expect(rows.style.top).toBe("");
  });

  /// And pulled back onto the window where a list wider than its control would
  /// otherwise run off the right of it. The pairing pickers on the compose page
  /// are the ones this is for: their rows ask for 30rem against a trigger a
  /// third of that, and the last picker of a row stands near the far edge. A
  /// fixed box that fell past the window would be rows nothing could scroll to.
  it("pulls a list wider than its control back onto the window", () => {
    laid({ control: 100, rows: 250, across: 800, wide: 480 });
    picking();

    // Eight off the far edge: 1000 less the 480 the rows stand at, less the gap
    // they keep so the shadow under them has somewhere to sit.
    expect(opened(UNDER).style.left).toBe("512px");
  });

  /// And left where it is where it fits, which is every list the width of the
  /// control it came out of.
  it("leaves a list that fits at its control's left edge", () => {
    laid({ control: 100, rows: 250, across: 600 });
    picking();

    expect(opened(UNDER).style.left).toBe("600px");
  });

  /// Measured again while they are down, because a fixed box does not move with
  /// the anchor: the pane the control stands in scrolls under it, and the rows
  /// would otherwise be left behind where the field used to be.
  it("follows the control when something under it scrolls", () => {
    laid({ control: 100, rows: 250 });
    picking();

    expect(opened(UNDER).style.top).toBe("140px");

    laid({ control: 40, rows: 250 });
    fireEvent.scroll(window);

    expect(opened(UNDER).style.top).toBe("80px");
  });

  /// And where it fits neither side, the side that shows more of it — which is
  /// the rule doing what it is for rather than a case of its own, the list being
  /// capped and scrolled from its top whichever way it hangs.
  it("takes the roomier side where it fits neither", () => {
    laid({ control: 500, rows: 2000 });
    picking();

    expect(opened(UNDER).classList.contains(styles.above!)).toBe(true);
  });

  /// Under it all the same where under it is the roomier side: the ordinary way
  /// round is the one to be in wherever being in it costs nothing.
  it("stays under the control where under it is the roomier side", () => {
    laid({ control: 300, rows: 2000 });
    picking();

    expect(opened(UNDER).classList.contains(styles.above!)).toBe(false);
  });

  /// And it is measured again each time, rather than settled on the first: the
  /// pane the control stands in scrolls under it.
  it("measures again every time the rows come down", () => {
    laid({ control: 800, rows: 250 });
    picking();

    expect(opened(UNDER).classList.contains(styles.above!)).toBe(true);

    fireEvent.click(control());
    laid({ control: 100, rows: 250 });

    expect(opened(UNDER).classList.contains(styles.above!)).toBe(false);
  });
});

/// The rows at the foot that press rather than pick — the Repo dropdown's, and
/// no other control's.
///
/// What is asked here is the whole of what makes them a row *kind* rather than
/// another option: they are never picked, never what the closed control shows,
/// and never mistaken for the choice being gone — while the keyboard walks them
/// with the rest, because a list somebody is reading down does not stop at the
/// rule.
describe("the rows that press rather than pick", () => {
  it("draws them at the foot of the list, behind a rule", () => {
    pressing();

    const list = opened(UNDER);
    const rule = list.querySelector('[role="separator"]')!;

    expect(actionRows(UNDER)).toEqual(["Create repo", "Open repo"]);

    // Behind the rule and after every option: the rule is a break in one list
    // rather than the edge of a second.
    for (const row of offered(UNDER)) {
      expect(
        rule.compareDocumentPosition(row) & Node.DOCUMENT_POSITION_PRECEDING,
      ).toBeTruthy();
    }
    for (const row of actions(UNDER)) {
      expect(
        rule.compareDocumentPosition(row) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }
  });

  /// Not a choice, and never one: nothing about it can be selected, and what the
  /// control offers is the rows above the rule.
  it("is never the choice, and never what the control shows", () => {
    const { chosen, pressed } = pressing();

    expect(
      actions(UNDER).map((row) => row.getAttribute("aria-selected")),
    ).toEqual(["false", "false"]);

    press(UNDER, "Open repo");

    // The press went out, the rows went with it, and nothing was picked — so
    // the control is saying exactly what it said before.
    expect(pressed()).toEqual(["Open repo"]);
    expect(chosen()).toBe("");
    expect(showing(UNDER)).toBe("Not chosen");
    expect(expanded(UNDER)).toBe(false);
  });

  /// A choice already made is left where it was: pressing one of these is not an
  /// unpicking, and a control that fell to its placeholder would be saying the
  /// repository had gone.
  it("leaves a choice that was already made standing", () => {
    const { chosen } = pressing("2:grok-4.6");

    press(UNDER, "Create repo");

    expect(chosen()).toBe("2:grok-4.6");
    expect(showing(UNDER)).toBe("Grok 4.6");
  });

  /// The walk runs across both lists as one — End goes to the last row of all,
  /// which is the last of these — and Enter presses rather than picks.
  it("is walked with the rest, and Enter presses it", () => {
    const { chosen, pressed } = pressing();

    fireEvent.keyDown(control(), { key: "ArrowDown" });
    fireEvent.keyDown(control(), { key: "End" });

    expect(control().getAttribute("aria-activedescendant")).toBe(
      actions(UNDER)[1]!.id,
    );

    fireEvent.keyDown(control(), { key: "Enter" });

    expect(pressed()).toEqual(["Open repo"]);
    expect(chosen()).toBe("");
  });

  /// And every other control goes on holding none of this: a rule and two rows
  /// under a list of accounts would be a foot with nothing in it.
  it("is drawn nowhere a caller offers none", () => {
    picking();

    expect(actions(UNDER)).toEqual([]);
    expect(opened(UNDER).querySelector('[role="separator"]')).toBeNull();
  });
});

/// And the one question about this control that is not about the control: a
/// listbox is worth its keep only where a row has something a native `<option>`
/// cannot hold, so which modules draw one is written down here rather than left
/// to spread.
///
/// Two things earn it. A row that carries a **mark** beside its words, which is
/// the four pairing pickers and the profile form's harness type; and a list with
/// rows at its foot that **press** rather than pick, which is the Repo's, where
/// **Create repo** and **Open repo** stand behind the rule — an `<option>` that
/// acted is the bug class this whole module was written against.
describe("where the listbox is drawn at all", () => {
  it("reads every source in the viewer", () => {
    expect(Object.keys(SOURCES).length).toBeGreaterThan(10);
  });

  /// Every other choice in the app stays a native `<select>` — the branches, the
  /// merge strategy — because a control the app draws itself has to be given the
  /// keyboard, the roles and the tap targets back, and none of those rows has
  /// either reason to ask for it.
  /// Read for the element rather than for the word: the browsing path field
  /// names this control in its own header, having borrowed the chrome and the
  /// keyboard off it, and naming one is not drawing one.
  it("is drawn by the three modules whose rows have earned it", () => {
    const drawing = Object.entries(SOURCES)
      .filter(([path]) => path !== "../src/picking.tsx")
      .filter(([, source]) => /<Listbox\b/.test(source))
      .map(([path]) => path)
      .sort();

    expect(drawing).toEqual([
      "../src/profiles/ProfileList.tsx",
      "../src/workbench/Setup.tsx",
      "../src/workbench/Steer.tsx",
    ]);
  });
});

