//! How wide the workbench's panes stand: the arithmetic underneath it, the
//! dividers that drive it, and where a device remembers what it was left at.
//!
//! Two halves, and they are separable on purpose. The widths are shares of the
//! frame worked out in `src/workbench/panes.ts`, which knows nothing of the
//! page; what the page adds is a handle to drag, a frame to measure a drag
//! against, and the rule that neither exists until the window is wide enough to
//! stand two panes side by side.
//!
//! jsdom lays nothing out, so two things are stood in for. Which breakpoints
//! hold is `matchMedia`, which the page asks rather than infers — so a test can
//! answer it. And the frame's own width is a `getBoundingClientRect` put on the
//! element, because a drag is a point on the screen until something measures it
//! against the thing it is a share of.

import { fireEvent, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The terminal's own scroller, which is the one thing here still written beside
// the Screen rather than beside the pane it fills.
import screenCss from "../src/workbench/Screen.module.css?raw";
// The frame, both ways: the hashed names to query the page by, and the source
// to read the rules that jsdom lays nothing out for.
import shell from "../src/workbench/Workbench.module.css";
import stylesheet from "../src/workbench/Workbench.module.css?raw";
import {
  ALL_THREE,
  BESIDE,
  DEFAULTS,
  MINIMUMS,
  clamped,
  dragged,
  nudged,
  range,
  remember,
  restore,
  widths,
} from "../src/workbench/panes";
import { drawn, mount, theWorkbench } from "./bench";

/// Where the two widths are kept, asked for by the names a browser would find
/// them under rather than through the module that writes them.
const SIDEBAR = "verkstead.pane-sidebar";
const TIMELINE = "verkstead.pane-timeline";

/// How wide the frame is pretending to be, in the pixels a drag is reported in.
/// Round, so that a share of it is a number worth reading in an assertion.
const FRAME = 1000;

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

/// The window this test is being read on, said the only way the page asks:
/// which of the workbench's two breakpoints hold.
function windowIs(width: "narrow" | "two panes" | "three panes"): void {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: query === BESIDE ? width !== "narrow" : width === "three panes",
    media: query,
    addEventListener() {},
    removeEventListener() {},
  }));
}

/// The workbench mounted on such a window, with a frame wide enough to measure
/// a drag against.
async function bench(width: Parameters<typeof windowIs>[0]) {
  windowIs(width);
  theWorkbench();

  const { container } = mount();
  const frame = await drawn<HTMLElement>(container, `.${shell.workbench}`);

  frame.getBoundingClientRect = () =>
    ({ left: 0, right: FRAME, width: FRAME, top: 0, bottom: 0, height: 0 }) as DOMRect;

  return { container, frame };
}

/// The dividers on the page, in the order they part the panes.
function dividers(container: ParentNode): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(`.${shell.divider}`)];
}

/// Drag a divider to a point across the frame and let go of it.
function dragTo(divider: HTMLElement, x: number): void {
  fireEvent.pointerDown(divider, { clientX: 0 });
  fireEvent.pointerMove(window, { clientX: x });
  fireEvent.pointerUp(window, { clientX: x });
}

describe("the widths a device remembers", () => {
  it("answers with the defaults until something has been dragged", () => {
    expect(widths()).toEqual(DEFAULTS);
  });

  it("reads back what a drag settled on", () => {
    remember({ sidebar: 34, timeline: 26 });

    expect(localStorage.getItem(SIDEBAR)).toBe("34");
    expect(localStorage.getItem(TIMELINE)).toBe("26");
    expect(widths()).toEqual({ sidebar: 34, timeline: 26 });
  });

  /// A storage somebody has edited by hand, or one written by a version of this
  /// page that meant something else by the key. Nothing that is not a share of
  /// a whole is one.
  it("takes anything that is not a share as nothing said", () => {
    for (const junk of ["", "banana", "0", "100", "-20", "NaN"]) {
      localStorage.setItem(SIDEBAR, junk);
      expect(widths().sidebar, `${junk} is not a width`).toBe(DEFAULTS.sidebar);
    }
  });

  /// What a double-click on a divider does. Both widths, because what it puts
  /// back is *the defaults*.
  it("gives up both widths at once", () => {
    remember({ sidebar: 34, timeline: 26 });
    restore();

    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(TIMELINE)).toBeNull();
    expect(widths()).toEqual(DEFAULTS);
  });
});

