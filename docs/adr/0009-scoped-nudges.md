# Scoped, notify-only Nudges; the viewer poll retired

Amends [ADR-0005](0005-layered-viewer-freshness.md).

The contentless Nudge stopped scaling the day the workbench went live. ADR-0005
said the server would broadcast only durable Set changes; in practice eighteen
call sites announce, including every 500 ms batch of session output — and the
viewer's whole reaction was an unfiltered `invalidateQueries()`. While a
session talked, every active query refetched roughly twice a second: whole
transcripts re-downloaded, and the `gh`-backed pull-request query hit GitHub's
API at the same cadence. The proposed cure — replace it all with a WebSocket —
aimed at the wrong layer: SSE already streams; the waste was the reaction.

A Nudge now carries a kind and, where one applies, the Conversation it belongs
to — a Rust enum in the schema crate, ts-rs-typed like every other wire shape —
and the client maps kinds to query keys in one table in `nudge.ts`, so the
server never learns the viewer's cache layout. Nudges stay **notify-only**:
data rides no event, and HTTP remains the single path for content, rendering
and types. The wide announce set is blessed rather than pruned — the workbench
genuinely wants push for transcript, screen, commit and liveness movement, so
the durable-changes-only rule of ADR-0005 is overturned along with
contentlessness. The transcript endpoint grows an `?after=<cursor>` form so the
one query that must refetch on every batch reads only the new turns.

The 10-second viewer poll is retired. Its backstop duty passes to
invalidate-everything at three moments — every SSE (re)connect, every return to
visibility, and any Nudge kind the page does not recognise — with
`refetchOnWindowFocus` staying on. The push relay keeps the degenerate
everywhere-scoped Nudge: a push is a rare, human-facing moment, and refetching
everything then is fine. Liveness, which the poll used to carry, becomes a
Nudge kind of its own.

## Considered Options

- **Replace SSE with a WebSocket** — the original request. Buys nothing for a
  one-way signal SSE already delivers, re-solves reconnect and iOS suspension,
  and leaves the refetch-everything reaction untouched.
- **Keep contentless, debounce or add per-query opt-outs** — far less code,
  but every Nudge still refetches everything (or the opt-out list grows into
  an untyped shadow of the event taxonomy).
- **Data-carrying events** — no fetch round-trip at all, but every payload
  then exists in two delivery paths, and ordering and reconnect gaps become
  correctness problems instead of one cheap refetch.
- **Query keys on the wire** — the dumbest client, but the server would own
  the viewer's cache layout and every frontend query rename becomes a server
  change.
- **Keep the poll** — ADR-0005's own backstop, but with reconnect and
  visibility catch-up in place it duplicated the stream it backstopped.

## Consequences

- Eighteen announce sites get classified into kinds (`transcript`, `screen`,
  `commit`, `set`, `conversation` scoped to a Conversation; `conversations`,
  `repos`, `profiles` global; `liveness` new). Announcing without a kind is no
  longer possible.
- The reconcile-or-static rule of ADR-0005 stands and becomes unskippable: a
  project-owned query wrapper forces every caller to name a reconcile key or
  declare `static`, and a lint wall blocks raw `useQuery` around it. The rule
  was written down once and still missed on seven of eleven queries; review
  demonstrably does not hold it.
- A silently dead SSE connection on a foregrounded, untouched page is the one
  gap the poll covered that catch-up does not. Accepted: the keep-alive makes
  EventSource notice most deaths, and any focus change or reconnect heals it
  wholesale.
- An old page against a newer server degrades gracefully: unknown kinds mean
  refetch everything, which is exactly the old behaviour.
- The Capture still refetches whole for a running stub session; incremental
  reads stop at the Transcript until that pane hurts.
