//! The table of contents down a Set's margin: the way around a Set whose Diff
//! can run to thousands of lines.
//!
//! One nav in two shapes — the sidebar in the pane's margin, and the bar with
//! the list under it where the pane is too narrow to keep one — so which of the
//! two the reader gets is measured off the pane. These tests ask about the one
//! list both are drawn from, the jump that lands on a folded file, and the
//! highlight that follows the reader down the page.
//!
//! What the browser knows about where the page is scrolled to, jsdom has none
//! of: the scroll-spy's observer is stood in for, so a test says what has
//! crossed the reading line rather than laying out a page to make it happen.
//! What is being asked is what the page does with the answer.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SetView } from "../src/api/types";
// The nav's own names, which are the module's now rather than the page's.
import contents from "../src/set/Contents.module.css";
import { reading, texts } from "./reading";
import { readable } from "./serving";
import alongside from "./fixtures/set-alongside.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };
import locked from "./fixtures/set-locked.json" with { type: "json" };

const WAITING = readable(answering);

/// The Set whose Diff is two repositories': what the nav groups.
const ALONGSIDE = readable(alongside);

/// The two settled standings, each given the Diff the waiting fixture carries:
/// a Set is read for what it asked about however it stands, so the nav has the
/// same work to do on all three.
const ANSWERED: SetView = { ...readable(answered), diff: WAITING.diff };
const LOCKED: SetView = { ...readable(locked), diff: WAITING.diff };

/// A Set with neither of the two sections a Set can be without.
const BARE: SetView = { ...WAITING, preface_html: null, diff: [] };

/// What one crossing of the reading line says, as the spy is told it.
type Crossing = { target: { id: string }; isIntersecting: boolean };

/// One page's scroll-spy, stood in for: what it was told to watch, and the way
/// to tell it something has crossed the line.
type Spy = {
  observing: string[];
  disconnected: boolean;
  margin: string | undefined;
  cross(started: Record<string, boolean>): void;
};

/// The spies the page makes while a test runs, newest last. Stubbed for the
/// whole file: a page that made a real observer here would watch a document
/// jsdom never lays out, and say nothing ever.
function watching(): Spy[] {
  const spies: Spy[] = [];

  class Stub {
    private readonly told: (crossings: Crossing[]) => void;
    private readonly spy: Spy;

    constructor(
      told: (crossings: Crossing[]) => void,
      watch?: { rootMargin?: string },
    ) {
      this.told = told;
      this.spy = {
        observing: [],
        disconnected: false,
        margin: watch?.rootMargin,
        cross: (started) => {
          this.told(
            Object.entries(started).map(([id, isIntersecting]) => ({
              target: { id },
              isIntersecting,
            })),
          );
        },
      };
      spies.push(this.spy);
    }

    observe(target: Element) {
      this.spy.observing.push(target.id);
    }

    unobserve() {}

    disconnect() {
      this.spy.disconnected = true;
    }

    takeRecords() {
      return [];
    }
  }

  vi.stubGlobal("IntersectionObserver", Stub);
  return spies;
}

let spies: Spy[];

beforeEach(() => {
  spies = watching();
  // jsdom lays nothing out and so scrolls nothing; what the jump is worth is
  // that it asked the browser to take the reader there. Asked for no motion —
  // the stub reads as `prefers-reduced-motion: reduce` — the jump asks with
  // `scrollIntoView`, which is the ask these tests can see: the animated ask
  // works from geometry, and jsdom has none to give it.
  Element.prototype.scrollIntoView = vi.fn();
  vi.stubGlobal("matchMedia", () => ({ matches: true }));
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  localStorage.clear();
});

/// The nav's scroll-spy, told apart by the reading line it watches with — the
/// margin that puts the line near the bottom of the window. The last one made,
/// since a test reads one Set at a time.
function spy(): Spy {
  const spying = spies.filter((made) => made.margin !== undefined).at(-1);
  expect(spying, "expected the page to be following the reader").toBeTruthy();
  return spying as Spy;
}

/// The nav, which every page has.
function navOf(page: ParentNode): HTMLElement {
  const nav = page.querySelector<HTMLElement>(`nav.${contents.contents}`);
  expect(nav, "expected a table of contents in the page").toBeTruthy();
  return nav as HTMLElement;
}

/// Every jump the nav offers, in the order it lists them.
function jumps(nav: ParentNode): string[] {
  return [...nav.querySelectorAll(`a.${contents.link}`)].map(
    (link) => link.getAttribute("href") ?? "",
  );
}

