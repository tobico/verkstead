//! The window's own drag region, which is every pane's head.
//!
//! The app's window has no title bar (ADR-0020), so what moves it is the bar at
//! the top of whatever page is open — and every one of those bars is
//! `PaneHead`, so the region is one rule in one stylesheet rather than a thing
//! each pane was told about. What is asked here is that rule and its exception,
//! against the controls a head really carries.
//!
//! **Read out of the stylesheet rather than written down again.** jsdom drops
//! `-webkit-app-region` on the floor — it is Chromium's own property, and
//! nothing in `cssstyle` has heard of it — so `getComputedStyle` answers with
//! nothing and there is no cascade to query. So the sheet is read as text, the
//! way `diagrams.test.ts` reads the palette, and the selector the rule is
//! written over is the selector the elements are put to: a rule that stopped
//! covering a head's buttons fails here, and so does one narrowed to a list of
//! the controls that happen to exist today.
//!
//! **And the whole of what a browser sees of it is nothing.** The property is
//! inert outside a frameless Electron window, and there is no branch anywhere
//! near it: the head is the same markup with the app's bridge on the window and
//! without one, which is what the last case pins.

import { faGear } from "@fortawesome/free-solid-svg-icons";
import { fireEvent, render } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import { IconButton } from "../src/IconButton";
import { Menu } from "../src/Menu";
import { Switch } from "../src/Switch";
import { NAME, type Bridge } from "../src/settings/bridge";
import { PaneHead } from "../src/workbench/PaneHead";
import styles from "../src/workbench/PaneHead.module.css";
// The same sheet as text. The two declarations under test are Chromium's own
// and jsdom parses neither, so what says which selector they are written over is
// the source of the rule.
import sheet from "../src/workbench/PaneHead.module.css?raw";

afterEach(() => {
  vi.unstubAllGlobals();
});

/// Every rule in the sheet with its comments taken out: the selector as written,
/// and what it declares.
///
/// Flat, so the `@media` block's own brace leaves a fragment behind rather than
/// a rule — which is harmless, because nothing is looked up here except by the
/// property it declares, and the two properties under test are declared once
/// each.
function rules(): { selects: string; declares: string }[] {
  return sheet
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("}")
    .flatMap((block) => {
      const parts = block.split("{");
      if (parts.length < 2) {
        return [];
      }

      return [
        {
          selects: parts.at(-2)!.trim().replace(/\s+/g, " "),
          declares: parts.at(-1)!.trim(),
        },
      ];
    });
}

/// The selector `property: value` is declared over, and the insistence that it
/// is declared exactly once: two rules saying where the window is dragged by
/// would be two answers to one question.
function declaring(property: string, value: string): string {
  const declared = rules()
    .filter((rule) => rule.declares.includes(`${property}: ${value};`))
    .map((rule) => rule.selects);

  expect(declared).toHaveLength(1);

  return declared[0]!;
}

/// A selector out of the stylesheet as the document has it: every plain class
/// name in it swapped for the hashed one the bundler gave the module, which is
/// what the component draws and so what an element can be put to.
function asDrawn(selector: string): string {
  return selector.replace(/\.([\w-]+)/g, (_, name: string) => {
    const hashed = (styles as Record<string, string | undefined>)[name];

    expect(hashed, `the sheet selects .${name} and does not define it`).toBeTypeOf(
      "string",
    );

    return `.${hashed}`;
  });
}

/// Where the window is dragged by, and what is excepted from it — the two
/// selectors, as the document has them.
const DRAGS = asDrawn(declaring("-webkit-app-region", "drag"));
const EXCEPTED = asDrawn(declaring("-webkit-app-region", "no-drag"));

/// A head with one of everything a head carries in it: the way back out, an
/// icon button, a ⋯ menu, a switch, and a group of buttons like the
/// Transcript/Screen pair `Output.tsx` draws.
///
/// The real components rather than stand-ins for them, because what the
/// exception has to cover is the markup they actually render — a switch is a
/// `<label>` around a box, and a menu is a trigger with a card under it.
function aHead() {
  return render(() => (
    <PaneHead back={{ to: "Timeline", go: () => {} }} title="Code">
      <IconButton of={faGear} label="Settings" open={false} press={() => {}} />

      <Menu class="example" label="Editor settings" name="Editor settings" mark>
        {() => (
          <button type="button" role="menuitem">
            Word wrap
          </button>
        )}
      </Menu>

      <Switch label="Record" on={false} flip={() => {}} />

      <div role="group" aria-label="How to read this session">
        <span aria-hidden="true" />
        <button type="button" aria-pressed="true">
          Transcript
        </button>
      </div>

      <a href="/somewhere">Somewhere</a>
    </PaneHead>
  ));
}

/// The head itself out of whatever was mounted.
function headOf(container: ParentNode): HTMLElement {
  const head = container.querySelector<HTMLElement>(`.${styles.head}`);

  if (!head) {
    throw new Error("no pane head was drawn");
  }

  return head;
}

