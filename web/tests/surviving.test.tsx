//! What a re-read leaves standing: the elements the human's own state lives on,
//! and the choice a dropdown would send.
//!
//! Every Nudge makes the open page read again what it is showing (ADR-0005), and
//! a re-read that rebuilds its DOM takes the reader with it — a spinner starts
//! its animation over, an open dropdown closes, and the choice inside one can
//! come apart from what is displayed. ADR-0009 answered that in two places: the
//! merge every query now has to name in `src/freshness.ts`, which keeps the
//! elements alive, and the [`Picker`] in `src/picking.tsx`, which keeps a
//! `<select>` honest even when they are not.
//!
//! So this file asserts about identity rather than about appearance. Everything
//! here would pass on text: the rebuilt row says the same words, and the
//! dropdown that snapped to the first repository looks exactly like one nobody
//! touched. What tells them apart is whether the very element is still there,
//! and whether what is on screen is what would go on the wire.
//!
//! The fixtures are the golden ones `cargo test` writes from the real endpoints,
//! and the re-read is driven the way a Nudge drives it — see `nudged` in
//! `bench.tsx`.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it, vi } from "vitest";

import type {
  ConversationEntry,
  ConversationView,
} from "../src/api/types";
// The one menu, which is what the sidebar offers its repositories through.
import menu from "../src/Menu.module.css";
// The ring the sidebar's card turns while a session runs.
import marks from "../src/workbench/Mark.module.css";
import { Picker } from "../src/picking";
import {
  OPEN,
  PROFILES,
  REPOS,
  SIDEBAR,
  drawn,
  mount,
  nodes,
  nudged,
  survived,
  theWorkbench,
} from "./bench";
import { json, serving, whenever } from "./serving";
import building from "./fixtures/conversation-building.json" with { type: "json" };

/// The renderer is a page's own doing and nothing here has a Diagram; mocked so
/// this file does not load megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

/// A conversation with a worktree and nothing running in it, which is where the
/// manual task composer — and the third profile picker — is drawn.
const BUILDING = building as ConversationView;

/// The sidebar with a session talking on one of its rows, which is what puts a
/// spinner on a card.
const BUSY: ConversationEntry[] = SIDEBAR.map((entry) =>
  entry.id === 5 ? { ...entry, working: true } : entry,
);

/// The repo the sidebar's menu is left offering when the other is unregistered.
const FIRST = REPOS[0]!;

/// The workbench with the repos answered from a list a test can move under it,
/// which is what a repository being unregistered elsewhere looks like from here.
function theRepos(...answers: Parameters<typeof serving>) {
  const standing = { repos: REPOS };
  const fetching = theWorkbench(
    whenever("/api/ui/repos", () => json(standing.repos)()),
    ...answers,
  );
  return {
    fetching,
    /// The list as the server would answer for it now.
    holds: (repos: typeof REPOS) => {
      standing.repos = repos;
    },
  };
}

describe("a picker whose options are rebuilt", () => {
  /// Two rows and a choice, drawn straight rather than through a page: what is
  /// asked here is the control's own guarantee, so nothing about a query or a
  /// merge is in the way of the answer.
  function picking(chosenAt = "2") {
    const [rows, setRows] = createSignal([
      { id: 1, name: "verkstead" },
      { id: 2, name: "askance" },
    ]);
    const [chosen, setChosen] = createSignal(chosenAt);

    const { container } = render(() => (
      <Picker
        id="repo"
        options={rows()}
        value={(repo) => String(repo.id)}
        label={(repo) => repo.name}
        chosen={chosen()}
        pick={setChosen}
        gone={() => setChosen("")}
      />
    ));

    return {
      select: container.querySelector("select")!,
      chosen,
      /// The list read again, as rows the merge did not keep: every option is a
      /// new element, which is what an unmerged re-read leaves behind.
      rebuilt: setRows,
    };
  }

  it("shows the same choice after every option is a new element", () => {
    const { select, rebuilt } = picking();
    expect(select.value).toBe("2");

    rebuilt([
      { id: 1, name: "verkstead" },
      { id: 2, name: "askance" },
    ]);

    // The same two rows and the same choice — and, without the control
    // re-applying it, a browser that has quietly selected the other one.
    expect(select.value).toBe("2");
    expect(select.selectedOptions[0]!.textContent).toBe("askance");
  });

  it("shows nothing chosen when the chosen row is gone, and says so", () => {
    const { select, chosen, rebuilt } = picking();

    rebuilt([{ id: 1, name: "verkstead" }]);

    // Not the row that happens to be first now: what was picked is gone, and
    // the honest reading of that is that nothing is picked.
    expect(select.value).toBe("");
    expect(select.selectedOptions[0]!.textContent).toBe("Not chosen");
    // And the signal behind it holds what the control shows, rather than a
    // repository nobody could see.
    expect(chosen()).toBe("");
  });
});