/// The nav's line jumping to this id.
function line(nav: ParentNode, anchor: string): HTMLAnchorElement {
  const found = nav.querySelector<HTMLAnchorElement>(`a[href="#${anchor}"]`);
  expect(found, `expected a line jumping to #${anchor}`).toBeTruthy();
  return found as HTMLAnchorElement;
}

/// Where the nav says the reader is: the jump the highlighted line points at.
///
/// Read off the marked line rather than out of a class list, because what the
/// highlight is worth is which part of the page it names.
function highlighted(nav: ParentNode): string {
  const lit = nav.querySelectorAll(`a.${contents.link}.${contents.here}`);
  expect(lit, "exactly one line is ever the highlight").toHaveLength(1);
  return lit[0]!.getAttribute("href")!.replace("#", "");
}

/// What the bar says it is at — the whole of the nav on a narrow viewport,
/// until it is tapped.
function barSays(nav: ParentNode): string {
  const said = nav.querySelector(`.${contents.barName}`);
  expect(said, "expected a bar naming where the reader is").toBeTruthy();
  return said!.textContent ?? "";
}

describe("the table of contents", () => {
  it("mirrors the page top to bottom", async () => {
    const nav = navOf(await reading(WAITING));

    expect(
      jumps(nav),
      "the nav lists the sections in the order the page has them",
    ).toEqual([
      "#preface",
      "#diff",
      "#diff-1",
      "#diff-2",
      "#questions",
      "#q1",
      "#q2",
      "#q3",
      "#postscript",
    ]);
    expect(
      nav.querySelector('a[href="#q2a"]'),
      "a Sub-question scrolls into view with its parent, so it is not listed " +
        "separately",
    ).toBeNull();
    expect(
      line(nav, "q1").textContent,
      "a Question is listed by its label and its own words",
    ).toContain("Q1");
    expect(line(nav, "q1").textContent).toContain(
      "Where should the request counter live?",
    );
  });

  it("names a markdown Question by its words alone", async () => {
    // The nav is a line of text in a narrow column, and the page draws that
    // same Question as blocks: the two are rendered from the one source, and
    // this is the seam where they meet.
    const nav = navOf(await reading(WAITING));

    expect(
      line(nav, "q1").textContent,
      "the nav wants the words, with the list flattened into the line",
    ).toContain(
      "Where should the request counter live? in-process, per instance in " +
        "redis, shared across instances",
    );
    expect(line(nav, "q2").textContent, "a fenced block is words too").toContain(
      "How should a throttled client be told to back off? fn allowance()",
    );
    expect(
      nav.querySelectorAll("p, ul, pre, code"),
      "the nav is text, so the markup the Question is drawn with has no place " +
        "in it",
    ).toHaveLength(0);
  });

  it("names the Diff's files in Diff order, and in full on hover", async () => {
    const nav = navOf(await reading(WAITING));

    // The paths travel with the Set rather than being read back out of the
    // rendered Diff, and the nth of them has to be what the nth fold shows.
    expect(line(nav, "diff-1").textContent).toBe("src/limits.rs");
    expect(line(nav, "diff-2").textContent).toBe("notes.txt");
    expect(
      line(nav, "diff-1").title,
      "the column is narrow, so the whole of a cut path is readable here",
    ).toBe("src/limits.rs");
  });

  it("groups a Diff's files under the repository each came out of", async () => {
    const nav = navOf(await reading(ALONGSIDE));

    expect(
      texts(nav, `li.${contents.group}`),
      "one heading per block, in the order the Diff draws them",
    ).toEqual(["verkstead", "askance"]);

    // Still one *Diff* entry with the files under it, and a file still jumps
    // to the fold it names — the grouping is a heading among the lines rather
    // than a section of its own.
    expect(jumps(nav)).toEqual([
      "#preface",
      "#diff",
      "#diff-1",
      "#diff-2",
      "#diff-3",
      "#questions",
      "#q1",
      "#q2",
      "#q3",
      "#postscript",
    ]);
    expect(line(nav, "diff-3").textContent).toBe("src/set.rs");
  });

  it("names no repository over a Diff of one", async () => {
    const nav = navOf(await reading(WAITING));

    expect(
      nav.querySelectorAll(`li.${contents.group}`),
      "the label earns its place when repos mix, in the nav as on the page",
    ).toHaveLength(0);
  });

  it("lists only the sections the Set has", async () => {
    const nav = navOf(await reading(BARE));

    expect(
      jumps(nav),
      "the Questions are the one section every Set has",
    ).toEqual(["#questions", "#q1", "#q2", "#q3", "#postscript"]);
  });

  it("is one nav, one bar and one list on every standing", async () => {
    for (const set of [WAITING, ANSWERED, LOCKED]) {
      const page = await reading(set);

      // Which of the bar and the sidebar the reader gets is the stylesheet's
      // business at a width, so there is no second copy to fall out of step.
      expect(page.querySelectorAll(`nav.${contents.contents}`)).toHaveLength(1);
      expect(page.querySelectorAll(`button.${contents.bar}`)).toHaveLength(1);
      expect(page.querySelectorAll(`ol.${contents.sections}`)).toHaveLength(1);

      // A Set is read for what it asked about however it stands.
      for (const jump of ["#preface", "#diff-1", "#questions", "#q1"]) {
        expect(jumps(navOf(page)), `expected ${jump}`).toContain(jump);
      }
    }
  });
});

