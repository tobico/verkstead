//! A view held at the bottom of a record still being written, and the human
//! taking it back off the bottom.
//!
//! `src/scrolling.ts` is what the Transcript of a running session follows with,
//! and what the details pane is asked about in `workbench.test.tsx` — there
//! through the pane, and here on its own, because the pause and the resume are
//! about where a box is scrolled to and jsdom lays nothing out. So the box is
//! written down rather than measured: an element whose height and reach are
//! values, and whose scroll position is one that says so when it moves.
//!
//! Which is the whole of what a browser would have brought. Everything the
//! following decides — that content arriving is not a scroll away from the
//! bottom, and that its own scrolling is not the human's — it decides off those
//! numbers and the events, and both are here.

import { createRoot, createSignal } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import { followBottom } from "../src/scrolling";

/// A box that scrolls, with something drawn inside it for the following to be
/// anchored to.
///
/// The overflow is an inline style because that is the one place jsdom computes
/// one from: the walk up from the anchor asks the layout which box moves, and
/// the app's own answer is a stylesheet jsdom does not resolve.
function box(size: { tall: number; high: number }) {
  const element = document.createElement("div");
  element.style.overflowY = "auto";
  document.body.append(element);

  const inside = document.createElement("ol");
  element.append(inside);

  let at = 0;
  let tall = size.tall;

  Object.defineProperty(element, "clientHeight", { get: () => size.high });
  Object.defineProperty(element, "scrollHeight", { get: () => tall });
  Object.defineProperty(element, "scrollTop", {
    get: () => at,
    set: (to: number) => {
      at = to;
      // What a browser does after moving one: the box says where it is now,
      // and saying so is the only way anything watching finds out.
      element.dispatchEvent(new Event("scroll"));
    },
  });

  return {
    /// What the following is anchored to.
    inside,

    /// Where the box is scrolled to, and as far as it goes.
    get at() {
      return at;
    },
    get end() {
      return tall - size.high;
    },

    /// More record, which is the growth the following is about.
    grow(by: number) {
      tall += by;
    },

    /// And the human moving it themselves: the gesture, and then the scroll it
    /// caused. Both, because it is the pairing that says whose the scroll was.
    byHand(to: number) {
      window.dispatchEvent(new Event("wheel"));
      element.scrollTop = to;
    },
  };
}

/// One following, with a signal standing for the record growing under it.
function following(anchor: HTMLElement, live = true) {
  const [grew, setGrew] = createSignal(0);
  let count = 0;

  const stop = createRoot((dispose) => {
    followBottom(
      () => anchor,
      () => live,
      grew,
    );
    return dispose;
  });

  return { stop, grew: () => setGrew(++count) };
}

const stopping: Array<() => void> = [];

afterEach(() => {
  while (stopping.length > 0) stopping.pop()!();
  document.body.replaceChildren();
  vi.unstubAllGlobals();
  // The page's own height, where a test stood one in for the layout jsdom has
  // none of. Written on the instance, so it goes with the test.
  delete (document.documentElement as unknown as Record<string, unknown>)
    .scrollHeight;
});

/// Start one and have it torn down with the test, an effect left watching a
/// window outliving the box it was about.
function follows(anchor: HTMLElement, live = true) {
  const { stop, grew } = following(anchor, live);
  stopping.push(stop);
  return grew;
}

describe("following a record that is still being written", () => {
  /// What opening a running session's output is: the record is read to the end
  /// and the view is put there, because the line being said now is the last one.
  it("lands at the end of the record it is opened on", () => {
    const scrolling = box({ tall: 400, high: 100 });
    follows(scrolling.inside);

    expect(scrolling.at).toBe(300);
  });

  /// And stays there as the session talks. Content arriving under a view at the
  /// bottom is not the human scrolling away from it — nothing moved but the
  /// record's own length — so the view goes after the new end.
  it("goes on to each new end as the record grows", () => {
    const scrolling = box({ tall: 400, high: 100 });
    const grew = follows(scrolling.inside);

    scrolling.grow(200);
    grew();
    expect(scrolling.at).toBe(500);

    scrolling.grow(200);
    grew();
    expect(scrolling.at).toBe(700);
  });

  /// The human reading something further back: the following stops where they
  /// stopped it, and the record goes on arriving underneath without moving them.
  it("holds where the human scrolled to, whatever arrives beneath", () => {
    const scrolling = box({ tall: 400, high: 100 });
    const grew = follows(scrolling.inside);

    scrolling.byHand(0);

    scrolling.grow(200);
    grew();
    expect(scrolling.at).toBe(0);

    scrolling.grow(200);
    grew();
    expect(scrolling.at).toBe(0);
  });

  /// And putting the view back at the bottom is asking for the following again,
  /// which is the only way back to it: the view is where the human said it is,
  /// so their saying it is at the end is what resumes.
  it("follows again once the human comes back to the end", () => {
    const scrolling = box({ tall: 400, high: 100 });
    const grew = follows(scrolling.inside);

    scrolling.byHand(0);
    scrolling.grow(200);
    grew();

    scrolling.byHand(scrolling.end);

    scrolling.grow(200);
    grew();
    expect(scrolling.at).toBe(scrolling.end);
  });

  /// A record that has stopped growing is left where the reader arrives, which
  /// for every other document in the pane is the top.
  it("never moves a record nothing is being added to", () => {
    const scrolling = box({ tall: 400, high: 100 });
    const grew = follows(scrolling.inside, false);

    expect(scrolling.at).toBe(0);

    scrolling.grow(200);
    grew();
    expect(scrolling.at).toBe(0);
  });

  /// Which box moves is the layout's answer rather than the component's: below
  /// the width where the details pane scrolls on its own, the pane is the whole
  /// page and the page is what moves.
  it("moves the page itself where no box between it and the record scrolls", () => {
    const scrolled = vi.fn();
    vi.stubGlobal("scrollTo", scrolled);
    Object.defineProperty(document.documentElement, "scrollHeight", {
      configurable: true,
      get: () => 2000,
    });

    const anchor = document.createElement("ol");
    document.body.append(anchor);
    follows(anchor);

    expect(scrolled).toHaveBeenCalledWith(0, 2000 - window.innerHeight);
  });
});
