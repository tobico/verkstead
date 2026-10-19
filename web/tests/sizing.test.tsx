//! How wide the frame's three panes stand: the arithmetic underneath it, the
//! dividers that drive it, and where a device remembers what it was left at.
//!
//! Three bands for the frame with a list in it: under 60rem the list is a
//! collapsed strip beside one pane, from 60rem the strip stands beside the
//! timeline and the details, and from 100rem the list is the full sidebar.
//!
//! Two halves, and they are separable on purpose. The widths are shares of the
//! frame worked out in `src/widths.ts`, which knows nothing of the page; what
//! `src/Panes.tsx` adds is a handle to drag, a frame to measure a drag against,
//! and the rule that neither exists until the window is wide enough to stand
//! two panes side by side. Mostly it is mounted in the workbench here, that
//! being the page that had the frame before it was anybody else's; the last two
//! blocks mount it bare — which is how the settings page will get it, and how a
//! share gets the frame it has no list to fill the first pane of.
//!
//! jsdom lays nothing out, so two things are stood in for. Which breakpoints
//! hold is `matchMedia`, which the page asks rather than infers — so a test can
//! answer it. And the frame's own width is a `getBoundingClientRect` put on
//! every frame this file draws, because a drag is a point on the screen until
//! something measures it against the thing it is a share of — and because the
//! minimums under the widths are lengths, which are worth nothing as shares
//! until the frame they are shares of has a width to be one of.
//!
//! One of the blocks is about the frame's height rather than its widths, which
//! is one question and has never been a comfortable one: the page must never
//! scroll behind the panes. It is here because it is the same frame — and it is
//! asked twice, of the stylesheet where jsdom can lay no page out, and of the
//! page itself where the answer is a cascade rather than a geometry.

import { cleanup, fireEvent, render, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The terminal's own scroller, which is the one thing here still written beside
// the Screen rather than beside the pane it fills.
import attachedCss from "../src/workbench/Attached.module.css?raw";
// The frame itself, and its stylesheet both ways: the hashed names to query the
// page by, and the source to read the rules that jsdom lays nothing out for.
import { Panes } from "../src/Panes";
import shell from "../src/Panes.module.css";
import stylesheet from "../src/Panes.module.css?raw";
import {
  BESIDE,
  COLLAPSED,
  DEFAULTS,
  FULL,
  MINIMUMS,
  clamped,
  dragged,
  expanded,
  nudged,
  range,
  remember,
  restore,
  widths,
  type Frame,
  type Widths,
} from "../src/widths";
import type { ConversationView } from "../src/api/types";
import { OPEN, drawn, mount, theWorkbench } from "./bench";
import { json, whenever } from "./serving";
import roadmap from "./fixtures/conversation-roadmap.json" with { type: "json" };
import tasks from "./fixtures/conversation-tasks.json" with { type: "json" };

/// The two conversations with something pinned above their record: a task list
/// and a roadmap. Both are here for one reason — each draws a list of rows whose
/// done state is written twice, as a box to look at and as a word to hear, and
/// the word is taken out of the layout by being positioned. Which is the
/// element that put a scrollbar down the side of the workbench.
const TASKED = tasks as ConversationView;
const STAGED = roadmap as ConversationView;

/// Where the widths are kept, asked for by the names a browser would find them
/// under rather than through the module that writes them.
///
/// The middle pane's is still the name it was written under, when the frame was
/// the workbench's alone and that pane was the Timeline. Said here as much as
/// anywhere: a device that has dragged it has the width under this key, and a
/// rename would be a width quietly forgotten.
const SIDEBAR = "verkstead.pane-sidebar";
const MIDDLE = "verkstead.pane-timeline";

/// And the third, which is the frame with no list in it: the one border a share
/// has, between the record and whatever it has open.
const PAIR_KEY = "verkstead.pane-pair";

/// How wide the frame is pretending to be, in the pixels a drag is reported in:
/// 100rem at the 16px a rem is here, which is the window the list stands full
/// at and so the narrowest one that stands all three at their shares.
const FRAME = 1600;

/// And the window between the two lines, where the list is collapsed beside
/// the timeline and the details: 70rem.
const BETWEEN = 1120;

/// And a phone's, where the list is collapsed beside the one pane.
const PHONE = 375;

/// The same frame as the arithmetic is asked about it, and a two-pane window
/// that is not the same width — a minimum is a length now, so what it is worth
/// depends on which frame it is being met in.
const THREE: Frame = {
  rem: FRAME / 16,
  three: true,
  picking: true,
  collapsed: false,
};
const TWO: Frame = { rem: 70, three: false, picking: true, collapsed: false };

/// And the frame whose list is collapsed beside the timeline and the details,
/// which is every window between the two lines: the middle divider and nothing
/// else, measured from the strip rather than from the sidebar's share.
const COLLAPSED_THREE: Frame = {
  rem: BETWEEN / 16,
  three: true,
  picking: true,
  collapsed: true,
};

/// And the frame with nothing to pick from, which is the share's: two panes and
/// the one divider. [`FRAME`] wide, that being the window the page tests below
/// draw it in, so the same constant answers for the arithmetic and for what
/// they read off the frame.
const PAIR: Frame = {
  rem: FRAME / 16,
  three: false,
  picking: false,
  collapsed: false,
};

/// And the frame with a list but no middle pane — the workbench reading a
/// Conversation whose record is the one Event. Two panes with the sidebar's own
/// divider between them, which is [`TWO`] again in a window wide enough for the
/// third pane: what says all three are standing is a middle pane to fill the
/// column, and there is none.
const WIDENED: Frame = {
  rem: FRAME / 16,
  three: false,
  picking: true,
  collapsed: false,
};

/// A set of widths with one or two of them said: the rest are the defaults,
/// which is what a device that has dragged one frame and not the other holds.
function all(some: Partial<Widths>): Widths {
  return { ...DEFAULTS, ...some };
}

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
/// which of the frame's two lines hold. Under the first the list is collapsed
/// beside one pane, between them it is collapsed beside two, and past the
/// second it is full.
function windowIs(width: "narrow" | "beside" | "full"): void {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: query === BESIDE ? width !== "narrow" : width === "full",
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
    remember(all({ sidebar: 34, middle: 26 }), THREE);

    expect(localStorage.getItem(SIDEBAR)).toBe("34");
    expect(localStorage.getItem(MIDDLE)).toBe("26");
    expect(widths()).toEqual(all({ sidebar: 34, middle: 26 }));
  });

  /// And writes down what the frame in front of the human can move, and nothing
  /// else: a reader settling a share's panes has said nothing about how wide
  /// this device stands the workbench's columns.
  it("writes down only the widths the frame it settled in has", () => {
    remember(all({ sidebar: 34, middle: 26, pair: 55 }), PAIR);

    expect(localStorage.getItem(PAIR_KEY)).toBe("55");
    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(MIDDLE)).toBeNull();
    expect(widths()).toEqual(all({ pair: 55 }));
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
    remember(all({ sidebar: 34, middle: 26 }), THREE);

    expect(restore(all({ sidebar: 34, middle: 26 }), THREE)).toEqual(DEFAULTS);
    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(MIDDLE)).toBeNull();
    expect(widths()).toEqual(DEFAULTS);
  });

  /// The frame's own, that is. A share put back to its default is a share put
  /// back, and the workbench's columns are still where this device left them.
  it("puts back only what the frame it was asked in has", () => {
    remember(all({ sidebar: 34, middle: 26 }), THREE);
    remember(all({ pair: 55 }), PAIR);

    expect(restore(all({ sidebar: 34, middle: 26, pair: 55 }), PAIR)).toEqual(
      all({ sidebar: 34, middle: 26 }),
    );
    expect(localStorage.getItem(PAIR_KEY)).toBeNull();
    expect(localStorage.getItem(SIDEBAR)).toBe("34");
    expect(localStorage.getItem(MIDDLE)).toBe("26");
  });
});

