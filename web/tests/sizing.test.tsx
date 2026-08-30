//! How wide the frame's three panes stand: the arithmetic underneath it, the
//! dividers that drive it, and where a device remembers what it was left at.
//!
//! Two halves, and they are separable on purpose. The widths are shares of the
//! frame worked out in `src/widths.ts`, which knows nothing of the page; what
//! `src/Panes.tsx` adds is a handle to drag, a frame to measure a drag against,
//! and the rule that neither exists until the window is wide enough to stand
//! two panes side by side. Mostly it is mounted in the workbench here, that
//! being the page that had the frame before it was anybody else's; the last
//! block mounts it bare, which is how the settings page will get it.
//!
//! jsdom lays nothing out, so two things are stood in for. Which breakpoints
//! hold is `matchMedia`, which the page asks rather than infers — so a test can
//! answer it. And the frame's own width is a `getBoundingClientRect` put on
//! every frame this file draws, because a drag is a point on the screen until
//! something measures it against the thing it is a share of — and because the
//! minimums under the widths are lengths, which are worth nothing as shares
//! until the frame they are shares of has a width to be one of.

import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The terminal's own scroller, which is the one thing here still written beside
// the Screen rather than beside the pane it fills.
import screenCss from "../src/workbench/Screen.module.css?raw";
// The frame itself, and its stylesheet both ways: the hashed names to query the
// page by, and the source to read the rules that jsdom lays nothing out for.
import { Panes } from "../src/Panes";
import shell from "../src/Panes.module.css";
import stylesheet from "../src/Panes.module.css?raw";
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
  type Frame,
} from "../src/widths";
import { drawn, mount, theWorkbench } from "./bench";

/// Where the two widths are kept, asked for by the names a browser would find
/// them under rather than through the module that writes them.
///
/// The middle pane's is still the name it was written under, when the frame was
/// the workbench's alone and that pane was the Timeline. Said here as much as
/// anywhere: a device that has dragged it has the width under this key, and a
/// rename would be a width quietly forgotten.
const SIDEBAR = "verkstead.pane-sidebar";
const MIDDLE = "verkstead.pane-timeline";

/// How wide the frame is pretending to be, in the pixels a drag is reported in:
/// 80rem at the 16px a rem is here, which is the window the third pane arrives
/// at and so the narrowest one that stands all three.
const FRAME = 1280;

/// The same frame as the arithmetic is asked about it, and a two-pane window
/// that is not the same width — a minimum is a length now, so what it is worth
/// depends on which frame it is being met in.
const THREE: Frame = { rem: FRAME / 16, three: true };
const TWO: Frame = { rem: 70, three: false };

/// And what one of those lengths is worth as a share of a frame, which is the
/// whole of the conversion the widths do.
function share(length: number, frame: Frame): number {
  return (length / frame.rem) * 100;
}

/// The frames this file draws, given the width jsdom would otherwise leave
/// every element in the document at: nought, which is a frame with no room in
/// it to divide. Put on the element rather than passed in, because the page
/// measures what it drew — and on the frames alone, so that everything else on
/// the workbench is the unlaid-out nothing the rest of this suite reads.
const measured = Element.prototype.getBoundingClientRect;

/// And how wide, for the one test that stands the panes in a smaller window:
/// [`bench`] sets it, and every test that says nothing gets the frame above.
let across = FRAME;

beforeEach(() => {
  across = FRAME;

  Element.prototype.getBoundingClientRect = function (this: Element) {
    return this.matches(`.${shell.panes}`)
      ? ({
          left: 0,
          right: across,
          width: across,
          top: 0,
          bottom: 0,
          height: 0,
        } as DOMRect)
      : measured.call(this);
  };

  localStorage.clear();
});

afterEach(() => {
  Element.prototype.getBoundingClientRect = measured;
  vi.unstubAllGlobals();
  localStorage.clear();
});

/// The window this test is being read on, said the only way the page asks:
/// which of the frame's two breakpoints hold.
function windowIs(width: "narrow" | "two panes" | "three panes"): void {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: query === BESIDE ? width !== "narrow" : width === "three panes",
    media: query,
    addEventListener() {},
    removeEventListener() {},
  }));
}

/// The workbench mounted on such a window, with a frame wide enough to measure
/// a drag against — [`FRAME`] wide unless the test is about what a narrower one
/// owes its panes.
async function bench(width: Parameters<typeof windowIs>[0], wide = FRAME) {
  across = wide;
  windowIs(width);
  theWorkbench();

  const { container } = mount();
  const frame = await drawn<HTMLElement>(container, `.${shell.panes}`);

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
    remember({ sidebar: 34, middle: 26 });

    expect(localStorage.getItem(SIDEBAR)).toBe("34");
    expect(localStorage.getItem(MIDDLE)).toBe("26");
    expect(widths()).toEqual({ sidebar: 34, middle: 26 });
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
    remember({ sidebar: 34, middle: 26 });
    restore();

    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(MIDDLE)).toBeNull();
    expect(widths()).toEqual(DEFAULTS);
  });
});

