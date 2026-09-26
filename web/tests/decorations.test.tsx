//! The window's own drag region, the room its controls take, and what the strip
//! they are drawn on is painted and sized from: the three things a page drawn in
//! a window with no title bar has to answer for.
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
//! **And the whole of what a browser sees of the region is nothing.** The
//! property is inert outside a frameless Electron window, so nothing guards it
//! and the head is the same markup either way. What a head *gives up* to be one
//! is not inert out there — a selection is an ordinary thing an ordinary
//! browser does — so that much is drawn on the bridge and asked about on both
//! sides of it, which is what that half's last cases pin.
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
//!
//! And the third part is the one that goes the other way: the head's two colours,
//! how tall its band stands and where the row inside it is, pushed over that same
//! bridge so that the strip the platform draws its controls on is the same paper
//! as the head beneath it — and so that a Mac's traffic lights stand in that row
//! rather than over the chrome's padding. That push makes the stub bridge the app
//! being told rather than the app answering, and the list it keeps is the record
//! of what it was told. Three stand-ins buy the whole of it: the body painted,
//! because jsdom resolves no `var()` and a paper is read off what the page is
//! actually drawn in; the root font size set, because the band and the row are
//! rules written in rem and the rem is the one thing in them no stylesheet can be
//! told in advance; and a `matchMedia` that can change its mind, jsdom's being
//! unable to and a flip of the scheme being the whole of what moves either
//! colour.

import { faGear } from "@fortawesome/free-solid-svg-icons";
import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Gate } from "../src/App";
import { DragBar } from "../src/DragBar";
import bare from "../src/DragBar.module.css";
// And that component's sheet as text, for the reason the head's is read as text:
// the property the strip is a drag region by is Chromium's own and jsdom parses
// none of it.
import barely from "../src/DragBar.module.css?raw";
import { CLEAR, insets, reserved, type Area } from "../src/controls";
import { band, dress, middle, worn } from "../src/head";
import { IconButton } from "../src/IconButton";
import { Menu } from "../src/Menu";
import { Panes } from "../src/Panes";
import shell from "../src/Panes.module.css";
// And the frame's stylesheet as text, for the rules that say which head stands
// at an edge of the window: jsdom holds no breakpoint and lays out no grid.
import stylesheet from "../src/Panes.module.css?raw";
import { Switch } from "../src/Switch";
import { ALL_THREE, BESIDE } from "../src/widths";
import type { OnboardingView } from "../src/api/types";
import { NAME, type Bridge, type Head } from "../src/settings/bridge";
import { PaneHead } from "../src/workbench/PaneHead";
import styles from "../src/workbench/PaneHead.module.css";
// The same sheet as text. The two declarations under test are Chromium's own
// and jsdom parses neither, so what says which selector they are written over is
// the source of the rule.
import sheet from "../src/workbench/PaneHead.module.css?raw";
import { SET_UP, theWorkbench } from "./bench";
import { hangs, json, whenever } from "./serving";
import fresh from "./fixtures/onboarding-fresh.json" with { type: "json" };

afterEach(() => {
  vi.unstubAllGlobals();
  Reflect.deleteProperty(navigator, "windowControlsOverlay");
});

