//! Mounting the workbench the way it is really mounted, for the tests that read
//! what it drew — and the one way they make it read the world again.
//!
//! Shared because two files' worth of assertions are about one page: what it
//! draws, and what of that it still has after a re-read. One mount between them
//! means both are asking about the page the app really builds, over the golden
//! fixtures `cargo test` writes from the real endpoints.

import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { expect } from "vitest";

import type {
  ConversationEntry,
  ConversationView,
  ProfileEntry,
  RepoEntry,
  ShowingArchived,
} from "../src/api/types";
import { Conversations } from "../src/workbench/Conversations";
import { Workbench } from "../src/workbench/Workbench";
import { json, serving, whenever } from "./serving";
import conversation from "./fixtures/conversation.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };

/// The sidebar, and the Conversation the fixtures open — the second row of it,
/// still drafting.
export const SIDEBAR = conversations as ConversationEntry[];
export const OPEN = conversation as ConversationView;

/// The two lists the pickers on the page are drawn from.
export const REPOS = repos as RepoEntry[];
export const PROFILES = profiles as ProfileEntry[];

/// And the third, which belongs to the repo the open Conversation is in: what
/// the base dropdown offers under the rule.
///
/// Written here rather than read out of a fixture, because it is a list of
/// strings: there is no shape for a golden file to hold true, and the endpoint
/// reads it out of a real git repository, which the fixtures have none of.
export const BRANCHES = ["main", "release-1.4", "origin/main"];

/// And the companion's own, which is a different repository with a list of its
/// own: what the base dropdown on its row offers, and nothing the conversation
/// could branch from.
export const COMPANION_BRANCHES = ["trunk", "origin/trunk"];

/// And the sidebar's one setting, off — which is where a workbench nobody has
/// archived anything in stands. Written here rather than read out of a fixture
/// for the reason the branches are: one boolean is not a shape a golden file
/// could hold true.
export const HIDING_ARCHIVED: ShowingArchived = { showing: false };

/// The conversations pane alone, standing wherever the URL says.
///
/// For the one thing about the sidebar that is not about the workbench: the
/// gear at the head of it reads as open while the settings are what is being
/// looked at, and the settings are not a page the workbench answers. Mounted on
/// a route that matches whatever it is handed, because what is being asked is
/// what the pane draws at a path rather than what any router does with one.
export function mountSidebar(at: string) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  const history = createMemoryHistory();
  history.set({ value: at });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route
            path="*"
            component={() => <Conversations selected="" open={() => {}} />}
          />
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
    client,
  };
}

/// The workbench on its own routes, so the Conversation it reads is the one the
/// URL names — and so that opening one is a navigation, which is what it is in
/// the app.
///
/// The client comes back with the render: `invalidateQueries()` on it is
/// exactly what a Nudge does to the page — see `lookAgain` in `src/nudge.ts` —
/// so a test about one needs no fake `EventSource`.
export function mount(at = "/") {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real page is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  const history = createMemoryHistory();
  history.set({ value: at });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/" component={Workbench} />
          {/* And the details panes nested under the Conversation, exactly as
              `App.tsx` nests them: the nesting is what keeps the middle pane up
              while the leaf under it changes, so a mount that flattened them
              would be testing a page the app does not build. */}
          <Route path="/conversations/:id" component={Workbench}>
            <Route path="/" />
            <Route path="/events/:event" />
            <Route path="/backlog" />
            <Route path="/roadmaps/:name" />
          </Route>
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
    client,
  };
}

/// The lists every pane of the workbench is drawn over, in whatever order a page
/// happens to ask for them.
export function theWorkbench(...answers: Parameters<typeof serving>) {
  return serving(
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever(`/api/ui/repos/${OPEN.repo.id}/branches`, json(BRANCHES)),
    // Every companion the fixture carries has a base dropdown of its own, over
    // its own repository's branches.
    ...OPEN.companions.map((companion) =>
      whenever(
        `/api/ui/repos/${companion.repo.id}/branches`,
        json(COMPANION_BRANCHES),
      ),
    ),
    whenever("/api/ui/abandoned-roadmaps", json([])),
    whenever(`/api/ui/conversations/${OPEN.id}`, json(OPEN)),
    ...answers,
  );
}

/// Wait for an element to be drawn, by selector.
///
/// Throwing rather than returning null when it is not there yet, because that is
/// what `waitFor` retries on — a callback that hands back null has answered, and
/// the wait ends on the first try with nothing.
export function drawn<T extends Element>(
  container: ParentNode,
  selector: string,
): Promise<T> {
  return waitFor(() => {
    const found = container.querySelector<T>(selector);
    if (!found) {
      throw new Error(`nothing matching ${selector} has been drawn`);
    }
    return found;
  });
}

/// The page reading everything it is showing again, exactly as a Nudge makes it:
/// `invalidateQueries()` with nothing named is the whole of what `lookAgain` in
/// `src/nudge.ts` does with one.
///
/// Awaited to the end of the reads it caused, so what a test looks at afterwards
/// is the page the second read left behind rather than the page mid-flight.
export async function nudged(client: QueryClient): Promise<void> {
  await client.invalidateQueries();
}

/// Every element matching `selector`, in the order the page has them — held onto
/// across a re-read so that what is there after can be compared with what was
/// there before.
///
/// Empty is a failure rather than an answer: a test that took hold of nothing
/// and found nothing afterwards would pass while saying nothing at all.
export function nodes<T extends Element>(
  container: ParentNode,
  selector: string,
): T[] {
  const found = [...container.querySelectorAll<T>(selector)];
  expect(
    found.length,
    `expected the page to be showing something matching ${selector}`,
  ).toBeGreaterThan(0);
  return found;
}

/// The same elements, still: the same count, in the same order, and each one the
/// very object it was.
///
/// Identity is the whole question. A rebuilt element is equal to the one it
/// replaced in everything a reader could name — same tag, same text, same
/// classes — and different in the one thing that matters: the state the browser
/// keeps on the node itself, which is a spinner mid-spin and a dropdown
/// mid-choice.
export function survived(before: Element[], after: Element[]): void {
  expect(after).toHaveLength(before.length);
  before.forEach((element, at) => {
    expect(
      after[at],
      `the element at ${at} was rebuilt rather than left alone`,
    ).toBe(element);
  });
}
