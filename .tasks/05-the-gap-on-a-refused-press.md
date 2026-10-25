# 05. The gap on a refused press

## What to build

A press or a read this device relays to a member, and the member fails,
already leaves a sentence naming the machine — the `Refusal` built by
`put_to`, `word_from` and the relaying hop a browser's request crosses (*winvm
refused it: 404 Not Found.*, *winvm couldn't be reached.*). Where the two
versions differ, that refusal gains task 04's gap sentence after what it
already says: *winvm refused it: 404 Not Found. winvm is on 0.2.0 and this
device is on 0.2.2. Update winvm.*

- Only where the far end actually answered and refused, or answered something
  this device could not read. *couldn't be reached* and *no longer one of this
  device's members* are not about versions, and gain nothing.
- Reuse task 04's helper; the version on each side comes from the member row
  and this binary.
- No refusal is added on skew: the peer code's tolerate-it rules stay.

## Acceptance criteria

- [ ] Editing a mirrored Profile at home on a member a release apart that
      refuses it reads the refusal and both versions in one line.
- [ ] The same refusal from a member on this device's version reads exactly as
      it does today.
- [ ] An unreachable member's refusal reads exactly as it does today.
