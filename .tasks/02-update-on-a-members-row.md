# 02. Update on a member's row

## What to build

**Update** on a member's Devices row, pressed from any workbench (a phone
included), that asks first and then tells the member. The row reads
*updating…* with the member's own percent, and the member's own failure becomes
the row's trouble line. Task 03 adds the restart and the coming back; this task
ends the row's *updating…* only on a reported failure.

- **Shown** when the member's install kind (task 01) updates itself, the row
  is tagged *update available*, the member is reachable, and no Update is
  going for it, exactly as this device's own row decides (`offeredUpdate` in
  the Remote access pane).
- **The confirm card** is the Devices list's existing `Confirm`, as this
  device's Update already uses it. Its question is *Update crow?* and its body
  names the version and the sessions running on crow, saying they will be
  stopped and resumed (the deferred wording Set 1381; stage 04's card is the
  model, branches named and drafts counted as *2 drafts*). Update goes ahead
  regardless. The sessions come off the member's list that the merging module
  already holds in memory (each row's `working`), named the way
  `merging::settled` names them, as `running_there` does for the Annex. No new
  call. Carry them on the member's row in the Devices view, beside
  `update_available`.
- **The press** is a viewer route on the pressing device, under
  `/api/ui/devices/members/{device}/…` like Unlink. It sends task 01's peer
  call and answers with the Devices section read again. **The pressing device
  holds a record of each member Update it started**, in memory as stage 04's
  `Updating` does, so every workbench on that device (a phone too) draws it,
  and it announces `Nudge::Devices` as it moves.
- **Reading the member back**: when a `devices` Nudge comes down that member's
  Nudge stream, the pressing device reads task 01's `GET` and keeps the percent
  and any trouble. The row polls while it reads *updating…*, the way this
  device's does.
- **Trouble lines** go on the member's row, where `LinkedDevice::trouble`
  already joins a member's lines, and stand until the next press:
  - the member refused by name: its sentence;
  - the member reported a failed download or check: *crow couldn't be
    updated*, with the updater's reason, worded like this device's line
    (`annex::booting::sentence`);
  - **unreachable at the press**: the member is skipped, nothing is remembered
    for later (ADR-0027), and the row says it was unreachable so it wasn't
    updated. The human presses again.

The proof is the server suite (Set 1416): two in-process devices, with a
stand-in for the member's desktop app that hears the stdout line, or the
equivalent hook, and posts progress or failure to the member's own API. Then
the viewer's tests with fixtures.

## Acceptance criteria

- [ ] Pressed for crow with two sessions running there, the card reads *Update crow?* and names both sessions and the version; Update proceeds and crow's row reads *updating…*, then *updating… 40%* once crow reports it.
- [ ] A download that fails on crow becomes crow's trouble line on the pressing device's row, read on a second workbench too.
- [ ] A member unreachable at the press is not told, and its row says it was unreachable.
