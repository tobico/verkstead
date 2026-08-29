//! The pressable icon, tested where it lives: one button drawn in more than one
//! pane, and the state it carries about itself.
//!
//! What is asked here is the whole of what the component owes a caller — that
//! it draws the shape it was handed, that it is named by a word rather than by
//! the shape, that it presses, and that the one whose pane is open says so the
//! way the open card beside it does. What each caller *does* with a press is
//! that caller's own suite: the gear at the head of the conversations is
//! `workbench.test.tsx`.

import { fireEvent, render } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";

import { faGear, faPlus } from "@fortawesome/free-solid-svg-icons";

// The card this stands beside in the metaphor, for the one assertion that is
// about the two of them together: an open icon and an open card are the same
// news, so they are said in the same fill.
import cardCss from "../src/CardButton.module.css?raw";
import { IconButton } from "../src/IconButton";
import styles from "../src/IconButton.module.css";
import stylesheet from "../src/IconButton.module.css?raw";

/// One of them, open or not, counting what its presses come to.
function mount(open = false): {
  button: HTMLButtonElement;
  presses: () => number;
} {
  let presses = 0;

  const { container } = render(() => (
    <IconButton
      of={faGear}
      label="Settings"
      open={open}
      press={() => (presses += 1)}
    />
  ));

  return {
    button: container.querySelector<HTMLButtonElement>("button")!,
    presses: () => presses,
  };
}

describe("a pressable icon", () => {
  it("draws the shape it was handed", () => {
    const { container } = render(() => (
      <IconButton of={faPlus} label="Add a repo" open={false} press={() => {}} />
    ));

    expect(container.querySelector("path")!.getAttribute("d")).toBe(
      faPlus.icon[4],
    );
  });

  /// An icon says nothing when it is read aloud, so the button is named by the
  /// word handed in — and the shape inside it is hidden, rather than being a
  /// second thing for a screen reader to find and have nothing to say about.
  it("is named by its label rather than by its icon", () => {
    const { button } = mount();

    expect(button.getAttribute("aria-label")).toBe("Settings");
    expect(button.querySelector("svg")!.getAttribute("aria-hidden")).toBe(
      "true",
    );
  });

  it("presses", () => {
    const { button, presses } = mount();

    fireEvent.click(button);

    expect(presses()).toBe(1);
  });

  /// The one state it draws about itself, and the caller's to hold: what is
  /// open is a fact about where the human is rather than about this button.
  it("says whether its pane is open", () => {
    expect(mount().button.getAttribute("aria-pressed")).toBe("false");

    const { button } = mount(true);
    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(button.classList).toContain(styles.open);
  });

  /// No toggle, which is the card's metaphor and not a menu's: pressing the
  /// open one makes the same press again — the press that opened it — and the
  /// button does not close anything by being pressed.
  it("makes the same press again when it is already open", () => {
    const { button, presses } = mount(true);

    fireEvent.click(button);

    expect(presses()).toBe(1);
    expect(button.classList).toContain(styles.open);
  });

  /// And it says it in the fill the open card says it in. Two things selected in
  /// one pane should not be two answers to the same question, and jsdom lays
  /// nothing out to ask it any other way.
  it("paints the open one as the open card is painted", () => {
    const at = stylesheet.indexOf("\n.open {");
    expect(at, "expected the sheet to hold the open button's own rule").toBeGreaterThan(-1);
    expect(stylesheet.slice(at, stylesheet.indexOf("\n}", at))).toContain(
      "background: var(--card);",
    );
    expect(cardCss).toContain("background: var(--card);");
  });
});