describe("how far a divider goes", () => {
  /// A pane with no width is a pane whose divider cannot be found again, so
  /// every one of them keeps a floor — and with all three standing, each floor
  /// has to fit beside the others.
  it("leaves every pane something to be, with all three standing", () => {
    expect(clamped({ sidebar: 90, middle: 90 }, THREE)).toEqual({
      sidebar: 100 - share(MINIMUMS.middle + MINIMUMS.details, THREE),
      middle: share(MINIMUMS.middle, THREE),
    });

    expect(clamped({ sidebar: 1, middle: 1 }, THREE)).toEqual({
      sidebar: share(MINIMUMS.sidebar, THREE),
      middle: share(MINIMUMS.middle, THREE),
    });
  });

  /// With two panes up the second column is whichever level is being read and
  /// takes whatever the sidebar leaves, so the middle pane's width decides
  /// nothing — and a sidebar dragged wide here must not quietly rewrite the
  /// layout it is not in.
  it("leaves the middle pane's width alone while only two panes stand", () => {
    expect(clamped({ sidebar: 95, middle: 30 }, TWO)).toEqual({
      sidebar: 100 - share(MINIMUMS.details, TWO),
      middle: 30,
    });
  });

  /// The sidebar's divider says where the sidebar ends, so where it is dropped
  /// is the width. The middle one's says where the middle pane ends, which is a
  /// share of the whole frame rather than of what is left of it.
  it("reads a drop as the pane it is the far edge of", () => {
    const settled = { sidebar: 20, middle: 30 };

    expect(dragged(settled, "sidebar", 34, THREE).sidebar).toBe(34);
    expect(dragged(settled, "middle", 55, THREE).middle).toBe(35);
  });

  /// And the travel said out loud, which is what the handle carries: with all
  /// three up the sidebar has to leave room for the middle pane as well as the
  /// details, and with two it only has to leave room for what is being read.
  it("says how far it may go", () => {
    expect(range("sidebar", DEFAULTS, THREE)).toEqual({
      least: share(MINIMUMS.sidebar, THREE),
      most: 100 - share(MINIMUMS.middle + MINIMUMS.details, THREE),
    });
    expect(range("sidebar", DEFAULTS, TWO)).toEqual({
      least: share(MINIMUMS.sidebar, TWO),
      most: 100 - share(MINIMUMS.details, TWO),
    });
    expect(range("middle", DEFAULTS, THREE)).toEqual({
      least: share(MINIMUMS.middle, THREE),
      most: 100 - DEFAULTS.sidebar - share(MINIMUMS.details, THREE),
    });
  });

  it("moves by a point at a time for a keyboard", () => {
    expect(nudged(DEFAULTS, "sidebar", 1, THREE).sidebar).toBe(
      DEFAULTS.sidebar + 1,
    );
    expect(nudged({ sidebar: 20, middle: 40 }, "middle", -1, THREE).middle).toBe(
      39,
    );

    // And stops where a drag stops.
    const floor = share(MINIMUMS.sidebar, THREE);

    expect(
      nudged({ sidebar: floor, middle: 30 }, "sidebar", -1, THREE),
    ).toEqual({ sidebar: floor, middle: 30 });
  });
});

