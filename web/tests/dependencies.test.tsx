//! The wizard's first step: the three screens it is now — what to install, the
//! run installing it, and what is left to do by hand — and the eight tabs of
//! instructions on the last of them.
//!
//! Two halves, and they are asked about in two ways. **What is drawn** is put in
//! front of the component as a reading — the golden fixtures `cargo test` writes
//! out of the real endpoint, so what the step is drawn over is what the server
//! actually said. **What the instructions say** is a table of this side's own,
//! the same eight answers on every Verkstead, so it is read as the table it is:
//! a command that is wrong is wrong on every machine, and nothing has to be
//! mounted to catch it.
//!
//! Some of the readings here are made rather than served: a machine whose
//! `bwrap` will not run, the two platforms this runner is not, and the run that
//! ended with one row it could not install. Each is a fixture with a field moved
//! — see [`stating`] and [`landed`] — because what the wizard has to draw for one
//! is a fact about the page, while what the server says about it is a Rust test
//! in `crates/server/src/onboarding.rs` and asked about there.

import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { createSignal } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  Dependency,
  DependencyState,
  Distro,
  InstallState,
  OnboardingView,
  RunView,
  Seen,
} from "../src/api/types";
import { Dependencies } from "../src/setup/Dependencies";
import { SetupPage } from "../src/setup/SetupPage";
import { DISTROS, GUIDES, instructionFor } from "../src/setup/instructions";
import { SETUP_SCREEN, SETUP_STEP } from "../src/setup/steps";
import { json, serving, whenever } from "./serving";
import failed from "./fixtures/onboarding-failed.json" with { type: "json" };
import fresh from "./fixtures/onboarding-fresh.json" with { type: "json" };
import installing from "./fixtures/onboarding-installing.json" with { type: "json" };
import partWay from "./fixtures/onboarding-part-way.json" with { type: "json" };

/// A bare Ubuntu machine: nothing installed at all, and so no step met.
const FRESH = fresh as OnboardingView;

/// And the same machine once what a session needs is on it: a sandbox, `git`
/// and one harness, with the other three and `gh` still absent.
const PART_WAY = partWay as OnboardingView;

/// A run going: three rows ticked, the password dialog still up, and nothing
/// behind it yet.
const INSTALLING = installing as OnboardingView;

/// And a run over with nothing to show for it: the dialog was dismissed, so
/// every row of that elevated unit failed in the machine's own words — with a
/// vendor's own installer under them that would not run, failed in its.
const FAILED = failed as OnboardingView;

/// The two paths this step talks to.
const INSTALL = "/api/ui/onboarding/install";
const CANCEL = "/api/ui/onboarding/install/cancel";

/// What a machine with unprivileged user namespaces switched off says when
/// `bwrap` is asked for a namespace — the line that names what to change, which
/// is why the row carries it rather than a sentence of the wizard's own.
const REFUSAL =
  "bwrap: No permissions to creating new namespace, likely because the kernel " +
  "does not allow non-privileged user namespaces";

/// And what `apt` says about a name no archive carries, which is what the run
/// puts on a row it could not install.
const UNPACKAGED = "E: Unable to locate package codex";

/// The same reading with one row's state replaced.
///
/// The states this step has to draw outnumber the readings the server writes
/// fixtures for, and the missing ones are a machine's rather than a page's: a
/// `bwrap` that would not run, and the sandbox row on the two platforms this
/// runner is not. Every arm of the server's own answer is asked about in Rust,
/// where a platform is a value; what is asked here is what the page draws when
/// one arrives.
function stating(
  reading: OnboardingView,
  dependency: Dependency,
  state: DependencyState,
): OnboardingView {
  return {
    ...reading,
    dependencies: reading.dependencies.map((row) =>
      row.dependency === dependency ? { ...row, state } : row,
    ),
  };
}

/// And with what the run has made of one row replaced, which is the other half
/// of what a row carries while an install is going.
function made(
  reading: OnboardingView,
  dependency: Dependency,
  install: InstallState,
): OnboardingView {
  return {
    ...reading,
    dependencies: reading.dependencies.map((row) =>
      row.dependency === dependency ? { ...row, install } : row,
    ),
  };
}

/// A row that is there, where which file it is, is not what the test is about
/// — the sandbox on the two platforms that have no program to find is exactly
/// this on the wire.
const THERE: DependencyState = { state: "Present", at: null, target: null };

/// One row as a run that installed it leaves it: on the machine, and with
/// nothing left on it for the run to say.
///
/// **Which is the server's rule rather than this helper's convenience**, and
/// the reason this is `Idle` even where the row is one the run *failed* and
/// somebody then installed by hand: a row the probe finds is a row the reading
/// carries nothing of the run on — see `installing` in
/// `crates/server/src/onboarding.rs`. A fixture that put a refusal beside a
/// present row would be this suite drawing a screen the server never sends.
function landed(
  reading: OnboardingView,
  dependency: Dependency,
): OnboardingView {
  return stating(
    made(reading, dependency, { install: "Idle" }),
    dependency,
    THERE,
  );
}

