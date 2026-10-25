# 04. A member's failed read on its row, naming the gap

## What to build

Today, when this device fails to read a member on its own account — mirroring
its Agent Profiles, mirroring its Authors, merging its Conversations into the
sidebar — the failure goes to the log and nowhere else (`read_of` in the
relaying module, and the "cannot read" branches in mirroring after it). That is
the winvm incident: nothing on either screen. This task puts it on a screen.

- **The last failed background read of each member is kept in memory**, keyed
  by the member and by what was being read, and **cleared by the next read of
  that kind that succeeds**. Not stored: it is about this run.
- **Drawn as the trouble line on that member's Devices row**, through the
  existing `LinkedDevice.trouble` slot the Annex's line already uses (and
  combined with it, not clobbering it, on an Annex's row). It says what could
  not be read, as a sentence per `docs/copy.md`'s refusal rule, e.g. *winvm's
  Agent Profiles couldn't be read.* The Devices view is nudged when the line
  appears or clears.
- **The gap sentence**, where the two versions differ, goes after it:
  - both known: *winvm is on 0.2.0 and this device is on 0.2.2.*
  - the member sent none: *winvm is on an older release than this device's
    0.2.2.* (Set 1394)
  - then the one action, naming the older device: *Update winvm.* — or
    *Update this device.* where this device is the older one.
  - nothing about versions when they are the same.
  Write the sentence as one helper, because task 05 puts the same sentence on
  press refusals.

## Acceptance criteria

- [ ] With a member a release apart whose Profiles answer cannot be read, its
      row reads *winvm's Agent Profiles couldn't be read. winvm is on 0.2.0 and
      this device is on 0.2.2. Update winvm.* (names as the suite has them).
- [ ] A member on the same version that fails the same read shows the failure
      line with no version sentence; one that sends no version shows the
      older-release sentence.
- [ ] The line goes once the next read of that member succeeds.
