//! What the Components section says of a machine (ADR-0024): nothing at all
//! where it cannot have an Annex, *not installed* where it can and has none,
//! and how the box stands where there is one — running or stopped, the release
//! the Keeper last pinned it to, and the disk its image takes.
//!
//! The reads are fixtures the server's own suite wrote off the real endpoint —
//! see `crates/server/tests/ui_content.rs` — so the page is drawn from the shape
//! it really answers with. The states nobody stood a box up in are that same
//! answer with its state changed, the way the Devices tests vary a member.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { describe, expect, it, vi } from "vitest";

import type { AnnexComponent, ComponentsView } from "../src/api/types";
import { ComponentsCard, ComponentsPane } from "../src/settings/Components";
import absent from "./fixtures/components-absent.json" with { type: "json" };
import booted from "./fixtures/components-booted.json" with { type: "json" };
import notInstalled from "./fixtures/components-not-installed.json" with { type: "json" };
import { json, serving, whenever } from "./serving";

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

function mounting(read: ComponentsView, what: () => JSX.Element) {
  serving(whenever("/api/ui/components", json(read)));

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
    expect(row.querySelector("button"), "Install is a later task's").toBeNull();
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
  });

  it("says virtualization is off in the wizard's words", async () => {
    pane({ annex: { ...NOT_INSTALLED.annex!, state: "VirtualizationOff" } });

    expect((await theRow()).textContent).toContain(
      "Virtualization is off in this PC's firmware settings, so WSL can't run.",
    );
  });
});