describe("how far a divider goes", () => {
  /// A pane with no width is a pane whose divider cannot be found again, so
  /// every one of them keeps a floor — and with all three standing, each floor
  /// has to fit beside the others.
  it("leaves every pane something to be, with all three standing", () => {
    expect(clamped({ sidebar: 90, timeline: 90 }, true)).toEqual({
      sidebar: 100 - MINIMUMS.timeline - MINIMUMS.details,
      timeline: MINIMUMS.timeline,
    });

    expect(clamped({ sidebar: 1, timeline: 1 }, true)).toEqual({
      sidebar: MINIMUMS.sidebar,
      timeline: MINIMUMS.timeline,
    });
  });

  /// With two panes up the second column is whichever level is being read and
  /// takes whatever the sidebar leaves, so the timeline's width decides nothing
  /// — and a sidebar dragged wide here must not quietly rewrite the layout it
  /// is not in.
  it("leaves the timeline's width alone while only two panes stand", () => {
    expect(clamped({ sidebar: 95, timeline: 30 }, false)).toEqual({
      sidebar: 100 - MINIMUMS.details,
      timeline: 30,
    });
  });

  /// The sidebar's divider says where the sidebar ends, so where it is dropped
  /// is the width. The timeline's says where the timeline ends, which is a
  /// share of the whole frame rather than of what is left of it.
  it("reads a drop as the pane it is the far edge of", () => {
    const settled = { sidebar: 20, timeline: 30 };

    expect(dragged(settled, "sidebar", 34, true).sidebar).toBe(34);
    expect(dragged(settled, "timeline", 55, true).timeline).toBe(35);
  });

  /// And the travel said out loud, which is what the handle carries: with all
  /// three up the sidebar has to leave room for the timeline as well as the
  /// details, and with two it only has to leave room for what is being read.
  it("says how far it may go", () => {
    expect(range("sidebar", DEFAULTS, true)).toEqual({
      least: MINIMUMS.sidebar,
      most: 100 - MINIMUMS.timeline - MINIMUMS.details,
    });
    expect(range("sidebar", DEFAULTS, false)).toEqual({
      least: MINIMUMS.sidebar,
      most: 100 - MINIMUMS.details,
    });
    expect(range("timeline", DEFAULTS, true)).toEqual({
      least: MINIMUMS.timeline,
      most: 100 - DEFAULTS.sidebar - MINIMUMS.details,
    });
  });

  it("moves by a point at a time for a keyboard", () => {
    expect(nudged(DEFAULTS, "sidebar", 1, true).sidebar).toBe(
      DEFAULTS.sidebar + 1,
    );
    expect(nudged(DEFAULTS, "timeline", -1, true).timeline).toBe(
      DEFAULTS.timeline - 1,
    );

    // And stops where a drag stops.
    expect(nudged({ sidebar: MINIMUMS.sidebar, timeline: 30 }, "sidebar", -1, true))
      .toEqual({ sidebar: MINIMUMS.sidebar, timeline: 30 });
  });
});

