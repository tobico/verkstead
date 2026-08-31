//! How a conflicted pull request is resolved, on the settings page: what the
//! card says of it, what the picker sends, and where the cost of a rebase is
//! said.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying which strategy is in force, and the picker in the details pane it
//! opens.
//!
//! What is worth proving here is the warning and the pass-through. The warning,
//! because a rebase force-pushes and that is the one consequence a human cannot
//! be expected to know from the word — so it has to be on the page wherever a
//! rebase is the answer, and nowhere else, or it stops being read. The
//! pass-through, because the page has one endpoint and it writes both settings
//! files: a picker that dropped the author or the token on the way would be a
//! setting that cost the credentials.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with — `settings.json` being a
//! Verkstead configured for a rebase and `settings-unset.json` one nobody has
//! told anything.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ConflictResolution, SettingsSaved, SettingsView } from "../src/api/types";
import { ConflictsCard, ConflictsPane } from "../src/settings/Conflicts";
import styles from "../src/settings/Conflicts.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

/// A Verkstead told to rebase, and one nobody has told anything — which is a
/// merge.
const REBASING = told as SettingsView;
const UNSET = unset as SettingsView;

afterEach(() => {
  vi.unstubAllGlobals();
});

function mounting(what: () => JSX.Element) {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

function mountCard(open = false) {
  const press = vi.fn();
  return {
    ...mounting(() => <ConflictsCard open={open} press={press} />),
    press,
  };
}

function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <ConflictsPane back={back} />), back };
}

function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// What a save answers with, which is the settings as they now stand.
function answering(standing: SettingsView): SettingsSaved {
  return { settings: standing, verified: null };
}

/// The same settings resolving conflicts the other way, which is what a save of
/// this picker comes back as.
function resolving(
  standing: SettingsView,
  conflict_resolution: ConflictResolution,
): SettingsView {
  return { ...standing, conflict_resolution };
}

function sent(fetching: ReturnType<typeof serving>): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  );
  expect(written, "expected the page to have saved").toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

function thePicker(): HTMLSelectElement {
  return screen.getByLabelText(
    /How a conflicted pull request is resolved/,
  ) as HTMLSelectElement;
}

/// Whether the cost of a rebase is on the page, wherever the page is.
function warned(container: ParentNode): boolean {
  return container.querySelector(`.${styles.warning}`) !== null;
}

describe("the card", () => {
  /// What a Verkstead nobody has been to says, which is the whole shape of the
  /// setting: a merge, and no warning, because nothing here rewrites a branch.
  it("says a conflict is merged where nobody has configured anything", async () => {
    theSettings(UNSET);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/merging the base branch in/));

    expect(warned(container)).toBe(false);
  });

  /// And what one configured for a rebase says — with the cost of it, because
  /// whoever needs to read that is precisely whoever is not editing.
  it("says a conflict is rebased where that is what was configured, and warns", async () => {
    theSettings(REBASING);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/rebasing the branch onto its base/));

    expect(warned(container)).toBe(true);
    expect(container.textContent).toContain("force-pushed");
  });
});

describe("the pane", () => {
  it("shows the strategy in force", async () => {
    theSettings(REBASING);
    mountPane();

    await waitFor(() => expect(thePicker().value).toBe("Rebase"));
  });

  /// The picker is its own press: picking is the save, the way flipping the
  /// build cache's switch is.
  it("saves the moment a strategy is picked, and leaves everything else alone", async () => {
    const fetching = theSettings(
      UNSET,
      json(answering(resolving(UNSET, "Rebase"))),
    );
    mountPane();

    await waitFor(() => expect(thePicker().value).toBe("Merge"));

    fireEvent.change(thePicker(), { target: { value: "Rebase" } });

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        // Untouched, all of it: one request writes the whole of both files, and
        // this section has no business with any of the rest.
        git_author: UNSET.git_author,
        github_token: "Keep",
        rust_build_cache: { enabled: UNSET.rust_build_cache.enabled, size: "" },
        watched_paths: [],
        sandbox_binds: [],
        conflict_resolution: "Rebase",
      }),
    );
  });

  /// And the answer is what the pane then draws — including the warning, which
  /// arrives with the choice rather than a read later.
  it("draws what the save answered with, warning and all", async () => {
    theSettings(UNSET, json(answering(resolving(UNSET, "Rebase"))));
    const { container } = mountPane();

    await waitFor(() => expect(thePicker().value).toBe("Merge"));
    expect(warned(container)).toBe(false);

    fireEvent.change(thePicker(), { target: { value: "Rebase" } });

    // Waited for, because what draws it is the save answering rather than the
    // press: the control moves at once and the page follows the server.
    await waitFor(() => expect(warned(container)).toBe(true));
    expect(thePicker().value).toBe("Rebase");
  });

  /// And back again, because a setting that could only be turned on would be one
  /// nobody could undo from a phone.
  it("takes a rebase back to a merge", async () => {
    const fetching = theSettings(
      REBASING,
      json(answering(resolving(REBASING, "Merge"))),
    );
    const { container } = mountPane();

    await waitFor(() => expect(thePicker().value).toBe("Rebase"));

    fireEvent.change(thePicker(), { target: { value: "Merge" } });

    await waitFor(() =>
      expect(
        (sent(fetching) as { conflict_resolution: unknown })
          .conflict_resolution,
      ).toBe("Merge"),
    );

    await waitFor(() => expect(warned(container)).toBe(false));
  });
});
