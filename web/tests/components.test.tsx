//! What the Components section says of a machine (ADR-0024): nothing at all
//! where it cannot have an Annex, *not installed* where it can and has none,
//! and how the box stands where there is one — running or stopped, the release
//! the Keeper last pinned it to, and the disk its image takes.
//!
//! The reads are fixtures the server's own suite wrote off the real endpoint —
//! see `crates/server/tests/ui_content.rs` — so the page is drawn from the shape
//! it really answers with. The states nobody stood a box up in are that same
//! answer with its state changed, the way the Devices tests vary a member.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  AnnexComponent,
  ComponentsView,
  DependencyView,
  OnboardingView,
  RunView,
} from "../src/api/types";
import { ComponentsCard, ComponentsPane } from "../src/settings/Components";
import absent from "./fixtures/components-absent.json" with { type: "json" };
import booted from "./fixtures/components-booted.json" with { type: "json" };
import notInstalled from "./fixtures/components-not-installed.json" with { type: "json" };
import installing from "./fixtures/onboarding-installing.json" with { type: "json" };
import { askedFor, json, serving, whenever } from "./serving";

/// A machine that cannot have an Annex, one that can and has none, and one
/// whose box this start booted — its link having found nobody, so its row
/// carries the Devices row's line too.
const ABSENT = absent as ComponentsView;
const NOT_INSTALLED = notInstalled as ComponentsView;
const BOOTED = booted as ComponentsView;

/// The booted box's row with `changed` over it.
function annex(changed: Partial<AnnexComponent>): ComponentsView {
  return { annex: { ...BOOTED.annex!, ...changed } };
}

/// The wizard's reading, which Install is answered with and its two screens are
/// drawn from: the shape the install suite wrote off a real run, as a Windows
/// Keeper reads it — the WSL row alone, and `run` over it.
const INSTALLING = installing as OnboardingView;

function wizard(run: RunView | null, wsl: DependencyView): OnboardingView {
  return {
    ...INSTALLING,
    mode: false,
    platform: "Windows",
    distro: "Windows",
    dependencies: [wsl],
    run,
  };
}

/// The WSL row, standing `state`: missing, its restart owed, or there.
function wslRow(
  state: "Absent" | "Restart" | "Present",
  install: DependencyView["install"] = { install: "Idle" },
): DependencyView {
  return {
    dependency: "Wsl",
    state:
      state === "Absent"
        ? { state: "Absent", trouble: null, seen: null }
        : state === "Restart"
          ? { state: "Owed", owed: "Restart" }
          : { state: "Present", at: null, target: null },
    install,
  };
}

/// A run for the WSL row, going and doing `status`, or over.
function run(status: string | null): RunView {
  return status === null
    ? {
        phase: "Done",
        status: "Everything that was ticked is installed.",
        done: 1,
        total: 1,
        cancelling: false,
      }
    : { phase: "Installing", status, done: 0, total: 1, cancelling: false };
}