/// Whether the window is dragged by this element: whether it stands in the
/// region at all, and whether the exception reaches it.
///
/// `closest` rather than `matches` for the first half, because the elements
/// asked about are the controls *inside* a head as well as the head itself — and
/// a control that only failed to be the head would answer *no* however badly the
/// exception were written.
function drags(element: Element): boolean {
  return element.closest(DRAGS) !== null && !element.matches(EXCEPTED);
}

describe("the head", () => {
  /// The one thing this task turns on, and the reason no pane was edited for
  /// it: the bar every pane's head is drawn as is the bar the window moves by.
  it("is the region the window is dragged by", () => {
    const { container } = aHead();

    expect(drags(headOf(container))).toBe(true);
  });

  /// Which the title is carried along by rather than excepted — a head's
  /// heading is words rather than a control, and the whole row moves the
  /// window.
  it("drags by its title as readily as by its paper", () => {
    const { container } = aHead();
    const title = container.querySelector("h1")!;

    expect(title.matches(EXCEPTED)).toBe(false);
  });

  /// And its text stops taking a selection, which is the price ADR-0020 settled
  /// this with: a bar a pointer sweeps a selection across cannot also be the
  /// bar the window moves by. jsdom knows this property, so it is the one thing
  /// here the cascade can be asked about.
  it("no longer takes a selection", () => {
    const { container } = aHead();

    expect(getComputedStyle(headOf(container)).userSelect).toBe("none");
  });
});

describe("what a head excepts", () => {
  /// The way out of the pane, which is the one to look at twice: it is a button
  /// across the whole width of the row, so a head whose way out dragged the
  /// window would be the loudest way to get this wrong.
  it("the way back out of the pane", () => {
    const { container } = aHead();
    const back = container.querySelector(`.${styles.back}`)!;

    expect(back.textContent).toBe("← Timeline");
    expect(drags(back)).toBe(false);
  });

  /// Every control the head was handed, by what a screen reader calls it: the
  /// gear, the ⋯ trigger, the switch and its box, the way on, and a link.
  it("every control standing in it", () => {
    const { container, getByLabelText, getByText } = aHead();

    const controls = [
      getByLabelText("Settings"),
      getByLabelText("Editor settings"),
      getByText("Record").closest("label")!,
      container.querySelector('input[role="switch"]')!,
      container.querySelector('[role="group"]')!,
      getByText("Transcript"),
      getByText("Somewhere"),
    ];

    for (const control of controls) {
      expect(drags(control)).toBe(false);
    }
  });

  /// And the card a menu drops inside a head, which is not a control the caller
  /// put there at all: it is the menu's own, it hangs over the head it was
  /// opened from, and a window that moved when somebody reached for a row would
  /// be a menu nobody could use.
  it("the card a menu drops in it", () => {
    const { container, getByLabelText, getByRole } = aHead();

    fireEvent.click(getByLabelText("Editor settings"));

    const drop = getByRole("menu");
    expect(headOf(container).contains(drop)).toBe(true);
    expect(drags(drop)).toBe(false);
    expect(drags(getByRole("menuitem"))).toBe(false);
  });

  /// The exception is written over what a head holds rather than over the list
  /// of things it holds today, which is what makes the next control added to a
  /// head one nobody has to remember: anything focusable is excepted by being
  /// focusable.
  it("anything focusable somebody adds later", () => {
    const { container } = render(() => (
      <PaneHead title="Later">
        <div tabIndex={0}>Something nobody has written yet</div>
      </PaneHead>
    ));

    expect(drags(container.querySelector("[tabindex]")!)).toBe(false);
  });
});

describe("a browser", () => {
  /// The whole of what the drag region is: two declarations in a stylesheet, one
  /// of them a property only a frameless Electron window has ever read. So there
  /// is nothing in the markup for a browser to behave differently about — no
  /// attribute, no handler, and nothing the page had to be told.
  it("is handed a head with nothing about a window on it", () => {
    const { container } = aHead();
    const head = headOf(container);

    expect(head.getAttribute("draggable")).toBeNull();
    expect(head.getAttribute("style")).toBeNull();

    // And the region is the head and nothing wider: the rule is written over the
    // class this component draws, so no other bar on the page is in it.
    expect(DRAGS).toBe(`.${styles.head}`);
  });

  /// And the same head either way, which is what says no page ever asks. The
  /// bridge is what tells the app from a browser everywhere it matters — see
  /// `src/settings/bridge.ts` — and this is the one piece of the decorations
  /// that does not consult it, because a rule a browser ignores needs no
  /// guarding.
  it("is handed the same head the app is", () => {
    const browser = aHead();
    const drawn = headOf(browser.container).outerHTML;
    browser.unmount();

    vi.stubGlobal(NAME, {
      platform: "linux",
      settings: () => Promise.resolve({ whenClosed: "tray", trayIcon: true }),
      set: () => Promise.resolve({ whenClosed: "tray", trayIcon: true }),
      logs: () => Promise.resolve(),
      startup: () => Promise.resolve({ possible: true, on: false }),
      register: () => Promise.resolve({ possible: true, on: false }),
    } satisfies Bridge);

    const app = aHead();

    expect(headOf(app.container).outerHTML).toBe(drawn);
  });
});
