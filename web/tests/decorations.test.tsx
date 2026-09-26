//! The window's own drag region and the room its controls take: the two halves
//! of a page drawn in a window with no title bar.
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
//! without one, which is what that half's last case pins.
//!
//! The second half is the other thing a window with no title bar does to a page:
//! the platform draws its own controls over a corner of it, so the head at that
//! corner has to keep out from under them. Three questions, and they are asked
//! apart because they are separable — the arithmetic in `src/controls.ts`, which
//! turns the strip the page was left into an inset at each edge and knows nothing
//! about a page; the frame in `src/Panes.tsx`, which carries those insets as
//! variables and reads the rectangle again whenever it moves; and the rules in
//! `src/Panes.module.css` that hand each inset to the head standing at that edge,
//! which are read as text for the reason the drag region's are — jsdom lays out
//! no grid and holds no breakpoint, so what says which head is padded in which
//! layout is the rule.
//!
//! **The stub bridge is what says which platform this is**, which is the one
//! thing a rectangle cannot say for itself: a right-hand inset is Windows' and
//! Linux's controls overlay and a left-hand one is a Mac's traffic lights,
//! because that is where each platform draws them. Nothing in the page branches
//! on it — the same sum answers both — so what the bridge does here is name the
//! machine the shape under test belongs to.

import { faGear } from "@fortawesome/free-solid-svg-icons";
import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CLEAR, insets, reserved, type Area } from "../src/controls";
import { IconButton } from "../src/IconButton";
import { Menu } from "../src/Menu";
import { Panes } from "../src/Panes";
import shell from "../src/Panes.module.css";
// And the frame's stylesheet as text, for the rules that say which head stands
// at an edge of the window: jsdom holds no breakpoint and lays out no grid.
import stylesheet from "../src/Panes.module.css?raw";
import { Switch } from "../src/Switch";
import { ALL_THREE, BESIDE } from "../src/widths";
import { NAME, type Bridge } from "../src/settings/bridge";
import { PaneHead } from "../src/workbench/PaneHead";
import styles from "../src/workbench/PaneHead.module.css";
// The same sheet as text. The two declarations under test are Chromium's own
// and jsdom parses neither, so what says which selector they are written over is
// the source of the rule.
import sheet from "../src/workbench/PaneHead.module.css?raw";

afterEach(() => {
  vi.unstubAllGlobals();
  Reflect.deleteProperty(navigator, "windowControlsOverlay");
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

/// The app's bridge on the window, saying which machine this is.
///
/// Which is the whole of what the bridge is for here: a page inside the app draws
/// what a browser's does, and what the platform decides is where it put the
/// window's controls — see the note at the top of this file.
function theApp(platform: string): void {
  const settings = { whenClosed: "tray", trayIcon: true } as const;
  const registered = { possible: true, on: false };

  vi.stubGlobal(NAME, {
    platform,
    settings: () => Promise.resolve(settings),
    set: () => Promise.resolve(settings),
    logs: () => Promise.resolve(),
    startup: () => Promise.resolve(registered),
    register: () => Promise.resolve(registered),
  } satisfies Bridge);
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

    theApp("linux");

    const app = aHead();

    expect(headOf(app.container).outerHTML).toBe(drawn);
  });
});

/// How wide the window the rectangles below were measured in stood, in the CSS
/// pixels `getTitlebarAreaRect()` answers in.
const ACROSS = 1006;

/// The strip the page is left under the controls overlay: the whole top of the
/// window but the last ninety-seven pixels of it. Measured in the app on Linux
/// while this stage was planned, which is the reading task 03 was written from.
const OVERLAID: Area = { x: 0, width: 909 };

/// And the strip a Mac leaves, the traffic lights being at the other corner: it
/// starts after them and runs to the window's far edge.
const TRAFFIC: Area = { x: 78, width: ACROSS - 78 };

