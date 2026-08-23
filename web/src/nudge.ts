//! Listening for the Nudge: the word that something moved, and what it was
//! (ADR-0009).
//!
//! A Nudge names a kind and, where the change belongs to one, the Conversation
//! it happened in. It carries nothing else — no payload rides an event, and
//! what the page does about one is an ordinary read of the server.
//!
//! There is still one reaction and it is the widest there is: look again at
//! everything the page is showing. Reading a kind narrowly is what the kinds
//! are *for* and is not written yet; until it is, every kind takes the path an
//! unrecognised kind will always take, which is also what makes a page safe
//! against a server newer than itself.
//!
//! It arrives two ways, and they are not alternatives. The server's stream is
//! instant while the page is alive but dies when iOS suspends the PWA; the
//! service worker relays every push it is woken by, which survives exactly that
//! suspension but needs notifications on and Apple's delivery to happen at all.
//! What the worker relays says nothing at all — a push is a rare, human-facing
//! moment, and reading everything back at one is no waste worth typing a kind
//! for.
//!
//! Nothing here has to work, either. Every Nudge is latency saved off the
//! ten-second poll underneath, and a page that gets none of them stays correct
//! at the poll's pace — which is what makes it safe for this to be a connection
//! that drops, a browser that has no `EventSource` or no worker, or a server
//! that is being restarted.

import type { QueryClient } from "@tanstack/solid-query";

import type { Nudge } from "./api/types";

/// The server's stream — see the `nudge` module on the other side of it.
const STREAM = "/api/ui/nudges";

/// What the service worker calls a Nudge when it posts one — see `assets/sw.js`,
/// which cannot share this constant: a worker is a static file served from the
/// site root rather than a module of this bundle.
const RELAYED = "nudge";

/// Listen on both channels, looking again at every Nudge either brings, until
/// the returned closer is called.
export function listenForNudges(queries: QueryClient): () => void {
  const closers = [overTheStream(queries), throughTheWorker(queries)];

  return () => {
    for (const close of closers) {
      close();
    }
  };
}

/// Hold the server's stream open, looking again at every Nudge down it.
///
/// The reconnect is a Nudge in itself. A stream comes back from a suspended
/// PWA or a restarted server knowing nothing about what it missed, and it does
/// not need to: what happened while it was dead is read back off the server
/// rather than replayed down the wire. That the first open is not treated the
/// same way is the one distinction drawn here — the page has only just read the
/// world it is opening this over.
function overTheStream(queries: QueryClient): () => void {
  // Absent in a browser without server-sent events, which loses the fast path
  // and nothing else: the poll is still there and still enough.
  if (typeof EventSource === "undefined") {
    return () => {};
  }

  const stream = new EventSource(STREAM);

  let established = false;
  stream.addEventListener("open", () => {
    if (established) {
      lookAgain(queries);
    }
    established = true;
  });

  // Named, so that whatever else may one day come down this stream is not
  // mistaken for a Nudge by a page too old to know about it.
  stream.addEventListener("nudge", (event) =>
    lookAgainAt(queries, whatMoved((event as MessageEvent<unknown>).data)),
  );

  return () => stream.close();
}

/// What the server said moved, or `null` where this page could not make a Nudge
/// of it.
///
/// Unreadable and unrecognised come to the same thing here and are meant to:
/// both are a page that does not know what happened, and what a page that does
/// not know does is read everything back.
function whatMoved(data: unknown): Nudge | null {
  if (typeof data !== "string") {
    return null;
  }

  try {
    return JSON.parse(data) as Nudge;
  } catch {
    return null;
  }
}

/// Read back what the Nudge was about.
///
/// One case, and it is the one an unrecognised kind takes: everything. Cases
/// that read back less are what this is here to grow — and they belong here
/// rather than on the server, so that renaming a query key stays the viewer's
/// own business (ADR-0009).
function lookAgainAt(queries: QueryClient, moved: Nudge | null): void {
  switch (moved?.kind) {
    default:
      lookAgain(queries);
  }
}

/// Hear out the service worker, which posts a Nudge to every open window on
/// every push it is delivered.
///
/// A push carries a Set's title and id, and none of that is read: what the page
/// does with the news is the same either way, and a Nudge that said which Set
/// would be a second thing to keep true.
function throughTheWorker(queries: QueryClient): () => void {
  // Absent in a browser with no service workers, and in any browser outside a
  // secure context — the same shrug the registration makes.
  const container = navigator.serviceWorker;
  if (!container) {
    return () => {};
  }

  const relayed = (event: MessageEvent) => {
    // Named, because a page is posted to by whatever has its window: anything
    // that does not say it is a Nudge is not treated as one.
    if ((event.data as { verkstead?: string } | null)?.verkstead === RELAYED) {
      lookAgain(queries);
    }
  };

  container.addEventListener("message", relayed);

  return () => container.removeEventListener("message", relayed);
}

/// Read back everything the app is showing.
///
/// Every active query at once. What is not on screen is left to be refetched
/// when something mounts it.
function lookAgain(queries: QueryClient): void {
  void queries.invalidateQueries();
}