/// Every rule in a sheet with its comments taken out: the selector as written,
/// and what it declares.
///
/// Flat, so the `@media` block's own brace leaves a fragment behind rather than
/// a rule — which is harmless, because nothing is looked up here except by the
/// property it declares, and the two properties under test are declared once
/// each.
///
/// Told which sheet, because there are two that draw a drag region: the head's
/// and the bare bar's. The rule is the same rule in both, and reading it out of
/// the sheet is the same reading.
function rules(sheet: string): { selects: string; declares: string }[] {
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
function declaring(sheet: string, property: string, value: string): string {
  const declared = rules(sheet)
    .filter((rule) => rule.declares.includes(`${property}: ${value};`))
    .map((rule) => rule.selects);

  expect(declared).toHaveLength(1);

  return declared[0]!;
}

/// A selector out of a stylesheet as the document has it: every plain class
/// name in it swapped for the hashed one the bundler gave the module, which is
/// what the component draws and so what an element can be put to.
function asDrawn(
  selector: string,
  names: Record<string, string | undefined>,
): string {
  return selector.replace(/\.([\w-]+)/g, (_, name: string) => {
    const hashed = names[name];

    expect(hashed, `the sheet selects .${name} and does not define it`).toBeTypeOf(
      "string",
    );

    return `.${hashed}`;
  });
}

/// Where the window is dragged by, and what is excepted from it — the two
/// selectors, as the document has them.
const DRAGS = asDrawn(declaring(sheet, "-webkit-app-region", "drag"), styles);
const EXCEPTED = asDrawn(
  declaring(sheet, "-webkit-app-region", "no-drag"),
  styles,
);

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

/// The app's bridge on the window, saying which machine this is — and holding on
/// to every head the page pushed over it.
///
/// Two things the bridge is for here. A page inside the app draws what a
/// browser's does, and what the platform decides is where it put the window's
/// controls — see the note at the top of this file. And it is what the page says
/// its head's colours and height *to*, so the list that comes back is the record
/// of the telling.
function theApp(platform: string): Head[] {
  const settings = { whenClosed: "tray", trayIcon: true } as const;
  const registered = { possible: true, on: false };

  // Every head the page has pushed, in the order it pushed them — which is the
  // whole of what the app can be asked about here.
  const pushed: Head[] = [];

  vi.stubGlobal(NAME, {
    platform,
    settings: () => Promise.resolve(settings),
    set: () => Promise.resolve(settings),
    logs: () => Promise.resolve(),
    startup: () => Promise.resolve(registered),
    register: () => Promise.resolve(registered),
    head: (worn: Head) => {
      pushed.push(worn);
    },
  } satisfies Bridge);

  return pushed;
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

  /// And inside the app its text stops taking a selection, which is the price
  /// ADR-0020 settled this with: a bar a pointer sweeps a selection across
  /// cannot also be the bar the window moves by. jsdom knows this property, so
  /// it is the one thing here the cascade can be asked about.
  it("gives up taking a selection inside the app", () => {
    theApp("linux");

    const { container } = aHead();

    expect(getComputedStyle(headOf(container)).userSelect).toBe("none");
  });

  /// And nowhere else, which is the half the property itself will not enforce.
  /// `-webkit-app-region` is inert outside the app and needs no guarding;
  /// `user-select` is an ordinary property every browser honours, so a head that
  /// declared it unconditionally took the selection away from a phone on the
  /// tailnet and from whoever opens a standalone share — neither of whom has a
  /// window to move, and either of whom may want to copy the name of what they
  /// are reading.
  it("goes on taking one in a browser", () => {
    const { container } = aHead();

    expect(getComputedStyle(headOf(container)).userSelect).not.toBe("none");
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

  /// And the same head as the app's but for the one name that gives up the
  /// selection, which is the whole of what the bridge changes about a head. The
  /// region itself is not on that list and does not need to be: a rule a browser
  /// ignores needs no guarding, and asking about one would be a branch drawn for
  /// nothing.
  it("is handed the app's head but for what gives up the selection", () => {
    const browser = aHead();
    const drawn = headOf(browser.container);
    const bare = [...drawn.classList];
    const markup = drawn.outerHTML;
    browser.unmount();

    theApp("linux");

    const app = headOf(aHead().container);

    expect([...app.classList]).toEqual([...bare, styles.inApp]);
    // And nothing else about it moved: the same tags, the same attributes, the
    // same controls — one class deep and no further.
    expect(app.outerHTML.replace(` ${styles.inApp}`, "")).toBe(markup);
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
///
/// Measured this time rather than supposed — driven on macOS 15 under Electron 43
/// against the packed app, where the overlay is there, says it is `visible`, and
/// answered `x: 81` with the lights inset twenty points and ending at seventy-five.
/// Said here against the same window the other rectangle was measured in, there
/// being no width in the sum: what a left inset is is where the page's own strip
/// begins.
const TRAFFIC: Area = { x: 81, width: ACROSS - 81 };

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
    expect(insets(TRAFFIC, ACROSS)).toEqual({ left: 81, right: 0 });
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
      "--controls-left": "81px",
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

    expect(frame.style.getPropertyValue("--controls-left")).toBe("81px");
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

describe("how tall the head's band stands", () => {
  afterEach(() => {
    document.documentElement.style.removeProperty("font-size");
  });

  /// The same seventy-five the window opens at — `BAND` in
  /// `desktop/src/decorations.ts`, which that package's suite pins as a number.
  /// Both sides, because the app and the page have to agree about it: one is what
  /// the overlay is until the page loads and the other is what it is afterwards,
  /// and a window that changed height at the moment its page arrived would be the
  /// visible form of them disagreeing.
  it("is the head's band at a sixteen-pixel root", () => {
    expect(band()).toBe(75);
  });

  /// The whole reason it is measured on the page rather than compiled into the
  /// app: a human who has told their browser to draw text larger has a taller
  /// head, and the overlay has to be as tall as the head it is drawn over.
  it("follows the rem the page is actually drawn at", () => {
    document.documentElement.style.fontSize = "20px";

    expect(band()).toBe(93);

    document.documentElement.style.fontSize = "12px";

    expect(band()).toBe(57);
  });

  /// Electron measures the overlay in whole pixels, so a fraction is a rectangle
  /// the page would read back rounded — which makes the rounding the page's to do.
  it("is a whole number of them", () => {
    document.documentElement.style.fontSize = "17px";

    expect(Number.isInteger(band())).toBe(true);
  });
});

describe("where the middle of the head's first row sits", () => {
  afterEach(() => {
    document.documentElement.style.removeProperty("font-size");
  });

  /// The thirty-nine the window opens at — `MIDDLE` in
  /// `desktop/src/decorations.ts`, which that package's suite pins as a number and
  /// works the traffic lights' point out of. Both sides, for the reason the band is
  /// pinned on both: the moment before the page loads and the moment after it are
  /// the same answer, so a Mac's lights do not jump when the workbench arrives.
  it("is the row's middle at a sixteen-pixel root", () => {
    expect(middle()).toBe(39);
  });

  /// Inside the band and below the middle of it, which is the whole reason it is a
  /// measurement of its own rather than the band halved: the band carries the rem
  /// the head keeps *below* its row and a quarter rem of chrome above it, so the
  /// row's middle sits lower than the strip's.
  it("is past the middle of the band and inside it", () => {
    expect(middle()).toBeGreaterThan(band() / 2);
    expect(middle()).toBeLessThan(band());
  });

  /// And it follows the rem, for the reason the band does: the rules that put the
  /// row where it is are written in rem, and a human who has told their browser to
  /// draw text larger has a head whose row stands further down the window. Which is
  /// what keeps the lights out of the app's arithmetic — there is none to do there
  /// but half of a button.
  it("follows the rem the page is actually drawn at", () => {
    document.documentElement.style.fontSize = "20px";

    expect(middle()).toBe(49);

    document.documentElement.style.fontSize = "12px";

    expect(middle()).toBe(30);
  });

  /// Whole pixels, a point being whole pixels — and the rounding is the page's to
  /// do for the same reason the band's is.
  it("is a whole number of them", () => {
    document.documentElement.style.fontSize = "17px";

    expect(Number.isInteger(middle())).toBe(true);
  });
});

describe("what the head is drawn in", () => {
  afterEach(() => {
    document.body.style.removeProperty("background-color");
    document.body.style.removeProperty("color");
  });

  /// Read off the page rather than named in the app (Set 847 Q11b), which is what
  /// lets the one overlay be right in both schemes. `#rrggbb` because that is what
  /// Electron parses, whatever notation the browser answered in.
  it("is the paper and the ink the page has resolved", () => {
    document.body.style.backgroundColor = "rgb(250, 248, 245)";
    document.body.style.color = "rgb(28, 26, 23)";

    expect(worn()).toEqual({ paper: "#faf8f5", ink: "#1c1a17", band: 75, middle: 39 });
  });

  /// And the other scheme, which is the same read a moment later: the page says
  /// what it is drawn in now rather than which of the two it is in.
  it("is the dark scheme's where that is what the page is in", () => {
    document.body.style.backgroundColor = "rgb(23, 22, 20)";
    document.body.style.color = "rgb(236, 231, 224)";

    expect(worn()).toMatchObject({ paper: "#171614", ink: "#ece7e0" });
  });

  /// A page with nothing behind it reports a background nobody set as fully
  /// transparent black, and an overlay painted from that is a black strip welded
  /// to the corner of the window. So it is not a colour and there is nothing to
  /// say.
  it("is nothing at all where the page is drawn on nothing", () => {
    expect(worn()).toBeUndefined();
  });

  /// And nothing where the browser answered in a notation the page cannot read
  /// three channels out of, which is the failure that would not have announced
  /// itself: `oklch(0.97 0.01 80)` is the same near-white paper, and its first
  /// three numbers make `#010000`. A near-black strip welded to the corner of the
  /// window is worse than the light one it opened at, so what is not one of the
  /// two notations Chromium serializes is nothing rather than a guess.
  ///
  /// Stood in for rather than set on the body: what is under test is what the
  /// page does with an answer it has not seen before, and jsdom decides for
  /// itself which of them it will keep.
  it("is nothing where the browser answered in a notation it cannot read", () => {
    vi.stubGlobal("getComputedStyle", () => ({
      backgroundColor: "oklch(0.97 0.01 80)",
      color: "oklch(0.2 0.01 80)",
      fontSize: "16px",
    }));

    expect(worn()).toBeUndefined();
  });

  /// While the space-separated spelling of the one it does read is the same three
  /// channels: that is the notation the comma'd one is being modernised into, and
  /// a page served by a browser that had is a page that still says its paper.
  it("is the same colour where the notation has been modernised", () => {
    vi.stubGlobal("getComputedStyle", () => ({
      backgroundColor: "rgb(250 248 245)",
      color: "rgb(28 26 23)",
      fontSize: "16px",
    }));

    expect(worn()).toEqual({ paper: "#faf8f5", ink: "#1c1a17", band: 75, middle: 39 });
  });
});

describe("telling the app", () => {
  /// The colour scheme as something a test can flip. jsdom answers the media query
  /// and has no way of changing its mind, and following a change is the whole of
  /// what this watches for — the same stand-in `diagrams.test.ts` builds, for the
  /// same reason.
  function scheme() {
    const listeners = new Set<() => void>();
    const query = {
      matches: false,
      addEventListener: (_: string, listen: () => void) => listeners.add(listen),
      removeEventListener: (_: string, listen: () => void) =>
        listeners.delete(listen),
    };

    vi.stubGlobal("matchMedia", () => query);

    return {
      flip(paper: string, ink: string) {
        query.matches = !query.matches;
        document.body.style.backgroundColor = paper;
        document.body.style.color = ink;
        for (const listen of [...listeners]) listen();
      },
      watched: () => listeners.size,
    };
  }

  afterEach(() => {
    document.body.style.removeProperty("background-color");
    document.body.style.removeProperty("color");
  });

  /// The whole of the channel in one case: the page says what it is drawn in the
  /// moment it is up, and says it again when the machine changes its mind. No
  /// restart, because the overlay is recoloured on a window that is already open.
  it("says what the head is drawn in, and again when the scheme flips", () => {
    document.body.style.backgroundColor = "rgb(250, 248, 245)";
    document.body.style.color = "rgb(28, 26, 23)";

    const pushed = theApp("linux");
    const watching = scheme();

    const stop = dress();

    expect(pushed).toEqual([{ paper: "#faf8f5", ink: "#1c1a17", band: 75, middle: 39 }]);

    watching.flip("rgb(23, 22, 20)", "rgb(236, 231, 224)");

    expect(pushed).toEqual([
      { paper: "#faf8f5", ink: "#1c1a17", band: 75, middle: 39 },
      { paper: "#171614", ink: "#ece7e0", band: 75, middle: 39 },
    ]);

    stop();
  });

  /// And it stops watching when the page goes, which is what the app holds the
  /// returned function for: a listener left on the query would be one per page.
  it("stops watching when the page has gone", () => {
    document.body.style.backgroundColor = "rgb(250, 248, 245)";
    document.body.style.color = "rgb(28, 26, 23)";

    theApp("linux");
    const watching = scheme();

    const stop = dress();
    expect(watching.watched()).toBe(1);

    stop();
    expect(watching.watched()).toBe(0);
  });

  /// A Mac is pushed to exactly as the other two are — the page has no platform
  /// branch in it, and what a platform with traffic lights rather than an overlay
  /// does about a push is the app's business. See `overlaid` in
  /// `desktop/src/decorations.ts`.
  it("says it on a Mac too, there being no branch on the platform", () => {
    document.body.style.backgroundColor = "rgb(250, 248, 245)";
    document.body.style.color = "rgb(28, 26, 23)";

    const pushed = theApp("darwin");
    scheme();

    dress()();

    expect(pushed).toHaveLength(1);
  });

  /// And the half that matters: a browser on this machine and a phone on the
  /// tailnet have no bridge, so there is nobody to say it to and nothing is
  /// watched for either. The same document, drawn the same way, telling nobody.
  it("says nothing at all in a browser", () => {
    document.body.style.backgroundColor = "rgb(250, 248, 245)";
    document.body.style.color = "rgb(28, 26, 23)";

    const watching = scheme();

    expect(() => dress()()).not.toThrow();
    expect(watching.watched()).toBe(0);
  });

  /// A page that cannot say what it is drawn in says nothing rather than pushing
  /// two of the three values: an overlay given a height and left the colour it
  /// opened at is the light scheme's paper on a dark desktop, which is worse than
  /// one that is merely the wrong height.
  it("says nothing where the page cannot say what it is drawn in", () => {
    const pushed = theApp("linux");
    scheme();

    dress()();

    expect(pushed).toEqual([]);
  });
});

/// The bar's own selectors, out of its own sheet: the strip that is the region,
/// and the room it takes in the page.
const BARE = asDrawn(declaring(barely, "-webkit-app-region", "drag"), bare);
const ROOM = `.${bare.bar}`;

/// What the strip declares, for the things about it that are not a class on an
/// element: it is fixed across the window rather than stuck to the page, it is
/// drawn at the band the component writes, and the page's own paper is behind
/// it. Found by the selector as the sheet writes it, which is what `rules` reads.
const STRIP = rules(barely).find((rule) => rule.selects === ".drag")?.declares;

/// A start on a bare machine — nothing installed, no Profile, no author — which
/// is the reading that leaves the wizard as the only page there is.
const FRESH = fresh as OnboardingView;

/// The wizard's own heading, and what the app answers a path nothing has: the
/// two of the three pages that draw anything at all.
const WIZARD = "Set Verkstead up";
const MISSED = "No such page.";

/// What the machine answers with, which is a value or a hang: how long the read
/// takes is one of the three cases.
type Reading = Parameters<typeof whenever>[1];

/// The app's own gate at a path, over a machine that reads as `machine` says.
///
/// The real gate rather than the pages under it, because which of the three
/// pages with no pane head is drawn is the gate's own answer: the wizard while
/// the mode is on, the no-such-page at a path nothing has while it is off, and
/// nothing at all for as long as the read of the machine is still in flight.
///
/// A query client of this test's own, for the reason `setup-routes.test.tsx`
/// keeps one: `App` builds one at module scope that outlives every render, and a
/// verdict cached by one test here would be the wrong page drawn in the next.
function opened(path: string, machine: Reading) {
  window.history.pushState({}, "", path);
  theWorkbench(whenever("/api/ui/onboarding", machine));

  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={client}>
      <Gate />
    </QueryClientProvider>
  ));
}

/// How many bare drag bars are on the page.
function bars(container: ParentNode): number {
  return container.querySelectorAll(ROOM).length;
}

describe("the bare drag bar", () => {
  /// The whole of what it is for: a page with no pane head has no bar the window
  /// moves by, and this is the bar — the same one declaration a head is the
  /// region by, over the strip this draws.
  it("is the region the window is dragged by", () => {
    theApp("linux");

    const { container } = render(() => <DragBar />);
    const room = container.querySelector(ROOM)!;
    const strip = container.querySelector(BARE)!;

    expect(room.contains(strip)).toBe(true);
  });

  /// And it stands as tall as a head, which is what the controls are drawn as
  /// tall as: a bar of any other height would leave them hanging over its edge
  /// or under it. The same `band` the app is told the overlay's height in, so
  /// the two cannot disagree.
  it("stands at the head's own band", () => {
    theApp("linux");

    const { container } = render(() => <DragBar />);
    const room = container.querySelector<HTMLElement>(ROOM)!;

    expect(room.style.getPropertyValue("--band")).toBe(`${band()}px`);
    // Both halves of it at that band: the room the page keeps for it, and the
    // strip drawn over the window.
    expect(barely).toContain(`.bar {\n  height: var(--band);\n}`);
    expect(STRIP).toContain("height: var(--band);");
  });

  /// Across the window rather than across the page, which is the one thing here
  /// a head does differently: a head spans its pane and a pane spans the window,
  /// while these pages are drawn in the column `styles/base.css` measures — and
  /// the controls are at the window's own corner, outside that column in any
  /// window wider than it.
  it("is drawn across the window rather than across the page", () => {
    expect(STRIP).toContain("position: fixed;");
    expect(STRIP).toContain("right: 0;");
    expect(STRIP).toContain("left: 0;");
    // And on the page's own paper, the way `.paneChrome` is: what scrolls up
    // passes under the strip rather than over it.
    expect(STRIP).toContain("background: var(--paper);");
  });

  /// And the half that keeps it out of a browser. A drag region is inert outside
  /// the app, so a bar drawn there would move nothing — but it would still be a
  /// band of the head's height across the top of a page a phone has no use for.
  it("is nothing at all where there is no bridge", () => {
    const { container } = render(() => <DragBar />);

    expect(container.innerHTML).toBe("");
  });
});

describe("the three pages with no pane head", () => {
  afterEach(() => window.history.pushState({}, "", "/"));

  /// The moment before the verdict about this machine lands, which draws nothing
  /// at all: the window cannot be moved for the length of that read, and how
  /// long that read takes is the machine's business rather than the page's.
  it("the moment before the verdict lands draws one", async () => {
    theApp("linux");

    const { container } = opened("/", hangs());

    await waitFor(() => expect(bars(container)).toBe(1));
  });

  /// The wizard, which is the only page there is while onboarding mode is on —
  /// and the page a fresh install opens on, so the first window anybody sees is
  /// one of the three.
  it("the wizard draws one", async () => {
    theApp("linux");

    const { container, findByText } = opened("/setup", json(FRESH));

    await findByText(WIZARD);
    expect(bars(container)).toBe(1);
  });

  /// And the no-such-page, which is one line of notice and nothing else.
  it("the no-such-page draws one", async () => {
    theApp("linux");

    const { container, findByText } = opened("/nonsense", json(SET_UP));

    await findByText(MISSED);
    expect(bars(container)).toBe(1);
  });

  /// And none of the three in a browser, where the pages are otherwise exactly
  /// what they were: the same wizard and the same notice, with no band of
  /// nothing across the top of either.
  it("draws none of them in a browser", async () => {
    const wizard = opened("/setup", json(FRESH));
    await wizard.findByText(WIZARD);
    expect(bars(wizard.container)).toBe(0);
    wizard.unmount();

    const missed = opened("/nonsense", json(SET_UP));
    await missed.findByText(MISSED);
    expect(bars(missed.container)).toBe(0);
    missed.unmount();

    const waiting = opened("/", hangs());
    expect(bars(waiting.container)).toBe(0);
  });
});
