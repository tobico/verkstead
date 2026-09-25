# 07. Drafting on a device

## Goal

A new Conversation is drafted onto whichever device should do the work. The
compose page grows a **device select** left of the Repo select — drawn only
where another device exists, reading the device last picked in this browser
and this device until then; picking one makes the Repo select list that
device's Repos and the pairings prefill from it, and Start creates the
Conversation there, the page landing on its remote URL. A saved draft's
composer has the same select, and moving it replays the draft onto the other
device and closes it here. Demonstrable end to end from A's browser: draft on
B, grill on B, see it in the merged list under B.

## Decisions in force

- **The device select is one control drawn twice**, as every setup control is
  — exported once, picked by each page's own handler
  ([ADR-0020](../../adr/0020-cluster-mode.md), *Drafting on a device*).
- **Remembered per browser**, the way pane widths are; this device until a
  pick. This device every time was rejected for the laptop-drives-desktop
  setup.
- **Not drawn alone**: with only this device, the row is as it was.
- **The compose page's replay goes through the relay unchanged** — the
  Conversation started against its Repo on that device, then a request per
  field — so no batched create and no second set of rules.
- **A saved draft moves by replay**: the fields replayed onto the other
  device, the draft closed here. The human chose this over a disabled select.
- **Repos and the prefill follow the device**: the Repo dropdown lists the
  picked device's Repos with its Open and Create rows, and the three-tier
  pairing prefill is asked of that device.

## Proposed tasks (provisional)

1. **The select** — exported beside the other setup controls, listing members
   with OS icons, hidden alone, with the per-browser memory.
   - Reload keeps the pick; a device that left the cluster falls back to this
     one.
2. **The compose page on a device** — `Composed` carries a device; the Repo
   list, the prefill and the replay go through the relay to it.
   - Start on a remote device lands on `/devices/{device}/conversations/{id}`.
3. **Open and Create on a remote device** — the dropdown's two rows browse
   and make directories there.
   - The path browser shows the remote machine's directories.
4. **Moving a saved draft** — the select on the draft's composer; a pick
   replays and closes.
   - The Timeline of the old draft says where it went; the new one carries
     the Brief, branch, base, companions and pairings.

## Re-verify at start

- Stage 04 landed: the relay serves `/api/ui/devices/{device}/…` for repos,
  pairings, conversations and the field endpoints.
- The controls are still exported from `web/src/workbench/Setup.tsx`, held
  in `Composed` in `web/src/workbench/composing.ts` with `blank`, `empty`,
  `parsed` and `create`, and drawn by `Compose.tsx` and `Composer.tsx`.
- The prefill chain is still `pairing_prefill` in
  `crates/server/src/conversations.rs`.
- Per-browser memory is still `web/src/device.ts` — note the name collision
  with linked devices; rename or namespace before adding to it.