describe("how far a divider goes", () => {
  /// A pane with no width is a pane whose divider cannot be found again, so
  /// every one of them keeps a floor — and with all three standing, each floor
  /// has to fit beside the others.
  it("leaves every pane something to be, with all three standing", () => {
    expect(clamped(all({ sidebar: 90, middle: 90 }), THREE)).toEqual(
      all({
        sidebar: 100 - share(MINIMUMS.middle + MINIMUMS.details, THREE),
        middle: share(MINIMUMS.middle, THREE),
      }),
    );

    expect(clamped(all({ sidebar: 1, middle: 1 }), THREE)).toEqual(
      all({
        sidebar: share(MINIMUMS.sidebar, THREE),
        middle: share(MINIMUMS.middle, THREE),
      }),
    );
  });

  /// With two panes up the second column is whichever level is being read and
  /// takes whatever the sidebar leaves, so the middle pane's width decides
  /// nothing — and a sidebar dragged wide here must not quietly rewrite the
  /// layout it is not in.
  it("leaves the middle pane's width alone while only two panes stand", () => {
    expect(clamped(all({ sidebar: 95, middle: 30 }), TWO)).toEqual(
      all({ sidebar: 100 - share(MINIMUMS.details, TWO), middle: 30 }),
    );
  });

  /// The frame with no list in it has no sidebar spent before its divider and
  /// no middle pane to leave room for beyond it, so what its one divider owes
  /// is a middle pane's width on the left and the details' on the right.
  it("holds the pair to the two floors it has", () => {
    expect(clamped(all({ pair: 95 }), PAIR)).toEqual(
      all({ pair: 100 - share(MINIMUMS.details, PAIR) }),
    );
    expect(clamped(all({ pair: 1 }), PAIR)).toEqual(
      all({ pair: share(MINIMUMS.middle, PAIR) }),
    );
  });

  /// And the workbench's two widths are no business of that frame's, however
  /// far its own has been dragged: a share is a document of its own, and the
  /// model would have to stay right if the two ever met in one browser.
  it("leaves the workbench's widths alone while the pair stands", () => {
    expect(clamped(all({ sidebar: 95, middle: 95, pair: 40 }), PAIR)).toEqual(
      all({ sidebar: 95, middle: 95, pair: 40 }),
    );
  });

  /// The sidebar's divider says where the sidebar ends, so where it is dropped
  /// is the width. The middle one's says where the middle pane ends, which is a
  /// share of the whole frame rather than of what is left of it.
  it("reads a drop as the pane it is the far edge of", () => {
    const settled = all({ sidebar: 20, middle: 30 });

    expect(dragged(settled, "sidebar", 34, THREE).sidebar).toBe(34);
    expect(dragged(settled, "middle", 55, THREE).middle).toBe(35);

    // And the pair's pane starts at the frame's own edge, so where that divider
    // is dropped is the width, as the sidebar's is.
    expect(dragged(DEFAULTS, "pair", 55, PAIR).pair).toBe(55);
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

    // And the pair's, which has nothing spent before it and only the details
    // beyond it.
    expect(range("pair", DEFAULTS, PAIR)).toEqual({
      least: share(MINIMUMS.middle, PAIR),
      most: 100 - share(MINIMUMS.details, PAIR),
    });
  });

  it("moves by a point at a time for a keyboard", () => {
    expect(nudged(DEFAULTS, "sidebar", 1, THREE).sidebar).toBe(
      DEFAULTS.sidebar + 1,
    );
    expect(
      nudged(all({ sidebar: 20, middle: 40 }), "middle", -1, THREE).middle,
    ).toBe(39);

    // And stops where a drag stops.
    const floor = share(MINIMUMS.sidebar, THREE);

    expect(
      nudged(all({ sidebar: floor, middle: 30 }), "sidebar", -1, THREE),
    ).toEqual(all({ sidebar: floor, middle: 30 }));
  });
});

