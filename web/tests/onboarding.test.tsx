//! The wizard's frame: the three steps, which of them stands met, which one is
//! open, and how often the page asks the machine again.
//!
//! What each step *contains* is its own file and its own tests. What is here is
//! the frame around them, and the two things about it that are not drawing: the
//! open step is this device's and never leaves it, and the read runs on an
//! interval for exactly as long as somebody is waiting for something to land.
//!
//! Fed from the golden fixtures `cargo test` writes out of the real endpoint —
//! a bare machine and one part way through — so what the frame is drawn over
//! here is what the server actually said.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type { OnboardingView, ProfileEntry } from "../src/api/types";
import { SetupPage } from "../src/setup/SetupPage";
import styles from "../src/setup/SetupPage.module.css";
import { SETUP_STEP, STEPS, TITLES } from "../src/setup/steps";
import { harnesses, listed } from "../src/setup/summaries";
import { askedFor, json, serving, whenever } from "./serving";
import fresh from "./fixtures/onboarding-fresh.json" with { type: "json" };
import installing from "./fixtures/onboarding-installing.json" with { type: "json" };
import partWay from "./fixtures/onboarding-part-way.json" with { type: "json" };
import ready from "./fixtures/onboarding-ready.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };

/// The one path this page talks to.
const ONBOARDING = "/api/ui/onboarding";

/// And the one it reads once the second step is met, for the line under it.
const PROFILES = "/api/ui/profiles";

/// A bare machine: nothing installed, no Profile, no author, and so no step
/// met at all.
const FRESH = fresh as OnboardingView;

/// And one part way through: what a session needs is on it, and the two steps
/// after that are still to do.
const PART_WAY = partWay as OnboardingView;

/// And one with an install of Verkstead's own going on it, which is the step
/// asking to be read faster than the frame would have read it.
const INSTALLING = installing as OnboardingView;

/// And one with every step met, whose rail has a line under the first two.
const READY = ready as OnboardingView;

/// The Profiles saved on it: three accounts, all of them Claude Code.
const PROFILE_LIST = profiles as ProfileEntry[];

/// The wizard alone, over a machine that stands as `machine` says.
///
/// No router: the page reads nothing off the URL, and what the URL has to be
/// for it to be reached at all is `setup-routes.test.tsx`'s subject.
function mount(machine: OnboardingView) {
  const fetching = serving(
    whenever(ONBOARDING, json(machine)),
    whenever(PROFILES, json(PROFILE_LIST)),
  );
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <SetupPage />
      </QueryClientProvider>
    )),
    fetching,
  };
}

/// The frame's rows, in the order it drew them.
function rows(container: ParentNode): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>("[data-step]")];
}

/// What the three steps are called, as the frame drew them.
function headings(container: ParentNode): Array<string | null> {
  return rows(container).map((row) => row.querySelector("h2")!.textContent);
}

/// The line under each step, or `null` where there is none.
function summaries(container: ParentNode): Array<string | null> {
  return rows(container).map(
    (row) => row.querySelector(`.${styles.summary}`)?.textContent ?? null,
  );
}

/// The paper's foot.
function foot(container: ParentNode): HTMLElement | null {
  return container.querySelector<HTMLElement>(`.${styles.foot}`);
}

/// And which step is the open one.
function opened(container: ParentNode): string | undefined {
  return rows(container).find((row) => row.classList.contains(styles.open!))
    ?.dataset.step;
}

/// The document going away and coming back, which is the phone put down and
/// picked up: `visibilityState` is read-only here as it is in a browser, so it
/// is redefined for as long as the test needs it.
function showing(state: "visible" | "hidden"): void {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => state,
  });
  document.dispatchEvent(new Event("visibilitychange"));
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  localStorage.clear();
  // Back to the state jsdom's own document reports.
  delete (document as { visibilityState?: unknown }).visibilityState;
});