describe("the dividers on the workbench", () => {
  /// One per border there is: the sidebar's wherever the sidebar stands beside
  /// something, the middle one's only where all three panes are up, and neither
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
    remember({ sidebar: 34, middle: 34 });

    const wide = await bench("three panes");
    expect(wide.frame.style.getPropertyValue("--pane-sidebar")).toBe("34%");
    expect(wide.frame.style.getPropertyValue("--pane-middle")).toBe("34%");

    const narrow = await bench("narrow");
    expect(narrow.frame.style.getPropertyValue("--pane-sidebar")).toBe("");
    expect(narrow.frame.style.getPropertyValue("--pane-middle")).toBe("");
  });

  it("starts where this device left the panes", async () => {
    remember({ sidebar: 40, middle: 25 });

    const { frame } = await bench("three panes");
    expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("40%");
  });

  it("moves the border the pointer drags, and writes it down", async () => {
    const { container, frame } = await bench("three panes");

    dragTo(dividers(container)[0]!, FRAME * 0.25);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("25%"),
    );
    expect(localStorage.getItem(SIDEBAR)).toBe("25");

    // The second divider is the far edge of the middle pane rather than of
    // what is left of the frame, so the sidebar comes off where it was dropped.
    dragTo(dividers(container)[1]!, FRAME * 0.6);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-middle")).toBe("35%"),
    );
    expect(localStorage.getItem(MIDDLE)).toBe("35");
  });

  it("holds the minimum however far the pointer goes", async () => {
    const { container, frame } = await bench("three panes");

    dragTo(dividers(container)[0]!, FRAME * 0.98);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${100 - share(MINIMUMS.middle + MINIMUMS.details, THREE)}%`,
      ),
    );

    dragTo(dividers(container)[0]!, -FRAME);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${share(MINIMUMS.sidebar, THREE)}%`,
      ),
    );
  });

  /// The floors are lengths rather than shares, which is the whole point of
  /// them: a pane holds a card, a Brief or a Diff, and those are the same size
  /// on every window. So the same minimum is a different percentage of every
  /// frame, and a narrower window owes the sidebar more of itself.
  it("keeps a pane's width, not its share, as the window narrows", async () => {
    remember({ sidebar: 5, middle: 30 });

    const wide = await bench("two panes");
    expect(wide.frame.style.getPropertyValue("--pane-sidebar")).toBe(
      `${share(MINIMUMS.sidebar, { rem: FRAME / 16, three: false })}%`,
    );

    const narrow = await bench("two panes", 960);
    expect(narrow.frame.style.getPropertyValue("--pane-sidebar")).toBe(
      `${share(MINIMUMS.sidebar, { rem: 60, three: false })}%`,
    );
  });

  it("puts the defaults back on a double-click", async () => {
    remember({ sidebar: 40, middle: 25 });

    const { container, frame } = await bench("three panes");
    fireEvent.dblClick(dividers(container)[0]!);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${DEFAULTS.sidebar}%`,
      ),
    );
    expect(frame.style.getPropertyValue("--pane-middle")).toBe(
      `${DEFAULTS.middle}%`,
    );
    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(MIDDLE)).toBeNull();
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
    const [sidebar, middle] = dividers(container);

    expect(sidebar!.getAttribute("role")).toBe("separator");
    expect(sidebar!.getAttribute("aria-orientation")).toBe("vertical");
    expect(sidebar!.getAttribute("aria-valuenow")).toBe(
      String(DEFAULTS.sidebar),
    );
    expect(sidebar!.getAttribute("aria-valuemin")).toBe(
      String(Math.round(share(MINIMUMS.sidebar, THREE))),
    );
    expect(sidebar!.getAttribute("aria-valuemax")).toBe(
      String(
        Math.round(100 - share(MINIMUMS.middle + MINIMUMS.details, THREE)),
      ),
    );

    expect(middle!.getAttribute("aria-valuenow")).toBe(
      String(DEFAULTS.middle),
    );
    expect(middle!.getAttribute("aria-valuemax")).toBe(
      String(
        Math.round(100 - DEFAULTS.sidebar - share(MINIMUMS.details, THREE)),
      ),
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
        "      var(--pane-sidebar, 20%) auto var(--pane-middle, 30%) auto 1fr;",
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
      ".panes > .detailsPane {\n" +
        "  padding-inline: max(1.25rem, (100% - 60rem) / 2);\n}",
    );

    // And the pane a terminal fills is still the pane that ends where the
    // window does: the cap is inline, and says nothing about a height.
    expect(stylesheet).toContain(
      ".panes > .detailsPane:has(.paneScreen) {\n" +
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
        "  .panes > .detailsPane:has(.paneScreen) {\n" +
        "    height: auto;\n  }\n}",
    );

    // And the terminal inside it, which is a scroller of its own.
    expect(screenCss).toMatch(
      /\.screen \.terminalHost \{[^}]*overscroll-behavior: contain;/,
    );
  });
});

/// The frame apart from the page that had it first: it draws three panes it is
/// handed and knows nothing about what is in them, which is what lets the
/// settings page stand on the same one.
describe("the frame on its own", () => {
  it("draws the three panes it is handed, in the order they are walked", () => {
    windowIs("three panes");

    const { container } = render(() => (
      <Panes
        pane="middle"
        middleLabel="Settings"
        conversations={<p>the list</p>}
        middle={<p>the root</p>}
        details={<p>the detail</p>}
      />
    ));

    const panes = [...container.querySelectorAll("section")];

    expect(panes.map((pane) => pane.getAttribute("aria-label"))).toEqual([
      "Conversations",
      "Settings",
      "Details",
    ]);
    expect(panes.map((pane) => pane.textContent)).toEqual([
      "the list",
      "the root",
      "the detail",
    ]);

    // And which level a narrow window would be showing is the caller's word for
    // it, carried through untouched.
    const frame = container.querySelector<HTMLElement>(`.${shell.panes}`)!;
    expect(frame.dataset.pane).toBe("middle");
  });

  /// The middle pane is the only one whose name changes from page to page, so
  /// the handle beside it says which pane it moves in the page's own words.
  it("names the middle divider after what the pane holds", () => {
    windowIs("three panes");

    const { container } = render(() => (
      <Panes
        pane="middle"
        middleLabel="Settings"
        conversations={<p>the list</p>}
        middle={<p>the root</p>}
        details={<p>the detail</p>}
      />
    ));

    expect(
      dividers(container).map((divider) => divider.getAttribute("aria-label")),
    ).toEqual(["Resize the conversations pane", "Resize the settings pane"]);
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
