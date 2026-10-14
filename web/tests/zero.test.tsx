//! The zero state: what Verkstead is when there is nothing to list.
//!
//! It is a fact about the sidebar's list rather than about onboarding, so every
//! test here says what it is by what it serves for that list: nothing on it, and
//! the page is the compose page across the whole window; something on it, and
//! the pane is back with the compose page beside it. Nobody here is a fresh
//! install — the machine is set up in all of them, which is what makes the
//! state a fact about the list rather than about the machine.
//!
//! What decides it is `src/workbench/zero.ts`, read by the two pages that care:
//! the workbench, which has nowhere to be with an empty sidebar, and the compose
//! page, which draws the sidebar or does without it.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { QueryClientProvider, QueryClient } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  ConversationEntry,
  ConversationView,
  ShowingArchived,
} from "../src/api/types";
import act from "../src/workbench/Actions.module.css";
import dropdown from "../src/Menu.module.css";
import shell from "../src/Panes.module.css";
import shellCss from "../src/Panes.module.css?raw";
import { SettingsPage, panes as settingsPanes } from "../src/settings/SettingsPage";
import archived from "../src/workbench/Archived.module.css";
import wordmark from "../src/workbench/Wordmark.module.css";
import {
  BRANCHES,
  HIDING_ARCHIVED,
  HIDING_SOMETHING,
  PROFILES,
  REPOS,
  SET_UP,
  SIDEBAR,
  drawn,
  mount,
  nudged,
  theWorkbench,
} from "./bench";
import { hangs, json, serving, whenever } from "./serving";
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };

afterEach(() => {
  vi.unstubAllGlobals();
});

/// The archived Conversation the switch brings back: the fixture's own first
/// row, which is what an archived one looks like once it is on the list again.
const PUT_AWAY: ConversationEntry[] = [SIDEBAR[0]!];

/// The one Conversation the press test archives: a Grilling one, which is a row
/// with the actions menu on it — a Draft has no press that puts it away.
const GRILLING = grilling as ConversationView;

/// A workbench whose list is whatever the test hands in, and whose switch stands
/// wherever it says. Everything else is the fixtures' own.
function listing(
  rows: () => ConversationEntry[],
  switching: () => ShowingArchived = () => HIDING_ARCHIVED,
  ...answers: Parameters<typeof theWorkbench>
) {
  return theWorkbench(
    whenever("/api/ui/conversations", () => json(rows())()),
    whenever("/api/ui/conversations/archived", () => json(switching())()),
    ...answers,
  );
}

/// The conversations pane, or null where the frame was handed none.
function sidebar(container: ParentNode): Element | null {
  return container.querySelector(`.${shell.conversationsPane}`);
}

