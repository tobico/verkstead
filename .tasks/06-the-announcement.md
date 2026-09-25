# 06. The announcement

## What to build

A third device joins through either of the two, and lands on all three lists with
one press. **The introducer is what announces it** (ADR-0020, *A cluster is a
membership*): B, having just confirmed A, tells each of its own members about A
over the link it already holds to each of them, and no member confirms that
announcement — it arrived over a link that member has already verified.

**Why it is the introducer's to make rather than the newcomer's.** A newcomer
introducing itself would be a stranger asking a member to record it, and a member
has no way to tell that from anybody else who can reach its peer port: the one
confirmation would be the cluster's only gate, and every other member would be
joinable without passing it. Announced by the introducer, the claim comes over a
verified link, and the newcomer's own first call is then an ordinary one from a
device that member knows.

**The first member-only route in the stage.** The announcement is a member's own
call, so it stands **inside** the Member Gate and needs no exemption — the un-gated
surface was closed at three in task 05 and this is not a fourth. A caller that
presents a certificate the receiving device holds no membership for is refused
there, which is what makes *a device that announces itself is refused* true
without anything being written to make it true: a newcomer announcing itself is
not yet a member of the device it is announcing to.

**What it carries and what the receiver does.** The newcomer's id, name, OS,
addresses and certificate — the same shape the handover in task 05 hands over, and
worth being the same shape. The receiver records it as a member and its list grows
a row. Recording is idempotent against the **Device Id**: an announcement about a
device already recorded updates what is known about it rather than making a second
row, which is what keeps the announcement safe to make again.

**A member that cannot be reached.** The announcement goes over the dialling task
02 built, so a member that answers nothing is dimmed *unreachable* and the
announcement was not made. Nothing here retries in a loop: the member is told when
it next answers, and what makes that happen is the same machinery task 08 needs for
a fingerprint nobody has acknowledged — so if you build a *what is still owed to
whom* record here, build it once and leave it where task 08 can use it. An
announcement that could not be made must not fail the join: the human pressed
Allow, the newcomer is a member, and a machine that was off is the machine's
problem rather than the press's.

**Three servers, which is the stage's demonstration.** Start three, link the
second to the first, link the third to *either* of them, and all three lists read
the same three devices with no further press anywhere. That is the end-to-end
check this whole stage was cut for, and it is worth writing as one test that stands
up three servers rather than as three that stand up one each.

## Acceptance criteria

- [ ] With three servers, a join through either of the two already linked leaves
      all three reading Devices as the same three devices, with no press beyond the
      single Allow.
- [ ] A device announcing itself to a member is refused at the member gate, and a
      member announcing a newcomer to another member is not.
- [ ] An announcement about a device already recorded updates the row rather than
      making a second one.
- [ ] A member that answers nothing is dimmed *unreachable* and is recorded as
      still owed the announcement, and the join it was part of still completes.
- [ ] A newcomer's own first call to a member it was announced to is an ordinary
      member's call and gets through.