/// And one as a run that could not: still missing, with why on it.
function refused(
  reading: OnboardingView,
  dependency: Dependency,
  why: string,
): OnboardingView {
  return stating(
    made(reading, dependency, { install: "Failed", why }),
    dependency,
    { state: "Absent", trouble: null, seen: null },
  );
}

/// The same reading with the run on it replaced, or taken off.
function running(reading: OnboardingView, run: RunView | null): OnboardingView {
  return { ...reading, run };
}

/// A run that is over.
const OVER: RunView = {
  phase: "Done",
  status: "2 of 3 installed",
  done: 3,
  total: 3,
  cancelling: false,
};

/// The three rows [`INSTALLING`] ticked, all of them landed: what the reading
/// after a run that worked says.
const LANDED = running(
  landed(landed(landed(INSTALLING, "Sandbox"), "Git"), "Codex"),
  { ...OVER, status: "3 of 3 installed" },
);

/// And the same run with one row it could not install: the sandbox and `git`
/// landed, and this distribution has no package under the name Codex is.
const ONE_FAILED = running(
  refused(landed(landed(INSTALLING, "Sandbox"), "Git"), "Codex", UNPACKAGED),
  OVER,
);

/// A Mac, where the sandbox is Apple's own and there is nothing to install.
const A_MAC: OnboardingView = {
  ...stating(FRESH, "Sandbox", THERE),
  platform: "MacOs",
  distro: "MacOs",
};

/// And a Windows machine, where a session's boundary is an identity rather than
/// a program: the sandbox row is the local account Verkstead's sessions run as,
/// and this one has not got it.
const WINDOWS: OnboardingView = {
  ...stating(FRESH, "Sandbox", {
    state: "Absent",
    trouble: "there is no local account vk-0123456789ab for this Data Directory",
    seen: null,
  }),
  platform: "Windows",
  distro: "Windows",
};

/// The step alone, over a machine that stands as `machine` says — and with the
/// readings after it in the test's own hands, which is what the frame does for
/// it in the app.
///
/// A query client and no router: the presses this step makes are mutations, and
/// how often the reading is taken again is the frame's — see
/// `onboarding.test.tsx`.
function mount(machine: OnboardingView) {
  const onwards = vi.fn();
  const poll = vi.fn();
  const [reading, setReading] = createSignal(machine);
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  // The frame in miniature: what a press of this step's own answers with is
  // handed to the query rather than back to the step, and the reading the step
  // is drawn over is whatever the query last held. See `SetupPage.tsx`, which
  // is the same sentence in the app.
  client.getQueryCache().subscribe((event) => {
    if (event.query.queryKey[0] === "onboarding" && event.query.state.data) {
      setReading(event.query.state.data as OnboardingView);
    }
  });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <Dependencies reading={reading()} onwards={onwards} poll={poll} />
      </QueryClientProvider>
    )),
    onwards,
    poll,
    /// The machine read again, which is what the frame hands down every few
    /// seconds.
    read: setReading,
  };
}

/// A row that is absent because the name was seen somewhere a session cannot
/// use it — which is a `PATH` to fix rather than a program to install.
function elsewhere(seen: Seen): DependencyState {
  return { state: "Absent", trouble: null, seen };
}

/// One row of the step.
function row(container: ParentNode, dependency: Dependency): HTMLElement {
  return container.querySelector<HTMLElement>(
    `[data-dependency="${dependency}"]`,
  )!;
}

/// Which rows are drawn at all, in the order they were drawn.
function drawn(container: ParentNode): Array<string | undefined> {
  return [
    ...container.querySelectorAll<HTMLElement>("[data-dependency]"),
  ].map((row) => row.dataset.dependency);
}

/// The box on one row, where it carries one.
function box(
  container: ParentNode,
  dependency: Dependency,
): HTMLInputElement | null {
  return row(container, dependency).querySelector<HTMLInputElement>(
    'input[type="checkbox"]',
  );
}

/// Every tab, in the order they were drawn.
function tabs(container: ParentNode): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>("[data-distro]")];
}

/// And which one is showing.
function showing(container: ParentNode): string | undefined {
  return container.querySelector<HTMLElement>(
    '[data-distro][aria-pressed="true"]',
  )?.dataset.distro;
}

/// A press, by the words on it.
///
/// Found by its own words rather than by a role query, which is what every other
/// suite here does with a button that may not be there yet.
function press(container: ParentNode, words: string): HTMLButtonElement {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent === words,
  )!;
}