describe("a Verkstead with nothing to list", () => {
  /// The redirect is decided where the list is read rather than in the route
  /// table: a table cannot know what the sidebar's query answered.
  it("sends the bare workbench to the compose page", async () => {
    listing(() => []);
    const { history } = mount();

    await waitFor(() => expect(history.get()).toBe("/compose"));
  });

  /// Replacing rather than pushing, so Back does not walk into a page that will
  /// only redirect again.
  it("replaces the workbench in the history rather than pushing over it", async () => {
    listing(() => []);
    const { history } = mount();

    await waitFor(() => expect(history.get()).toBe("/compose"));

    history.back();
    // Nothing behind it to go back to, so the entry it replaced is not there to
    // be returned to and the page stands where it is.
    await waitFor(() => expect(history.get()).toBe("/compose"));
  });

  /// And the page it lands on is the compose page with no list beside it: one
  /// pane rather than two, because there is no level above this one to walk back
  /// out to.
  it("draws the compose page with no conversations pane", async () => {
    listing(() => []);
    const { container } = mount("/compose");

    await drawn(container, `.${shell.detailsPane}`);
    expect(sidebar(container)).toBeNull();
    expect(
      container.querySelector(`.${shell.panes}`)!.classList.contains(shell.alone!),
    ).toBe(true);
  });

  /// Which the frame draws across the window rather than a fifth of the way in
  /// from the left: the columns it is handed are named for panes that are not
  /// there.
  it("stands that one pane across the whole frame", () => {
    const at = shellCss.indexOf("\n.panes.alone {");
    expect(at, "expected the sheet to hold the one-pane frame").toBeGreaterThan(-1);
    expect(shellCss.slice(at, shellCss.indexOf("\n}", at))).toContain(
      "grid-template-columns: 1fr;",
    );
  });

  /// Entered at the wordmark rather than at a way back to a pane that is not
  /// there — the same head the sidebar is entered at, gear and all, so the way
  /// out to the settings is where it always was.
  it("heads that page with the wordmark and the gear", async () => {
    listing(() => []);
    const { container } = mount("/compose");

    const heading = await drawn(container, `h1.${wordmark.wordmark}`);
    expect(heading.textContent).toContain("Verkstead");
    await drawn(container, 'button[aria-label="Settings"]');

    // And not the pane's own title, there being no pane beside this one for it
    // to be one of.
    expect(screen.queryByText("New conversation")).toBeNull();
  });

  /// The gear goes where it goes everywhere else, which is the whole of why it
  /// is drawn here: a page with no sidebar would otherwise be a page with no way
  /// to the settings at all.
  it("reaches the settings from that gear", async () => {
    listing(() => []);
    const { container, history } = mount("/compose");

    fireEvent.click(await drawn(container, 'button[aria-label="Settings"]'));
    await waitFor(() => expect(history.get()).toBe("/settings"));
  });

  /// And the settings page keeps its sidebar while the state holds, which is the
  /// one page in it that does: it is a list of its own, and a settings page with
  /// no way back to the conversations would be a page somebody had to type their
  /// way out of.
  it("leaves the settings page its sidebar all the same", async () => {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever("/api/ui/settings", json({ error: "not asked" }, 503)),
      whenever("/api/ui/update", json("Current")),
      whenever("/api/ui/abandoned-roadmaps", json([])),
    );

    const queries = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    const history = createMemoryHistory();
    history.set({ value: "/settings" });

    const { container } = render(() => (
      <QueryClientProvider client={queries}>
        <MemoryRouter history={history}>
          <Route path="/settings" component={SettingsPage}>
            {settingsPanes()}
          </Route>
        </MemoryRouter>
      </QueryClientProvider>
    ));

    await drawn(container, `.${shell.conversationsPane}`);
    await waitFor(() =>
      screen.getByText(/No conversations yet\./),
    );
  });
});