describe("the wizard's frame", () => {
  it("draws the three steps, in the order a machine is set up in", async () => {
    const { container } = mount(FRESH);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    expect(rows(container).map((row) => row.dataset.step)).toEqual([...STEPS]);
    // The headings rather than the rows: the open step has its own step drawn
    // under its heading, and what the frame is about is the three names.
    expect(headings(container)).toEqual(STEPS.map((step) => TITLES[step]));
  });

  it("marks the steps that stand met and the ones that do not", async () => {
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    expect(rows(container).map((row) => row.dataset.met)).toEqual([
      "yes",
      "no",
      "no",
    ]);
  });

  /// A step nobody has met yet is not one to press into: that would be the skip
  /// ADR-0016 refused, drawn as navigation.
  it("offers a way back into a met step and none into an unmet one", async () => {
    localStorage.setItem(SETUP_STEP, "accounts");
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    // The heading's own press rather than any press in the row: the open step
    // has its own controls under its heading, and what is being asked about is
    // the way *back* into a step.
    const pressable = rows(container).filter((row) =>
      row.querySelector("h2 button"),
    );
    expect(pressable.map((row) => row.dataset.step)).toEqual(["dependencies"]);
  });
});

describe("the rail", () => {
  /// The brand is the page's heading, and the steps are under it.
  it("is headed by the brand", async () => {
    const { container } = mount(FRESH);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    expect(container.querySelector("h1")!.textContent).toBe("Verkstead");
  });

  it("names the machine it is reading, and how often", async () => {
    const { container } = mount(FRESH);

    await waitFor(() =>
      expect(container.textContent).toContain("Reading ada-box every 10 s"),
    );
  });

  it("says it reads faster while a run is installing", async () => {
    const { container } = mount(INSTALLING);

    await waitFor(() =>
      expect(container.textContent).toContain(
        "Reading ada-box every 2 s while it installs",
      ),
    );
  });

  /// What meeting a step came to: the rows the machine has, and the harnesses
  /// the Profiles saved run.
  it("says what a met step came to under it", async () => {
    const { container } = mount(READY);

    await waitFor(() => expect(summaries(container)[1]).toBe("Claude Code"));
    expect(summaries(container)).toEqual([
      "A sandbox, git, and Claude Code",
      "Claude Code",
      null,
    ]);
  });

  it("says nothing under a step that is not met", async () => {
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    expect(summaries(container)).toEqual([
      "A sandbox, git, and Claude Code",
      null,
      null,
    ]);
  });
});

describe("the phone's Steps sheet", () => {
  /// The header's button, which the stylesheet draws only below 60rem.
  function steps(container: ParentNode): HTMLButtonElement {
    return [...container.querySelectorAll<HTMLButtonElement>("button")].find(
      (button) => button.textContent === "Steps",
    )!;
  }

  /// The sheet, where it is up.
  function sheet(container: ParentNode): HTMLDialogElement | null {
    return container.querySelector<HTMLDialogElement>("dialog[open]");
  }

  /// Which rows on the sheet carry an Open.
  function openable(dialog: ParentNode): Array<string | undefined> {
    return rows(dialog)
      .filter((row) =>
        [...row.querySelectorAll("button")].some(
          (button) => button.textContent === "Open",
        ),
      )
      .map((row) => row.dataset.step);
  }

  it("is nowhere until Steps is pressed", async () => {
    const { container } = mount(FRESH);

    await waitFor(() => expect(steps(container)).toBeTruthy());
    expect(container.querySelector("dialog")).toBeNull();
  });

  it("lists the three steps as the rail does", async () => {
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(steps(container)).toBeTruthy());
    fireEvent.click(steps(container));

    const dialog = sheet(container)!;
    expect(dialog).toBeTruthy();
    expect(headings(dialog)).toEqual(STEPS.map((step) => TITLES[step]));
    expect(rows(dialog).map((row) => row.dataset.met)).toEqual([
      "yes",
      "no",
      "no",
    ]);
    expect(summaries(dialog)[0]).toBe("A sandbox, git, and Claude Code");
    // Its own Open is the way into a step, so no heading there is a press.
    expect(dialog.querySelector("h2 button")).toBeNull();
  });

  /// Open on a met step that is not the open one, and nowhere else: not on the
  /// step already open, and not on one nobody has met, which would be the skip.
  it("offers Open only on a met step that is not open", async () => {
    localStorage.setItem(SETUP_STEP, "accounts");
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(steps(container)).toBeTruthy());
    fireEvent.click(steps(container));

    expect(openable(sheet(container)!)).toEqual(["dependencies"]);
  });

  it("draws no Open on the step that is open, met or not", async () => {
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(steps(container)).toBeTruthy());
    fireEvent.click(steps(container));

    expect(openable(sheet(container)!)).toEqual([]);
  });

  it("opens the step Open is on, and goes", async () => {
    localStorage.setItem(SETUP_STEP, "accounts");
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(steps(container)).toBeTruthy());
    fireEvent.click(steps(container));
    const open = [...sheet(container)!.querySelectorAll("button")].find(
      (button) => button.textContent === "Open",
    )!;
    fireEvent.click(open);

    expect(sheet(container)).toBeNull();
    expect(opened(container)).toBe("dependencies");
    expect(container.querySelector(`.${styles.title}`)!.textContent).toBe(
      TITLES.dependencies,
    );
  });

  it("goes on Close, on Escape, and on a press on the veil", async () => {
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(steps(container)).toBeTruthy());

    fireEvent.click(steps(container));
    const close = [...sheet(container)!.querySelectorAll("button")].find(
      (button) => button.textContent === "Close",
    )!;
    fireEvent.click(close);
    expect(sheet(container)).toBeNull();

    fireEvent.click(steps(container));
    fireEvent.keyDown(document, { key: "Escape" });
    expect(sheet(container)).toBeNull();

    // A press on the backdrop is reported as a press on the dialog itself.
    fireEvent.click(steps(container));
    fireEvent.click(sheet(container)!);
    expect(sheet(container)).toBeNull();
    // And the step that was open stays open.
    expect(opened(container)).toBe("dependencies");
  });
});

