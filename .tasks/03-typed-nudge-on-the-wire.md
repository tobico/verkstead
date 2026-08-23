# 03. Typed Nudge on the wire

## What to build

Give the Nudge its kind and scope, end to end, without yet changing what the
viewer does with it.

Server side: a Nudge enum in the schema crate — a kind, plus the Conversation
it belongs to where one does — serialized as the SSE event's data and typed
for the viewer through ts-rs like every other wire shape. Announcing without a
kind must no longer be possible: every announce site passes one, which means
classifying all of them (transcript batches, screen changes, commit sweep,
session start/end, sets, settling, interruptions, manual tasks, continuing,
wrapping, grillings, conversation changes). Kinds follow the taxonomy in
ADR-0009: `transcript`, `screen`, `commit`, `set`, `conversation` scoped to a
Conversation; `conversations`, `repos`, `profiles` global. (The `liveness`
kind is task 04's, arriving with the poll's retirement.) Nudges stay
notify-only — kind and scope, never content.

Client side: parse the typed event, and treat every kind by the fallback that
will remain the safety net forever after — invalidate everything. Behaviour
is unchanged by design; what this slice proves is the typed event flowing
from every announce site through SSE into a parsed structure, and the
unknown-kind path being the one path. The push relay stays contentless and
untouched.

The two Nudge channels merged into the SSE stream (the broadcast channel and
the store's settlements channel) must both come out typed.

## Acceptance criteria

- [ ] Announcing without a kind does not compile; all announce sites classify
      per the ADR-0009 taxonomy, conversation-scoped where the change belongs
      to one
- [ ] The SSE event carries the typed JSON; the TypeScript type is
      ts-rs-generated and a golden fixture covers the wire shape
- [ ] The viewer parses the event and falls back to invalidate-everything for
      every kind, unknown kinds included — observed behaviour is unchanged
- [ ] Rust and vitest suites pass; the push relay is untouched