/// The press onwards, which reads the same on every screen of the wizard.
function onwards(container: ParentNode): HTMLButtonElement {
  return press(container, "Next");
}

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("the checkbox screen", () => {
  /// The two rows nothing can run without start ticked; which of the four
  /// harnesses somebody wants is theirs to say, and `gh` is a choice.
  it("ticks the sandbox and git, and leaves the harnesses and gh alone", () => {
    const { container } = mount(FRESH);

    expect(box(container, "Sandbox")!.checked).toBe(true);
    expect(box(container, "Git")!.checked).toBe(true);

    for (const row of ["Claude", "Codex", "Grok", "OpenCode", "Gh"] as const) {
      expect(box(container, row)!.checked).toBe(false);
    }

    // And nothing a session could start on is ticked yet: two of the three the
    // objective wants are there, and no harness is.
    expect(onwards(container).disabled).toBe(true);
  });

  it("releases Next once a harness is ticked, and holds it again when one is not", () => {
    const { container } = mount(FRESH);

    fireEvent.click(box(container, "Claude")!);
    expect(onwards(container).disabled).toBe(false);

    // And the gating rows are what they are called: taking git off is a machine
    // no session could start on again.
    fireEvent.click(box(container, "Git")!);
    expect(onwards(container).disabled).toBe(true);
  });

  /// The whole of what a row asks for is the box, so the name is what names it:
  /// a press anywhere on the row's head is a tick.
  it("labels the box with the row's own name", () => {
    const { container } = mount(FRESH);
    const label = row(container, "Git").querySelector("label")!;

    expect(label.textContent).toContain("git");
    expect(label.querySelector('input[type="checkbox"]')).toBe(
      box(container, "Git"),
    );
  });

  /// A row that is there keeps its mark: the box stands where the mark would
  /// be, and a row that has nothing to install has nothing to tick.
  it("keeps the mark on a row that is already there", () => {
    const { container } = mount(PART_WAY);
    const git = row(container, "Git");

    expect(git.dataset.state).toBe("Present");
    expect(git.querySelector('[aria-label="done"]')).not.toBeNull();
    expect(box(container, "Git")).toBeNull();

    // And one that is missing carries a box in the same place.
    expect(box(container, "Codex")!.checked).toBe(false);
  });

  /// And the sandbox row on Windows is a box like any other: what it is about
  /// there is the account a session runs as, which the wizard can make.
  it("carries a box on the Windows sandbox row", () => {
    const { container } = mount(WINDOWS);
    const sandbox = row(container, "Sandbox");

    expect(sandbox.dataset.state).toBe("Absent");
    expect(box(container, "Sandbox")!.checked).toBe(true);
    expect(sandbox.textContent).toContain("there is no local account");
  });

  /// Everything this screen used to carry about installing something by hand is
  /// the hint screen's now: what is asked for here is a tick.
  it("carries no tabs, no PATH list and no instruction", () => {
    const { container } = mount(FRESH);

    expect(tabs(container)).toHaveLength(0);
    expect(container.textContent).not.toContain("reads that PATH once");
    expect(container.textContent).not.toContain("/machine/bin");
    expect(container.textContent).not.toContain("apt install");
  });

  /// A `bwrap` that is installed and will not make a namespace says why in its
  /// own words, and the line naming the sysctl to set is the one worth reading.
  it("puts the failed run's own words under the sandbox row", () => {
    const { container } = mount(
      stating(FRESH, "Sandbox", {
        state: "Absent",
        trouble: REFUSAL,
        seen: null,
      }),
    );

    expect(row(container, "Sandbox").querySelector("pre")!.textContent).toBe(
      REFUSAL,
    );
  });
});

describe("the press that starts the run", () => {
  it("posts what is ticked and opens the install screen", async () => {
    const fetching = serving(
      whenever(INSTALL, json(INSTALLING), "POST"),
      json(FRESH),
    );
    const { container } = mount(FRESH);

    fireEvent.click(box(container, "Codex")!);
    fireEvent.click(onwards(container));

    await waitFor(() =>
      expect(container.querySelector("progress")).not.toBeNull(),
    );

    const [path, init] = fetching.mock.calls[0]!;
    expect(String(path)).toBe(INSTALL);
    expect(init?.method).toBe("POST");
    expect(JSON.parse(String(init?.body))).toEqual({
      dependencies: ["Sandbox", "Git", "Codex"],
    });
  });

  /// A machine with nothing to install is a machine to walk past: the step is
  /// met by what is already on it.
  it("goes straight on where nothing is ticked", () => {
    const fetching = serving(json(PART_WAY));
    const { container, onwards: pressed } = mount(PART_WAY);

    fireEvent.click(onwards(container));

    expect(pressed).toHaveBeenCalledTimes(1);
    expect(fetching).not.toHaveBeenCalled();
  });

  it("is refused while what a session needs would still be missing", () => {
    const { container, onwards: pressed } = mount(FRESH);

    fireEvent.click(onwards(container));
    expect(pressed).not.toHaveBeenCalled();
  });

  /// The server refuses a press the wizard should not have made — a second run
  /// while one is going, and a press after the wizard is over — and what it
  /// refuses in is the sentence to draw.
  it("says what the server refused, in the server's own words", async () => {
    serving(
      whenever(
        INSTALL,
        json({ error: "an install is already running" }, 409),
        "POST",
      ),
      json(FRESH),
    );
    const { container } = mount(FRESH);

    fireEvent.click(box(container, "Claude")!);
    fireEvent.click(onwards(container));

    await waitFor(() =>
      expect(container.textContent).toContain("an install is already running"),
    );
    // And the screen is where it was: nothing was started, so there is nothing
    // to watch.
    expect(container.querySelector("progress")).toBeNull();
  });
});

