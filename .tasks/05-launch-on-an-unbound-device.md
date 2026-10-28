# 05. Launch on an unbound device, brokenness per device

## What to build

A session under a Profile not bound on this device runs from a **copy** under
the Data Directory (`accounts/<uuid>`). The copy is fetched from a bound device
before each launch, the way a mirror's was fetched from its home. Stage 03
replaces this fetch with the replicated login.

- **The account fetch** asks a reachable device whose binding holds a login,
  addressed by the uuid on the peer account route. The route answers for any
  Profile **bound on the answering device**, where it used to answer for one
  of that device's own rows. The allowlist, the ADR-0025 strip and writing
  beside and renaming over are what they were.
- **The login is written back** to the device it was fetched from, as the
  session ends. A Terminal's ending does the same.
- **The memory sync** pulls from and writes back to that same bound device,
  addressed by uuid, where it went to the home before. Stage 04 rewrites it to
  union and fan out.
- In this stage a Profile has exactly one binding (a second is stage 03's
  press), so *a reachable bound device* is that one device. Write it as the
  first reachable one anyway.

**Brokenness stays per device**, answered on every read and drawn on the card
and on every picker row:

- **The harness is absent here**, in the onboarding probe's own words.
- **Not bound here, and no bound device that holds a login is reachable.** The
  reading names no device.

A bound device's own readings (a path missing, signed out) are as today. The
old mirror readings, *home unreachable* and *no login at home*, go: the gap
reading replaces them, and a binding without a login file is not a source. A
broken Profile stays in the pickers. A **Start** under it is refused by name,
in the row's words. A bound device that went away between the reading and the
launch is still a refusal in the session's Capture.

Windows' same-volume rule holds by construction for the copy: it is under the
Data Directory, as the mirror was.

## Acceptance criteria

- [ ] A session on B under a Profile bound on A alone starts with A's login
      and the memory of the Repo from A, and as it ends writes the refreshed
      login back to A.
- [ ] With A off, the Profile reads broken on B's card and pickers, and a
      Start on B is refused naming the gap.
- [ ] On a device without the harness, the row reads in the onboarding
      probe's words.
