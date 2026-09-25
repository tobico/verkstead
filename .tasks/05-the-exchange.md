# 05. The exchange

## What to build

Two Verksteads become one cluster. The press on B in task 04 now reaches A: B
dials A back, each side checks the certificate it meets against the one it pinned,
and both Devices lists read the same two devices. The third device is task 06.

**The dial-back** (ADR-0020, *A cluster is a membership*, *The join*). On Allow, B
dials A back at the addresses A gave in its request — in order, through the dialling
this stage's task 02 built — and checks the certificate it meets is the one the
pending request is holding. A device that answers on A's address with some other
certificate is not the device that asked, and the exchange stops there.

**The third un-gated route, and the last of them.** The dial-back arrives on A
before A has recorded B, so it cannot be a member's call; it is matched against the
certificate A's own pending record is holding instead of against the member list,
and it is refused once the ten minutes are up. That completes the un-gated surface
stage 01 wrote down as a list of three — the identity endpoint, the join post, and
this — and nothing grows it afterwards. Everything else on the peer listener is a
member's or is refused.

**What is handed over.** B hands A its own id, name, OS, addresses and
certificate, and every member's. A checks B's certificate is the one it saw when it
posted the join, and records B and each of those members. In a cluster of two there
are no others, so what lands is B alone — the handover is written to carry the
whole roster all the same, because that is what makes task 06 nothing more than
*and now tell the others*. A member arriving this way needs no further press: it
came over a link A has just verified against a certificate it pinned itself.

**A deny and an expiry come back the same way.** The human settled that B dials A
back on a deny too, matched against A's pending record exactly as an allow is —
the ADR spelled out the dial-back on Allow and left the other two, and without
this A sits reading *waiting* until somebody cancels it. So the dial-back carries
either the acceptance with its roster or the refusal; A's pending row reads it and
is dismissed, and a denied request leaves nothing recorded on either side. An
expiry on B is told to A the same way where A can still be reached, and A's own
clock is what covers the case where it cannot.

**Both lists read the same.** After this, reading Devices on A lists B and on B
lists A, each with the other's name, OS icon and addresses, and each card's count
says one other device is linked.

**What is not here.** Nothing is announced to any third device — task 06 — and
nothing can be unlinked — task 07.

## Acceptance criteria

- [ ] Allow on B leaves both devices recording each other: reading Devices on
      either lists the same two, with names, OS and addresses, and no pending row
      left on A.
- [ ] B refuses the exchange where the certificate it meets at A's address is not
      the one the pending request holds, and A refuses a dial-back whose
      certificate is not the one it met when it posted the join.
- [ ] The dial-back is answered by A although B is not yet a member, and is
      refused once the request's ten minutes are up.
- [ ] A deny on B reaches A's pending row, which reads it and is dismissed, with
      nothing recorded on either side.
- [ ] The handover carries every member B holds rather than B alone, so a roster
      of more than one arrives in one call.
- [ ] A first call A makes to B afterwards is an ordinary member's call, gated on
      the certificate B now holds for it.