describe("the install screen", () => {
  /// A run that is going is the screen to be on whatever this device was left
  /// on: somebody pressed Next here a moment ago, or on the phone beside it.
  it("draws the bar, the count and the status line the server wrote", () => {
    const { container } = mount(INSTALLING);
    const bar = container.querySelector("progress")!;

    expect(bar.value).toBe(0);
    expect(bar.max).toBe(3);
    expect(container.textContent).toContain("0/3");
    expect(container.querySelector("[data-status]")!.textContent).toBe(
      "Waiting for the password dialog on ada-box",
    );
  });

  it("fills the bar as the run gets through it", () => {
    const { container, read } = mount(INSTALLING);

    read(running(INSTALLING, { ...OVER, phase: "Installing", done: 2 }));

    expect(container.querySelector("progress")!.value).toBe(2);
    expect(container.textContent).toContain("2/3");
  });

  it("posts the cancel, and reads cancelling until the unit under way is over", async () => {
    // The press cannot stop a package manager that is already unpacking, so
    // what it answers with is the run still going, with the cancel on it.
    const waiting = running(INSTALLING, {
      ...OVER,
      phase: "Installing",
      cancelling: true,
    });
    const fetching = serving(
      whenever(CANCEL, json(waiting), "POST"),
      json(INSTALLING),
    );
    const { container } = mount(INSTALLING);

    fireEvent.click(press(container, "Cancel"));

    await waitFor(() => expect(fetching).toHaveBeenCalled());
    expect(String(fetching.mock.calls[0]![0])).toBe(CANCEL);

    // And the button says what it is waiting for until the unit under way is
    // over.
    await waitFor(() =>
      expect(press(container, "Cancelling…")).not.toBeUndefined(),
    );
    expect(press(container, "Cancelling…").disabled).toBe(true);
  });

  /// Nobody is asked to press anything about a run they watched finish.
  it("moves the step on by itself when every ticked row landed", () => {
    const { container, read, onwards: pressed } = mount(INSTALLING);

    expect(pressed).not.toHaveBeenCalled();

    read(LANDED);

    expect(pressed).toHaveBeenCalledTimes(1);
    expect(container.querySelector("progress")).toBeNull();
  });

  /// And where one did not, the screen that says what to do about it — with
  /// that row alone on it, whatever else was ticked.
  it("opens the hint screen where a ticked row is still missing", () => {
    const { container, read, onwards: pressed } = mount(INSTALLING);

    read(ONE_FAILED);

    expect(pressed).not.toHaveBeenCalled();
    expect(drawn(container)).toEqual(["Codex"]);
    expect(row(container, "Codex").querySelector("[data-failed]")!.textContent).toBe(
      UNPACKAGED,
    );
    expect(container.querySelector("[data-detected]")!.textContent).toBe(
      "0/1 detected",
    );
    expect(onwards(container).disabled).toBe(true);
  });
});

