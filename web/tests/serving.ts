//! A stand-in for the server, shared by the component tests.
//!
//! Every payload a test serves through this comes out of `tests/fixtures/`,
//! which `cargo test` writes from the real endpoints — so what a component is
//! fed here is what the server actually said.

import { vi } from "vitest";

import type { SetReading, SetView, UnreadableSet } from "../src/api/types";

/// One answer per fetch in the order given. The last answer is repeated,
/// because a page polls for as long as it is open and a test should not have to
/// say how many times.
///
/// An answer handed to [`whenever`] is held out of that order and belongs to the
/// one path it names: a page with two things to fetch has no fixed order between
/// them, and a test about one of them should not have to say when the other went
/// out.
export function serving(...answers: Array<Answer>) {
  const asked: Array<() => Promise<Response>> = [];
  const held = new Map<string, () => Promise<Response>>();
  for (const answer of answers) {
    if (typeof answer === "function") {
      asked.push(answer);
    } else {
      held.set(answer.key, answer.answer);
    }
  }

  let taken = 0;
  // Typed as `fetch` is called rather than as a bare thunk, so a test can read
  // back what a page put on the wire and not just that it asked.
  const fetching = vi.fn((path: RequestInfo | URL, init?: RequestInit) => {
    const answer = held.get(`${init?.method ?? "GET"} ${String(path)}`);
    return answer ? answer() : asked[Math.min(taken++, asked.length - 1)]!();
  });
  vi.stubGlobal("fetch", fetching);
  return fetching;
}

/// How many times a page asked for one path.
///
/// What a test counting reads means is the reads it is about, and a page fetches
/// more than one thing: the banner asks on its own hour-long query, off a query
/// client the whole file shares, so which test in a file pays for that request
/// is not something any of them should be counting.
export function askedFor(
  fetching: ReturnType<typeof serving>,
  path: string,
): number {
  return fetching.mock.calls.filter(([asked]) => String(asked) === path).length;
}

/// What a test hands [`serving`]: an answer in the sequence, or one belonging to
/// a request.
type Answer =
  | (() => Promise<Response>)
  | { key: string; answer: () => Promise<Response> };

/// One answer for one request, however often and whenever it is made. For the
/// endpoint a page fetches alongside the one a test is about.
///
/// The method is part of what it belongs to, because a path is not: one path
/// answers to a GET and a POST both — the workbench's Conversations do — and an
/// answer held for the reading of a list must not also be what the server said
/// about writing to it.
export function whenever(
  path: string,
  answer: () => Promise<Response>,
  method = "GET",
): Answer {
  return { key: `${method} ${path}`, answer };
}

/// One answer, as the server would have written it.
export function json(body: unknown, status = 200): () => Promise<Response> {
  return () =>
    Promise.resolve(
      new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
      }),
    );
}

/// The Set inside a fixture of `/api/ui/sets/{id}`.
///
/// That endpoint says first which of the two kinds of reading it is holding —
/// the Set where this build can read the stored body, and the record itself
/// where it cannot — so the fixtures do too. A test about a Set to read or fill
/// in takes the Set out of the reading here, once, and goes on asking about the
/// Set.
export function readable(fixture: unknown): SetView {
  const reading = fixture as SetReading;

  if (!("Set" in reading)) {
    throw new Error("expected a fixture of a Set this build can read");
  }

  return reading.Set;
}

/// And the record inside a fixture of one it could not read.
export function unreadable(fixture: unknown): UnreadableSet {
  const reading = fixture as SetReading;

  if (!("Unreadable" in reading)) {
    throw new Error("expected a fixture of a Set this build cannot read");
  }

  return reading.Unreadable;
}

/// A Set on its way back out of that endpoint, which is how a test serves one:
/// the reading is what the page reads, and the Set is what the test is about.
export function reads(set: SetView): SetReading {
  return { Set: set };
}