describe("the dividers on the workbench", () => {
  /// One per border there is: the sidebar's wherever the sidebar stands beside
  /// something, the timeline's only where all three panes are up, and neither
  /// on a window that shows one pane at a time.
  it("puts a handle on every border there is", async () => {
    const three = await bench("three panes");
    expect(dividers(three.container)).toHaveLength(2);

    const two = await bench("two panes");
    expect(dividers(two.container)).toHaveLength(1);
    expect(dividers(two.container)[0]!.getAttribute("aria-label")).toBe(
      "Resize the conversations pane",
    );

    const narrow = await bench("narrow");
    expect(dividers(narrow.container)).toEqual([]);
  });

  /// And the widths themselves reach the frame only there. Below the
  /// breakpoint the page is walked through one pane at a time and what this
  /// device remembers about a desktop's columns is not read at all.
  it("names the widths on the frame only where panes stand together", async () => {
    remember({ sidebar: 34, timeline: 26 });

    const wide = await bench("three panes");
    expect(wide.frame.style.getPropertyValue("--pane-sidebar")).toBe("34%");
    expect(wide.frame.style.getPropertyValue("--pane-timeline")).toBe("26%");

    const narrow = await bench("narrow");
    expect(narrow.frame.style.getPropertyValue("--pane-sidebar")).toBe("");
    expect(narrow.frame.style.getPropertyValue("--pane-timeline")).toBe("");
  });

  it("starts where this device left the panes", async () => {
    remember({ sidebar: 40, timeline: 25 });

    const { frame } = await bench("three panes");
    expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("40%");
  });

  it("moves the border the pointer drags, and writes it down", async () => {
    const { container, frame } = await bench("three panes");

    dragTo(dividers(container)[0]!, FRAME * 0.35);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("35%"),
    );
    expect(localStorage.getItem(SIDEBAR)).toBe("35");

    // The second divider is the far edge of the timeline rather than of what is
    // left of the frame, so the sidebar comes off where it was dropped.
    dragTo(dividers(container)[1]!, FRAME * 0.6);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-timeline")).toBe("25%"),
    );
    expect(localStorage.getItem(TIMELINE)).toBe("25");
  });

  it("holds the minimum however far the pointer goes", async () => {
    const { container, frame } = await bench("three panes");

    dragTo(dividers(container)[0]!, FRAME * 0.98);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${100 - MINIMUMS.timeline - MINIMUMS.details}%`,
      ),
    );

    dragTo(dividers(container)[0]!, -FRAME);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${MINIMUMS.sidebar}%`,
      ),
    );
  });

  it("puts the defaults back on a double-click", async () => {
    remember({ sidebar: 40, timeline: 25 });

    const { container, frame } = await bench("three panes");
    fireEvent.dblClick(dividers(container)[0]!);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${DEFAULTS.sidebar}%`,
      ),
    );
    expect(frame.style.getPropertyValue("--pane-timeline")).toBe(
      `${DEFAULTS.timeline}%`,
    );
    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(TIMELINE)).toBeNull();
  });

  /// A handle nobody can put a pointer on is still a handle, so the arrow keys
  /// move it — and settle it, there being no letting go of a key.
  it("moves with the arrow keys too", async () => {
    const { container, frame } = await bench("three panes");

    fireEvent.keyDown(dividers(container)[0]!, { key: "ArrowRight" });

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${DEFAULTS.sidebar + 1}%`,
      ),
    );
    expect(localStorage.getItem(SIDEBAR)).toBe(String(DEFAULTS.sidebar + 1));
  });

  /// What the handle is, for anything reading the page rather than looking at
  /// it: a separator with the share of the workbench it decides on it.
  it("says what it is and where it stands", async () => {
    const { container } = await bench("three panes");
    const [sidebar, timeline] = dividers(container);

    expect(sidebar!.getAttribute("role")).toBe("separator");
    expect(sidebar!.getAttribute("aria-orientation")).toBe("vertical");
    expect(sidebar!.getAttribute("aria-valuenow")).toBe(
      String(DEFAULTS.sidebar),
    );
    expect(sidebar!.getAttribute("aria-valuemin")).toBe(
      String(MINIMUMS.sidebar),
    );
    expect(sidebar!.getAttribute("aria-valuemax")).toBe(
      String(100 - MINIMUMS.timeline - MINIMUMS.details),
    );

    expect(timeline!.getAttribute("aria-valuenow")).toBe(
      String(DEFAULTS.timeline),
    );
    expect(timeline!.getAttribute("aria-valuemax")).toBe(
      String(100 - DEFAULTS.sidebar - MINIMUMS.details),
    );
  });
});