/// The phone's header strip, which names the open step beside its dots.
describe("the header's strip", () => {
  it("fills the open step's dot and names it", async () => {
    localStorage.setItem(SETUP_STEP, "accounts");
    const { container } = mount(PART_WAY);

    await waitFor(() =>
      expect(container.querySelector(`.${styles.strip}`)).toBeTruthy(),
    );
    const strip = container.querySelector(`.${styles.strip}`)!;
    expect(
      [...strip.querySelectorAll("li")].map((dot) =>
        dot.classList.contains(styles.here!),
      ),
    ).toEqual([false, true, false]);
    expect(strip.querySelector(`.${styles.where}`)!.textContent).toBe(
      TITLES.accounts,
    );
    // Not a fourth list of rows for anything to find the frame's steps by.
    expect(strip.querySelector("[data-step]")).toBeNull();
  });
});

describe("the lines under a met step", () => {
  it("joins two names with and, and more with the serial comma", () => {
    expect(listed(["git"])).toBe("git");
    expect(listed(["Claude Code", "Codex"])).toBe("Claude Code and Codex");
    expect(listed(["A sandbox", "git", "Claude Code"])).toBe(
      "A sandbox, git, and Claude Code",
    );
  });

  /// Each harness once, however many accounts run under it.
  it("names each harness once", () => {
    const codex = {
      ...PROFILE_LIST[0]!,
      id: 9,
      account: { agent_type: "Codex", home: "/srv/accounts/codex" },
    } as ProfileEntry;

    expect(harnesses([codex, ...PROFILE_LIST])).toBe("Claude Code and Codex");
  });
});

describe("the foot", () => {
  /// The step's press stands in the frame's foot rather than under its rows.
  it("holds the open step's press", async () => {
    const { container } = mount(FRESH);

    await waitFor(() => expect(foot(container)).toBeTruthy());
    expect(
      [...foot(container)!.querySelectorAll("button")].map(
        (button) => button.textContent,
      ),
    ).toEqual(["Install 2", "Install by hand instead"]);
  });

  /// Back opens the step before, and there is none before the first.
  it("goes back to the step before", async () => {
    localStorage.setItem(SETUP_STEP, "accounts");
    const { container } = mount(PART_WAY);

    await waitFor(() => expect(foot(container)).toBeTruthy());
    const back = [...foot(container)!.querySelectorAll("button")].find(
      (button) => button.textContent === "Back",
    )!;
    fireEvent.click(back);

    await waitFor(() => expect(opened(container)).toBe("dependencies"));
    expect(
      [...foot(container)!.querySelectorAll("button")].map(
        (button) => button.textContent,
      ),
    ).toEqual(["Next", "Install by hand instead"]);
  });

  it("titles the paper with the open step", async () => {
    localStorage.setItem(SETUP_STEP, "accounts");
    const { container } = mount(PART_WAY);

    await waitFor(() =>
      expect(container.textContent).toContain("Step 2 of 3"),
    );
    expect(container.querySelector(`.${styles.title}`)!.textContent).toBe(
      TITLES.accounts,
    );
  });

  /// The first step's run and its by-hand screen are titled for what they are
  /// doing, and the step's own title is back once the step is left.
  it("titles the paper for the install screen while a run goes", async () => {
    const { container } = mount(INSTALLING);
    const title = () =>
      container.querySelector(`.${styles.title}`)?.textContent;

    await waitFor(() => expect(title()).toBe("Installing"));
    expect(container.textContent).toContain("Step 1 of 3");
  });
});