/// The list collapsed to its strip, which is every window under the full band:
/// the column is [`COLLAPSED`] wide whatever share this device settled the
/// sidebar at, so the share is left exactly as it was and is read only for how
/// wide the list expands to.
describe("a collapsed list", () => {
  it("leaves the sidebar's share as it was settled", () => {
    expect(clamped(all({ sidebar: 95, middle: 40 }), COLLAPSED_THREE)).toEqual(
      all({ sidebar: 95, middle: 40 }),
    );
    expect(
      clamped(all({ sidebar: 95 }), { ...COLLAPSED_THREE, three: false }),
    ).toEqual(all({ sidebar: 95 }));
  });

  /// The middle pane starts after the strip rather than after the sidebar's
  /// share, so that is what comes off a drop and what is spent before it.
  it("measures the middle divider from the strip", () => {
    const strip = share(COLLAPSED, COLLAPSED_THREE);

    expect(
      dragged(all({ sidebar: 40 }), "middle", 50, COLLAPSED_THREE).middle,
    ).toBeCloseTo(50 - strip);
    const { least, most } = range(
      "middle",
      all({ sidebar: 40 }),
      COLLAPSED_THREE,
    );
    expect(least).toBeCloseTo(share(MINIMUMS.middle, COLLAPSED_THREE));
    expect(most).toBeCloseTo(
      100 - strip - share(MINIMUMS.details, COLLAPSED_THREE),
    );
  });

  /// It has no divider of its own, so a drag of the middle one writes that
  /// width and nothing else — and a double-click puts back that one alone.
  it("writes down and puts back the middle pane's width alone", () => {
    remember(all({ sidebar: 34, middle: 26 }), COLLAPSED_THREE);

    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(MIDDLE)).toBe("26");

    localStorage.setItem(SIDEBAR, "34");

    expect(restore(all({ sidebar: 34, middle: 26 }), COLLAPSED_THREE)).toEqual(
      all({ sidebar: 34 }),
    );
    expect(localStorage.getItem(SIDEBAR)).toBe("34");
    expect(localStorage.getItem(MIDDLE)).toBeNull();

    // And with no middle pane there is nothing at all to write.
    remember(all({ sidebar: 50, middle: 50 }), {
      ...COLLAPSED_THREE,
      three: false,
    });
    expect(localStorage.getItem(SIDEBAR)).toBe("34");
    expect(localStorage.getItem(MIDDLE)).toBeNull();
  });

  /// How wide it expands to is the share it would stand at full, met against
  /// this frame the way the full band meets it: a share that fits is the share,
  /// one too wide leaves the other two their floors, and a phone has room for
  /// nothing but the sidebar's own.
  it("expands to the share the full band would stand it at", () => {
    expect(expanded(all({ sidebar: 25 }), COLLAPSED_THREE)).toBe(25);
    expect(expanded(all({ sidebar: 60 }), COLLAPSED_THREE)).toBe(
      100 - share(MINIMUMS.middle + MINIMUMS.details, COLLAPSED_THREE),
    );

    const phone: Frame = { ...COLLAPSED_THREE, rem: PHONE / 16 };
    expect(expanded(all({ sidebar: 25 }), phone)).toBe(
      share(MINIMUMS.sidebar, phone),
    );
  });
});

