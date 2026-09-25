# 08. The renewal announced

## What to build

Stage 01's **Changeover** gets somebody to tell. A device that re-issued its
certificate announces the new fingerprint to every member over the link it already
holds, each member records it against the same **Device Id**, and the renewing
device goes on presenting the old certificate until every one of them has
acknowledged (ADR-0020, *A device is an id and a certificate*). Without this the
re-issue stage 01 built would strand a cluster on fingerprints nobody holds — a
membership is a set of fingerprints, and the handshake is the one place an expiry is
a fact rather than a date in a file.

**What already exists.** A start with fewer than thirty of the certificate's ninety
days left makes a fresh one, keeps it beside the outgoing one, and reports what it
did: *not due*, *nobody to tell*, or *this many yet to tell*. The listener presents
the outgoing certificate for as long as anybody is owed. Task 01 made *how many are
owed* read off rows. What is missing is the telling, and the acknowledgement that
takes a member off that list one at a time.

**The announcement is a member's own call**, inside the Member Gate, going over the
dialling task 02 built — and it is made **presenting the outgoing certificate**,
because that is the one every member still holds. A member records the new
fingerprint against the same device id: the id was invented once and lasts as long
as the Data Directory, and it is the certificate that is renewed. The member
answers that it has, and that answer is what takes it off the owed list.

**A member may hold either certificate for a moment**, and must accept both while
the changeover is in flight — the announcement arrives over the old one and the
renewing device's very next call may be over either. Settle which and be explicit
about it: the simplest reading is that a member accepts the old until it has
acknowledged the new and the new thereafter, and that the renewing device switches
only when the last acknowledgement is in.

**When the last one acknowledges**, the changeover completes: the new certificate
becomes the one presented, and the file the incoming one was waiting in goes — which
is what a re-issue with no members does at the start that began it today. That path
should end up the same path rather than a second one beside it.

**A member that was down.** It is announced to again when it next answers, through
the same *still owed* record tasks 06 and 07 put in — three things are owed to a
member that was unreachable now, and one record holding all three is what keeps
them from being three loops. What triggers the retry wants settling: the next dial
that gets through to that member is the honest answer, since something dials it
whenever the cluster does anything, and a device whose members are all quiet is a
device with nothing owed that matters yet.

**And one that never answers** is a member the human unlinks, which task 07 makes
possible on a dimmed row. Nothing here waits for ever on a machine that is gone: the
changeover simply stays in flight, the old certificate keeps going out, and the
startup line keeps saying both fingerprints.

## Acceptance criteria

- [ ] A device that re-issues its certificate stays reachable throughout, from
      every member, and every member ends up holding the new fingerprint against
      the same device id.
- [ ] The announcement is made presenting the outgoing certificate, and is refused
      where the caller is not a member.
- [ ] Until the last acknowledgement the listener presents the outgoing
      certificate; on the last one the changeover completes and the incoming file
      goes, by the same path a re-issue with no members takes today.
- [ ] A member that was unreachable when the certificate was re-issued is told when
      it next answers, and the changeover finishes then.
- [ ] A member that never answers leaves the changeover in flight rather than
      failing anything, with the startup line naming both fingerprints, and the
      human can unlink it.
- [ ] Demonstrable with three servers: re-issue on one of them and every call in
      every direction goes on working while it is in flight and afterwards.
