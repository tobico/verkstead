//! The one dropdown menu, tested where it lives rather than three times over on
//! three pages.
//!
//! What every menu in the UI owes the human is here — it opens, it takes the
//! Escape key, a press away from it takes it back, and the focus goes home
//! afterwards — and it is asserted against `Menu` itself, because that is now
//! the only place any of it is written. What the pages' own suites still carry
//! is the half that is theirs: that the rows do what they say, and that each of
//! the three menus is drawn through this component at all.

import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";

import { Menu } from "../src/Menu";
// The one menu, both ways: the hashed names to query the page by, and the
// source to read the rules off, jsdom laying nothing out to read them from.
import menu from "../src/Menu.module.css";
import stylesheet from "../src/Menu.module.css?raw";
// The two schemes' palettes, which is where the shadow itself is named.
import tokens from "../src/styles/base.css?raw";
// The callers' paint, each in the module of the component that passes the
// class: a caller's class is hashed, so it reaches this component's parts by
// the elements they are rather than by a name it cannot spell.
import contents from "../src/set/Contents.module.css?raw";
import standing from "../src/set/Standing.module.css?raw";
import sidebar from "../src/workbench/Conversations.module.css?raw";
import timeline from "../src/workbench/Timeline.module.css?raw";

/// A menu with one row in it, which is enough of one to press.
function mount(): { container: HTMLElement; opened: () => number } {
  let openings = 0;

  const { container } = render(() => (
    <Menu
      class="example"
      label="Example actions"
      name="Example actions"
      opening={() => (openings += 1)}
      trigger="⋯"
    >
      {() => (
        <button type="button" role="menuitem" class="row">
          Do the thing
        </button>
      )}
    </Menu>
  ));

  return { container, opened: () => openings };
}

/// The trigger, which is the only part of a closed menu on the page.
function trigger(container: ParentNode): HTMLButtonElement {
  return container.querySelector<HTMLButtonElement>(`.${menu.trigger}`)!;
}

/// What it drops, or nothing where it is closed.
function drop(container: ParentNode): HTMLElement | null {
  return container.querySelector<HTMLElement>(`.${menu.drop}`);
}

describe("a dropdown menu", () => {
  /// Closed, nothing of it is on the page: not hidden, not there. Which is what
  /// standing something behind a menu is for.
  it("drops nothing until it is pressed", () => {
    const { container } = mount();

    expect(trigger(container)).toBeTruthy();
    expect(drop(container)).toBeNull();
    expect(container.querySelector(".row")).toBeNull();
  });

  it("drops its rows when the trigger is pressed", () => {
    const { container } = mount();

    fireEvent.click(trigger(container));

    expect(drop(container)).toBeTruthy();
    expect(container.querySelector(".row")!.textContent).toBe("Do the thing");
  });

  /// Which is what a screen reader is told: a button that opens a menu, the
  /// menu it opens, and whether it is open now.
  it("ties the trigger to what it drops", () => {
    const { container } = mount();
    const button = trigger(container);

    expect(button.getAttribute("aria-haspopup")).toBe("menu");
    expect(button.getAttribute("aria-expanded")).toBe("false");
    expect(button.getAttribute("aria-label")).toBe("Example actions");

    fireEvent.click(button);

    expect(button.getAttribute("aria-expanded")).toBe("true");
    expect(button.getAttribute("aria-controls")).toBe(drop(container)!.id);
    expect(drop(container)!.getAttribute("role")).toBe("menu");
  });

  /// The way out that needs no aim, and the focus back on the button it came
  /// from rather than at the top of the page.
  it("closes on escape, and gives the trigger back the focus", async () => {
    const { container } = mount();
    fireEvent.click(trigger(container));

    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() => expect(drop(container)).toBeNull());
    expect(document.activeElement).toBe(trigger(container));
  });

  /// A press away from it lands on the backdrop rather than on the page, so the
  /// press that takes the menu back cannot also press what was underneath.
  it("closes on a press outside it", async () => {
    const { container } = mount();
    fireEvent.click(trigger(container));

    fireEvent.click(container.querySelector(`.${menu.backdrop}`)!);

    await waitFor(() => expect(drop(container)).toBeNull());
  });

  it("closes when what it was handed to close it is called", async () => {
    let shut = (): void => {};
    const { container } = render(() => (
      <Menu class="example" trigger="⋯" closer={(close) => (shut = close)}>
        {() => <button type="button">Do the thing</button>}
      </Menu>
    ));

    fireEvent.click(trigger(container));
    expect(drop(container)).toBeTruthy();

    shut();

    await waitFor(() => expect(drop(container)).toBeNull());
  });

  /// The rows are built on the way open and thrown away on the way shut, which
  /// is what a row that takes the focus as it appears depends on.
  it("builds its rows afresh on every opening", () => {
    const { container, opened } = mount();

    fireEvent.click(trigger(container));
    const first = container.querySelector(".row");
    fireEvent.click(trigger(container));
    fireEvent.click(trigger(container));

    expect(container.querySelector(".row")).not.toBe(first);
    expect(opened()).toBe(2);
  });

  /// A disabled trigger still says what it says — a badge mid-archive is the
  /// case this is for — and drops nothing.
  it("does not open while its trigger is disabled", () => {
    const { container } = render(() => (
      <Menu class="example" trigger="⋯" disabled>
        {() => <button type="button">Do the thing</button>}
      </Menu>
    ));

    fireEvent.click(trigger(container));

    expect(drop(container)).toBeNull();
  });
});