describe("the dividers on the workbench", () => {
  /// One per border there is: the sidebar's wherever the list stands full,
  /// the middle one's wherever the timeline stands beside the details, and
  /// neither on a window that shows one pane at a time. A collapsed list has
  /// none of its own.
  it("puts a handle on every border there is", async () => {
    const full = await bench("full");
    expect(dividers(full.container)).toHaveLength(2);

    const beside = await bench("beside", BETWEEN);
    expect(dividers(beside.container)).toHaveLength(1);
    expect(dividers(beside.container)[0]!.getAttribute("aria-label")).toBe(
      "Resize the timeline pane",
    );

    const narrow = await bench("narrow", PHONE);
    expect(dividers(narrow.container)).toEqual([]);
  });

  /// And the widths themselves reach the frame only where they are drawn: the
  /// sidebar's where the list is full, the middle pane's wherever it stands
  /// beside the details, and nothing of either on a phone — which has only how
  /// wide the collapsed list expands to.
  it("names the widths on the frame only where they are drawn", async () => {
    remember(all({ sidebar: 30, middle: 40 }), THREE);

    const wide = await bench("full");
    expect(wide.frame.style.getPropertyValue("--pane-sidebar")).toBe("30%");
    expect(wide.frame.style.getPropertyValue("--pane-middle")).toBe("40%");
    expect(wide.frame.style.getPropertyValue("--pane-expanded")).toBe("");

    const between = await bench("beside", BETWEEN);
    expect(between.frame.style.getPropertyValue("--pane-sidebar")).toBe("");
    expect(between.frame.style.getPropertyValue("--pane-middle")).toBe("40%");

    const narrow = await bench("narrow", PHONE);
    expect(narrow.frame.style.getPropertyValue("--pane-sidebar")).toBe("");
    expect(narrow.frame.style.getPropertyValue("--pane-middle")).toBe("");
  });

  /// A share dragged where the list stands full is the width it expands to
  /// where it is collapsed, and a phone expands it to the sidebar's floor.
  it("expands the collapsed list to the share dragged at full width", async () => {
    const { container } = await bench("full");

    dragTo(dividers(container)[0]!, FRAME * 0.25);
    await waitFor(() => expect(localStorage.getItem(SIDEBAR)).toBe("25"));
    cleanup();

    const between = await bench("beside", BETWEEN);
    expect(between.frame.style.getPropertyValue("--pane-expanded")).toBe(
      `${(25 * BETWEEN) / 16 / 100}rem`,
    );

    const phone = await bench("narrow", PHONE);
    expect(phone.frame.style.getPropertyValue("--pane-expanded")).toBe(
      `${MINIMUMS.sidebar}rem`,
    );
  });

  /// With nothing open the list is the level being shown, so a collapsed one
  /// stands expanded over the empty paper beside it — and opening a
  /// Conversation is what lets it collapse.
  it("stands the collapsed list expanded while nothing is open", async () => {
    const list = (container: ParentNode) =>
      container.querySelector<HTMLElement>(`.${shell.conversationsPane}`)!;

    for (const width of ["narrow", "beside"] as const) {
      const { container } = await bench(width);
      expect(list(container).hasAttribute("data-expanded"), width).toBe(true);
      cleanup();
    }

    const full = await bench("full");
    expect(list(full.container).hasAttribute("data-expanded")).toBe(false);
    cleanup();

    windowIs("beside");
    theWorkbench();
    const { container } = mount(`/conversations/${OPEN.id}`);
    await drawn(container, `.${shell.panes}`);
    expect(list(container).hasAttribute("data-expanded")).toBe(false);
  });

  it("starts where this device left the panes", async () => {
    remember(all({ sidebar: 40, middle: 25 }), THREE);

    const { frame } = await bench("full");
    expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("40%");
  });

  it("moves the border the pointer drags, and writes it down", async () => {
    const { container, frame } = await bench("full");

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
    const { container, frame } = await bench("full");

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
    remember(all({ sidebar: 5, middle: 30 }), THREE);

    const wider = await bench("full", 2400);
    expect(wider.frame.style.getPropertyValue("--pane-sidebar")).toBe(
      `${share(MINIMUMS.sidebar, { ...THREE, rem: 150 })}%`,
    );

    const wide = await bench("full");
    expect(wide.frame.style.getPropertyValue("--pane-sidebar")).toBe(
      `${share(MINIMUMS.sidebar, THREE)}%`,
    );
  });

  it("puts the defaults back on a double-click", async () => {
    remember(all({ sidebar: 40, middle: 25 }), THREE);

    const { container, frame } = await bench("full");
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

  /// The defaults *this frame* has, that is. Between the two lines only the
  /// middle divider is up, and a double-click on it is a human asking for the
  /// width they can see back — not for the sidebar's, which they have no handle
  /// on and will not meet again until the window is wider.
  ///
  /// Said here because it is the one place the bands' widths could get mixed
  /// up without anybody noticing: what a double-click here left behind would
  /// show up as a column somebody never touched, on some other afternoon, at
  /// some other window size.
  it("leaves the sidebar's width alone where its divider is not up", async () => {
    remember(all({ sidebar: 25, middle: 40 }), THREE);

    const { container, frame } = await bench("beside", BETWEEN);
    expect(dividers(container)).toHaveLength(1);

    fireEvent.dblClick(dividers(container)[0]!);

    // The default, held to the timeline's floor: a 70rem window owes the
    // timeline more of itself than the default share is.
    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-middle")).toBe(
        `${share(MINIMUMS.middle, COLLAPSED_THREE)}%`,
      ),
    );
    expect(localStorage.getItem(MIDDLE)).toBeNull();

    // And the share the full band would be drawn at is where this device left
    // it, in the storage and in how wide the collapsed list expands to.
    expect(localStorage.getItem(SIDEBAR)).toBe("25");
    expect(frame.style.getPropertyValue("--pane-expanded")).toBe(
      `${(25 * BETWEEN) / 16 / 100}rem`,
    );
  });

  /// And dragged there, the middle divider is the far edge of a pane that
  /// starts after the strip, so the strip is what comes off where it was
  /// dropped.
  it("moves the middle divider from the strip between the two lines", async () => {
    const { container, frame } = await bench("beside", BETWEEN);

    dragTo(dividers(container)[0]!, BETWEEN * 0.5);

    const strip = share(COLLAPSED, COLLAPSED_THREE);
    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-middle")).toBe(
        `${50 - strip}%`,
      ),
    );
    expect(localStorage.getItem(MIDDLE)).toBe(String(50 - strip));
    expect(localStorage.getItem(SIDEBAR)).toBeNull();
  });

  /// A handle nobody can put a pointer on is still a handle, so the arrow keys
  /// move it — and settle it, there being no letting go of a key.
  it("moves with the arrow keys too", async () => {
    const { container, frame } = await bench("full");

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
    const { container } = await bench("full");
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
  it("builds every layout out of the shares the page names", () => {
    expect(stylesheet).toContain(
      "grid-template-columns: var(--pane-sidebar, 20%) auto 1fr;",
    );
    expect(stylesheet).toContain(
      "grid-template-columns:\n" +
        "      var(--pane-sidebar, 20%) auto var(--pane-middle, 30%) auto 1fr;",
    );

    // And the frame with no list in it, whose one divider stands where the
    // sidebar's and the middle pane's do in the others.
    expect(stylesheet).toContain(
      "  .panes.two {\n" +
        "    grid-template-columns: var(--pane-pair, 40%) auto 1fr;\n  }",
    );

    // And the list's own two bands below the full one: the strip beside one
    // pane, and the strip beside the timeline and the details.
    expect(stylesheet).toContain(
      ".panes.listing {\n" +
        `  grid-template-columns: ${COLLAPSED}rem 1fr;\n`,
    );
    expect(stylesheet).toContain(
      `    grid-template-columns: ${COLLAPSED}rem var(--pane-middle, 30%) auto 1fr;`,
    );

    // The breakpoints the page asks `matchMedia` about are the ones the rules
    // are written at, and nothing may let the two drift apart.
    expect(stylesheet).toContain(`@media ${BESIDE} {`);
    expect(stylesheet).toContain(`@media ${FULL} {`);
  });

  /// Collapsed, the list is laid out at the width it expands to and clipped to
  /// the strip, so every height in it is its expanded height; expanded, the
  /// clip lets the whole pane and its shadow out over the panes beside it,
  /// under a pointer resting on it, a keyboard's focus inside it, or the page
  /// saying nothing is open.
  it("clips the collapsed list to its strip and lets it out over the rest", () => {
    const collapsed = layout("(width < 100rem)");

    expect(collapsed).toContain(
      "  .panes.listing > .conversationsPane {\n" +
        "    z-index: 1;\n" +
        "    width: var(--pane-expanded, 16rem);\n" +
        "    background: var(--paper);\n" +
        `    clip-path: inset(0 calc(100% - ${COLLAPSED}rem) 0 0);\n  }`,
    );
    expect(collapsed).toContain(
      "  .panes.listing > .conversationsPane[data-expanded],\n" +
        "  .panes.listing > .conversationsPane:focus-visible,\n" +
        "  .panes.listing > .conversationsPane:has(:focus-visible) {\n" +
        "    clip-path: inset(0 -3rem 0 0);\n" +
        "    box-shadow: var(--lift);\n  }",
    );
    expect(layout("(width < 100rem) and (hover: hover)")).toContain(
      "  .panes.listing > .conversationsPane:hover {\n" +
        "    clip-path: inset(0 -3rem 0 0);\n" +
        "    box-shadow: var(--lift);\n  }",
    );

    // And moving between the two only where movement is welcome.
    expect(
      layout("(width < 100rem) and (prefers-reduced-motion: no-preference)"),
    ).toContain("clip-path 150ms ease");
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
    // draws an edge down the side of a column a divider stands beside. The one
    // edge a pane draws is the collapsed strip's, which has no divider — and
    // from 60rem the details pane, beyond the middle divider, draws none.
    expect(stylesheet).not.toContain("border-right: 1px solid var(--edge)");
    expect(layout("(width < 100rem)")).toContain(
      "    border-left: 1px solid var(--edge);\n",
    );
    expect(layout(BESIDE)).toContain(
      "  .panes.listing > .detailsPane {\n    border-left: 0;\n  }",
    );
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

  /// The composer is the one thing a details pane holds that is not read down
  /// the page: a box to fill in and a press under it, standing in the middle of
  /// whatever room the pane has. Which takes the column padding off — the box
  /// centres itself across the pane exactly as it did across the column, and
  /// the pane's header goes back to the pane's own edge rather than hanging a
  /// column's width in from it.
  it("stands the composer in the middle of its pane, against the pane's edge", () => {
    expect(stylesheet).toContain(
      ".panes > .detailsPane:has(.paneComposer) {\n" +
        "  flex-direction: column;\n" +
        "  min-height: 100dvh;\n" +
        "  padding-inline: 1.25rem;\n}",
    );

    // Auto margins rather than a centring that would hang a tall composer over
    // both edges with the top of it out of reach: what they take is the room
    // that is left over, and there is none to take on a phone.
    expect(stylesheet).toContain(".paneComposer {\n  margin-block: auto;\n}");

    // A column at each of the widths that show the pane, the pane being shown
    // by a different rule at each of them.
    expect(stylesheet).toContain(
      '.panes[data-pane="details"] > .detailsPane:has(.paneComposer) {\n' +
        "  display: flex;\n}",
    );
    expect(stylesheet).toContain(
      "  .panes.listing > .detailsPane:has(.paneComposer),\n" +
        "  .panes.listing > .detailsPane:has(.paneScreen) {\n    display: flex;\n  }",
    );
    expect(stylesheet).toContain(
      "  .panes > .detailsPane:has(.paneComposer) {\n    display: flex;\n  }",
    );

    // And the window's height it asked for while it was the page goes where
    // every other pane's does: above the width where the frame is that tall.
    expect(stylesheet).toContain(
      `@media ${BESIDE} {\n` +
        "  .panes > .detailsPane:has(.paneComposer) {\n" +
        "    min-height: auto;\n  }\n}",
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

    // And the frame with a list is all of that at every width, a phone's
    // included: the list stands beside a pane everywhere.
    expect(stylesheet).toContain(
      ".panes.listing {\n" +
        "  grid-template-columns: 3rem 1fr;\n" +
        "  height: 100dvh;\n" +
        "  overflow: hidden;\n}",
    );
    expect(stylesheet).toContain(
      ".panes.listing > .pane {\n" +
        "  overflow-y: auto;\n" +
        "  overscroll-behavior: contain;\n}",
    );
    expect(stylesheet).toContain(
      ".panes.listing > .detailsPane:has(.paneScreen) {\n  height: auto;\n}",
    );
    expect(stylesheet).toContain(
      ".panes.listing > .detailsPane:has(.paneComposer) {\n  min-height: auto;\n}",
    );

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
    expect(attachedCss).toMatch(
      /\.screen \.terminalHost \{[^}]*overscroll-behavior: contain;/,
    );
  });

  /// And the half of that clip which is not an `overflow` at all: the frame and
  /// each pane are containing blocks, so that there is nothing for the clip to
  /// miss.
  ///
  /// An `overflow` reaches an absolutely positioned descendant only where the
  /// box carrying it is what that descendant is laid out from. Positioned, both
  /// boxes are; static, neither is, and such an element is laid out against the
  /// document instead — which is the one thing in the app no clip and no height
  /// can reach.
  it("is what everything inside it is laid out from", () => {
    expect(stylesheet).toContain(".panes {\n  position: relative;\n");
    expect(stylesheet).toContain(".pane {\n  position: relative;\n");
  });
});

/// And the same thing asked of the page rather than of the stylesheet: whatever
/// the workbench draws, nothing in it is laid out against the document.
///
/// This is the rule the frame's own height and its `overscroll-behavior` cannot
/// state. Both are about boxes standing in the flow, and an absolutely
/// positioned element with no positioned ancestor is in nobody's flow: it is
/// placed where its pane has scrolled to, measured from the top of the document,
/// and it makes the document that tall. A span a screen reader is meant to find
/// and nobody is meant to see is enough — which is what put a scrollbar down the
/// side of the whole workbench, outside every pane, and pushed the frame off the
/// screen when it was used.
///
/// jsdom lays nothing out, and this needs no layout: what is asked is which box
/// each element would be laid out *from*, and that is the cascade rather than
/// the geometry. So the walk below is the browser's own — up the ancestors to
/// the first that is positioned — over the real page, with the real stylesheets
/// behind it.
describe("what the frame is laid out from", () => {
  /// The box an absolutely positioned element is laid out from: the nearest
  /// ancestor that is positioned, or `null` where the walk reaches the top of
  /// the frame without finding one — which is the document, and the whole of
  /// what must never happen.
  function laidOutFrom(element: Element, frame: Element): Element | null {
    for (let at = element.parentElement; at !== null; at = at.parentElement) {
      if (positioned(at)) {
        return at;
      }
      if (at === frame) {
        return null;
      }
    }

    return null;
  }

  /// Whether a box is one the next thing down may be laid out from.
  ///
  /// Said as the four values that are rather than as everything that is not
  /// `static`, because jsdom answers for a property nothing declared with the
  /// empty string rather than with its initial value — and read the other way
  /// round, every element on the page would be a containing block and the walk
  /// above would stop on the first one it met.
  function positioned(element: Element): boolean {
    return ["relative", "absolute", "fixed", "sticky"].includes(
      getComputedStyle(element).position,
    );
  }

  /// How an element is named when it is the one at fault, there being nothing
  /// else to call it: its tag and whatever classes it is wearing.
  function named(element: Element): string {
    return `${element.tagName.toLowerCase()}.${[...element.classList].join(".")}`;
  }

  /// Everything the page has drawn that is absolutely positioned, which is what
  /// the question is about — and what makes an empty answer worth having.
  function loose(frame: Element): Element[] {
    return [...frame.querySelectorAll("*")].filter(
      (element) => getComputedStyle(element).position === "absolute",
    );
  }

  /// The page, with a pane opened over each of the fixtures that draws one of
  /// these. Every one of them is the workbench: the frame is where the rule is
  /// written, so the pages that stand in it are what has to hold it.
  async function drawnPage(at?: string) {
    windowIs("full");
    theWorkbench(
      whenever(`/api/ui/conversations/${TASKED.id}`, json(TASKED)),
      whenever(`/api/ui/conversations/${STAGED.id}`, json(STAGED)),
    );

    const { container } = mount(at);
    const frame = await drawn<HTMLElement>(container, `.${shell.panes}`);

    // Nothing is asked of a page that has not finished arriving: the pinned
    // lists are the last thing on it and the whole of what two of these mount.
    await waitFor(() =>
      expect(frame.querySelectorAll(`.${shell.pane} *`).length).toBeGreaterThan(
        20,
      ),
    );

    return frame;
  }

  /// The three pages: the conversation the fixtures open, and the two carrying
  /// a pinned list — a task list and a roadmap, which are where the spans that
  /// broke this are drawn.
  ///
  /// Swept together rather than one test each, because the sweep only says
  /// something where it found something: a page drawing nothing positioned is
  /// worth walking and is no evidence on its own, and it is the three of them
  /// between them that make an empty answer mean anything.
  const PAGES = [
    undefined,
    `/conversations/${TASKED.id}`,
    `/conversations/${STAGED.id}`,
  ];

  /// Every absolutely positioned element the three pages draw, with the box it
  /// is laid out from and the pane it stands in.
  async function swept() {
    const found = [];

    for (const at of PAGES) {
      const frame = await drawnPage(at);
      for (const element of loose(frame)) {
        found.push({
          element,
          from: laidOutFrom(element, frame),
          pane: element.closest(`.${shell.pane}`),
        });
      }
      cleanup();
    }

    // A sweep that found nothing to ask about would pass while saying nothing.
    expect(
      found.length,
      "the pages should be drawing something absolutely positioned",
    ).toBeGreaterThan(0);

    return found;
  }

  it("lays nothing out against the document", async () => {
    expect(
      (await swept())
        .filter((one) => one.from === null)
        .map((one) => named(one.element)),
    ).toEqual([]);
  });

  /// And the stronger reading of the same rule, which is what the pane's own
  /// `position` buys over the frame's: a stray position stays inside the pane it
  /// was written in. The frame clips only above the first breakpoint; a pane is
  /// the box that scrolls at every width, so a pane is where a mistake belongs.
  it("keeps what a pane holds inside it", async () => {
    expect(
      (await swept())
        .filter(
          (one) =>
            one.pane !== null &&
            (one.from === null || !one.pane.contains(one.from)),
        )
        .map((one) => named(one.element)),
    ).toEqual([]);
  });
});

/// The frame apart from the page that had it first: it draws three panes it is
/// handed and knows nothing about what is in them, which is what lets the
/// settings page stand on the same one.
describe("the frame on its own", () => {
  it("draws the three panes it is handed, in the order they are walked", () => {
    windowIs("full");

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
    windowIs("full");

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

/// And the frame a share is drawn in: the same frame handed no conversations
/// pane, which is two panes with one border between them. What that border does
/// is what every other divider does — dragged, nudged, and put back — over a
/// width that is the pair's own.
describe("the divider between two panes with no list beside them", () => {
  /// Mounted the way a share mounts it: no conversations pane at all, and so no
  /// sidebar and no third level for a breakpoint to bring in.
  function pair(width: Parameters<typeof windowIs>[0], wide = FRAME) {
    across = wide;
    windowIs(width);

    const { container } = render(() => (
      <Panes
        pane="middle"
        middleLabel="Timeline"
        middle={<p>the record</p>}
        details={<p>what it has open</p>}
      />
    ));

    return {
      container,
      frame: container.querySelector<HTMLElement>(`.${shell.panes}`)!,
    };
  }

  /// One divider, between the two panes it parts — and it is there from the
  /// width the two stand side by side at rather than from the one the third
  /// pane would have arrived at.
  it("puts one handle between the panes, and none below the breakpoint", () => {
    const beside = pair("beside");

    expect(
      [...beside.frame.children].map((child) =>
        child.getAttribute("aria-label"),
      ),
    ).toEqual(["Timeline", "Resize the timeline pane", "Details"]);

    // A wider window changes nothing: there is no list to stand full, so
    // there is no second border for one to make.
    expect(dividers(pair("full").container)).toHaveLength(1);

    // And below the breakpoint the frame is walked one pane at a time, with no
    // border to drag and nothing this device remembers being read.
    const narrow = pair("narrow");
    expect(dividers(narrow.container)).toEqual([]);
    expect(narrow.frame.style.getPropertyValue("--pane-pair")).toBe("");
  });

  it("starts where this device left the two panes", () => {
    remember(all({ pair: 55 }), PAIR);

    const { frame } = pair("beside");

    expect(frame.style.getPropertyValue("--pane-pair")).toBe("55%");

    // And names nothing else on the frame: the widths the workbench's columns
    // are drawn at belong to a frame that is not standing here.
    expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("");
    expect(frame.style.getPropertyValue("--pane-middle")).toBe("");
  });

  it("moves the border the pointer drags, and writes it down", async () => {
    const { container, frame } = pair("beside");

    dragTo(dividers(container)[0]!, FRAME * 0.5);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-pair")).toBe("50%"),
    );
    expect(localStorage.getItem(PAIR_KEY)).toBe("50");

    // And what the workbench's frame was left at is still what it was left at.
    expect(localStorage.getItem(SIDEBAR)).toBeNull();
    expect(localStorage.getItem(MIDDLE)).toBeNull();
  });

  it("holds both panes to their minimums however far the pointer goes", async () => {
    const { container, frame } = pair("beside");

    dragTo(dividers(container)[0]!, FRAME * 0.98);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-pair")).toBe(
        `${100 - share(MINIMUMS.details, PAIR)}%`,
      ),
    );

    dragTo(dividers(container)[0]!, -FRAME);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-pair")).toBe(
        `${share(MINIMUMS.middle, PAIR)}%`,
      ),
    );
  });

  it("moves with the arrow keys, and gives the default back on a double-click", async () => {
    remember(all({ pair: 55 }), PAIR);

    const { container, frame } = pair("beside");

    fireEvent.keyDown(dividers(container)[0]!, { key: "ArrowLeft" });

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-pair")).toBe("54%"),
    );
    expect(localStorage.getItem(PAIR_KEY)).toBe("54");

    fireEvent.dblClick(dividers(container)[0]!);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-pair")).toBe(
        `${DEFAULTS.pair}%`,
      ),
    );
    expect(localStorage.getItem(PAIR_KEY)).toBeNull();
  });

  /// What the handle is, for anything reading the page rather than looking at
  /// it: a separator named for the pane it moves, carrying the share of the
  /// frame that pane is worth and how far it may be taken.
  it("says what it is and where it stands", () => {
    const { container } = pair("beside");
    const [divider] = dividers(container);

    expect(divider!.getAttribute("role")).toBe("separator");
    expect(divider!.getAttribute("aria-orientation")).toBe("vertical");
    expect(divider!.getAttribute("aria-valuenow")).toBe(String(DEFAULTS.pair));
    expect(divider!.getAttribute("aria-valuemin")).toBe(
      String(Math.round(share(MINIMUMS.middle, PAIR))),
    );
    expect(divider!.getAttribute("aria-valuemax")).toBe(
      String(Math.round(100 - share(MINIMUMS.details, PAIR))),
    );
  });

  /// A share is opened off a disk as often as off a server, and some of those
  /// contexts have no storage to be had at all. The divider still moves; the
  /// width simply lasts as long as the tab.
  it("drags all the same where the browser refuses storage", async () => {
    const refused = () => {
      throw new Error("storage is not available");
    };

    vi.stubGlobal("localStorage", {
      getItem: refused,
      setItem: refused,
      removeItem: refused,
      clear() {},
    });

    const { container, frame } = pair("beside");
    expect(frame.style.getPropertyValue("--pane-pair")).toBe(
      `${DEFAULTS.pair}%`,
    );

    dragTo(dividers(container)[0]!, FRAME * 0.5);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-pair")).toBe("50%"),
    );

    // And a double-click, which is the other half of what a divider writes.
    fireEvent.dblClick(dividers(container)[0]!);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-pair")).toBe(
        `${DEFAULTS.pair}%`,
      ),
    );
  });
});