describe("the room the window's controls take", () => {
  /// The arithmetic, and the one thing to keep hold of about it: what the page is
  /// handed is the rectangle *it* was left, so what the controls took is the
  /// sliver beyond the far edge of it.
  it("is whatever lies beyond the strip the page was left", () => {
    expect(insets(OVERLAID, ACROSS)).toEqual({ left: 0, right: 97 });
  });

  /// The same sum with no branch in it, on the platform that puts them at the
  /// other end: what the page is left starts after the traffic lights, so `x` is
  /// the inset and there is nothing at the right-hand edge.
  it("is whatever lies in front of it where a Mac put them there", () => {
    expect(insets(TRAFFIC, ACROSS)).toEqual({ left: 78, right: 0 });
  });

  /// And nothing at all where there is no overlay to ask, which is every browser
  /// and every phone.
  it("is nothing where there is no overlay", () => {
    expect(insets(undefined, ACROSS)).toEqual(CLEAR);
  });

  /// Nor anything negative, which is a head padded backwards. A titlebar area
  /// wider than the window it is in is COSMIC's first answer — the
  /// `geometrychange` a moment later is the true one — and a window nobody has
  /// measured yet is nought across.
  it("is nothing where the strip does not fit the window it is in", () => {
    expect(insets({ x: 0, width: 1200 }, ACROSS)).toEqual(CLEAR);
    expect(insets(OVERLAID, 0)).toEqual(CLEAR);
  });

  /// And the edge nothing was taken from is left unsaid rather than written as
  /// nought: the stylesheet has the nought behind each name, so a frame that
  /// names neither is a frame nothing was taken from.
  it("is written on the frame for the edge it was taken from, and no other", () => {
    expect(reserved(insets(OVERLAID, ACROSS))).toEqual({
      "--controls-right": "97px",
    });
    expect(reserved(insets(TRAFFIC, ACROSS))).toEqual({
      "--controls-left": "78px",
    });
    expect(reserved(CLEAR)).toEqual({});
  });
});

/// The overlay as the frame finds it, on `navigator`: the strip the page is left,
/// and the `geometrychange` that says it has moved.
///
/// Answers with a way to move it, because the rectangle at load is not always the
/// true one — which is a thing about this API rather than a nicety, and so one of
/// the cases.
function overlaid(area: Area | null): (moved: Area) => void {
  let left = area;
  const heard = new Set<() => void>();

  Object.defineProperty(navigator, "windowControlsOverlay", {
    configurable: true,
    value: {
      get visible() {
        return left !== null;
      },
      getTitlebarAreaRect: () => left ?? { x: 0, y: 0, width: 0, height: 0 },
      addEventListener: (_kind: string, listener: () => void) =>
        heard.add(listener),
      removeEventListener: (_kind: string, listener: () => void) =>
        heard.delete(listener),
    },
  });

  return (moved) => {
    left = moved;
    heard.forEach((listener) => listener());
  };
}

/// The frame with a head in each of its three panes, which is how the workbench
/// hands it over.
function theFrame(): HTMLElement {
  const { container } = render(() => (
    <Panes
      pane="details"
      middleLabel="Timeline"
      conversations={<PaneHead title="Conversations" />}
      middle={<PaneHead title="Timeline" />}
      details={<PaneHead title="Details" />}
    />
  ));

  const frame = container.querySelector<HTMLElement>(`.${shell.panes}`);

  if (!frame) {
    throw new Error("no frame was drawn");
  }

  return frame;
}

describe("the frame", () => {
  /// Which is all the frame does with them: what the controls took is a pair of
  /// variables on it, as its column widths are, and the stylesheet is what hands
  /// each of them to the head standing at that edge.
  it("carries the room the controls took, as variables of its own", () => {
    theApp("linux");
    overlaid(OVERLAID);
    vi.stubGlobal("innerWidth", ACROSS);

    const frame = theFrame();

    expect(frame.style.getPropertyValue("--controls-right")).toBe("97px");
    expect(frame.style.getPropertyValue("--controls-left")).toBe("");
  });

  /// And the other platform through the same code: a Mac's traffic lights are at
  /// the top-left, which is where the Wordmark is, so the inset that does the
  /// work there is the left one. There is no Mac here to see it on — stage 06 is
  /// where that is looked at — so what is owed is the arithmetic.
  it("carries a left inset where the platform is a Mac", () => {
    theApp("darwin");
    overlaid(TRAFFIC);
    vi.stubGlobal("innerWidth", ACROSS);

    const frame = theFrame();

    expect(frame.style.getPropertyValue("--controls-left")).toBe("78px");
    expect(frame.style.getPropertyValue("--controls-right")).toBe("");
  });

  /// The rectangle read again, which is not an optimisation: the first reading on
  /// COSMIC gave a titlebar area wider than the window, and the page that read it
  /// once would pad by that on every launch. The same event carries a maximise, an
  /// unmaximise and a resize, which is the overlay going away and coming back.
  it("reads the rectangle again whenever the controls move", async () => {
    theApp("linux");
    vi.stubGlobal("innerWidth", ACROSS);
    const moved = overlaid({ x: 0, width: 1200 });

    const frame = theFrame();
    expect(frame.style.getPropertyValue("--controls-right")).toBe("");

    moved(OVERLAID);
    await waitFor(() =>
      expect(frame.style.getPropertyValue("--controls-right")).toBe("97px"),
    );

    moved({ x: 0, width: ACROSS });
    await waitFor(() =>
      expect(frame.style.getPropertyValue("--controls-right")).toBe(""),
    );
  });

  /// And in a browser the frame carries nothing at all: there is no overlay to
  /// measure, so neither variable is written and the element is as bare as it was
  /// before any of this existed.
  it("carries neither of them in a browser", () => {
    expect(theFrame().getAttribute("style")).toBeNull();
  });
});