describe("the step this device is on", () => {
  it("opens the first one where the device has never had one", async () => {
    const { container } = mount(FRESH);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    expect(opened(container)).toBe("dependencies");
  });

  /// Which is the whole point of keeping it here: a reload is a fresh mount of
  /// this page, and it opens where the human left it rather than at the start.
  it("opens the one it was left on", async () => {
    localStorage.setItem(SETUP_STEP, "git");
    const { container } = mount(FRESH);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    expect(opened(container)).toBe("git");
  });

  /// A step this build does not have — a browser left on an older one — is the
  /// first step rather than a page with nothing open on it.
  it("opens the first one where what was left is a step it does not have", async () => {
    localStorage.setItem(SETUP_STEP, "harnesses");
    const { container } = mount(FRESH);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    expect(opened(container)).toBe("dependencies");
  });

  it("writes the step it opens down, and tells the server nothing about it", async () => {
    localStorage.setItem(SETUP_STEP, "accounts");
    const { container, fetching } = mount(PART_WAY);

    await waitFor(() => expect(rows(container)).toHaveLength(3));
    fireEvent.click(container.querySelector("h2 button")!);

    await waitFor(() => expect(opened(container)).toBe("dependencies"));
    expect(localStorage.getItem(SETUP_STEP)).toBe("dependencies");

    // And the whole of what this page said to the server is that it would like
    // to know how the machine stands: which step somebody is on is a fact about
    // the tab in front of them, and there is nothing on this wire for it.
    for (const [path, init] of fetching.mock.calls) {
      expect(path).toBe(ONBOARDING);
      expect(init?.method ?? "GET").toBe("GET");
      expect(init?.body).toBeUndefined();
    }
  });
});

describe("how often the machine is read again", () => {
  /// Somebody is at a terminal waiting for an install to land, and what tells
  /// them it did is a row on this page.
  it("reads again every ten seconds while the open step is unmet", async () => {
    const { fetching } = mount(FRESH);
    await waitFor(() => expect(askedFor(fetching, ONBOARDING)).toBe(1));

    await vi.advanceTimersByTimeAsync(10_000);
    expect(askedFor(fetching, ONBOARDING)).toBe(2);

    await vi.advanceTimersByTimeAsync(10_000);
    expect(askedFor(fetching, ONBOARDING)).toBe(3);
  });

  /// And stops the moment it is met: there is nothing left to watch for, and a
  /// page that went on running a `bwrap` every ten seconds would be doing it
  /// for nobody.
  it("stops once the open step is met", async () => {
    const { fetching } = mount(PART_WAY);
    await waitFor(() => expect(askedFor(fetching, ONBOARDING)).toBe(1));

    await vi.advanceTimersByTimeAsync(60_000);
    expect(askedFor(fetching, ONBOARDING)).toBe(1);
  });

  /// And faster where the open step asks for it: the dependencies step is
  /// drawing a run of Verkstead's own, and how often that screen moves at all is
  /// how often the machine is read.
  it("reads at the cadence the open step asks for", async () => {
    const { fetching } = mount(INSTALLING);
    await waitFor(() => expect(askedFor(fetching, ONBOARDING)).toBe(1));

    await vi.advanceTimersByTimeAsync(2_000);
    expect(askedFor(fetching, ONBOARDING)).toBe(2);

    await vi.advanceTimersByTimeAsync(2_000);
    expect(askedFor(fetching, ONBOARDING)).toBe(3);
  });

  /// What the stopped interval lets go is covered by the re-read the app makes
  /// on coming back — the phone put down and picked up. Driven through `App`,
  /// because that is where the setting is: a mount that built its own query
  /// client would only be asking about the client it built.
  it("reads again when the app is brought back, whatever the interval is doing", async () => {
    window.history.pushState({}, "", "/setup");
    const fetching = serving(whenever(ONBOARDING, json(PART_WAY)));

    render(() => <App />);
    await waitFor(() => screen.getByText("Setup"));

    const before = askedFor(fetching, ONBOARDING);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(askedFor(fetching, ONBOARDING)).toBe(before);

    showing("hidden");
    showing("visible");

    await waitFor(() =>
      expect(askedFor(fetching, ONBOARDING)).toBe(before + 1),
    );
  });
});