describe("what a Nudge leaves standing", () => {
  it("keeps the row a session's spinner is spinning on", async () => {
    theWorkbench(whenever("/api/ui/conversations", json(BUSY)));
    const { container, client } = mount();
    const spinner = await drawn(container, `.conversation-row .${marks.mark}.${marks.working}`);

    await nudged(client);

    // The same element, so the animation is where it was rather than back at
    // its first frame — which is what the merge on the conversations query is
    // for.
    expect(container.querySelector(`.conversation-row .${marks.mark}.${marks.working}`)).toBe(
      spinner,
    );
  });

  /// The repos are a menu rather than a `<select>` now, and the merge is what
  /// keeps its rows alive: a Nudge landing while the menu is open would
  /// otherwise rebuild the row the human had tabbed to and take their focus
  /// with it.
  it("keeps the open menu's repo rows", async () => {
    theWorkbench();
    const { container, client } = mount();
    fireEvent.click(await drawn(container, `.new-conversation > .${menu.trigger}`));
    await drawn(container, ".in-repo");
    const offered = nodes(container, ".in-repo");

    await nudged(client);

    survived(offered, nodes(container, ".in-repo"));
  });

  it("keeps both pairing pickers' options", async () => {
    theWorkbench();
    const { container, client } = mount(`/conversations/${OPEN.id}`);
    await drawn(container, "#grilling-pairing option");
    const grilling = nodes(container, "#grilling-pairing option");
    const implementing = nodes(container, "#implementation-pairing option");

    await nudged(client);

    survived(grilling, nodes(container, "#grilling-pairing option"));
    survived(implementing, nodes(container, "#implementation-pairing option"));
  });

  /// The third picker, and the one a Nudge is loudest around: it sits under a
  /// half-typed instruction while a session talks above it.
  it("keeps the manual task composer's options", async () => {
    theWorkbench(
      whenever(`/api/ui/conversations/${BUILDING.id}`, json(BUILDING)),
    );
    const { container, client } = mount(`/conversations/${BUILDING.id}`);
    await drawn(container, "#manual-task-pairing option");
    const options = nodes(container, "#manual-task-pairing option");

    await nudged(client);

    survived(options, nodes(container, "#manual-task-pairing option"));
  });
});

describe("what a picker shows and what it would send", () => {
  /// The sidebar's own repo choice is no longer one of these: the menu that
  /// replaced the box holds nothing between the press and the wire, so the row
  /// pressed *is* the repository sent and there is no divergence left to guard
  /// against. What is asked here instead is that a repository unregistered
  /// from somewhere else stops being offered.
  it("stops offering a repo that has been unregistered", async () => {
    const { holds } = theRepos();
    const { container, client } = mount();
    fireEvent.click(await drawn(container, `.new-conversation > .${menu.trigger}`));
    await drawn(container, ".in-repo");

    holds([FIRST]);
    await nudged(client);

    await waitFor(() =>
      expect(
        [...container.querySelectorAll(".in-repo")].map(
          (row) => row.textContent,
        ),
      ).toEqual([FIRST.name]),
    );
  });

  /// The same guarantee on the pane where the choice is the server's rather than
  /// the page's: a pairing whose profile has been deleted is not swapped for
  /// whichever one happens to be first, because that is a session running under
  /// an account nobody chose.
  it("shows no pairing at all when the chosen profile is deleted", async () => {
    const chosen = OPEN.grilling_pairing!;
    const standing = { profiles: PROFILES };
    theWorkbench(whenever("/api/ui/profiles", () => json(standing.profiles)()));
    const { client } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));
    const grilling = screen.getByLabelText("Grilling") as HTMLSelectElement;
    expect(grilling.value).toBe(`${chosen.profile.id}:${chosen.model}`);

    standing.profiles = PROFILES.filter(
      (profile) => profile.id !== chosen.profile.id,
    );
    await nudged(client);

    expect(grilling.value).toBe("");
    expect(grilling.selectedOptions[0]!.textContent).toBe("Not chosen");
  });

  /// And the same when the profile is still there but no longer lists the model
  /// it was paired with: half a pairing is not a pairing, so what is shown is
  /// nothing rather than the same account on a model nobody chose.
  it("shows no pairing at all when the chosen model leaves the list", async () => {
    const chosen = OPEN.grilling_pairing!;
    const standing = { profiles: PROFILES };
    theWorkbench(whenever("/api/ui/profiles", () => json(standing.profiles)()));
    const { client } = mount(`/conversations/${OPEN.id}`);
    await waitFor(() => screen.getByLabelText("Grilling"));
    const grilling = screen.getByLabelText("Grilling") as HTMLSelectElement;

    standing.profiles = PROFILES.map((profile) =>
      profile.id === chosen.profile.id
        ? {
            ...profile,
            models: profile.models.filter((model) => model !== chosen.model),
          }
        : profile,
    );
    await nudged(client);

    expect(grilling.value).toBe("");
  });
});
