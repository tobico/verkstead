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
  const asked: Array<(init?: RequestInit) => Promise<Response>> = [];
  const held = new Map<string, (init?: RequestInit) => Promise<Response>>();
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
    // Handed what the page put on the wire, for the answer that has to behave
    // like the network rather than like a value — see [`hangs`], which needs
    // the signal to know when the caller gave up.
    return answer ? answer(init) : asked[Math.min(taken++, asked.length - 1)]!(init);
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
export type Answer =
  | ((init?: RequestInit) => Promise<Response>)
  | { key: string; answer: (init?: RequestInit) => Promise<Response> };

/// One answer for one request, however often and whenever it is made. For the
/// endpoint a page fetches alongside the one a test is about.
///
/// The method is part of what it belongs to, because a path is not: one path
/// answers to a GET and a POST both — the workbench's Conversations do — and an
/// answer held for the reading of a list must not also be what the server said
/// about writing to it.
export function whenever(
  path: string,
  answer: (init?: RequestInit) => Promise<Response>,
  method = "GET",
): Answer {
  return { key: `${method} ${path}`, answer };
}

/// A request that never answers, until whoever made it gives up on it.
///
/// The hang, which is a thing a network does and a mocked value cannot: the
/// promise settles only when the caller's own signal aborts, and it settles the
/// way `fetch` does — rejecting with whatever the signal gave as its reason. A
/// read with no deadline handed one of these waits for the length of the test,
/// which is the point.
export function hangs(): (init?: RequestInit) => Promise<Response> {
  return (init) =>
    new Promise((_, reject) => {
      const signal = init?.signal;

      if (!signal) {
        return;
      }

      if (signal.aborted) {
        reject(signal.reason as Error);
        return;
      }

      signal.addEventListener("abort", () => reject(signal.reason as Error));
    });
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