describe("the hint screen", () => {
  /// The screen a reload lands back on, which is what the rows the run failed
  /// say: what was ticked is this device's and nowhere on the wire.
  function hinting(machine: OnboardingView) {
    localStorage.setItem(SETUP_SCREEN, "hints");

    return mount(machine);
  }

  it("draws the rows the run could not install, with the instruction under each", () => {
    const { container } = hinting(ONE_FAILED);

    expect(drawn(container)).toEqual(["Codex"]);
    expect(row(container, "Codex").textContent).toContain(
      "npm install -g @openai/codex",
    );
  });

  /// The password dialog dismissed: every row of that one elevated unit failed
  /// in whatever the platform refused it in, so the hint screen is all of them
  /// — and the vendor's own installer that ran after it is there beside them,
  /// in the line it printed rather than in the dialog's.
  it("draws every row a unit failed, with what each of them said", () => {
    const { container } = hinting(FAILED);

    expect(drawn(container)).toEqual(["Sandbox", "Git", "Claude"]);
    expect(
      row(container, "Sandbox").querySelector("[data-failed]")!.textContent,
    ).toBe("Error: (-128) User canceled.");
    expect(
      row(container, "Claude").querySelector("[data-failed]")!.textContent,
    ).toBe("The installer could not reach the network.");
    expect(container.querySelector("[data-detected]")!.textContent).toBe(
      "0/3 detected",
    );
  });

  it("opens the tab this machine says it is, and draws all eight", () => {
    const { container } = hinting(ONE_FAILED);

    expect(showing(container)).toBe("Ubuntu");
    expect(tabs(container).map((tab) => tab.dataset.distro)).toEqual([
      ...DISTROS,
    ]);
  });

  /// The detection is a guess off `/etc/os-release` — a derivative names its
  /// parent, and a machine nobody has heard of names nothing — so the other
  /// seven are a press away for the human who can see the machine.
  it("shows another machine's commands when its tab is pressed", () => {
    const { container } = hinting(ONE_FAILED);

    fireEvent.click(
      tabs(container).find((tab) => tab.dataset.distro === "Windows")!,
    );

    expect(showing(container)).toBe("Windows");
    expect(row(container, "Codex").textContent).toContain("npm install -g");
  });

  /// Every command on the Mac's tab is a `brew install`, so a Mac that reached
  /// this screen because Homebrew could not be installed needs the one line
  /// that gets it — drawn once above the rows rather than under each of the
  /// five that are one.
  it("draws the Mac's Homebrew above the rows, and no other tab's", () => {
    const { container } = hinting(ONE_FAILED);

    expect(container.querySelector("[data-before]")).toBeNull();

    fireEvent.click(
      tabs(container).find((tab) => tab.dataset.distro === "MacOs")!,
    );

    const before = container.querySelector("[data-before]")!;

    expect(before.textContent).toContain(
      "https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh",
    );
    expect(before.textContent).toContain("Every command below is Homebrew's");
  });

  /// A PATH read at startup is a PATH that does not have the directory this
  /// morning's install landed in, and the list itself is the server's own
  /// rather than a sentence about one: both are facts about this machine, so
  /// they are drawn once, above the rows, whichever tab is showing.
  it("lists the PATH a session is given, and says it was read once", () => {
    const { container } = hinting(ONE_FAILED);
    const said = () =>
      [...container.querySelectorAll("p")].filter((line) =>
        line.textContent!.includes("reads that PATH once"),
      );
    const path = () =>
      [...container.querySelectorAll("ol li code")].map(
        (entry) => entry.textContent,
      );

    expect(said()).toHaveLength(1);
    expect(said()[0]!.textContent).toContain("started again");
    expect(path()).toEqual(ONE_FAILED.path);

    // The tab is which machine the commands are for, which neither of those is
    // about: pressing another one leaves them where they were.
    for (const distro of DISTROS) {
      fireEvent.click(
        tabs(container).find((tab) => tab.dataset.distro === distro)!,
      );

      expect(said()).toHaveLength(1);
      expect(path()).toEqual(ONE_FAILED.path);
    }
  });

  /// The counter is the whole of what Next is waiting for, and it moves under
  /// somebody who is watching it: the page reads the machine again while they
  /// are in the other window installing.
  it("counts what has been detected, and releases Next when they all are", () => {
    const { container, read, onwards: pressed } = hinting(ONE_FAILED);

    expect(container.querySelector("[data-detected]")!.textContent).toBe(
      "0/1 detected",
    );
    expect(onwards(container).disabled).toBe(true);

    read(landed(ONE_FAILED, "Codex"));

    expect(container.querySelector("[data-detected]")!.textContent).toBe(
      "1/1 detected",
    );

    fireEvent.click(onwards(container));
    expect(pressed).toHaveBeenCalledTimes(1);
  });

  /// A row lands where it was rather than leaving the list: the count is what
  /// moves, and a list that emptied itself would be a screen with nothing on it
  /// saying nothing had happened.
  it("keeps a row that lands, and ticks it", () => {
    const { container, read } = hinting(ONE_FAILED);

    read(landed(ONE_FAILED, "Codex"));

    expect(drawn(container)).toEqual(["Codex"]);
    expect(row(container, "Codex").dataset.state).toBe("Present");
  });

  /// Which is how a row nobody wants after all is unticked: the ticks are on
  /// the screen before this one.
  it("goes back to the ticks", () => {
    const { container } = hinting(ONE_FAILED);

    fireEvent.click(press(container, "Back"));

    expect(drawn(container)).toEqual([
      "Sandbox",
      "Git",
      "Claude",
      "Codex",
      "Grok",
      "OpenCode",
      "Gh",
    ]);
    expect(box(container, "Codex")!.checked).toBe(true);
  });
});

describe("how often the step asks for the machine", () => {
  /// The bar and the status line are drawn from the reading, so the interval is
  /// how often that screen moves at all.
  it("asks for two seconds while a run is going", () => {
    const { poll } = mount(INSTALLING);

    expect(poll).toHaveBeenLastCalledWith(2_000);
  });

  /// And goes on asking on the hint screen, which is the one place the step
  /// stands met and there is still something to watch for.
  it("asks for the ordinary interval while a hinted row is missing, and lets go once it lands", () => {
    const { poll, read } = mount(INSTALLING);

    read(ONE_FAILED);
    expect(poll).toHaveBeenLastCalledWith(10_000);

    read(landed(ONE_FAILED, "Codex"));
    expect(poll).toHaveBeenLastCalledWith(null);
  });

  it("asks for nothing at all while somebody is ticking", () => {
    const { poll } = mount(FRESH);

    expect(poll).toHaveBeenLastCalledWith(null);
  });
});

