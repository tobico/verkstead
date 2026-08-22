//! Mounting the Set page the way it is really mounted, for the tests that read
//! what it drew.
//!
//! Shared because three files' worth of assertions are about one page: the
//! record it draws, the Diff on it, and the table of contents down its margin.
//! One mount between them means all three are asking about the page the app
//! really builds.

import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { cleanup, render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { expect } from "vitest";

import type { AskView, SetView } from "../src/api/types";
import { SetPage } from "../src/set/SetPage";
import { json, serving, whenever } from "./serving";

/// Where a Set leads back to: the Conversation it was asked from. Not this
/// page's subject, so it is a stand-in — what a test asks is where the way out
/// points, which is the link's `href`.
const Elsewhere = () => <p class="elsewhere" />;

/// The page on its own route, so the id it fetches is the one the URL names, and
/// inside a router, because the way back out of a Set is a link.
///
/// Settling a Set is not a navigation and never was one to this page: answering
/// it or closing it unanswered leaves the human where they are, reading the same
/// sheet back as the record of what became of it.
export function mount(id = "1") {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real page is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  const history = createMemoryHistory();
  history.set({ value: `/sets/${id}` });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/sets/:id" component={SetPage} />
          <Route path="/conversations/:id" component={Elsewhere} />
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
    // For the tests about a Nudge: `invalidateQueries()` on this is exactly
    // what one does to the page — see `lookAgain` in `src/nudge.ts`.
    client,
  };
}

/// The page once the Set it was asked for has arrived.
///
/// Whatever was drawn before it goes first, so that a test reading one Set
/// after another is reading one page at a time: two pages in the document at
/// once are two `#preface`s, and an id that names two elements names neither.
export async function reading(set: SetView): Promise<HTMLElement> {
  cleanup();
  serving(json(set));
  const { container } = mount();
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());
  return container;
}

/// The same page, kept hold of for the tests that fill it in: every read of the
/// Set is answered with the Set, and the answers given are what the submit or
/// the archive the page goes on to make comes back with.
///
/// The reads are held to their own path rather than left in the order the
/// answers are handed out, because settling a Set does not take the page
/// anywhere: it reads the Set again where it stands, and a sequence would hand
/// that read whatever the submit was answered with.
///
/// The fetch mock comes back with it, because what a sheet was filled in with is
/// read off the request it sent — and so does the history, so a test can say the
/// page stayed where it was.
export async function answering(
  set: SetView,
  ...answers: Array<() => Promise<globalThis.Response>>
) {
  cleanup();

  /// How the Set reads back right now, which `settles` moves on: a test that
  /// answers a Set is a test whose next read of it is the record.
  let standing = set;

  const fetching = serving(
    whenever(`/api/ui/sets/${set.id}`, () => json(standing)()),
    ...answers,
  );
  const { container, history } = mount(String(set.id));
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());

  return {
    page: container,
    fetching,
    history,
    settles: (into: SetView) => {
      standing = into;
    },
  };
}

/// The same Set, closing with a Postscript.
///
/// The markup is written here rather than taken from a fixture: the Set
/// `cargo test` writes those from is the one that closes with nothing, and what
/// its Postscript would be rendered into is asked of the server in
/// `ui_content.rs`. What is asked here is where the page puts it — so this is
/// the shape that renderer really emits, prose and a list with a code span in
/// it, and nothing about the rendering rides on it.
export const POSTSCRIPT =
  "<p>Worth taking up in the comment:</p>\n<ul>\n<li>whether <code>ops/export</code> gets an allowlist entry</li>\n</ul>\n";

export function withPostscript(set: SetView): SetView {
  return { ...set, postscript_html: POSTSCRIPT };
}

/// The same Set with `Q2` turned into a Heading — its own Options taken away,
/// leaving the Sub-questions it heads.
///
/// The flag travels with the Set rather than being worked out here, exactly as
/// it does on the wire: the server reads the shape once and the page takes its
/// word for it, so the two cannot come to different readings of one Set.
export function withHeading(set: SetView): SetView {
  return {
    ...set,
    questions: set.questions.map((question) =>
      question.ask.name === "Q2"
        ? { ...question, heading: true, ask: { ...question.ask, options: [] } }
        : question,
    ),
  };
}

/// The same Set with its Options declared as Answer Tables: axes on `Q1`, which
/// carries the Recommendation, and on the Sub-question `Q2a`, which carries
/// none — the two shapes the ★ column is drawn for and not.
///
/// The headers and the cells are written here rather than taken from a fixture,
/// as the Postscript's markup is: what a header and a cell are *rendered* into
/// is asked of the server in `ui_content.rs`, and what is asked here is what the
/// page draws them as. So this is the inline HTML that renderer really emits, a
/// code span among it, and nothing about the rendering rides on it.
export function withTable(set: SetView): SetView {
  const tabulated = (ask: AskView, columns: string[], rows: string[][]): AskView => ({
    ...ask,
    columns,
    options: ask.options.map((option, at) => ({ ...option, cells: rows[at]! })),
  });

  return {
    ...set,
    questions: set.questions.map((question) => {
      if (question.ask.name === "Q1") {
        return {
          ...question,
          ask: tabulated(
            question.ask,
            ["Latency", "<code>ops</code> cost"],
            [
              ["Sub-<code>ms</code>", "None"],
              ["A hop", "A box to run"],
            ],
          ),
        };
      }

      return {
        ...question,
        subquestions: question.subquestions.map((subquestion) =>
          subquestion.name === "Q2a"
            ? tabulated(subquestion, ["Precision"], [["Exact"], ["Rounded"]])
            : subquestion,
        ),
      };
    }),
  };
}

/// Everything the page has *sent* — the submits and the archives, and never the
/// reads it makes around them.
///
/// The two are counted apart because a settled Set is read back where it stands:
/// the page does not leave, so a submit is followed by the read that redraws the
/// sheet as the record. A test counting every call would be counting that too,
/// and would say "the Response was sent twice" when it was sent once.
export function posts(
  fetching: ReturnType<typeof serving>,
): Array<Parameters<typeof fetch>> {
  return fetching.mock.calls.filter(([, init]) => init?.method === "POST");
}

/// The body of the last thing the page sent, as JSON — what it actually put on
/// the wire, rather than what it was asked to.
export function sent(fetching: ReturnType<typeof serving>): unknown {
  const last = posts(fetching).at(-1);
  expect(last, "expected the page to have sent something").toBeTruthy();
  return JSON.parse(String(last![1]?.body));
}

/// Everything of `selector` in the page, as the text of each. Shared with the
/// lists' own mount, because reading a page's order back is the same act on
/// either.
export { texts } from "./listing";