/// And the frame the workbench is drawn in while the Conversation it is reading
/// has nothing on its record but the one Event: the same frame handed no middle
/// pane, which is the sidebar and the details with one border between them.
///
/// The border is the sidebar's own, moving the sidebar's own width — so the
/// arithmetic is the three-pane frame's arithmetic with the middle column taken
/// out, and what this device settled that column at is left where it is for the
/// moment a second Event brings it back.
describe("the frame with no middle pane in it", () => {
  /// Mounted the way the workbench mounts it while a record is the one Event:
  /// a list to pick from, and nothing between it and the details.
  function widened(width: Parameters<typeof windowIs>[0], wide = FRAME) {
    across = wide;
    windowIs(width);

    const { container } = render(() => (
      <Panes
        pane="details"
        middleLabel="Timeline"
        conversations={<p>the list</p>}
        details={<p>the composer</p>}
      />
    ));

    return {
      container,
      frame: container.querySelector<HTMLElement>(`.${shell.panes}`)!,
    };
  }

  /// Two panes and the sidebar's border where the list stands full, and no
  /// border at all where it is collapsed: there is no middle pane for the
  /// middle divider to be the far edge of.
  it("puts one handle between the panes, and none while the list is collapsed", () => {
    const wide = widened("full");

    expect(
      [...wide.frame.children].map((child) => child.getAttribute("aria-label")),
    ).toEqual(["Conversations", "Resize the conversations pane", "Details"]);
    expect(wide.frame.querySelector(`.${shell.middlePane}`)).toBeNull();

    const beside = widened("beside");
    expect(
      [...beside.frame.children].map((child) =>
        child.getAttribute("aria-label"),
      ),
    ).toEqual(["Conversations", "Details"]);
    expect(beside.frame.style.getPropertyValue("--pane-sidebar")).toBe("");
    expect(beside.frame.style.getPropertyValue("--pane-middle")).toBe("");

    const narrow = widened("narrow");
    expect(dividers(narrow.container)).toEqual([]);
    expect(narrow.frame.style.getPropertyValue("--pane-sidebar")).toBe("");
  });

  /// The sidebar's width, and no width for the column that is not drawn: a name
  /// written over a column that is not there would be this frame answering for
  /// the one the Timeline comes back to.
  it("names the sidebar's width and no other", () => {
    remember(all({ sidebar: 34, middle: 34 }), THREE);

    const { frame } = widened("full");

    expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("34%");
    expect(frame.style.getPropertyValue("--pane-middle")).toBe("");
    expect(frame.style.getPropertyValue("--pane-pair")).toBe("");
  });

  /// And the drag writes the sidebar's own key, leaving the middle pane's
  /// exactly as this device left it — which is what hands the three-pane frame
  /// its columns back the moment a second Event puts the Timeline up.
  it("moves the sidebar and leaves the timeline's width where it was", async () => {
    remember(all({ sidebar: 20, middle: 33 }), THREE);

    const { container, frame } = widened("full");

    dragTo(dividers(container)[0]!, FRAME * 0.3);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe("30%"),
    );
    expect(localStorage.getItem(SIDEBAR)).toBe("30");
    expect(localStorage.getItem(MIDDLE)).toBe("33");
  });

  /// What has to be left standing beyond the sidebar is the details alone: the
  /// pane the sidebar is beside *is* the details here, and there is no middle
  /// pane owed anything between them.
  it("leaves the details their own width and nothing else's", async () => {
    const { container, frame } = widened("full");

    dragTo(dividers(container)[0]!, FRAME * 0.98);

    await waitFor(() =>
      expect(frame.style.getPropertyValue("--pane-sidebar")).toBe(
        `${100 - share(MINIMUMS.details, WIDENED)}%`,
      ),
    );
  });

  /// And the rules that draw it, jsdom laying out no grids: the sidebar's
  /// column and everything after it, the details shown whatever level
  /// `data-pane` names, and the way back out taken off a pane whose list is
  /// already beside it.
  it("draws the two columns and takes the way back off", () => {
    expect(stylesheet).toContain(
      `@media ${FULL} {\n` +
        "  .panes.widened {\n" +
        "    grid-template-columns: var(--pane-sidebar, 20%) auto 1fr;\n  }",
    );
    expect(stylesheet).toContain(
      "  .panes.widened {\n" +
        `    grid-template-columns: ${COLLAPSED}rem 1fr;\n  }`,
    );
    expect(layout(BESIDE)).toContain(
      "  .panes.listing > .middlePane,\n" +
        "  .panes.listing > .detailsPane {\n    display: block;\n  }",
    );
    expect(layout(BESIDE)).toContain(
      "  .panes.listing .detailsPane .paneBack,\n" +
        "  .panes.listing .paneForward {\n    display: none;\n  }",
    );

    // And what the pane holds is still read at the app's own measure, in the
    // middle of whatever room the pane has: the rule is the details pane's own
    // and says nothing about which frame it is standing in.
    expect(stylesheet).toContain(
      ".panes > .detailsPane {\n" +
        "  padding-inline: max(1.25rem, (100% - 60rem) / 2);\n}",
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
