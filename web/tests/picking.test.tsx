//! The listbox the app draws for itself: what the keyboard does to it, what a
//! screen reader is told about it, and the mark each row carries.
//!
//! A native `<select>` arrives with all of this and a control drawn out of
//! ordinary elements arrives with none of it, so the whole of what was given
//! back is asserted here rather than trusted: the workbench is answered from a
//! phone and from a keyboard as readily as from a mouse, and every one of the
//! five choices this control stands in for is a choice about who runs somebody's
//! work.
//!
//! Driven straight rather than through a page — what is asked is the control's
//! own, so no query, no card and no modal is in the way of the answer. Where a
//! page's own picker is the subject, the test is with that page:
//! `workbench.test.tsx` for the four pairing pickers, `profiles.test.tsx` for
//! the profile form's harness type, and `surviving.test.tsx` for what a re-read
//! leaves of a choice.

import { fireEvent, render, screen } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it } from "vitest";

import type { AgentType } from "../src/agents";
import { Listbox } from "../src/picking";
import styles from "../src/picking.module.css";
import css from "../src/picking.module.css?raw";
import claudeMarkFile from "../src/marks/claude-color.svg?raw";
import grokMarkFile from "../src/marks/grok.svg?raw";
import { art, marked } from "./marking";
import {
  expanded,
  offered,
  opened,
  pick,
  picker,
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
/// account at all — which is the shape the grilling and review pickers have.
const ROWS: { value: string; label: string; mark: AgentType | null }[] = [
  { value: ":", label: "No grilling", mark: null },
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

/// The one picker every test here drives.
const UNDER = "Run it under";

/// And the control itself, for the attributes a screen reader reads it by.
const control = () => picker(UNDER);

describe("what the listbox says it is", () => {
  /// The label reaching the control is why it is a `button` rather than a `div`
  /// with a role: only a labelable element is what a `<label for=…>` names, and
  /// every one of the five callers labels its picker that way.
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
      "No grilling",
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
});

/// And the one question about this control that is not about the control: a
/// listbox is worth its keep only where a row has something to draw, so which
/// modules draw one is written down here rather than left to spread.
describe("where the listbox is drawn at all", () => {
  it("reads every source in the viewer", () => {
    expect(Object.keys(SOURCES).length).toBeGreaterThan(10);
  });

  /// Every other choice in the app stays a native `<select>` — the repos, the
  /// branches, the merge strategy — because a control the app draws itself has
  /// to be given the keyboard, the roles and the tap targets back, and none of
  /// those rows has a mark to justify it.
  it("is drawn by the three modules whose rows carry marks", () => {
    const drawing = Object.entries(SOURCES)
      .filter(([path]) => path !== "../src/picking.tsx")
      .filter(([, source]) => /\bListbox\b/.test(source))
      .map(([path]) => path)
      .sort();

    expect(drawing).toEqual([
      "../src/profiles/ProfileList.tsx",
      "../src/workbench/Setup.tsx",
      "../src/workbench/Steer.tsx",
    ]);
  });
});

