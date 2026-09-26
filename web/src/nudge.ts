//! Listening for the Nudge: the word that something moved, and what it was
//! (ADR-0009).
//!
//! A Nudge names a kind, the Conversation it happened in where the change
//! belongs to one, and the device where the news is a member's rather than this
//! one's (see below). It carries nothing else — no payload rides an event, and
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
//! **And a member's news arrives on it too.** A Conversation of a member's is
//! read through the device the browser opened (ADR-0020, *The opened device
//! relays*), so its news comes the same way rather than over a second
//! connection: that device holds one Nudge stream to each of its members and
//! announces what comes down one on this stream, under the Device Id it came
//! from — see `relaying::freshness` on the other side of the wire. So a Nudge
//! says *whose* it is, and every key below is built for that device: a Set
//! answered on a member invalidates that member's queries and nothing of this
//! device's, ids colliding by construction (see `reaching.ts`). A Nudge with no
//! device is this device's own and means exactly what it always meant.
//!
//! **And a member's stream that was away says `everything` of that device.** The
//! reconnect's own reaction, aimed at one device: what the hub missed while a
//! member was off is unknowable, so a page drawing that member reads back
//! whatever of it is on screen — while a page drawing this device's work, or
//! another member's, reads nothing back for it.
//!
//! The one gap left is a stream that died silently on a page nobody touches: no
//! reconnect, no return to visibility, and nothing to notice it. The keep-alive
//! is what makes `EventSource` see most such deaths, and any focus change heals
//! the rest (ADR-0009).

import type { QueryClient, QueryKey } from "@tanstack/solid-query";

import type { Nudge, Nudged } from "./api/types";
import { keyOf, type Device } from "./reaching";

/// The server's stream — see the `nudge` module on the other side of it.
const STREAM = "/api/ui/nudges";

/// What the service worker calls a Nudge when it posts one — see `assets/sw.js`,
/// which cannot share this constant: a worker is a static file served from the
/// site root rather than a module of this bundle.
const RELAYED = "nudge";

/// Every pane following the disk for itself, and which Conversation each of
/// them is drawn on — see [`whenFilesMove`].
const following = new Set<{
  device: Device;
  conversation: number;
  look: () => void;
}>();

/// Hear every `files` Nudge for one Conversation, for as long as the returned
/// closer is not called.
///
/// **The one seam beside the table.** What a Nudge comes to is nearly always an
/// invalidation — [`standsFor`] names the queries a kind is about and the cache
/// reads them back — and the Code pane's tree is the thing on the page no query
/// key reaches: its folder listings are held above the pane with the tabs and
/// the unsaved text, so that a swap to an Event and back does not throw away
/// the walk down to a file (see `workbench/keeping.ts`). So the pane takes out
/// a subscription of its own while it is drawn, rather than the tree being
/// rewritten as queries — and what it does with the news is its own business:
/// re-read the folders it has expanded, and the files it has open, where a
/// version that has moved is a clean editor taking the new text and a dirty one
/// raising its bar.
///
/// Two subscribers on the one pane rather than one, the tree's and the tabs':
/// what each does with the news is its own, and neither is the other's to know
/// about.
///
/// **And the widest reaction reaches it too.** A page that cannot say what it
/// missed reads back everything it is showing (see [`lookAgain`]), and a tree is
/// something it is showing — so a reconnect, a relayed push and the document
/// becoming visible again tell every subscriber, whichever Conversation it is
/// on.
export function whenFilesMove(
  device: Device,
  conversation: number,
  look: () => void,
): () => void {
  const listener = { device, conversation, look };

  following.add(listener);

  return () => {
    following.delete(listener);
  };
}

