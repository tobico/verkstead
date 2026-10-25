# 02. Re-read on take-up

## What to build

A member's identity is read again every time this device takes up that
member's **Nudge stream**, so a member that restarted on a new release shows it
on its row without a press or a reload. This is fresh by construction: a
version only changes across a restart, and a restart of either end ends the
stream and has it taken up again. **No poll and no timer is added** — ordinary
members are read on no timer today, and that stays.

- The stream is held by `held` / `listening` / `read` in the relaying
  freshness module (not a `stay_fresh` as the brief guessed). The take-up is
  the point in `read` after the stream answered successfully, where
  `Nudge::Everything` is announced under the member.
- At that point, read the member's identity with the dial the Annex join
  already uses (`Peers::identity`), which checks the answer against the pinned
  certificate and records it. A failed read is a log line and nothing else; it
  must not end or hold up the stream.
- **Announce `Nudge::Devices` only when the recorded row actually changed**
  (version, name, OS or addresses). `Devices` makes mirroring and the Author
  mirroring re-read every member, so an unconditional announce would cost a
  sweep of the whole cluster on every take-up of every stream.

## Acceptance criteria

- [ ] Restarting a member on a different version changes its row on this
      device's open Devices view without a reload — proved in a two-device
      suite by swapping the version the member answers with and ending its
      stream.
- [ ] A take-up where nothing changed announces no `Devices` Nudge.
- [ ] A member whose identity read fails keeps its stream and its row as they
      were.
