# 04. The relay

## Goal

A member's whole workbench is reachable through any other member. Open
`/devices/{device}/conversations/{id}` on A for a Conversation that lives on
B, and everything works as if on B — the Timeline, answering a Set, the
terminal and Code pane with their sockets, an attachment upload, the Repo
dropdown's Open and Create — with A's cookie and nothing else in the browser.
The hub holds one nudge stream to each member and re-announces every member
nudge locally under the device, so the page stays fresh. Demonstrable end to
end: grill a Conversation on B, drive it from A's browser to Done.

## Decisions in force

- **The opened device relays; the client stays same-origin**
  ([ADR-0020](../../adr/0020-cluster-mode.md), *The opened device relays*).
  Teaching the client N origins was rejected: CORS and cross-origin cookies
  on every device, every device served to the phone, a cache keyed by server.
- **A member serves the whole `/api/ui/` namespace over the peer listener**,
  authenticated by the handshake rather than the key cookie. A member's
  workbench key never leaves it.
- **Verbatim forwarding** under `/api/ui/devices/{device}/…`: method, path,
  body, headers that matter, and the answer untouched — including the SSE
  nudge stream, the two websockets (terminal attach, file watchers) and
  multipart uploads. Everything works from this stage; a reading-first cut
  was rejected so a remote Conversation is never a lesser one.
- **Routes**: `/devices/{device}/conversations/{id}` and every nested leaf
  under it; local URLs keep their shape. Query keys and `pathOf` learn an
  optional device.
- **The hub's member streams**: one nudge stream per member, reconnecting,
  every member nudge re-announced locally as the same kind under the device,
  so `standsFor` invalidates the device-keyed queries.
- **Unreachable** reads on the page the same way a dropped nudge stream does
  today, and presses are refused by name.

## Proposed tasks (provisional)

1. **The peer-side API** — the `/api/ui/` router mounted on the peer listener
   behind the member verifier instead of the key gate.
   - A member's call answers; a non-member's is refused; the key cookie is
     neither sent nor needed.
2. **The plain relay** — `/api/ui/devices/{device}/…` forwarding request and
   answer for ordinary calls, over a client that presents this device's
   certificate and tries the member's addresses in order.
   - A remote Conversation's view loads on the hub.
   - A member that is down answers a named refusal, not a timeout page.
3. **Streams and sockets** — the SSE stream, the terminal socket and the file
   watcher socket relayed; multipart uploads streamed through.
   - A remote terminal echoes keystrokes; a remote Code pane follows the disk.
4. **The client's device dimension** — `pathOf`, the route table, query keys
   and the fetch helpers take an optional device; the page reads it off the
   URL.
   - Navigating between a local and a remote Conversation keeps both caches.
5. **Member nudge streams on the hub** — held per member, re-announced under
   the device, reconnecting.
   - A Set answered on B refreshes A's open page without a reload.

## Re-verify at start

- Stage 02 landed: the member store and the verifier exist.
- The client is still all relative paths in `web/src/api/client.ts`, one
  `EventSource` at `web/src/nudge.ts`, and the only websocket is the
  attach/screen one plus the watcher one from the Code pane roadmap.
- The key gate is still applied in `crates/server/src/key.rs` as middleware
  over `router_keyed` in `crates/server/src/lib.rs`, so the same router can
  be mounted twice behind two gates.
- The route table is still the flat one in `web/src/App.tsx` with the leaves
  under `/conversations/:id`.
- `crates/server/src/watchers.rs` still holds a watcher per attached pane and
  needs the attach to survive a relay hop.