describe("where each program was found", () => {
  /// Which `claude` a session got is the whole of what this feature is about: a
  /// distribution's, too old to connect, and the human's own under
  /// `~/.local/bin` are the same tick and two different programs.
  it("draws the resolved path under a row that is there", () => {
    const { container } = mount(PART_WAY);
    const git = row(container, "Git").querySelector("[data-where]")!;

    expect(git.querySelector("[data-at]")!.textContent).toBe("/machine/bin/git");
    expect(git.querySelector("[data-target]")).toBeNull();
  });

  /// And the file at the end of the link, which is what the vendor's own
  /// installer leaves: the name a session resolved, and the version it really
  /// runs.
  it("draws the link's target after it where the two differ", () => {
    const { container } = mount(PART_WAY);
    const claude = row(container, "Claude").querySelector("[data-where]")!;

    expect(claude.querySelector("[data-at]")!.textContent).toBe(
      "/machine/bin/claude",
    );
    expect(claude.querySelector("[data-target]")!.textContent).toBe(
      "/home/you/.local/share/claude/versions/0.0.0/claude",
    );
  });

  /// A row that named no file has nothing to draw: the sandbox on a Mac is
  /// Apple's own rather than a program anybody went looking for.
  it("draws nothing under a row that named no file", () => {
    const { container } = mount(A_MAC);

    expect(row(container, "Sandbox").querySelector("[data-where]")).toBeNull();
  });

  /// A program on the server's own `PATH` that no session's holds: the human
  /// has it, and what is wanted is the directory on the `PATH` Verkstead is
  /// started with rather than another install.
  it("says where a name was seen when a session cannot reach it", () => {
    const { container } = mount(
      stating(
        PART_WAY,
        "Codex",
        elsewhere({ seen: "Beyond", at: "/opt/foo/bin/codex" }),
      ),
    );
    const codex = row(container, "Codex");
    const note = codex.querySelector('[data-seen="Beyond"]')!;

    expect(codex.dataset.state).toBe("Absent");
    expect(note.textContent).toContain("/opt/foo/bin/codex");
    expect(note.textContent).toContain("not on the PATH a session gets");
  });

  /// A link into an install nothing binds, and one with nothing at the end of
  /// it: two different things to do about, so two different sentences.
  it("says where a link leads, and when it leads nowhere", () => {
    const { container } = mount(
      stating(
        stating(
          PART_WAY,
          "Codex",
          elsewhere({
            seen: "Leading",
            at: "/home/you/.local/bin/codex",
            target: "/opt/codex/codex",
          }),
        ),
        "Grok",
        elsewhere({ seen: "Dangling", at: "/home/you/.local/bin/grok" }),
      ),
    );

    const leading = row(container, "Codex").querySelector(
      '[data-seen="Leading"]',
    )!;

    expect(leading.textContent).toContain("/home/you/.local/bin/codex");
    expect(leading.textContent).toContain("/opt/codex/codex");
    expect(leading.textContent).toContain("a session cannot reach");

    const dangling = row(container, "Grok").querySelector(
      '[data-seen="Dangling"]',
    )!;

    expect(dangling.textContent).toContain("/home/you/.local/bin/grok");
    expect(dangling.textContent).toContain("nothing at the end of it");
  });

  /// And a name on no `PATH` at all was seen nowhere: nothing is said under it
  /// but what to install.
  it("says nothing under a name that was seen nowhere", () => {
    const { container } = mount(PART_WAY);
    const grok = row(container, "Grok");

    expect(grok.dataset.state).toBe("Absent");
    expect(grok.querySelector("[data-seen]")).toBeNull();
  });
});

