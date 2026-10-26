# 05. The notification on trouble

## What to build

A device that could not be updated raises one notification, a full sentence
naming the device, to the phones of the device that pressed. A success raises
none; the rows show success (ADR-0027).

- Raised wherever tasks 02–04 put an update's trouble line on a member's row:
  refused, failed download or check, back on the old version, or never back by
  the deadline. Skipping an **unreachable** member raises none, since its row
  says so and nothing was attempted.
- **This device too** (Set 1416): stage 04's own trouble line (a failed
  download reported by the app, or a relaunch that came up on another version)
  raises the same shape of sentence, from this device's own server.
- **The carrier** is `push::mentioned` with a new `Word` variant, the
  device-level kind a join already uses, not the per-Conversation `News`. Its
  tap opens the Remote access pane. The device's name is cut to `A_NAME` as
  the join's is. Example: *crow couldn't be updated: it came back on 0.2.2.*
  Follow `docs/copy.md` on notifications.
- One notification per device per press: a line that is re-read or re-drawn
  does not raise it again.

## Acceptance criteria

- [ ] A member whose update failed, or whose version did not change, raises one notification naming it, and a member that updated raises none.
- [ ] This device's own failed Update raises the same shape of sentence.
- [ ] An unreachable member skipped by Update all raises no notification.
