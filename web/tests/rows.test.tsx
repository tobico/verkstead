//! The press a row that wraps its control answers to — `rowPress` in
//! `src/rows.ts`, asked through the two components that are nothing but such a
//! row: the switch and the settings page's checkbox.
//!
//! Asked here rather than at every site that draws one, because the arrangement
//! is the handler's rather than each caller's: what a caller owns is what a press
//! means, and its own suite is where that is asked. The rows a Question Set is
//! answered by are in `answering.test.tsx`, `choosing.test.tsx` and
//! `ending.test.tsx`; the desktop positions in `desktop.test.tsx`; the steer
//! targets in `workbench.test.tsx`.
//!
//! **The gesture at issue is a press that slid**, which is the one a browser
//! stops forwarding to the control — see `sliding.ts` for the whole of what that
//! means and how it is staged. A row that answered only the forwarded click did
//! nothing at all for it, which is what was reported of an answer and was true of
//! every row in the app.

import { fireEvent, render } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";

import { Check } from "../src/Check";
import { rowPress } from "../src/rows";
import { Switch } from "../src/Switch";
import { slidPress } from "./sliding";

/// Every component under `src/`, as its source — for the sweep at the foot of
/// this file, which is about which rows carry the handler rather than about what
/// any of them draws.
const COMPONENTS = import.meta.glob("../src/**/*.tsx", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/// One row, drawn as whichever of the two components is asked for, counting what
/// its presses come to.
///
/// `flip` is handed the state being asked for, so what is recorded is what each
/// press asked the caller to make it — which is the whole of what a row says.
function mount(
  as: typeof Switch | typeof Check,
  { on = false, disabled = false } = {},
) {
  const asked: boolean[] = [];
  const { container } = render(() =>
    as({
      label: "Show the archived ones",
      on,
      disabled,
      flip: (into: boolean) => asked.push(into),
    }),
  );

  const row = container.querySelector("label")!;

  return {
    row,
    box: row.querySelector("input")!,
    words: row.querySelector("span")!,
    asked: () => asked,
  };
}

for (const [name, as] of [
  ["the switch", Switch],
  ["the settings checkbox", Check],
] as const) {
  describe(`${name} as a row to press`, () => {
    it("flips on a press that slid over its words", () => {
      const { words, asked } = mount(as);

      // The press the browser forwards nothing for, and the row used to do
      // nothing about: the whole row is offered as the thing to press, so the
      // whole row has to answer one.
      slidPress(words);

      expect(asked()).toEqual([true]);
    });

    it("asks for the opposite of where it stands, whichever way that is", () => {
      const { words, asked } = mount(as, { on: true });

      slidPress(words);

      expect(
        asked(),
        "a row that is on is a row being asked to go off",
      ).toEqual([false]);
    });

    it("counts a still press once, though the label forwards one as well", () => {
      const { words, asked } = mount(as);

      // A still press is forwarded to the box, and that forwarded click is
      // indistinguishable from the human pressing the box itself. Cancelling the
      // forwarding is what keeps one gesture to one flip — two would ask for the
      // flip and then ask for it back.
      fireEvent.click(words);

      expect(asked()).toEqual([true]);
    });

    it("counts a press on the box itself once, like a press anywhere else", () => {
      const { box, asked } = mount(as);

      fireEvent.click(box);

      expect(asked()).toEqual([true]);
    });

    it("leaves the keyboard on the box the press was for", () => {
      const { box, words } = mount(as);

      // What the forwarding did before the row took the press off it. Without
      // it a press would leave focus wherever it had been, so the space bar
      // would flip something else.
      slidPress(words);

      expect(document.activeElement).toBe(box);
    });

    it("does not flip a row that will not take one", () => {
      const { words, asked } = mount(as, { disabled: true });

      // The browser refuses a disabled control whatever a label forwards it, so
      // a row answering for itself would otherwise be the way round that
      // refusal — and a disabled row is one the caller has said no to.
      slidPress(words);
      fireEvent.click(words);

      expect(asked()).toEqual([]);
    });
  });
}

/// What the row does *not* answer: a press that landed on something inside it
/// with a press of its own to answer.
///
/// A `<label>` forwards nothing from interactive content, so a link inside a row
/// opened and picked nothing long before the row answered its own click. What
/// would take that away is not the answering but the cancelling: the row cancels
/// the forwarding, and a blanket cancel takes the link's own default with it.
///
/// Asked of the handler rather than of a component, because no row in the app
/// draws a link today and the one that will is a Question Set's Options — the
/// agent's markdown, which the server renders links in on purpose. `answering`'s
/// suite asks it of that row; this asks it of the arrangement.
describe("a press that landed on interactive content inside the row", () => {
  /// A row holding whatever markup is handed to it, and a count of what its
  /// presses came to.
  function around(inside: string) {
    const acted: number[] = [];
    const { container } = render(() => (
      <label onClick={rowPress(() => acted.push(1))}>
        <input type="checkbox" />
        <span innerHTML={inside} />
      </label>
    ));

    return { container, acted: () => acted.length };
  }

  /// A press, and whether anything was left of its default — which is the whole
  /// of what a link needs to open, and the one thing a test environment with no
  /// navigation of its own can ask about.
  function press(on: Element): boolean {
    const click = new MouseEvent("click", { bubbles: true, cancelable: true });
    on.dispatchEvent(click);
    return !click.defaultPrevented;
  }

  it("leaves a link in the row to open, and picks nothing", () => {
    const { container, acted } = around(
      'see <a href="https://example.com/pr">the pull request</a>',
    );

    expect(press(container.querySelector("a")!), "the link still opens").toBe(
      true,
    );
    expect(acted(), "and the row is not what was pressed").toBe(0);
  });

  it("answers a press on the words beside that link as it always did", () => {
    const { container, acted } = around(
      'see <a href="https://example.com/pr">the pull request</a>',
    );

    expect(press(container.querySelector("span")!)).toBe(false);
    expect(acted()).toBe(1);
  });

  it("leaves an anchor that goes nowhere to the row, as the standard does", () => {
    const { container, acted } = around("an <a>anchor</a> with no href");

    // Interactive content is an `a` *with* an `href`. One without is a name for
    // a place on the page, and a row holding one is a row like any other.
    expect(press(container.querySelector("a")!)).toBe(false);
    expect(acted()).toBe(1);
  });
});

/// And every row in the app that wraps a control is one of these, whether or not
/// anybody thought of it while reading this file.
///
/// The sweep is here because the mistake is one of omission: the arrangement is
/// invisible in the markup — a `<label>` round a control looks pressable whether
/// or not anything answers the press — and the browser withholds the forwarding
/// it used to ride on only when a hand slides, which nothing in a test
/// environment does. So the next row written the old way would be a row that
/// looks right, reads right, passes everything, and quietly does nothing for a
/// press that was not perfectly still. Eleven of them were, and four of those
/// were missed on a first reading of the same grep this does properly.
///
/// Read off the source rather than off a rendering, because what is being asked
/// is about every row there is and not about the handful a test happens to mount.
describe("every row in the app that wraps a control", () => {
  /// One `<label>` that has a checkbox or a radio inside it, which is what makes
  /// it a row rather than a field's name: its attributes, and the markup it
  /// wraps. Labels do not nest, so the shallow match is the whole of it.
  const ROWS = /<label\b([^>]*)>([\s\S]*?)<\/label>/g;

  /// The comments out, as `hovering.test.ts` takes them out of a stylesheet and
  /// for exactly its reason: the prose about this arrangement mentions `<label`
  /// rather more often than the markup does, and one of those sentences was
  /// read as a row with the handler missing.
  ///
  /// **A comment's newlines stay behind it**, so what is counted below is the
  /// line of the file rather than the line of this reading of it — a block
  /// comment spanning fifteen lines would otherwise move every row under it
  /// fifteen lines up, and the address this test fails with is the whole of
  /// what it is for. A line comment keeps its newline already, the match
  /// stopping at the end of the line.
  const bare = (source: string): string =>
    source
      .replace(/\/\*[\s\S]*?\*\//g, (comment) => comment.replace(/[^\n]/g, ""))
      .replace(/^[ \t]*\/\/.*$/gm, "");

  /// A control that can never be pressed, which is a record of a tick rather
  /// than a tick — `disabled` written bare, with no expression behind it to
  /// change its mind. The browser refuses one whatever a label forwards it, so
  /// there is no press for a row to answer and nothing for it to carry.
  const NEVER = /\bdisabled\s*(?=\/?>|\s[a-zA-Z{])/;

  const wrapping = Object.entries(COMPONENTS).flatMap(([file, whole]) => {
    const source = bare(whole);

    return [...source.matchAll(ROWS)]
      .filter((row) => /type="(checkbox|radio)"/.test(row[2]!))
      .map((row) => ({
        where: `${file.replace("../src/", "")}:${
          source.slice(0, row.index).split("\n").length
        }`,
        pressed: /onClick=\{rowPress\(/.test(row[1]!),
        never: NEVER.test(row[2]!),
      }));
  });

  it("finds them, so a sweep that found none cannot pass", () => {
    expect(wrapping.length).toBeGreaterThan(10);
  });

  it("names each row at the line the file really holds it on", () => {
    // What this test fails with is an address, and an address is worth what it
    // opens. Counting the line off a reading with the comments squeezed out of
    // it named a line of a file nobody has — sixty-nine lines out in the one
    // that holds five of the rows.
    expect(
      wrapping
        .filter(({ where }) => {
          const [file, line] = where.split(":");
          const source = COMPONENTS[`../src/${file}`]!.split("\n");

          return !source[Number(line) - 1]?.includes("<label");
        })
        .map((row) => row.where),
      "each of these addresses a line that does not open a row",
    ).toEqual([]);
  });

  it("answers its own press, or is a tick nobody can press", () => {
    expect(
      wrapping
        .filter((row) => !row.pressed && !row.never)
        .map((row) => row.where),
      "each of these wraps a control and leaves the press to the click a label " +
        "forwards — which a browser withholds as soon as the pointer moves. " +
        "Hang `rowPress` on the label, and give the row `cursor: pointer` and " +
        "`user-select: none`",
    ).toEqual([]);
  });
});
