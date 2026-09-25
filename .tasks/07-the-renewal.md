# 07. The renewal

## What to build

The certificate is renewed before it runs out, rather than issued once for ever
(ADR-0020, *A device is an id and a certificate*). An expired certificate is
refused at the handshake, so a single long-lived one would take every link in a
cluster down together on the same day, the only way back being to re-link every
device by hand. A validity long enough never to matter was the other way and was
not taken: the id and the certificate outlive any guess made at first start, and a
link that silently stops working years later is the failure nobody would diagnose.

**The settled numbers: a validity of 90 days, re-issued when 30 days are left.**
Task 01 wrote the validity down; this task acts on it. A start with the expiry
inside 30 days makes a new certificate; a start with it further off leaves the
certificate alone. The check belongs at start rather than on a timer, and a server
left running past its own re-issue window is a case worth thinking about explicitly
rather than by accident.

**Both certificates are kept over the changeover.** Until every member has
acknowledged the new fingerprint the device keeps *presenting* the old one, so the
changeover never costs a call: a member that was unreachable is announced to again
when it next answers, and one that never does is a member the human unlinks
anyway. The announcement itself — telling every member the new fingerprint over the
link already held, the way an introducer announces a newcomer — is stage 02's, there
being no member to tell yet.

**So here the re-issue has nobody to announce to, and says so.** With no members
the changeover completes at once: the new certificate becomes the presented one
immediately and the old one is done with. What this task builds is the bookkeeping
that makes stage 02's announcement a matter of filling in who to tell — the two
certificates held side by side, the question *is anything still owed an
announcement*, and the answer being no.

Both fingerprints are printable over the changeover, because that is how a human
or a later stage tells which one a peer met.

## Acceptance criteria

- [ ] A start with the expiry inside 30 days re-issues the certificate; a start
      with it further off leaves the certificate and its fingerprint alone.
- [ ] Both fingerprints are printable over the changeover, and the listener
      presents the outgoing certificate while any member is unacknowledged.
- [ ] With no members a re-issue completes at once, and says it had nobody to
      announce to.
- [ ] The device id is untouched by a re-issue: it is the certificate that is
      renewed, and a device keeps its id for as long as its Data Directory lasts.
