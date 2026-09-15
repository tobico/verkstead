//! `/setup` is a page while onboarding mode is on and no page at all while it
//! is off, and the app's route table is where both of those are said.
//!
//! The sibling of `settings-routes.test.tsx`, and for the same seam: a path that
//! reaches no route is a page that silently is not there, and nothing about the
//! page itself is wrong when it happens. What is walked here is one table more
//! than that file walks, because the gate is a choice *between* two of them —
//! while the mode is on the router is handed `/setup` and a redirect for
//! everything else, and while it is off it is handed the app as it has always
//! been, with no `/setup` in it.
//!
//! So this mounts the real gate over a stated reading, rather than the routes
//! under a stand-in: which table the app builds is exactly what is being asked,
//! and a test that picked one itself would be asking nothing. Under it are the
//! workbench's own lists, so that whatever the gate lets through has something
//! to draw.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { describe, expect, it } from "vitest";

import { Gate } from "../src/App";
import type { OnboardingView } from "../src/api/types";
import { OPEN, SET_UP, theWorkbench } from "./bench";
import { json, whenever } from "./serving";
import fresh from "./fixtures/onboarding-fresh.json" with { type: "json" };

/// A start on a bare machine: nothing installed, no Profile, no author — which
/// is the mode on.
const FRESH = fresh as OnboardingView;

/// The wizard's own heading, which is what says the page reached was it.
const WIZARD = "Set Verkstead up";

/// And what the app answers a path nothing has.
const MISSED = "No such page.";

/// The gate at `path`, over a machine that stands as `machine` says.
///
/// A query client of this test's own rather than the app's: `App` builds one at
/// module scope and it outlives every render, so a verdict cached by one test
/// here would be the wrong app drawn for a moment in the next — with a redirect
/// fired out of it before the read that would have corrected it landed.
function at(path: string, machine: OnboardingView): void {
  window.history.pushState({}, "", path);
  theWorkbench(whenever("/api/ui/onboarding", json(machine)));

  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  render(() => (
    <QueryClientProvider client={client}>
      <Gate />
    </QueryClientProvider>
  ));
}

describe("while onboarding mode is on", () => {
  /// Every kind of URL the app has: the workbench's root, the page before there
  /// is a Conversation, the settings, a Conversation and one of its details
  /// panes. None of them is anywhere to be.
  for (const path of [
    "/",
    "/compose",
    "/settings",
    "/settings/git",
    `/conversations/${OPEN.id}`,
    `/conversations/${OPEN.id}/backlog`,
    "/nonsense",
  ]) {
    it(`lands on /setup from ${path}`, async () => {
      at(path, FRESH);

      await waitFor(() => screen.getByText(WIZARD));
      expect(window.location.pathname).toBe("/setup");
    });
  }

  /// And `/setup` itself is the page rather than a redirect onto itself.
  it("draws the wizard at /setup", async () => {
    at("/setup", FRESH);

    await waitFor(() => screen.getByText(WIZARD));
    expect(window.location.pathname).toBe("/setup");
  });
});

describe("while it is off", () => {
  /// The wizard is a first run rather than a page to visit, so its path is one
  /// nothing answers — the same nothing `/repos` and `/profiles` became when
  /// they were folded into the settings.
  it("answers /setup with the no-such-page fallback", async () => {
    at("/setup", SET_UP);

    await waitFor(() => screen.getByText(MISSED));
    expect(screen.queryByText(WIZARD)).toBeNull();
  });

  /// And every other URL is the page it always was, at the path it was asked
  /// for: nothing here redirects anything.
  it("leaves every other URL where it was", async () => {
    at("/compose", SET_UP);

    await waitFor(() => screen.getByText("Save as draft"));
    expect(window.location.pathname).toBe("/compose");
    expect(screen.queryByText(WIZARD)).toBeNull();
  });
});
