//! Listening for the Nudge: the word that something moved, and what it was
//! (ADR-0009).
//!
//! A Nudge names a kind and, where the change belongs to one, the Conversation
//! it happened in. It carries nothing else — no payload rides an event, and
//! what the page does about one is an ordinary read of the server.
//!
//! What each kind stands for is [`standsFor`] below, and it is the client's
//! table rather than the server's: the kinds are a vocabulary for what moved,
//! and which queries hold what moved is a fact about this cache alone. So
//! renaming a query key is not a server change.
//!
//! A kind this page has never heard of falls to the widest reaction there is —
//! read back everything on screen — which is what every kind used to get, and
//! is what makes a page safe against a server newer than itself.
//!
//! It arrives two ways, and they are not alternatives. The server's stream is
//! instant while the page is being looked at — it is held only then, see
//! [`overTheStream`] — but dies when iOS suspends the PWA; the service worker
//! relays every push it is woken by, which survives exactly that suspension but
//! needs notifications on and Apple's delivery to happen at all.
//! What the worker relays says nothing at all — a push is a rare, human-facing
//! moment, and reading everything back at one is no waste worth typing a kind
//! for.
//!
//! Nothing here has to land, and there is no poll underneath any more. What
//! stands behind a Nudge that never arrived is catch-up: the stream coming back
//! reads the world back whole, and so does the document becoming visible again.
//! Between them they cover the connection that drops, the browser with no
//! `EventSource` and no worker, and the server being restarted — and they cover
//! it at the moment the human looks, rather than ten seconds at a time.
//!
//! The one gap left is a stream that died silently on a page nobody touches: no
//! reconnect, no return to visibility, and nothing to notice it. The keep-alive
//! is what makes `EventSource` see most such deaths, and any focus change heals
//! the rest (ADR-0009).

import type { QueryClient, QueryKey } from "@tanstack/solid-query";

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
  const closers = [
    overTheStream(queries),
    throughTheWorker(queries),
    onComingBack(queries),
  ];

  return () => {
    for (const close of closers) {
      close();
    }
  };
}