describe("what each machine is told to run", () => {
  /// What each machine installs bubblewrap with — the five distributions whose
  /// package names are written down. *Other Linux* is not among them: what it
  /// gets is the generic list, because a command pasted from the wrong package
  /// manager is worse than a sentence.
  const BUBBLEWRAP: Partial<Record<Distro, string>> = {
    NixOs: "environment.systemPackages = [ pkgs.bubblewrap ];",
    Ubuntu: "sudo apt install bubblewrap",
    Fedora: "sudo dnf install bubblewrap",
    Debian: "sudo apt install bubblewrap",
    Arch: "sudo pacman -S bubblewrap",
  };

  /// And git, which every OS with a package manager has one for.
  const GIT: Partial<Record<Distro, string>> = {
    MacOs: "brew install git",
    Windows: "winget install --id Git.Git",
    NixOs: "environment.systemPackages = [ pkgs.git ];",
    Ubuntu: "sudo apt install git",
    Fedora: "sudo dnf install git",
    Debian: "sudo apt install git",
    Arch: "sudo pacman -S git",
  };

  it("names an exact command for bubblewrap on every distribution that has one", () => {
    for (const [distro, command] of Object.entries(BUBBLEWRAP)) {
      expect(GUIDES[distro as Distro].rows.Sandbox.command).toBe(command);
    }

    // And says what is needed in words where there is no command to give.
    expect(GUIDES.OtherLinux.rows.Sandbox.command).toBeUndefined();
    expect(GUIDES.OtherLinux.rows.Sandbox.note).toContain("bubblewrap");
  });

  it("names an exact command for git on every OS that has a package manager", () => {
    for (const [distro, command] of Object.entries(GIT)) {
      expect(GUIDES[distro as Distro].rows.Git.command).toBe(command);
    }

    expect(GUIDES.OtherLinux.rows.Git.note).toContain("git");
  });

  it("names an exact command for each harness the OS packages", () => {
    for (const distro of DISTROS) {
      for (const harness of ["Claude", "Codex", "OpenCode"] as const) {
        expect(GUIDES[distro].rows[harness].command).toBeTruthy();
      }
    }

    // The casks Homebrew keeps those two under, which a plain `brew install`
    // would not find.
    expect(GUIDES.MacOs.rows.Claude.command).toBe(
      "brew install --cask claude-code",
    );
    expect(GUIDES.MacOs.rows.Codex.command).toBe("brew install --cask codex");
    expect(GUIDES.MacOs.rows.OpenCode.command).toBe("brew install opencode");
  });

  /// The `grok-cli` in nixpkgs is somebody else's agent and the one on npm is a
  /// proxy around claude-code: either would install a different program under
  /// the name a session launches. So Grok Build is xAI's own page on all eight
  /// tabs and a command on none of them.
  it("gives Grok Build the vendor's page on every one of them", () => {
    for (const distro of DISTROS) {
      const grok = GUIDES[distro].rows.Grok;

      expect(grok.command).toBeUndefined();
      expect(grok.link).toBe("https://x.ai/cli");
    }
  });

  it("says where a binary has to land on the row it belongs to", () => {
    for (const distro of DISTROS) {
      // Where a session looks is the server's own list, drawn above the rows
      // from the wire — no tab says it. What a tab still says is where each
      // installer puts its binary, which on Windows is the same directory
      // under the name Windows gives it.
      expect(GUIDES[distro].rows.Claude.note).toContain(
        distro === "Windows" ? "%USERPROFILE%\\.local\\bin" : "~/.local/bin",
      );
    }
  });

  /// The whole of why this feature exists: a distribution's claude can be too
  /// old to connect — Ubuntu's under WSL was — so the install that stays
  /// current is what a row leads with, and the packaged one waits under it.
  it("leads Claude Code with the native installer on every tab but the Mac's", () => {
    for (const distro of DISTROS) {
      if (distro === "MacOs") continue;

      const claude = GUIDES[distro].rows.Claude;

      expect(claude.command).toBe(
        distro === "Windows"
          ? "irm https://claude.ai/install.ps1 | iex"
          : "curl -fsSL https://claude.ai/install.sh | bash",
      );

      // And says the one thing the command cannot: which PATH has to name
      // where it landed, and that Verkstead reads that PATH once.
      expect(claude.note).toContain("PATH of the shell Verkstead is started");
      expect(claude.note).toContain("started again");

      // With this machine's own package kept under it rather than instead of
      // it: a line to paste, with a note of its own.
      expect(claude.alternative?.command).toBeTruthy();
      expect(claude.alternative?.note).toBeTruthy();
    }
  });

  /// Homebrew's prefix is on the floor under every Mac session's PATH and
  /// ~/.local/bin is on none of it, because an app started from the Dock has
  /// launchd's PATH rather than a shell's.
  it("leads the Mac with Homebrew, and says what the Dock does", () => {
    const claude = GUIDES.MacOs.rows.Claude;

    expect(claude.command).toBe("brew install --cask claude-code");
    expect(claude.alternative).toBeUndefined();
    expect(claude.note).toContain("Dock");
    expect(claude.note).toContain("launchd");
  });

  /// The Mac is the one tab whose every command wants the same thing first,
  /// and Homebrew is a thing Verkstead installs rather than a thing it assumes:
  /// a Mac reaches this screen at all when that install is what failed.
  it("names Homebrew's own line on the Mac's tab and on no other", () => {
    expect(GUIDES.MacOs.before?.command).toBe(
      '/bin/bash -c "$(curl -fsSL ' +
        'https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',
    );
    expect(GUIDES.MacOs.before?.note).toContain("/opt/homebrew");

    for (const distro of DISTROS) {
      if (distro === "MacOs") continue;

      expect(GUIDES[distro].before).toBeUndefined();
    }
  });

  /// The sentence this step used to carry on all eight tabs, which said the
  /// vendor's installer put claude somewhere no session could look. A session
  /// looks wherever the server's own PATH names now, so nothing may say it.
  it("says on no tab that a session cannot look where the installer lands", () => {
    for (const distro of DISTROS) {
      for (const instruction of Object.values(GUIDES[distro].rows)) {
        for (const note of [instruction.note, instruction.alternative?.note]) {
          expect(note ?? "").not.toContain("not on the PATH a session gets");
          expect(note ?? "").not.toContain("which is not on the PATH");
        }
      }
    }
  });

  /// The Windows sandbox row is an account rather than an install, and the
  /// wizard makes one: what this tab carries is the same thing by hand, for
  /// whoever is here because it could not.
  it("names the elevated command the Windows sandbox row is met by", () => {
    expect(GUIDES.Windows.rows.Sandbox.command).toBe(
      "verkstead session-account create",
    );
    expect(GUIDES.Windows.rows.Sandbox.note).toContain(
      "Ticking this row makes it for you",
    );
    expect(GUIDES.Windows.rows.Sandbox.note).toContain(
      "Run as administrator",
    );
  });

  /// And it names the Data Directory this server keeps, which is the one line
  /// on any of the eight tabs that is about this machine rather than about an
  /// operating system.
  ///
  /// **Because the verb resolves the platform's default without it.** An
  /// account is named after the directory whose sessions run as it, so a bare
  /// line pasted on a server keeping one of its own makes an account of a
  /// different name, leaves the row absent and says nothing about why — and the
  /// row gates the step, so that is the wizard stuck. Which is a server started
  /// from a terminal or a unit file, and that is precisely the one that reaches
  /// this screen: it had no way to raise the dialog.
  it("names this server's Data Directory in the Windows sandbox line", () => {
    expect(
      instructionFor("Windows", "Sandbox", "C:\\ProgramData\\verkstead")
        .command,
    ).toBe(
      'verkstead session-account create --data-dir "C:\\ProgramData\\verkstead"',
    );

    // And the bare line where the reading names no directory, that being the
    // best there is to offer rather than a reason to draw nothing.
    expect(instructionFor("Windows", "Sandbox", null).command).toBe(
      "verkstead session-account create",
    );

    // Every other row on that tab, and the same row on every other tab, is the
    // written-down answer and nothing to do with this server.
    expect(instructionFor("Windows", "Git", "C:\\elsewhere")).toEqual(
      GUIDES.Windows.rows.Git,
    );
    expect(instructionFor("Ubuntu", "Sandbox", "/var/lib/verkstead")).toEqual(
      GUIDES.Ubuntu.rows.Sandbox,
    );
  });
});

