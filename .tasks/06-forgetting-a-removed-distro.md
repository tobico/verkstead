# 06. Forgetting a removed distro

## What to build

**A distro removed by hand is noticed rather than offered** (ADR-0024). There is
no Remove on the Components row. At the Keeper's start, where the probe finds
**no box**, while the membership holds a member row that this device keeps
(`kept_by` naming this device), the Keeper **forgets its Annex**. "No box"
means no WSL at all, or WSL answering with no distro made to the end (Set 30
Q6), and "made to the end" is the `made` marker stage 03 writes.

1. The member row is dropped from this device's membership.
2. **Every other member is told**, over the same peer route an unlink uses
   (`DELETE /api/peer/v1/members/{device}`, `peer/unlinking.rs`). Its handler
   already drops a device that is not the receiver's own Keeper or Annex, so
   the far ends need nothing new.
3. The Devices pages are nudged. The row is gone on every device, and the
   Components row reads *not installed*.

**Forgetting is not Unlink.** Stage 01 refuses Unlink on an Annex everywhere,
and that refusal stays. This is a separate path on the Keeper, the one that
drops an Annex's row, and it reuses the broadcast without going through the
press or its refusal. The Annex is never told because it is gone, so unlike an
unlink the leaver is not dialled first.

Anything the Keeper held for its Annex goes with it: the Annex Forward points
at nothing, the trouble line is cleared, and the held version from task 01 is
dropped. A box that later comes back (Install from the pane, task 02) is linked
as a new Annex by the Keeper's Join.

A probe that could not be read at all (for example `wsl.exe` timing out) is not
the same as "no box", and forgets nothing.

## Acceptance criteria

- [ ] After `wsl --unregister Verkstead` and a restart of the Keeper, the
      Annex's row is gone from the Devices list on the Keeper and on every
      other member, and the Components row reads *not installed*. Proven on
      Windows with a second device in the cluster.
- [ ] With WSL itself gone, the same happens.
- [ ] Unlink on an Annex is still refused, and a probe that could not be read
      forgets nothing. Both are covered by the suites, the forget itself over
      the stub `wsl.exe` with a member row seeded.