/// What the widths are *for* is a grid, and jsdom lays out no grids: the rules
/// themselves are what is read, as everywhere else in this suite.
describe("the rules the widths are read by", () => {
  it("builds both layouts out of the shares the page names", () => {
    expect(stylesheet).toContain(
      "grid-template-columns: var(--pane-sidebar, 20%) auto 1fr;",
    );
    expect(stylesheet).toContain(
      "grid-template-columns:\n" +
        "      var(--pane-sidebar, 20%) auto var(--pane-timeline, 30%) auto 1fr;",
    );

    // The breakpoints the page asks `matchMedia` about are the ones the rules
    // are written at, and nothing may let the two drift apart.
    expect(stylesheet).toContain(`@media ${BESIDE} {`);
    expect(stylesheet).toContain(`@media ${ALL_THREE} {`);
  });

  /// The divider draws the line between two panes, so the panes do not: two
  /// borders a pixel apart, one of them moving, is the sort of thing nobody
  /// sees and everybody notices.
  it("draws the border on the divider rather than on the panes", () => {
    expect(stylesheet).toContain(
      ".divider {\n" +
        "  position: relative;\n" +
        "  width: 0.5rem;\n" +
        "  cursor: col-resize;\n" +
        "  touch-action: none;\n}",
    );
    expect(stylesheet).toContain(
      ".divider::before {\n" +
        '  content: "";\n' +
        "  position: absolute;\n" +
        "  inset-block: 0;\n" +
        "  left: 50%;\n" +
        "  width: 1px;\n" +
        "  transform: translateX(-50%);\n" +
        "  background: var(--edge);\n}",
    );

    // And the panes themselves have given theirs up: nothing in the workbench
    // draws an edge down the side of a column any more.
    expect(stylesheet).not.toContain("border-right: 1px solid var(--edge)");
  });

  /// However wide the pane is dragged, what it holds is read at the width
  /// everything else in the app is read at, and sits in the middle of the room
  /// it has.
  it("caps the details pane's content at the page's own measure", () => {
    expect(stylesheet).toContain(
      ".workbench > .detailsPane {\n" +
        "  padding-inline: max(1rem, (100% - 60rem) / 2);\n}",
    );

    // And the pane a terminal fills is still the pane that ends where the
    // window does: the cap is inline, and says nothing about a height.
    expect(stylesheet).toContain(
      ".workbench > .detailsPane:has(.paneScreen) {\n" +
        "  flex-direction: column;\n" +
        "  height: 100dvh;\n",
    );
  });

  /// Where the panes stand side by side the frame is the window, and the page
  /// behind it never scrolls: not by a pane standing past the bottom of the
  /// frame, and not by a pane that has been scrolled to its end handing the
  /// rest of the gesture out to the document. Both of those put blank space
  /// under the workbench and pushed it off the screen.
  it("keeps the page from scrolling under the panes", () => {
    const beside = layout(BESIDE);

    // The frame is the window's height, and whatever gets its own height wrong
    // is clipped rather than turned into document to scroll.
    expect(beside).toContain("    height: 100dvh;\n");
    expect(beside).toContain("    overflow: hidden;\n");

    // Each pane scrolls on its own, and stops when it runs out.
    expect(beside).toContain(
      "  .pane {\n    overflow-y: auto;\n" +
        "    /* A pane scrolled to its end stops there. Left to itself the browser hands\n" +
        "       the rest of the gesture to the document, which is the same page-under-\n" +
        "       the-panes scroll by another route. */\n" +
        "    overscroll-behavior: contain;\n  }",
    );

    // The one pane with a height of its own takes it from the row it is in
    // rather than asking the viewport a second time: two resolutions of one
    // `dvh` that disagree by a pixel are a pixel of page. Said after the rule
    // it is overriding rather than up here with the rest of the layout, which
    // is the only place it could win.
    expect(stylesheet).toContain(
      `@media ${BESIDE} {\n` +
        "  .workbench > .detailsPane:has(.paneScreen) {\n" +
        "    height: auto;\n  }\n}",
    );

    // And the terminal inside it, which is a scroller of its own.
    expect(screenCss).toMatch(
      /\.screen \.terminalHost \{[^}]*overscroll-behavior: contain;/,
    );
  });
});

/// What one of the workbench's two breakpoints holds, as text. The rules are
/// what these tests read — jsdom lays out no grids — and reading them a
/// breakpoint at a time is what says which side of it a rule is on.
function layout(breakpoint: string): string {
  const opened = stylesheet.indexOf(`@media ${breakpoint} {`);
  expect(opened, `the stylesheet should have a ${breakpoint} layout`).not.toBe(
    -1,
  );
  return stylesheet.slice(opened, stylesheet.indexOf("\n}\n", opened));
}