describe("the highlight", () => {
  it("is on the first section before anything has scrolled", async () => {
    const nav = navOf(await reading(WAITING));

    expect(
      highlighted(nav),
      "a page nobody has scrolled reads as being at the top of it",
    ).toBe("preface");
    expect(
      nav.querySelectorAll(`.${contents.within}`),
      "the quiet mark is on the section the highlight is inside, and at the " +
        "top of the page there is none",
    ).toHaveLength(0);
    expect(line(nav, "preface").getAttribute("aria-current")).toBe("location");
  });

  it("starts on whatever section the Set starts with", async () => {
    const nav = navOf(await reading({ ...WAITING, preface_html: null }));

    expect(
      highlighted(nav),
      "with no Preface the page opens on the Diff, and the first line of the " +
        "nav is the Diff's",
    ).toBe("diff");
  });

  it("follows the reader to the last part of the page to have begun", async () => {
    const nav = navOf(await reading(WAITING));

    spy().cross({ preface: true, diff: true, "diff-1": true });

    expect(highlighted(nav), "the file is where they are").toBe("diff-1");
    expect(
      line(nav, "diff").className,
      "and the Diff only says they are in it",
    ).toContain(contents.within!);
    expect(
      line(nav, "diff").getAttribute("aria-current"),
      "the section around them is not where they are",
    ).toBeNull();
  });

  it("watches every anchored part of the page, from a line a tenth down it", async () => {
    navOf(await reading(WAITING));

    expect(spy().observing).toEqual([
      "preface",
      "diff",
      "diff-1",
      "diff-2",
      "questions",
      "q1",
      "q2",
      "q3",
      "postscript",
    ]);
    expect(
      spy().margin,
      "what is under the reading line is what the reader has in front of them, " +
        "and a section long scrolled past still counts as started",
    ).toBe("100000px 0px -90% 0px");
  });

  it("goes back to the top when the reader scrolls back up", async () => {
    const nav = navOf(await reading(WAITING));

    spy().cross({ preface: true, diff: true, "diff-1": true });
    spy().cross({ diff: false, "diff-1": false });

    expect(highlighted(nav)).toBe("preface");
  });

  it("is let go of when the page is", async () => {
    await reading(WAITING);
    const following = spy();
    // The next page's mount takes the last one down.
    await reading(WAITING);

    expect(
      following.disconnected,
      "an observer still watching a page that has gone is a scroll away from " +
        "trouble",
    ).toBe(true);
  });
});

