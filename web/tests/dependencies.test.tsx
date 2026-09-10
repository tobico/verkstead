//! The wizard's first step: the rows this machine is missing, the eight tabs of
//! instructions over them, and the press that is refused until a session could
//! start at all.
//!
//! Two halves, and they are asked about in two ways. **What is drawn** is put in
//! front of the component as a reading — the golden fixtures `cargo test` writes
//! out of the real endpoint, so what the step is drawn over is what the server
//! actually said. **What the instructions say** is a table of this side's own,
//! the same eight answers on every Verkstead, so it is read as the table it is:
//! a command that is wrong is wrong on every machine, and nothing has to be
//! mounted to catch it.
//!
//! Two of the readings here are made rather than served: a machine whose `bwrap`
//! will not run, and the two platforms this runner is not. Each is a fixture with
//! one field moved — see [`stating`] — because what the wizard has to draw for a
//! Mac is a fact about the page, while what the server says about a Mac is a
//! Rust unit test in `crates/server/src/onboarding.rs` and asked about there.

import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  Dependency,
  DependencyState,
  Distro,
  OnboardingView,
  Seen,
} from "../src/api/types";
import { Dependencies } from "../src/setup/Dependencies";
import { SetupPage } from "../src/setup/SetupPage";
import { DISTROS, GUIDES } from "../src/setup/instructions";
import { SETUP_STEP } from "../src/setup/steps";
import { json, serving } from "./serving";
import fresh from "./fixtures/onboarding-fresh.json" with { type: "json" };
import partWay from "./fixtures/onboarding-part-way.json" with { type: "json" };

/// A bare Ubuntu machine: nothing installed at all, and so no step met.
const FRESH = fresh as OnboardingView;

/// And the same machine once what a session needs is on it: a sandbox, `git`
/// and one harness, with the other three and `gh` still absent.
const PART_WAY = partWay as OnboardingView;

/// What a machine with unprivileged user namespaces switched off says when
/// `bwrap` is asked for a namespace — the line that names what to change, which
/// is why the row carries it rather than a sentence of the wizard's own.
const REFUSAL =
  "bwrap: No permissions to creating new namespace, likely because the kernel " +
  "does not allow non-privileged user namespaces";

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

/// A row that is there, where which file it is, is not what the test is about
/// — the sandbox on the two platforms that have no program to find is exactly
/// this on the wire.
const THERE: DependencyState = { state: "Present", at: null, target: null };

/// A Mac, where the sandbox is Apple's own and there is nothing to install.
const A_MAC: OnboardingView = {
  ...stating(FRESH, "Sandbox", THERE),
  platform: "MacOs",
  distro: "MacOs",
};

/// And a Windows machine, where a session's boundary is an identity rather than
/// a program.
const WINDOWS: OnboardingView = {
  ...stating(FRESH, "Sandbox", { state: "NotApplicable" }),
  platform: "Windows",
  distro: "Windows",
};