/// The two ⋯ triggers — the sidebar's and the Conversation's — sit in the same
/// place in their two pane headers and mean the same thing there, so they are
/// one button rather than two painted alike. They were two: a rule each, hashed
/// off a class each, written apart and drifted into two sizes across the
/// divider between the panes. The mark and the paint under it are the menu's
/// now, and neither pane says anything about a trigger at all.
describe("the ⋯ at the head of a pane", () => {
  it("is the menu's own mark, not the caller's", () => {
    const { container } = render(() => (
      <Menu class="example" label="Example actions" mark>
        {() => <button type="button" role="menuitem" class="row" />}
      </Menu>
    ));

    const button = trigger(container);
    expect(button.textContent).toBe("⋯");
    expect(button.classList).toContain(menu.mark);
  });

  it("is painted here, where there is one of it", () => {
    expect(block(".trigger.mark")).toContain("font-size: 1.1rem;");
    expect(block('.trigger.mark[aria-expanded="true"]')).toContain(
      "color: var(--ink);",
    );
  });

  /// The point of moving it: neither pane keeps a trigger of its own to drift
  /// away from the other.
  it("leaves neither pane a button to paint", () => {
    expect(sidebar).not.toContain(".workbenchActions > button");
    expect(timeline).not.toContain(".conversationActions > button");
  });
});

/// What one rule declares, read off a stylesheet by the selector that carries
/// it. Enough to say what a menu is painted with, and no more.
function block(selector: string, sheet: string = stylesheet): string {
  const at = sheet.indexOf(`\n${selector} {\n`);
  expect(at, `expected the stylesheet to hold \`${selector}\``).toBeGreaterThan(
    -1,
  );
  return sheet.slice(at, sheet.indexOf("\n}", at));
}

/// One shadow rather than one apiece, which is the visible half of there being
/// one menu: a menu is drawn over whatever the human was reading, and it should
/// be plain which of the two is in front.
describe("what every menu is drawn with", () => {
  it("stands off the page by the one shared shadow", () => {
    expect(block(".drop")).toContain("box-shadow: var(--lift);");
  });

  /// In both schemes, because the light-mode shadow is invisible on dark paper
  /// and the dark-mode one would be a bruise on light.
  it("defines that shadow for either paper", () => {
    expect(tokens.match(/--lift:/g)).toHaveLength(2);
  });

  /// And a wash under it, so the page behind an open menu is dimmed the way the
  /// page behind the answer-set navigation's list is. The same wash, because
  /// they are the same kind of thing coming down over the same page.
  it("washes the page behind it, as the navigation's list does", () => {
    expect(block(".backdrop")).toContain("background: rgb(0 0 0 / 20%);");
    expect(block(".backdrop", contents)).toContain(
      "background: rgb(0 0 0 / 20%);",
    );
  });

  /// The point of the unification: no menu carries a shadow of its own to drift
  /// away from the shared one.
  it("leaves no menu a shadow of its own", () => {
    const callers: [string, string][] = [
      ['.newConversation > [role="menu"]', sidebar],
      ['.workbenchActions > [role="menu"]', sidebar],
      ['.conversationActions > [role="menu"]', timeline],
      ['.standing > [role="menu"]', standing],
    ];

    for (const [caller, sheet] of callers) {
      expect(block(caller, sheet)).not.toContain("box-shadow");
    }
  });
});