/// Hold the server's stream open while the page is being looked at, looking
/// again at every Nudge down it.
///
/// The reconnect is a Nudge in itself, and the widest one there is. A stream
/// comes back from a suspended PWA or a restarted server knowing nothing about
/// what it missed, and it does not need to: what happened while it was dead is
/// read back off the server rather than replayed down the wire. That the first
/// open is not treated the same way is the one distinction drawn here — the
/// page has only just read the world it is opening this over.
///
/// **And it is let go of the moment the page is hidden.** A stream is a
/// connection held for as long as the page lives, and a browser gives one origin
/// six of them over HTTP/1.1 — so the sixth Verkstead left open in a tab is the
/// one that pins the last of them, and from there every read the page in front
/// of the human makes waits for a connection that nothing is going to give back.
/// A background tab has no use for the news anyway: [`onComingBack`] reads the
/// world back whole the moment it is looked at again, which is exactly what a
/// page that was not listening needs. So the stream belongs to the page being
/// read rather than to every page open, and the connection goes back when the
/// human looks away.
///
/// Which is also why coming back is not treated as a reconnect: the stream
/// opening again is this, not a connection that dropped, and the catch-up it
/// would ask for is the one [`onComingBack`] is already making.
function overTheStream(queries: QueryClient): () => void {
  // Absent in a browser without server-sent events, which loses the fast path
  // and nothing else: coming back to the page is still a read of everything.
  if (typeof EventSource === "undefined") {
    return () => {};
  }

  let stream: EventSource | undefined;
  let established = false;

  const listen = () => {
    if (stream) {
      return;
    }

    const opened = new EventSource(STREAM);
    stream = opened;

    opened.addEventListener("open", () => {
      if (established) {
        lookAgain(queries);
      }
      established = true;
    });

    // Named, so that whatever else may one day come down this stream is not
    // mistaken for a Nudge by a page too old to know about it.
    opened.addEventListener("nudge", (event) =>
      lookAgainAt(queries, whatMoved((event as MessageEvent<unknown>).data)),
    );
  };

  /// Give the connection back. `established` goes with it, so that the stream
  /// opening for the page being looked at again is the first open it is rather
  /// than a reconnect — see above.
  const letGo = () => {
    stream?.close();
    stream = undefined;
    established = false;
  };

  const following = () => {
    if (document.visibilityState === "visible") {
      listen();
    } else {
      letGo();
    }
  };

  // Listened for before [`onComingBack`]'s, so a page coming back is listening
  // again before it reads: a Nudge landing between the two would otherwise be
  // one nothing heard and the read it followed had already been made.
  document.addEventListener("visibilitychange", following);
  following();

  return () => {
    document.removeEventListener("visibilitychange", following);
    letGo();
  };
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

/// Read back what the Nudge was about, and nothing else.
function lookAgainAt(queries: QueryClient, moved: Nudge | null): void {
  const reading = moved && standsFor(moved);

  if (!reading) {
    lookAgain(queries);
    return;
  }

  for (const key of reading) {
    // Active queries only, which is the default: what is not on screen is left
    // to be refetched when something mounts it.
    void queries.invalidateQueries({ queryKey: key });
  }
}

/// Which queries a kind of Nudge is about, or `null` for a kind this page does
/// not know — which is every kind a newer server has and this one does not.
///
/// Prefixes, because that is how a query key matches: `["transcript", 4]` names
/// every Transcript of Conversation 4 whichever session Event it belongs to,
/// and a Conversation the page is not showing has no active query to invalidate
/// at all. The Conversation's own key is the id as a string, that being what
/// the URL the query is keyed off carries.
///
/// A key named here that is frozen (`freshness: "static"`) is a no-op and is
/// still named: this table says what a kind is *about*, and what a re-read costs
/// is the query's own business (see `freshness.ts`).
function standsFor(moved: Nudge): readonly QueryKey[] | null {
  switch (moved.kind) {
    // Lines on a Transcript, and nothing else — this is the kind that arrives
    // twice a second while a session talks, so it is the one whose narrowness
    // the whole exercise was for. The Timeline row's own summary line moves
    // under `conversation`, announced separately by the half of the write that
    // moves it.
    case "transcript":
      return [["transcript", moved.conversation]];

    // A session printed: the Capture is what it printed, the Screen is the
    // terminal at the other end of it, and the same write moves the line the
    // Timeline row reads.
    case "screen":
      return [
        ["capture", moved.conversation],
        ["screen", moved.conversation],
        ["conversation", String(moved.conversation)],
      ];

    // A commit landed: a new Event on the Timeline, its diff, and the pull
    // request it was pushed onto — which is the one query here that costs a
    // GitHub API call, and so the one that must be read on this and nothing
    // else.
    case "commit":
      return [
        ["commit", moved.conversation],
        ["pull-request", moved.conversation],
        ["conversation", String(moved.conversation)],
      ];

    // A Set arrived, was answered, or was closed: the Set itself, the Timeline
    // Event standing for it, and the sidebar row, whose *waiting on you* mark
    // is exactly this.
    //
    // Every Set rather than one, because a Set is keyed by its own id and a
    // Nudge says which Conversation rather than which Set. Both places a Set is
    // drawn are one open document, so this is one read at most.
    case "set":
      return [
        ["set"],
        ["conversation", String(moved.conversation)],
        ["conversations"],
      ];

    // The same two places, and not the sidebar: whether an agent is listening
    // is a badge on a Set, and nobody is newly waiting on the human because of
    // it. This is the kind that used to be carried by the poll.
    case "liveness":
      return [["set"], ["conversation", String(moved.conversation)]];

    // The Conversation everywhere it is drawn, its sidebar row included: a
    // lifecycle that moved is a row that reads differently.
    case "conversation":
      return [["conversation", String(moved.conversation)], ["conversations"]];

    case "conversations":
      return [["conversations"]];

    // The roadmaps nothing is driving are read off the Repos every time they
    // are drawn, so they move when the Repos do.
    case "repos":
      return [["repos"], ["abandoned-roadmaps"]];

    case "profiles":
      return [["profiles"]];

    default:
      return null;
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

/// Read the world back whole every time the human comes back to it.
///
/// The other half of the catch-up the reconnect does, and the half that covers
/// the suspension a reconnect never happens after: an iOS PWA reopened, a phone
/// unlocked, a tab returned to. Whatever moved while the page was away arrived
/// on a stream nobody was reading, so what it missed is unknowable and
/// everything is what it reads.
///
/// Beside `refetchOnWindowFocus`, which the client has on, rather than instead
/// of it: focus is a window's and visibility is the document's, and the PWA
/// coming back from a suspend is reliably only the second.
function onComingBack(queries: QueryClient): () => void {
  const returned = () => {
    if (document.visibilityState === "visible") {
      lookAgain(queries);
    }
  };

  document.addEventListener("visibilitychange", returned);

  return () => document.removeEventListener("visibilitychange", returned);
}

/// Read back everything the app is showing.
///
/// Every active query at once. What is not on screen is left to be refetched
/// when something mounts it.
function lookAgain(queries: QueryClient): void {
  void queries.invalidateQueries();
}
