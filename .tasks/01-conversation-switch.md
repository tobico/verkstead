# 01. Fix the dropped conversation switch

## What to build

Switching between conversations sometimes leaves the old one on screen: the
URL updates but the page does not, and only a reload recovers. The cause is
diagnosed. Both routes render the same Workbench component, and the
conversation read uses solid-query's `reconcile: "id"` — but Solid's
reconcile exempts the store root from the key check, so when the query key
changes, the new conversation's payload is merged *into* the old one's store
object. The object identity never changes, the `<Match when>` never flips,
and the Timeline component — with all its local state (brief draft, carousel
position, output toggle) — survives the switch. A dropped-refetch race in
solid-query's Solid adapter (a `refetch` arriving while one is already
scheduled is silently discarded) compounds it.

Fix it structurally rather than by fighting the merge: key the
conversation-reading subtree on the selected id (for example `<Show keyed>`
around a component holding the query and the Timeline), so a switch tears
down and rebuilds the page. The `reconcile: "id"` freshness stays for updates
*within* one conversation — its purpose (not rebuilding rows under a live
session) is still right there.

## Acceptance criteria

- [ ] Switching conversations always displays the target, including when the
      target is already in the query cache
- [ ] Per-conversation local state (brief draft, pinned carousel position)
      does not leak from one conversation into another
- [ ] A test mounts one conversation, switches to a second, and asserts the
      second's content renders
