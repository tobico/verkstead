//! Mounting a Set the way it is really reached, for the tests that read what it
//! drew: the details pane of the Timeline Event it was asked on.
//!
//! Shared because three files' worth of assertions are about one drawing: the
//! record it makes of a Set, the Diff attached to it, and the table of contents
//! down its margin. One mount between them means all three are asking about the
//! pane the app really builds.
//!
//! The pane rather than the sheet, because the fetch is the pane's: a Set is
//! read under its own id, merged into what is already drawn, and read again
//! whenever a Nudge says the world moved. What the tests here drive is that
//! read.

import { cleanup, render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { expect, vi } from "vitest";

import type {
  AskView,
  QuestionSetEvent,
  SetView,
  UnreadableSet,
} from "../src/api/types";
import { Asked } from "../src/workbench/Asked";
import { json, reads, serving, whenever } from "./serving";

/// The Timeline row the pane is opened from. The pane reads one thing off it —
/// which Set to fetch — and fetches the rest, so the rest is what a row carries
/// and nothing any test here is about.
function row(id: string): QuestionSetEvent {
  return {
    id: 1,
    at: "2025-02-01T09:00:00Z",
    set_id: Number(id),
    title: "A question set",
    rows: [],
    standing: { Waiting: "waiting" },
  };
}

/// The pane over the Set the id names.
///
/// Settling a Set takes nobody anywhere: answering it or closing it unanswered
/// leaves the human in this pane, reading the same sheet back as the record of
/// what became of it. Which is what `back` is here for — the way out of the
/// pane, and a test's way of saying nothing took it.
export function mount(id = "1") {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real pane is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  const back = vi.fn();

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <Asked asked={row(id)} back={back} />
      </QueryClientProvider>
    )),
    back,
    // For the tests about a Nudge: `invalidateQueries()` on this is exactly
    // what one does to the pane — see `lookAgain` in `src/nudge.ts`.
    client,
  };
}

/// The pane once the Set it was asked for has arrived.
///
/// Whatever was drawn before it goes first, so that a test reading one Set
/// after another is reading one pane at a time: two of them in the document at
/// once are two `#preface`s, and an id that names two elements names neither.
export async function reading(set: SetView): Promise<HTMLElement> {
  cleanup();
  serving(json(reads(set)));
  const { container } = mount();
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());
  return container;
}

/// The same pane over a Set this build cannot read, which is the record rather
/// than the sheet — nothing to fill in and nothing to press.
export async function unreadably(set: UnreadableSet): Promise<HTMLElement> {
  cleanup();
  serving(json({ Unreadable: set }));
  const { container } = mount(String(set.id));
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());
  return container;
}

/// The same pane, kept hold of for the tests that fill it in: every read of the
/// Set is answered with the Set, and the answers given are what the submit or
/// the lock the pane goes on to make comes back with.
///
/// The reads are held to their own path rather than left in the order the
/// answers are handed out, because settling a Set does not take the reader
/// anywhere: the pane reads the Set again where it stands, and a sequence would
/// hand that read whatever the submit was answered with.
///
/// The fetch mock comes back with it, because what a sheet was filled in with is
/// read off the request it sent — and so does the way out of the pane, so a test
/// can say nobody was taken anywhere.
export async function answering(
  set: SetView,
  ...answers: Array<() => Promise<globalThis.Response>>
) {
  cleanup();

  /// How the Set reads back right now, which `settles` moves on: a test that
  /// answers a Set is a test whose next read of it is the record.
  let standing = set;

  const fetching = serving(
    whenever(`/api/ui/sets/${set.id}`, () => json(reads(standing))()),
    ...answers,
  );
  const { container, back } = mount(String(set.id));
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());

  return {
    page: container,
    fetching,
    back,
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

/// Everything the pane has *sent* — the submits and the locks, and never the
/// reads it makes around them.
///
/// The two are counted apart because a settled Set is read back where it stands:
/// the pane does not leave, so a submit is followed by the read that redraws the
/// sheet as the record. A test counting every call would be counting that too,
/// and would say "the Response was sent twice" when it was sent once.
export function posts(
  fetching: ReturnType<typeof serving>,
): Array<Parameters<typeof fetch>> {
  return fetching.mock.calls.filter(([, init]) => init?.method === "POST");
}

/// The body of the last thing the pane sent, as JSON — what it actually put on
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