/// One of the frame's two breakpoints as text — the same way `sizing.test.tsx`
/// reads them, and for the same reason: which side of a breakpoint a rule is on
/// is the whole of what it says, and jsdom holds no breakpoint at all.
function atWidth(breakpoint: string): string {
  const opened = stylesheet.indexOf(`@media ${breakpoint} {`);

  expect(opened, `the frame should have a ${breakpoint} layout`).not.toBe(-1);

  return stylesheet.slice(opened, stylesheet.indexOf("\n}\n", opened));
}

describe("which head the frame keeps clear", () => {
  /// Out of the head rather than off the pane, which is what keeps the record
  /// under it where it was: the only thing that moves is the band the controls
  /// are drawn over.
  it("pays the insets out of the head", () => {
    expect(stylesheet).toContain(
      ".panes .paneHead {\n" +
        "  padding-left: var(--head-left, 0px);\n" +
        "  padding-right: var(--head-right, 0px);\n}",
    );
  });

  /// The narrow window first, as everything in that sheet is: one pane is the
  /// whole window, so the head in it stands at both edges and both insets are
  /// every head's.
  it("hands both edges to every head while one pane is the window", () => {
    expect(stylesheet).toContain(
      ".panes {\n" +
        "  --head-left: var(--controls-left, 0px);\n" +
        "  --head-right: var(--controls-right, 0px);\n}",
    );
  });

  /// And once the panes stand side by side, the head at each edge is the pane in
  /// the column at that edge: the sidebar where there is a list to pick from, the
  /// record in the frame that has none, and the details pane in the frame that is
  /// nothing else.
  it("hands the left edge to whichever pane is the first column", () => {
    const beside = atWidth(BESIDE);

    expect(beside).toContain(
      "  .panes > .pane {\n    --head-left: 0px;\n    --head-right: 0px;\n  }",
    );
    expect(beside).toContain(
      "  .panes > .conversationsPane {\n" +
        "    --head-left: var(--controls-left, 0px);\n  }",
    );
    expect(beside).toContain(
      "  .panes.two > .middlePane {\n" +
        "    --head-left: var(--controls-left, 0px);\n  }",
    );
    expect(beside).toContain(
      "  .panes.alone > .detailsPane {\n" +
        "    --head-left: var(--controls-left, 0px);\n  }",
    );
  });

  /// The right edge is the details pane's in every frame that shows one — and the
  /// middle pane's as well while it is standing in the details pane's own column,
  /// which is what the two-panes-of-three width does. With all three up it stops,
  /// which is the one thing here that two breakpoints have to agree about.
  it("hands the right edge to the far column, whichever pane that is", () => {
    expect(atWidth(BESIDE)).toContain(
      "  .panes > .detailsPane {\n" +
        "    --head-right: var(--controls-right, 0px);\n  }",
    );
    expect(atWidth(BESIDE)).toContain(
      "  .panes:not(.two) > .middlePane {\n" +
        "    --head-right: var(--controls-right, 0px);\n  }",
    );
    expect(atWidth(ALL_THREE)).toContain(
      "  .panes:not(.two) > .middlePane {\n    --head-right: 0px;\n  }",
    );
  });
});
