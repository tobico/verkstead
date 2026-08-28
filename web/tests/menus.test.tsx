//! The one dropdown menu, tested where it lives rather than three times over on
//! three pages.
//!
//! What every menu in the UI owes the human is here — it opens, it takes the
//! Escape key, a press away from it takes it back, and the focus goes home
//! afterwards — and it is asserted against `Menu` itself, because that is now
//! the only place any of it is written. What the pages' own suites still carry
//! is the half that is theirs: that the rows do what they say, and that each of
//! the three menus is drawn through this component at all.
//!
//! The one opened by a right-click is the same component's other shape, and
//! what is asked of it here is what is different: it has no trigger, it comes
//! down where the pointer was, and it stays inside the window.

import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it } from "vitest";

import { ContextMenu, Menu, Nested } from "../src/Menu";
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
import actions from "../src/workbench/Actions.module.css?raw";
import sidebar from "../src/workbench/Conversations.module.css?raw";

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

  /// A disabled trigger still says what it says — a badge mid-lock is the
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

/// A menu with a level below its first: the row that opens one, the rows it
/// holds, and the way back out.
///
/// The whole of what makes it a *level* rather than a second menu is that none
/// of the chrome is doubled — one card, one backdrop, one Escape, one focus
/// given back — so that is what is asked of it here.
describe("a nested level of a menu", () => {
  /// A menu with an ordinary row and a row that opens a level, and a count of
  /// how many times that level has been built.
  function mountNested(): { container: HTMLElement; built: () => number } {
    let built = 0;

    const { container } = render(() => (
      <Menu class="example" label="Example actions" name="Example actions" trigger="⋯">
        {() => (
          <>
            <button type="button" role="menuitem" class="row">
              Do the thing
            </button>
            <Nested label="More things">
              {() => {
                built += 1;
                return (
                  <button type="button" role="menuitem" class="deeper">
                    Do the deeper thing
                  </button>
                );
              }}
            </Nested>
          </>
        )}
      </Menu>
    ));

    return { container, built: () => built };
  }

  /// The row that opens one, which is drawn as every other row is and says of
  /// itself that it opens a menu.
  function nested(container: ParentNode): HTMLButtonElement {
    return container.querySelector<HTMLButtonElement>(`.${menu.nested}`)!;
  }

  /// And the way back out of it, at the top of the rows it holds.
  function back(container: ParentNode): HTMLButtonElement | null {
    return container.querySelector<HTMLButtonElement>(`.${menu.back}`);
  }

  it("draws its row like any other, and opens rather than does", () => {
    const { container } = mountNested();
    fireEvent.click(trigger(container));

    expect(nested(container).textContent).toContain("More things");
    expect(nested(container).getAttribute("role")).toBe("menuitem");
    expect(nested(container).getAttribute("aria-haspopup")).toBe("menu");
  });

  /// The level replaces what the card was showing rather than standing beside
  /// it: the rows above it are a level up, not a heading over what is open.
  it("shows its rows in place of the ones it was opened from", () => {
    const { container } = mountNested();
    fireEvent.click(trigger(container));

    fireEvent.click(nested(container));

    expect(container.querySelector(".deeper")).toBeTruthy();
    expect(container.querySelector(".row")).toBeNull();
    expect(nested(container)).toBeNull();

    // One card and one wash under it, however deep the human has walked.
    expect(container.querySelectorAll(`.${menu.drop}`)).toHaveLength(1);
    expect(container.querySelectorAll(`.${menu.backdrop}`)).toHaveLength(1);
  });

  /// Built when the level is opened rather than when the menu is: a level
  /// listing something the caller is still reading should read it when the
  /// human asks for it.
  it("builds the level when it is opened, not when the menu is", () => {
    const { container, built } = mountNested();

    fireEvent.click(trigger(container));
    expect(built()).toBe(0);

    fireEvent.click(nested(container));
    expect(built()).toBe(1);
  });

  /// The way back, and the focus with it: the row that opened the level is
  /// where the hand was, and it is a new element by the time it is there again.
  it("goes back to the level it was opened from, and gives that row the focus", async () => {
    const { container } = mountNested();
    fireEvent.click(trigger(container));
    fireEvent.click(nested(container));

    expect(back(container)!.textContent).toContain("More things");

    fireEvent.click(back(container)!);

    expect(container.querySelector(".row")).toBeTruthy();
    expect(back(container)).toBeNull();
    await waitFor(() => expect(document.activeElement).toBe(nested(container)));
  });

  /// And on the way in, so a hand on the keyboard is inside the card rather
  /// than at the top of the page.
  it("takes the focus into the level as it opens", async () => {
    const { container } = mountNested();
    fireEvent.click(trigger(container));

    fireEvent.click(nested(container));

    await waitFor(() => expect(document.activeElement).toBe(back(container)));
  });

  /// One Escape handler, and it belongs to the menu: from a level down it takes
  /// the whole menu back and puts the focus on the trigger, exactly as it does
  /// from the first level.
  it("closes the whole menu on escape, from a level down", async () => {
    const { container } = mountNested();
    fireEvent.click(trigger(container));
    fireEvent.click(nested(container));

    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() => expect(drop(container)).toBeNull());
    expect(document.activeElement).toBe(trigger(container));
  });

  /// And a press away from it lands on the one backdrop, from a level down as
  /// from the first.
  it("closes the whole menu on a press outside it, from a level down", async () => {
    const { container } = mountNested();
    fireEvent.click(trigger(container));
    fireEvent.click(nested(container));

    fireEvent.click(container.querySelector(`.${menu.backdrop}`)!);

    await waitFor(() => expect(drop(container)).toBeNull());
  });

  /// A menu that came back down where it was left would be showing a level the
  /// human closed their way out of.
  it("comes back down at its first level", () => {
    const { container } = mountNested();
    fireEvent.click(trigger(container));
    fireEvent.click(nested(container));

    fireEvent.click(container.querySelector(`.${menu.backdrop}`)!);
    fireEvent.click(trigger(container));

    expect(container.querySelector(".row")).toBeTruthy();
    expect(container.querySelector(".deeper")).toBeNull();
  });

  /// The nesting belongs to the drop rather than to either shape of menu, so
  /// the one opened by a right-click has it without a line of its own.
  it("is the right-click menu's too", () => {
    const [at, setAt] = createSignal<{ x: number; y: number } | null>(null);

    const { container } = render(() => (
      <ContextMenu class="example" at={at()} close={() => setAt(null)}>
        {() => (
          <Nested label="More things">
            {() => (
              <button type="button" role="menuitem" class="deeper">
                Do the deeper thing
              </button>
            )}
          </Nested>
        )}
      </ContextMenu>
    ));

    setAt({ x: 10, y: 10 });
    fireEvent.click(nested(container));

    expect(container.querySelector(".deeper")).toBeTruthy();
    expect(container.querySelectorAll(`.${menu.drop}`)).toHaveLength(1);
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
    expect(actions).not.toContain(".conversationActions > button");
  });
});

