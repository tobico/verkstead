# 07. Unlink

## What to build

**Unlink removes a device from the cluster for everyone** (ADR-0020, *A cluster is
a membership*). Every member drops it and it is told to drop them all. Cutting only
this device's own pair was rejected: a membership is not a set of pairs, and a
half-unlinked cluster is a list that reads differently depending on which device
you open.

**Asked once, as Remove on a Repo is.** One confirm over the page, naming the
device, and then it happens. That is the one other place the Remote access pane
departs from *nothing is confirmed twice, everything is read rather than
configured* — the Add in task 03 is the first — and it departs for the same reason
Remove on a Repo does: it cannot be taken back. Follow the confirm the Repo list
already draws, card and button pair and all.

**What the press does.** The device is dropped from this device's members, every
other member is told to drop it, and the leaver itself is told to forget every
member it holds. Each of those goes over the dialling task 02 built, inside the
Member Gate — a member telling a member to drop a member is a member's own call.
The order matters only in that the human's own device must end up right whatever
else fails: a member that could not be reached is one that still holds the leaver,
and it is told when it next answers, through the same *still owed* record task 06
put in.

**Unlinking the leaver is a call to a device that is about to stop being a
member**, which is the one ordering trap here: tell it to forget everyone before
dropping it from this device's own members, or the call that tells it goes out to a
device this one no longer holds a membership for — and the leaver's own gate has to
still hold *this* device when the call arrives, which it will, because the leaver
drops everyone only when told to.

**An unreachable member can still be unlinked**, and that is most of why Unlink
exists at all: a machine that is never coming back is what the human reaches for
this on. The row is dimmed and the Unlink on it works exactly as any other's —
the leaver simply cannot be told, and nothing waits on telling it.

**The row.** Every member's row grows an Unlink; this device's own row does not
have one, there being nothing to unlink this machine from itself. After the last
member goes, the list is this device's row alone and the card's count reads as it
did before anything was linked.

## Acceptance criteria

- [ ] Unlink on a member's row asks once, naming the device, and on confirming
      drops it here, tells every other member to drop it, and tells the leaver to
      forget everyone.
- [ ] The leaver's own Devices list is then empty but for itself, and its card's
      count reads none.
- [ ] Unlink works on a member dimmed *unreachable*, and the cluster the human is
      standing in is left correct although the leaver could not be told.
- [ ] A member that could not be reached is recorded as still owed the removal and
      is told when it next answers.
- [ ] This device's own row offers no Unlink, and after the last member goes the
      list reads as it did before anything was linked.
- [ ] A device that has been unlinked is refused at the member gate on its next
      call, and every remaining member refuses it too.