/// The whole page for these, because what is being asked about is the reading
/// arriving again: the screens are the step's and the interval that moves them
/// is the frame's.
describe("the step over the page's own interval", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function page() {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });

    return render(() => (
      <QueryClientProvider client={client}>
        <SetupPage />
      </QueryClientProvider>
    ));
  }

  it("ticks a row installed in another window within ten seconds, and advances only when pressed", async () => {
    // The bare machine first, and every read after it a machine somebody has
    // just installed what was missing on.
    serving(json(FRESH), json(PART_WAY));

    const { container } = page();

    await waitFor(() => expect(row(container, "Sandbox")).toBeTruthy());
    expect(row(container, "Sandbox").dataset.state).toBe("Absent");
    expect(onwards(container).disabled).toBe(true);

    await vi.advanceTimersByTimeAsync(10_000);

    await waitFor(() =>
      expect(row(container, "Sandbox").dataset.state).toBe("Present"),
    );
    expect(onwards(container).disabled).toBe(false);

    // And nothing has moved: a step that advanced under somebody's hands is a
    // page that changed while they were reading it.
    expect(localStorage.getItem(SETUP_STEP)).toBeNull();
    expect(
      container
        .querySelector('[data-step="dependencies"]')!
        .getAttribute("aria-current"),
    ).toBe("step");

    fireEvent.click(onwards(container));

    await waitFor(() =>
      expect(localStorage.getItem(SETUP_STEP)).toBe("accounts"),
    );
  });

  /// The press, the run, and the step moving on when the run is over: what the
  /// human does is tick and press once.
  it("presses once, watches the run, and opens the next step when it lands", async () => {
    serving(
      whenever(INSTALL, json(INSTALLING), "POST"),
      json(FRESH),
      json(LANDED),
    );

    const { container } = page();

    await waitFor(() => expect(box(container, "Codex")).toBeTruthy());
    fireEvent.click(box(container, "Codex")!);
    fireEvent.click(onwards(container));

    // The bar at nothing done of the three that were ticked, and the server's
    // own line under it.
    await waitFor(() =>
      expect(container.querySelector("progress")).not.toBeNull(),
    );
    expect(container.textContent).toContain("0/3");
    expect(container.querySelector("[data-status]")!.textContent).toBe(
      "Waiting for the password dialog on ada-box",
    );

    // And the run lands two seconds later, with nobody pressing anything.
    await vi.advanceTimersByTimeAsync(2_000);

    await waitFor(() =>
      expect(localStorage.getItem(SETUP_STEP)).toBe("accounts"),
    );
  });
});