/// The step alone, over a machine that stands as `machine` says.
///
/// No query client and no router: the step is handed the reading the page
/// already has, and how often that reading is taken again is the frame's own —
/// see `onboarding.test.tsx`.
function mount(machine: OnboardingView) {
  const onwards = vi.fn();

  return {
    ...render(() => <Dependencies reading={machine} onwards={onwards} />),
    onwards,
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

/// The press onwards, whatever it is drawn as.
///
/// Found by its own words rather than by a role query, which is what every other
/// suite here does with a button that may not be there yet.
function onwards(container: ParentNode): HTMLButtonElement {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent === "Continue",
  )!;
}

describe("the eight tabs of instructions", () => {
  it("opens the one this machine says it is, and draws all eight", () => {
    const { container } = mount(FRESH);

    expect(showing(container)).toBe("Ubuntu");
    expect(tabs(container).map((tab) => tab.dataset.distro)).toEqual([
      ...DISTROS,
    ]);
  });

  /// The detection is a guess off `/etc/os-release` — a derivative names its
  /// parent, and a machine nobody has heard of names nothing — so the other
  /// seven are a press away for the human who can see the machine.
  it("shows another machine's commands when its tab is pressed", () => {
    const { container } = mount(FRESH);

    expect(row(container, "Git").textContent).toContain("sudo apt install git");

    fireEvent.click(
      tabs(container).find((tab) => tab.dataset.distro === "Windows")!,
    );

    expect(showing(container)).toBe("Windows");
    expect(row(container, "Git").textContent).toContain(
      "winget install --id Git.Git",
    );
  });
});

describe("the rows", () => {
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

  it("ticks the sandbox on a Mac, where it is Apple's own", () => {
    const { container } = mount(A_MAC);
    const sandbox = row(container, "Sandbox");

    expect(sandbox.dataset.state).toBe("Present");
    expect(sandbox.querySelector('[aria-label="done"]')).not.toBeNull();
    // Nothing under a row that is already there: there is nothing to do about
    // it, so there is no instruction to draw.
    expect(sandbox.textContent).not.toContain("brew install");
  });

  /// Not applicable is not the same as nothing to do, which is the whole of
  /// what this row has to get across on this platform: the boundary there is an
  /// account rather than a program, so there is nothing to install and there is
  /// still one elevated command to run before a session will start at all.
  it("reads not applicable on Windows, and still names the elevated command", () => {
    const { container } = mount(WINDOWS);
    const sandbox = row(container, "Sandbox");

    expect(sandbox.dataset.state).toBe("NotApplicable");
    expect(sandbox.textContent).toContain("Not applicable");
    expect(sandbox.textContent).toContain("there is nothing to install");
    expect(sandbox.textContent).toContain("verkstead session-account create");
  });

  /// GitHub is a choice rather than a dependency, so its row is drawn like any
  /// other and gates nothing: this machine is missing it and Continue is
  /// pressable all the same.
  it("draws gh with its instruction and holds nothing up", () => {
    const { container } = mount(PART_WAY);
    const gh = row(container, "Gh");

    expect(gh.dataset.state).toBe("Absent");
    expect(gh.textContent).toContain("sudo apt install gh");
    expect(onwards(container).disabled).toBe(false);

    // And a machine that has one draws the row ticked, with nothing left to
    // say about it.
    const has = mount(stating(PART_WAY, "Gh", THERE));
    const ticked = row(has.container, "Gh");

    expect(ticked.querySelector('[aria-label="done"]')).not.toBeNull();
    expect(ticked.textContent).not.toContain("apt install");
  });

  /// Three harnesses left unticked hold nothing up either: a session runs under
  /// one Profile, and a Profile is of one agent type.
  it("draws the three harnesses this machine has not got and holds nothing up", () => {
    const { container } = mount(PART_WAY);

    expect(row(container, "Claude").dataset.state).toBe("Present");
    for (const harness of ["Codex", "Grok", "OpenCode"] as const) {
      expect(row(container, harness).dataset.state).toBe("Absent");
    }

    expect(onwards(container).disabled).toBe(false);
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

  /// The sentence this step used to carry on all eight tabs, which said the
  /// vendor's installer put claude somewhere no session could look. A session
  /// looks wherever the server's own PATH names now, so nothing may say it.
  it("says on no tab that a session cannot look where the installer lands", () => {
    for (const distro of DISTROS) {
      for (const instruction of Object.values(GUIDES[distro].rows)) {
        for (const note of [
          instruction.note,
          instruction.alternative?.note,
        ]) {
          expect(note ?? "").not.toContain("not on the PATH a session gets");
          expect(note ?? "").not.toContain("which is not on the PATH");
        }
      }
    }
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

    expect(
      row(container, "Sandbox").querySelector("[data-where]"),
    ).toBeNull();
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

    // And the instruction is still under it: a program somewhere a session
    // cannot open is a row that has not been met.
    expect(codex.textContent).toContain("npm install -g @openai/codex");
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

describe("where a session looks", () => {
  /// A PATH read at startup is a PATH that does not have the directory this
  /// morning's install landed in — which was Windows' own note when a session's
  /// PATH was a fixed list everywhere else, and holds on all three platforms
  /// now. So it is said once, above the rows, whichever tab is showing.
  it("says the PATH was read once at startup, once and on every tab", () => {
    const { container } = mount(PART_WAY);
    const said = () =>
      [...container.querySelectorAll("p")].filter((line) =>
        line.textContent!.includes("reads that PATH once"),
      );

    expect(said()).toHaveLength(1);
    expect(said()[0]!.textContent).toContain("started again");

    for (const distro of DISTROS) {
      fireEvent.click(
        tabs(container).find((tab) => tab.dataset.distro === distro)!,
      );

      expect(said()).toHaveLength(1);
    }
  });


  /// The list the server composed rather than a sentence about the fixed one a
  /// session's `PATH` used to be: it is a fact about this machine, so no tab
  /// could say it.
  it("lists the PATH a session is given, in order, on every tab", () => {
    const { container } = mount(PART_WAY);
    const drawn = () => [
      ...container.querySelectorAll("ol li code"),
    ].map((entry) => entry.textContent);

    expect(drawn()).toEqual(PART_WAY.path);

    // And the tab is which machine the commands are for, which the list is not
    // about: pressing another one leaves it where it was.
    fireEvent.click(
      tabs(container).find((tab) => tab.dataset.distro === "Windows")!,
    );

    expect(drawn()).toEqual(PART_WAY.path);
  });
});

describe("the second way to get a program", () => {
  /// Claude Code's row leads with the vendor's installer and keeps this
  /// machine's package under it: two commands, each with the press that copies
  /// it, because either is a line somebody is about to paste.
  it("draws the packaged install under the one the row leads with", () => {
    const { container } = mount(
      stating(PART_WAY, "Claude", { state: "Absent", trouble: null, seen: null }),
    );
    const claude = row(container, "Claude");
    const commands = [...claude.querySelectorAll("pre")].map(
      (line) => line.textContent,
    );

    expect(commands).toEqual([
      "curl -fsSL https://claude.ai/install.sh | bash",
      "sudo npm install -g @anthropic-ai/claude-code",
    ]);
    expect(claude.textContent).toContain("own package manager");
  });

  /// And a row with one instruction draws one: the Mac's Claude row is
  /// Homebrew and nothing under it.
  it("draws nothing under a row that has only the one", () => {
    const { container } = mount(
      stating(A_MAC, "Codex", { state: "Absent", trouble: null, seen: null }),
    );

    expect(
      row(container, "Codex").querySelectorAll("pre"),
    ).toHaveLength(1);
  });
});

describe("the press onwards", () => {
  it("is refused while what a session needs is missing", () => {
    const { container, onwards: pressed } = mount(FRESH);

    const button = onwards(container);
    expect(button.disabled).toBe(true);

    fireEvent.click(button);
    expect(pressed).not.toHaveBeenCalled();
  });

  it("opens the step after this one when it is pressed", () => {
    const { container, onwards: pressed } = mount(PART_WAY);

    fireEvent.click(onwards(container));
    expect(pressed).toHaveBeenCalledTimes(1);
  });
});

/// The whole page for this one, because what is being asked about is the install
/// landing in another window: the row that ticks is drawn by the step and the
/// re-read that ticks it is the frame's.
describe("an install that lands while the page is open", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    localStorage.clear();
  });

  it("ticks the row within ten seconds and releases Continue, and advances only when pressed", async () => {
    // The bare machine first, and every read after it a machine somebody has
    // just installed what was missing on.
    serving(json(FRESH), json(PART_WAY));

    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    const { container } = render(() => (
      <QueryClientProvider client={client}>
        <SetupPage />
      </QueryClientProvider>
    ));

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
      container.querySelector('[data-step="dependencies"]')!.getAttribute(
        "aria-current",
      ),
    ).toBe("step");

    fireEvent.click(onwards(container));

    await waitFor(() =>
      expect(localStorage.getItem(SETUP_STEP)).toBe("accounts"),
    );
  });
});