/// The same menu opened by a right-click: no trigger, no anchor, and the card
/// put where the pointer was rather than under a button.
describe("a context menu", () => {
  /// One with a row in it, and the way to put the pointer somewhere.
  function mountAt(): {
    container: HTMLElement;
    open: (x: number, y: number) => void;
    close: () => void;
    closings: () => number;
  } {
    const [at, setAt] = createSignal<{ x: number; y: number } | null>(null);
    let closings = 0;

    const { container } = render(() => (
      <ContextMenu
        class="example"
        name="Example actions"
        at={at()}
        close={() => {
          closings += 1;
          setAt(null);
        }}
      >
        {() => (
          <button type="button" role="menuitem" class="row">
            Do the thing
          </button>
        )}
      </ContextMenu>
    ));

    return {
      container,
      open: (x, y) => setAt({ x, y }),
      close: () => setAt(null),
      closings: () => closings,
    };
  }

  it("drops nothing while nothing has been right-clicked", () => {
    const { container } = mountAt();

    expect(drop(container)).toBeNull();
    expect(container.querySelector(`.${menu.trigger}`), "and nothing to press").toBeNull();
  });

  it("drops its rows where the pointer was", () => {
    const { container, open } = mountAt();

    open(140, 260);

    const dropped = drop(container)!;
    expect(dropped.textContent).toBe("Do the thing");
    expect(dropped.style.left).toBe("140px");
    expect(dropped.style.top).toBe("260px");
    expect(dropped.classList).toContain(menu.pointed);
  });

  /// Fixed to the window, because the coordinates a pointer event carries are
  /// the window's own.
  it("is fixed to the window rather than hung off the page", () => {
    expect(block(".drop.pointed")).toContain("position: fixed;");
    expect(block(".drop.pointed")).toContain("right: auto;");
  });

  /// A right-click near the bottom edge would otherwise drop a menu mostly
  /// below it, on a page that does not scroll to reach it.
  it("keeps the card inside the window", () => {
    const { container, open } = mountAt();

    // jsdom lays nothing out, so the card is told how big it is.
    const measured = 300;
    Object.defineProperty(HTMLDivElement.prototype, "getBoundingClientRect", {
      configurable: true,
      value(this: HTMLElement) {
        return this.classList.contains(menu.drop!)
          ? ({ width: measured, height: measured } as DOMRect)
          : ({ width: 0, height: 0 } as DOMRect);
      },
    });

    try {
      open(window.innerWidth - 10, window.innerHeight - 10);
    } finally {
      delete (HTMLDivElement.prototype as Partial<HTMLDivElement>)
        .getBoundingClientRect;
    }

    const dropped = drop(container)!;
    expect(Number.parseInt(dropped.style.left, 10)).toBe(
      window.innerWidth - measured - 8,
    );
    expect(Number.parseInt(dropped.style.top, 10)).toBe(
      window.innerHeight - measured - 8,
    );
  });

  it("says close on escape", async () => {
    const { container, open, closings } = mountAt();
    open(10, 10);

    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() => expect(drop(container)).toBeNull());
    expect(closings()).toBe(1);
  });

  it("says close on a press away from it", async () => {
    const { container, open, closings } = mountAt();
    open(10, 10);

    fireEvent.click(container.querySelector(`.${menu.backdrop}`)!);

    await waitFor(() => expect(drop(container)).toBeNull());
    expect(closings()).toBe(1);
  });

  /// A second right-click is a press away from this menu like any other, and
  /// the browser's own menu is not what the hand is asking for.
  it("takes itself back on a right-click away from it, and the browser's menu with it", async () => {
    const { container, open } = mountAt();
    open(10, 10);

    const away = fireEvent.contextMenu(container.querySelector(`.${menu.backdrop}`)!);

    expect(away, "the browser's own menu was taken off the press").toBe(false);
    await waitFor(() => expect(drop(container)).toBeNull());
  });

  /// As every menu's are: built on the way open and thrown away on the way
  /// shut.
  it("builds its rows afresh on every opening", () => {
    const { container, open, close } = mountAt();

    open(10, 10);
    const first = container.querySelector(".row");
    close();
    open(20, 20);

    expect(container.querySelector(".row")).not.toBe(first);
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
      ['.conversationActions > [role="menu"]', actions],
      ['.standing > [role="menu"]', standing],
    ];

    for (const [caller, sheet] of callers) {
      expect(block(caller, sheet)).not.toContain("box-shadow");
    }
  });
});
