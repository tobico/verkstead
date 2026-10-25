# 05. Updating members

## Goal

Every member's Devices row has **Update** when that member runs a desktop
install and a newer stable release exists, and the Update Notice's banner has
**Update all**. A press on a member's row shows the confirm card — *Update
crow?*, the version and the sessions running there in its body — then tells
the member, whose row reads *updating…*, then *restarting…* once the member
goes quiet, until it comes back on its new version. Update all tells every reachable
member first and updates this device last, once each has come back or left a
trouble line. A member unreachable at the press is skipped, its row saying
so. A member that could not be updated raises a notification. Demonstrable
with two desktop devices, pressed from a phone.

## Decisions in force

- **Each device updates itself when told**
  ([ADR-0027](../../adr/0027-cluster-updates.md), *A device updates
  itself*): the pressing device sends a peer call and the member runs stage
  04's exchange on itself. Pushing the bits over the peer link was not taken.
- **Members first, this device last**, because the device drawing the page
  restarts too and its restart ends the page's view of the others. All at
  once was not taken.
- **Unreachable is skipped, with a line.** Remembering the press for a
  sleeping laptop was the recommendation and was not taken; the human
  presses again.
- **Click only.** No automatic updates; the design leaves room for a setting.
- **The words are stage 04's**, from the deferred wording Set (1381): the
  card asks *Update crow?* with the version in its body, and the row reads
  *updating…* and then *restarting…* once the member's Nudge stream drops.
- **A notification only on trouble**, one full sentence naming the device,
  as `docs/copy.md` has notifications; the rows show success.
- **A member that cannot update itself** refuses by name, and its row shows
  the by-hand line stage 04 gives it rather than a button; Update all passes
  it over.
- **The Update Notice's glossary entry** already says the banner carries
  Update all and still only informs until a press.

## Proposed tasks (provisional)

1. **Telling a member** — the members-only peer call carrying the release,
   answered by the member starting its own exchange or refusing by name; the
   member's state read back for its row.
   - A member on a bare binary refuses, and its row shows the by-hand line.
2. **Update on a member's row** — the button, the confirm card naming the
   member, the version and the sessions running there, the *updating…* state
   and then *restarting…* once the member's stream drops, until it reopens on
   the new version, the trouble line otherwise.
   - Pressed for crow, the card reads *Update crow?*; the row reads
     *updating…*, then *restarting…* while crow is down.
3. **Update all** — the banner's button, the ordering, each member's row
   changing in turn, this device last; an unreachable member skipped with a
   line.
   - With one member asleep, the others and this device update and the
     sleeper's row says it was unreachable.
4. **The notification on trouble** — a member whose update failed or whose
   version did not change raises one sentence to the human's devices.

## Re-verify at start

- Assumes stage 04 landed: the exchange, the install kind on the view, the
  by-hand line.
- Assumes the peer router is `crates/server/src/peer.rs` under
  `/api/peer/v1/` behind the Member Gate, and that the whole `/api/ui/*`
  namespace is already served to members (`peer/workbench.rs`) and reached
  from the browser through `/api/ui/members/{device}/…` — so the member's
  own stage-04 route, relayed, may be the call rather than a new peer route.
- Assumes each member's Conversations are held in memory by
  `crates/server/src/merging.rs`, which is where the sessions running there
  are counted for the card.
- Assumes the Annex's refusal while a session runs (`running_there` in
  `crates/server/src/annex/booting.rs`) is the model for refusing by name,
  and that news kinds for push go through `crates/server/src/push.rs`, where
  an unknown kind from a newer member is relayed as is.
- Assumes the Devices rows re-read on a `devices` Nudge and that a member's
  Nudge stream reopening is what says it is back.
