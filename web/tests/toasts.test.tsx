//! What just happened, said once and then gone — see `src/Toasts.tsx`.
//!
//! The layer is part of the layout rather than part of anything that uses it, so
//! this asks about it on its own: what it draws, when it goes, and what stops it
//! going. Who raises one, and what they say, is the business of whoever presses
//! something — `workbench.test.tsx` is where the two that publish are asked.
//!
//! The clock is the subject of half of it, so the clock is fake: a test that
//! waited ten seconds for a toast to go would be ten seconds of nobody learning
//! anything.

import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Toasts, toast } from "../src/Toasts";
import styles from "../src/Toasts.module.css";

/// How long one stands once nothing is reading it — `LINGER` in `Toasts.tsx`,
/// said again here because a test that read it from the module could not tell a
/// clock that never started from one set to nothing.
const LINGER = 10_000;

beforeEach(() => vi.useFakeTimers());

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

/// The layer, with nothing on it.
function mount() {
  return render(() => <Toasts />);
}

/// Everything standing on it.
const standing = (): string[] =>
  [...document.querySelectorAll(`.${styles.toast}`)].map(
    (one) => one.textContent ?? "",
  );

describe("the toast layer", () => {
  it("says what it was given", () => {
    mount();
    toast(() => <>The share is published.</>);

    expect(standing()).toEqual(["The share is published.×"]);
  });

  /// A node rather than a sentence, because what a toast carries is often the
  /// way to put something right.
  it("carries what is inside it, links and all", () => {
    mount();
    toast(() => (
      <>
        No token. <a href="/settings/github">Put one in.</a>
      </>
    ));

    expect(
      screen.getByRole("link", { name: "Put one in." }).getAttribute("href"),
    ).toBe("/settings/github");
  });

  /// Two presses are two outcomes, even where they were refused the same way.
  it("stacks one per thing that happened", () => {
    mount();
    toast(() => <>GitHub would not take it.</>);
    toast(() => <>GitHub would not take it.</>);

    expect(standing()).toHaveLength(2);
  });

  it("goes on its own once nothing is reading it", () => {
    mount();
    toast(() => <>Commented on #41.</>);

    vi.advanceTimersByTime(LINGER - 1);
    expect(standing()).toHaveLength(1);

    vi.advanceTimersByTime(1);
    expect(standing()).toEqual([]);
  });

  /// The clock stops under the pointer, because a toast carrying a link that
  /// went while the pointer was on the way to it is worse than no link at all.
  it("waits while it is being read", () => {
    mount();
    toast(() => <>Commented on #41.</>);

    fireEvent.mouseEnter(document.querySelector(`.${styles.toast}`)!);
    vi.advanceTimersByTime(LINGER * 3);
    expect(standing()).toHaveLength(1);

    // And starts again from the beginning when the pointer goes: the time it
    // stood before the pointer arrived is not time anybody spent reading it.
    fireEvent.mouseLeave(document.querySelector(`.${styles.toast}`)!);
    vi.advanceTimersByTime(LINGER - 1);
    expect(standing()).toHaveLength(1);

    vi.advanceTimersByTime(1);
    expect(standing()).toEqual([]);
  });

  it("goes when it is dismissed", () => {
    mount();
    toast(() => <>Commented on #41.</>);

    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(standing()).toEqual([]);
  });

  /// And a layer that goes takes what it was holding with it: an outcome belongs
  /// to the page it was raised on.
  it("holds nothing over from a page that has gone", () => {
    mount();
    toast(() => <>Commented on #41.</>);
    cleanup();

    mount();
    expect(standing()).toEqual([]);
  });
});