describe("the sidebar coming and going under the zero state", () => {
  /// Archiving the last Conversation is something another device hears about as
  /// a Nudge, which re-reads the list — and an empty list is the zero state
  /// wherever it is read.
  it("moves another device to the zero state on its next read", async () => {
    let rows: ConversationEntry[] = SIDEBAR;
    listing(() => rows);
    const { container, history, client } = mount("/compose");

    await drawn(container, `.${shell.conversationsPane}`);

    rows = [];
    await nudged(client);

    await waitFor(() => expect(sidebar(container)).toBeNull());
    // And the page it was already on is the page the state lands on, so nothing
    // moves under the human but the pane going.
    expect(history.get()).toBe("/compose");
  });

  /// And on *this* device the press is what moves it, rather than the read
  /// behind the press.
  ///
  /// The sidebar draws its list with whatever a press has already said about it
  /// laid over (`eager.ts`), so a state read off the server's last answer alone
  /// would disagree with what is on the screen for as long as the archiving was
  /// in the air: the compose page would arrive with the sidebar still beside it,
  /// saying there was nothing in it, and the frame would change under the human
  /// a round trip later. Here the archiving never lands at all and the server
  /// goes on saying there is one Conversation, so the sidebar going is the
  /// press's doing and nothing else's.
  it("takes the sidebar with the press rather than with the read behind it", async () => {
    const only = SIDEBAR.find((row) => row.id === GRILLING.id)!;

    serving(
      whenever("/api/ui/conversations", json([only])),
      whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
      whenever("/api/ui/repos", json(REPOS)),
      whenever("/api/ui/profiles", json(PROFILES)),
      whenever("/api/ui/onboarding", json(SET_UP)),
      whenever("/api/ui/abandoned-roadmaps", json([])),
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(`/api/ui/repos/${GRILLING.repo.id}/branches`, json(BRANCHES)),
      whenever(
        `/api/ui/conversations/${GRILLING.id}/close-and-archive`,
        hangs(),
        "POST",
      ),
      json(null),
    );

    const { container, history } = mount(`/conversations/${GRILLING.id}`);

    fireEvent.click(
      await drawn(
        container,
        `.${act.conversationActions} > .${dropdown.trigger}`,
      ),
    );
    fireEvent.click(await drawn(container, `.${act.closeAndArchive}`));

    // The one row is off the list the human is looking at, so there is nothing
    // to list — and the page that says so is the one they land on, whole.
    await waitFor(() => expect(history.get()).toBe("/compose"));
    await drawn(container, `h1.${wordmark.wordmark}`);
    expect(sidebar(container)).toBeNull();
  });

  /// And the other way about: something on the list is something to list, so the
  /// pane comes back.
  it("brings the sidebar back when something is unarchived elsewhere", async () => {
    let rows: ConversationEntry[] = [];
    listing(() => rows);
    const { container, client } = mount("/compose");

    // The state as it stands, waited for rather than read off a page that has
    // not drawn yet: a sidebar that is not there because nothing at all is is
    // not this page without one.
    await drawn(container, `h1.${wordmark.wordmark}`);
    expect(sidebar(container)).toBeNull();

    rows = SIDEBAR;
    await nudged(client);

    await drawn(container, `.${shell.conversationsPane}`);
  });

  /// The switch pinned to the corner is the other way out of the state, and the
  /// reason it is on this page at all: a list with the archived ones in it is not
  /// an empty list.
  it("brings the sidebar back with the archived ones in it", async () => {
    let rows: ConversationEntry[] = [];
    let switching = HIDING_SOMETHING;

    listing(
      () => rows,
      () => switching,
      whenever(
        "/api/ui/conversations/archived",
        () => {
          rows = PUT_AWAY;
          switching = { showing: true, any: true };
          return json(undefined, 204)();
        },
        "POST",
      ),
    );
    const { container } = mount("/compose");

    fireEvent.click(
      await waitFor(() => screen.getByLabelText("Show archived")),
    );

    const pane = await drawn(container, `.${shell.conversationsPane}`);
    await waitFor(() => expect(pane.textContent).toContain(PUT_AWAY[0]!.branch));
  });
});

describe("the switch pinned to the zero state's corner", () => {
  /// Drawn where there is something behind it, which is what the server says
  /// beside where the switch stands: the list is filtered by the setting, so an
  /// empty one cannot be asked.
  it("is drawn where something is archived", async () => {
    listing(
      () => [],
      () => HIDING_SOMETHING,
    );
    const { container } = mount("/compose");

    const switching = await waitFor(() => screen.getByLabelText("Show archived"));

    // At the foot of the pane, which is what stands it in the corner: last in
    // the column, wearing the frame's own name for a foot.
    const foot = container.querySelector(`.${shell.detailsPane}`)!.lastElementChild!;
    expect(foot.contains(switching)).toBe(true);
    expect(foot.classList.contains(archived.showArchived!)).toBe(true);
    expect(foot.classList.contains(shell.paneFoot!)).toBe(true);
  });

  /// And not drawn at all where nothing is: a switch that could bring nothing
  /// back is a control with no state to be in, which is where a fresh Verkstead
  /// stands.
  it("is not drawn where nothing is archived", async () => {
    listing(() => []);
    const { container } = mount("/compose");

    await drawn(container, `h1.${wordmark.wordmark}`);
    expect(screen.queryByLabelText("Show archived")).toBeNull();
  });

  /// And it is the sidebar's own switch wherever the sidebar stands, which is
  /// where it was before there was a page without one.
  it("stays at the foot of the sidebar where there is one", async () => {
    listing(() => SIDEBAR);
    const { container } = mount("/compose");

    const pane = await drawn(container, `.${shell.conversationsPane}`);
    const foot = pane.lastElementChild!;

    expect(foot.classList.contains(archived.showArchived!)).toBe(true);
    expect(
      container.querySelector(`.${shell.detailsPane}`)!.lastElementChild!.classList
        .contains(archived.showArchived!),
    ).toBe(false);
  });
});