/// Tell them: one Conversation's, every Conversation of one device's, or — where
/// the page cannot say what moved — all of them.
///
/// One Conversation is a device and an id together, ids being each device's own
/// and colliding by construction (see `reaching.ts`): a member's Conversation 4
/// moving is no news at all for this device's Conversation 4. A `conversation` of
/// `null` is every Conversation of that one device, which is what a member's
/// stream coming back says.
///
/// Over a copy, because what a subscriber does about the news is its own and
/// may be to stop listening.
function tell(
  moved: { device: Device; conversation: number | null } | null,
): void {
  for (const listener of [...following]) {
    if (
      moved === null ||
      (listener.device === moved.device &&
        (moved.conversation === null ||
          listener.conversation === moved.conversation))
    ) {
      listener.look();
    }
  }
}

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
    opened.addEventListener("nudge", (event) => {
      const moved = whatMoved((event as MessageEvent<unknown>).data);

      // Whose news it is comes off the Nudge itself: a member's carries the
      // Device Id this device heard it from, and this device's own carries
      // none — which is what `null` says here, and what every local Nudge has
      // always been.
      lookAgainAt(queries, moved?.device ?? null, moved);
    });
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
function whatMoved(data: unknown): Nudged | null {
  if (typeof data !== "string") {
    return null;
  }

  try {
    return JSON.parse(data) as Nudged;
  } catch {
    return null;
  }
}

/// Read back what the Nudge was about, and nothing else.
///
/// `device` is whose news it is — read off the Nudge by the caller, `null` for
/// this device's own — and it is what every key the table below names is built
/// for.
function lookAgainAt(
  queries: QueryClient,
  device: Device,
  moved: Nudged | null,
): void {
  const reading = moved && standsFor(device, moved);

  if (!reading) {
    lookAgain(queries);
    return;
  }

  for (const key of reading) {
    // Active queries only, which is the default: what is not on screen is left
    // to be refetched when something mounts it.
    void queries.invalidateQueries({ queryKey: key });
  }

  // And the pane that follows the disk for itself, where this is the kind it is
  // following: the queries above are its tree's roots, and what it holds beside
  // them is a walk no key names — see [`whenFilesMove`].
  if (moved.kind === "files") {
    tell({ device, conversation: moved.conversation });
  }

  // And every one of them on that device, where the news is that everything of
  // it moved: a member's stream that has just been taken up cannot say which
  // Conversation, so a pane following the disk on any of that device's reads
  // back exactly as it does on a reconnect of this page's own stream.
  if (moved.kind === "everything") {
    tell({ device, conversation: null });
  }
}