function mounting(read: ComponentsView, what: () => JSX.Element) {
  serving(
    whenever("/api/ui/components", json(read)),
    whenever("/api/ui/onboarding", json(wizard(null, wslRow("Absent")))),
  );

  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

const pane = (read: ComponentsView) =>
  mounting(read, () => <ComponentsPane back={vi.fn()} />);

const card = (read: ComponentsView) =>
  mounting(read, () => <ComponentsCard open={false} press={vi.fn()} />);

/// The Annex's row, once it is drawn.
async function theRow(): Promise<HTMLElement> {
  return waitFor(() => screen.getByText("Linux development (WSL)").closest("li")!);
}

describe("the card", () => {
  /// A machine that cannot have an Annex draws no card at all, so the page's
  /// order is what it always was.
  it("is not drawn on a machine that cannot have an Annex", async () => {
    const { container } = card(ABSENT);

    await new Promise((settled) => setTimeout(settled, 20));
    expect(container.textContent).toBe("");
  });

  it("says the Annex's state on a machine that can have one", async () => {
    card(NOT_INSTALLED);

    const title = await waitFor(() => screen.getByText("Components"));
    expect(title.closest("article")?.textContent).toContain("WSL not installed");
  });
});

describe("the pane", () => {
  it("reads not installed on a Keeper with no Annex", async () => {
    pane(NOT_INSTALLED);

    const row = await theRow();
    expect(row.textContent).toContain("not installed");
    expect(row.textContent).not.toContain("Verkstead");
    expect(row.querySelector("button")?.textContent).toBe("Install");
  });

  /// A running box reads *running*, the release it is pinned to and the disk
  /// it takes, and the line its Devices row carries where it has one.
  it("reads running, the release and the disk of a booted box", async () => {
    pane(BOOTED);

    const row = await theRow();
    expect(row.textContent).toContain("running");
    expect(row.textContent).toContain("Verkstead 0.9.1 · 12.9 GB");
    expect(row.textContent).toContain(
      "The annex didn't answer: its service is failed.",
    );
  });

  /// A stopped one reads *stopped* with the release last pinned: the version
  /// is the one the Keeper read at its boot, so nothing is started to draw it.
  it("reads stopped with the release last pinned", async () => {
    pane(annex({ state: "Stopped", trouble: null }));

    const row = await theRow();
    expect(row.querySelector("[data-state]")?.textContent).toBe("stopped");
    expect(row.textContent).toContain("Verkstead 0.9.1");
  });

  it("reads updating while the box is rebuilt", async () => {
    pane(annex({ state: "Updating", trouble: null }));

    const row = await theRow();
    expect(row.querySelector("[data-state]")?.textContent).toBe("updating");
  });

  /// Before the first boot has read a release, no version is drawn rather than
  /// a guess.
  it("draws no version before one has been read", async () => {
    pane(annex({ version: null }));

    const row = await theRow();
    expect(row.textContent).not.toContain("Verkstead ");
    expect(row.textContent).toContain("12.9 GB");
  });

  /// What WSL is waiting on is said in the wizard row's own words.
  it("says a restart is owed, and virtualization off, as the wizard does", async () => {
    pane({ annex: { ...NOT_INSTALLED.annex!, state: "NeedsRestart" } });

    expect((await theRow()).textContent).toContain(
      "Restart Windows to finish turning WSL on.",
    );
    expect(
      (await theRow()).querySelector("button"),
      "a restart is the human's, and there is nothing to press until it is done",
    ).toBeNull();
  });

  it("says virtualization is off in the wizard's words", async () => {
    pane({ annex: { ...NOT_INSTALLED.annex!, state: "VirtualizationOff" } });

    expect((await theRow()).textContent).toContain(
      "Virtualization is off in this PC's firmware settings, so WSL can't run.",
    );
  });
});

describe("Install", () => {
  afterEach(() => vi.unstubAllGlobals());

  /// Long enough for the install screen's two-second read to have come round.
  const AFTER_A_POLL = 4_000;

  /// Serve a Keeper whose Annex is made by the press: no run and the row not
  /// installed until Install, the run `going` from then until `made()`, and
  /// then the run `over` with the box running.
  function aKeeper(going: OnboardingView, over: OnboardingView) {
    let stage: "idle" | "going" | "over" = "idle";

    const fetching = serving(
      whenever("/api/ui/components", () =>
        json(stage === "over" ? BOOTED : NOT_INSTALLED)(),
      ),
      whenever("/api/ui/onboarding", () =>
        json(
          stage === "idle"
            ? wizard(null, wslRow("Absent"))
            : stage === "going"
              ? going
              : over,
        )(),
      ),
      whenever(
        "/api/ui/components/install",
        () => {
          stage = "going";
          return json(going)();
        },
        "POST",
      ),
    );

    mounted();

    return { fetching, made: () => (stage = "over") };
  }

  /// The pane, over whatever `serving` was told rather than the defaults.
  function mounted() {
    const queries = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    return render(() => (
      <QueryClientProvider client={queries}>
        <ComponentsPane back={vi.fn()} />
      </QueryClientProvider>
    ));
  }

  /// The press is the wizard's run: its install screen while it goes, and the
  /// row back once it is over, reading what the probe now says.
  it("draws the wizard's install screen, then the row the run made", async () => {
    const keeper = aKeeper(
      wizard(
        run("Importing the Verkstead distro…"),
        wslRow("Absent", { install: "Installing" }),
      ),
      wizard(run(null), wslRow("Present")),
    );

    fireEvent.click(await waitFor(() => screen.getByText("Install")));

    await waitFor(() =>
      expect(screen.getByText("Importing the Verkstead distro…")).toBeTruthy(),
    );
    expect(
      keeper.fetching.mock.calls.some(
        ([path, init]) =>
          String(path) === "/api/ui/components/install" && init?.method === "POST",
      ),
    ).toBe(true);

    // The install screen reads again every two seconds while the run goes.
    keeper.made();

    await waitFor(
      () => expect(screen.getByText("running")).toBeTruthy(),
      { timeout: AFTER_A_POLL },
    );
    expect(screen.queryByText("Importing the Verkstead distro…")).toBeNull();
    expect(askedFor(keeper.fetching, "/api/ui/components")).toBeGreaterThan(1);
  });

  /// WSL turned on and its restart owed: the hint screen, with the row's own
  /// instruction, and Back to the row.
  it("lands on the hint screen when a restart is owed", async () => {
    const keeper = aKeeper(
      wizard(
        run("Turning on WSL…"),
        wslRow("Absent", { install: "Installing" }),
      ),
      wizard(run(null), wslRow("Restart")),
    );

    fireEvent.click(await waitFor(() => screen.getByText("Install")));
    await waitFor(() =>
      expect(screen.getByText("Turning on WSL…")).toBeTruthy(),
    );

    keeper.made();

    await waitFor(
      () =>
        expect(
          screen.getByText(
            "Restart Windows to finish turning WSL on, then come back to this step.",
          ),
        ).toBeTruthy(),
      { timeout: AFTER_A_POLL },
    );

    fireEvent.click(screen.getByText("Back"));
    await theRow();
  });

  /// A run already going when the pane opens is drawn, and not started again.
  it("draws a run already going rather than offering Install", async () => {
    serving(
      whenever("/api/ui/components", json(NOT_INSTALLED)),
      whenever(
        "/api/ui/onboarding",
        json(
          wizard(
            run("Downloading NixOS-WSL…"),
            wslRow("Absent", { install: "Installing" }),
          ),
        ),
      ),
    );

    mounted();

    await waitFor(() =>
      expect(screen.getByText("Downloading NixOS-WSL…")).toBeTruthy(),
    );
    expect(screen.queryByText("Install")).toBeNull();
  });
});