describe("a jump from the contents", () => {
  it("takes the reader to the file, unfolding it first", async () => {
    const page = await reading(WAITING);
    const fold = page.querySelector<HTMLDetailsElement>("#diff-2")!;
    fold.open = false;

    line(navOf(page), "diff-2").click();

    expect(
      fold.open,
      "landing on a closed fold is landing on nothing",
    ).toBe(true);
    expect(fold.scrollIntoView).toHaveBeenCalled();
  });

  it("leaves a hash to copy, and no step to come back through", async () => {
    const page = await reading(WAITING);
    const replacing = vi.spyOn(history, "replaceState");
    const pressing = vi.spyOn(history, "pushState");

    line(navOf(page), "q2").click();

    expect(replacing).toHaveBeenCalledWith(null, "", "#q2");
    expect(
      pressing,
      "moving around a page is not somewhere to come back to",
    ).not.toHaveBeenCalled();
  });

  it("holds the highlight where it landed until the reader scrolls by hand", async () => {
    const nav = navOf(await reading(WAITING));

    line(nav, "q3").click();
    expect(
      highlighted(nav),
      "the page cannot always bring a section to the top, so the highlight " +
        "goes where the reader asked to be",
    ).toBe("q3");

    // The spy answering for wherever the scroll ran out does not move it.
    spy().cross({ preface: true, diff: true });
    expect(highlighted(nav)).toBe("q3");

    // And the reader taking over puts it straight back on where the page is.
    window.dispatchEvent(new Event("wheel"));
    expect(highlighted(nav)).toBe("diff");
  });

  it("is left to the browser when the reader asked for a tab", async () => {
    const page = await reading(WAITING);
    const fold = page.querySelector<HTMLDetailsElement>("#diff-1")!;
    fold.open = false;

    // A modified click is the reader asking their browser for a tab or a
    // window, which is the browser's business and not ours.
    line(navOf(page), "diff-1").dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true, metaKey: true }),
    );

    expect(fold.open).toBe(false);
    expect(fold.scrollIntoView).not.toHaveBeenCalled();
  });
});

/// Whether the bar has its list down. The nav wears the shape it is in as well
/// — a Set is read in a pane, so it is always `paned` — and this is the one
/// class any of these tests is about.
function down(nav: HTMLElement): boolean {
  return nav.classList.contains(contents.open!);
}

describe("the bar", () => {
  it("names the line the nav has highlighted", async () => {
    const nav = navOf(await reading(WAITING));

    expect(barSays(nav)).toContain("Preface");
    expect(
      highlighted(nav),
      "which is the same line the sidebar marks — one scroll-spy answers for both",
    ).toBe("preface");

    spy().cross({ preface: true, diff: true, "diff-1": true });
    expect(
      barSays(nav),
      "and a file by the same cut path the sidebar shows it under",
    ).toBe("src/limits.rs");
  });

  it("names whatever section the Set starts with", async () => {
    const nav = navOf(await reading({ ...WAITING, preface_html: null }));

    expect(barSays(nav)).toContain("Diff");
    expect(barSays(nav)).not.toContain("Preface");
  });

  it("arrives shut, with the entries in the page all the same", async () => {
    const nav = navOf(await reading(WAITING));
    const bar = nav.querySelector(`button.${contents.bar}`)!;

    expect(bar.getAttribute("aria-expanded")).toBe("false");
    expect(bar.getAttribute("aria-controls")).toBe("contents-list");
    expect(down(nav), "nothing has opened it yet").toBe(false);
    expect(
      nav.querySelector("#contents-list"),
      "the same list the sidebar draws, so opening the bar has nothing to fetch",
    ).toBeTruthy();
  });

  it("brings the list down, and puts it away again", async () => {
    const nav = navOf(await reading(WAITING));
    const bar = nav.querySelector<HTMLButtonElement>(`button.${contents.bar}`)!;

    bar.click();
    expect(down(nav)).toBe(true);
    expect(bar.getAttribute("aria-expanded")).toBe("true");

    bar.click();
    expect(down(nav)).toBe(false);
  });

  it("puts the list away on Escape", async () => {
    const nav = navOf(await reading(WAITING));
    nav.querySelector<HTMLButtonElement>(`button.${contents.bar}`)!.click();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));

    expect(
      down(nav),
      "a list drawn over the page has to be dismissible from the keyboard",
    ).toBe(false);
  });

  it("puts the list away on a tap beside it, and presses nothing", async () => {
    const nav = navOf(await reading(WAITING));
    nav.querySelector<HTMLButtonElement>(`button.${contents.bar}`)!.click();

    const backdrop = nav.querySelector<HTMLElement>(`.${contents.backdrop}`);
    expect(
      backdrop,
      "the tap taking the list back must not also press something on the page " +
        "underneath",
    ).toBeTruthy();
    expect(backdrop!.getAttribute("aria-hidden")).toBe("true");

    backdrop!.click();
    expect(down(nav)).toBe(false);
    expect(nav.querySelector(`.${contents.backdrop}`)).toBeNull();
  });

  it("puts itself away when a line is pressed", async () => {
    const nav = navOf(await reading(WAITING));
    nav.querySelector<HTMLButtonElement>(`button.${contents.bar}`)!.click();

    line(nav, "q1").click();

    expect(down(nav), "the list has done what it was opened for").toBe(false);
  });
});