/// Which queries a kind of Nudge is about, or `null` for a kind this page does
/// not know — which is every kind a newer server has and this one does not.
///
/// Every key is built for the device the news is about — see `keyOf`, and the
/// reasoning there: ids collide by construction, so a table naming bare ids
/// would read a member's Set over this device's Conversation of the same
/// number. A key of this device's own is the key it has always been.
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
function standsFor(device: Device, moved: Nudge): readonly QueryKey[] | null {
  switch (moved.kind) {
    // Lines on a Transcript, and nothing else — this is the kind that arrives
    // twice a second while a session talks, so it is the one whose narrowness
    // the whole exercise was for. The Timeline row's own summary line moves
    // under `conversation`, announced separately by the half of the write that
    // moves it.
    case "transcript":
      return [keyOf(device, "transcript", moved.conversation)];

    // A session printed: the Capture is what it printed, the Screen is the
    // terminal at the other end of it, and the same write moves the line the
    // Timeline row reads.
    case "screen":
      return [
        keyOf(device, "capture", moved.conversation),
        keyOf(device, "screen", moved.conversation),
        keyOf(device, "conversation", String(moved.conversation)),
      ];

    // A commit landed: a new Event on the Timeline, its diff, and the pull
    // request it was pushed onto — which is the one query here that costs a
    // GitHub API call, and so the one that must be read on this and nothing
    // else.
    //
    // And the Code pane's marks, which is the one reading on this wire that two
    // kinds both stand for: a commit clears every mark in a Worktree without
    // touching a file, so nothing the `files` kind is about has moved and every
    // mark has changed (ADR 0019, *The tree*).
    case "commit":
      return [
        keyOf(device, "commit", moved.conversation),
        keyOf(device, "pull-request", moved.conversation),
        keyOf(device, "conversation", String(moved.conversation)),
        keyOf(device, "file-status", moved.conversation),
      ];

    // The Worktrees moved: something wrote, made, or took a file away, and the
    // Code pane's watcher said so (ADR 0019, *Following the disk*).
    //
    // The tree's roots — a root that appeared or went, in a reading that costs
    // a row apiece and disturbs nothing that is open — and the marks its rows
    // carry, which is git's account of the lot of them read again. The two
    // reads of this kind a query key can name. What is *inside* a root is not a
    // query at all: the folders the tree has expanded and what each open file
    // was read as are held above the pane together, and what re-reads those is
    // the pane's own subscription beside this table — see [`whenFilesMove`],
    // which this kind tells as well.
    //
    // Nothing above the pane is here on purpose. A file written in a Worktree
    // moves no record at all: there is no Event for it, no Timeline row and
    // nothing in the sidebar, and a commit that *would* move those is the
    // `commit` kind above. This is the one kind that is only ever about a pane
    // that happens to be open.
    case "files":
      return [
        keyOf(device, "file-roots", moved.conversation),
        keyOf(device, "file-status", moved.conversation),
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
        keyOf(device, "set"),
        keyOf(device, "conversation", String(moved.conversation)),
        ["conversations"],
      ];

    // The same two places, and not the sidebar: whether an agent is listening
    // is a badge on a Set, and nobody is newly waiting on the human because of
    // it. This is the kind that used to be carried by the poll.
    case "liveness":
      return [
        keyOf(device, "set"),
        keyOf(device, "conversation", String(moved.conversation)),
      ];

    // The Conversation everywhere it is drawn, its sidebar row included: a
    // lifecycle that moved is a row that reads differently.
    case "conversation":
      return [
        keyOf(device, "conversation", String(moved.conversation)),
        ["conversations"],
      ];

    // The sidebar's own list, which is this device's work whichever member's
    // news arrived — the merged list is a later stage. Unkeyed for that reason,
    // as it is in the three kinds above.
    case "conversations":
      return [["conversations"]];

    // The roadmaps nothing is driving are read off the Repos every time they
    // are drawn, so they move when the Repos do.
    case "repos":
      return [
        keyOf(device, "repos"),
        keyOf(device, "abandoned-roadmaps"),
      ];

    // The joins in flight: one was asked of this device, or one it was holding
    // was settled or ran out.
    //
    // The list the modal is drawn from, which is what raises it and what takes
    // it down — a device asking is a request that appeared, a press on another
    // workbench is one that is gone, and the ten minutes running out is the
    // same. And the Devices list beside it, because an Allow has just written a
    // member into it: the section on the Remote access pane is where the device
    // that was let in shows up, and nothing else would say so.
    case "joins":
      return [["joins"], ["devices"]];

    // And the cluster moving with no join here to have moved it: a member of it
    // named a device this one had not heard of, and it is a member now. Nobody
    // over here pressed anything, so the Devices section is the whole of what
    // this says — there is a row in it that was not there a moment ago.
    case "devices":
      return [["devices"]];

    // And what is *out there* moving, which is a different list and a different
    // question: a device nobody has typed an address for was heard advertising
    // itself, or one that had been heard stopped. The cluster has not changed for
    // it — the rows the pane drew of the membership are not re-read, which is the
    // whole reason the Discovered list is a read of its own — and the browse
    // behind it is held open by this read rather than by anything the page says.
    case "discovered":
      return [["discovered"]];

    case "profiles":
      return [keyOf(device, "profiles")];

    // And everything of one device's, which is what the stream to a member says
    // the moment it is taken up again: it knows nothing about what it missed, so
    // what is read back is whatever of that device is on screen.
    //
    // The device's own prefix is the whole of that reading, and is why the device
    // leads a key at all — see `keyOf`. For this device itself the prefix is the
    // empty one, which matches every query there is: the widest reading of the
    // widest kind, and the same thing a page that cannot say what it missed does.
    case "everything":
      return [keyOf(device)];

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

  // And every subscriber with them, whichever Conversation it is on: this is
  // the reaction of a page that cannot say what it missed, and what a pane
  // holds outside the cache went stale with everything in it.
  tell(null);
}
