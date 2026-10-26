# 04. Update all

## What to build

**Update all** on the Update Notice's banner. It updates every member that can
be updated, then this device, in one press.

- **Run by the server** (Set 1416), as one viewer route the banner presses,
  held to the end by the pressing device, so a phone that closes the page
  halfway does not stop it. A press while one is going reads the one already
  going.
- **The order**: every reachable member whose install updates itself and whose
  row is tagged *update available* is told **at once** (Set 1416), each through
  task 02's press and drawn on its own row as tasks 02–03 have it. Once every
  one of them is back on its new version or has a trouble line (task 03's
  10-minute deadline bounds the wait), **this device updates last**, through
  stage 04's own `Updating::begin`, where it can update itself and is behind
  the release. Then the page goes through *restarting…* as this device's row
  already does.
- **Passed over**: a member unreachable at the press is skipped, its row saying
  it was unreachable (task 02's line). Members that can't update themselves,
  older members that say no install kind, and Annexes (they follow their
  Keeper, task 01) are passed over silently, since their rows already say why.
  Members already current are left alone.
- **The banner**: where Update all has anything to do (any device it would
  update), the banner carries **Update all** beside, or in place of, stage 04's
  one-device Update. It goes through a confirm card naming the devices it will
  update and the sessions running on them, in `docs/copy.md`'s voice. Where it
  has nothing to do, the banner is as stage 04 left it. Check the **Update
  Notice** entry in `CONTEXT.md` still says what the banner does.
- If this device can't update itself (a nix hub, say), Update all updates the
  members and stops.

Suite: two members and a hub with one member asleep; the awake member and the
hub update, the sleeper's row says it was unreachable, and the hub's own Update
starts only after the awake member is back.

## Acceptance criteria

- [ ] With one member asleep, the other member and this device update, and the sleeper's row says it was unreachable.
- [ ] This device is told to update only once every member told is back on its new version or has a trouble line.
- [ ] The Update all run carries on with no page open; nix members and Annexes are passed over.
