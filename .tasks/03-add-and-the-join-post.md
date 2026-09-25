# 03. Add, the join post and the pending row

## What to build

The first half of a join: A asks, and B holds the question. Nobody presses
anything on B in this task — the modal is task 04 and the exchange is task 05 —
so what this delivers is a request that arrives, is kept for ten minutes, reads
back on the device that made it, and can be taken back.

**Add, in the Devices section.** Type an address and press it. That is the one
place in the Remote access pane where something is configured rather than read,
which is the same departure **Unlink** makes in task 07 and Remove on a Repo makes
already — everything else on that pane is the machine read again. A typed address
may carry a port or not; the peer port has a default and the human should not have
to know it.

**What A does.** It dials the address, **accepts whatever certificate B presents
for this one call** — there is nothing yet by which to know what B's certificate
ought to be, which is exactly what the human is about to confirm by eye — and
posts its own id, name, OS, addresses and certificate. It pins what B presented
into its own record of the request, because that pinned certificate is what task
05 matches B's dial-back against.

**What B does.** The join post stands **outside the Member Gate**, and this is
where the second of the three un-gated routes arrives (ADR-0020, *The peer
listener, and mutual TLS*). A join comes from a non-member by definition — that is
what a join is — and it is not un-authenticated for it: the certificate the
handshake took is pinned into the pending request the post creates, and everything
that follows is matched against it. B records the request and holds it **ten
minutes**; past that it is expired, and a post naming an expired request is
refused.

**A pending join is a store table on both sides**, beside the members — the human
settled this over holding it in memory, so that a restart does not silently drop
a request the human is halfway through confirming. The two sides' records are not
the same record and should not be one table pretending to be: B's holds the whole
of what A said about itself plus the certificate A presented and when the ten
minutes run out; A's holds the address it typed, the certificate it met at the far
end, and how the request has since gone.

**A's pending row.** In the Devices list, above or below the member rows as reads
best, saying *Waiting for confirmation on B* with **A's own fingerprint** and a
Cancel. A's fingerprint rather than B's, deliberately: the modal on B in task 04
draws the same string, and the two are there to be compared by eye by one person
reading off a phone and another off a screen. Cancel reaches B and the request is
gone there; an expiry reads on A's row and is dismissed from it.

**What A does not know yet.** Nothing on B has answered, so a pending row in this
task only ever leaves *waiting* for cancelled or expired. How an Allow or a Deny
reaches it is task 05.

## Acceptance criteria

- [ ] Add on A, against B's address, leaves a pending row on A reading *Waiting
      for confirmation on B* with A's own certificate fingerprint and a Cancel,
      and a recorded request on B holding A's id, name, OS, addresses and the
      certificate A presented.
- [ ] The join post reaches B from a device B holds no membership for, and every
      other gated route still refuses that same caller.
- [ ] A request older than ten minutes is expired: B refuses anything naming it,
      and A's row reads so and can be dismissed.
- [ ] Cancel on A's row takes the request off B as well as off A, and a second
      Cancel is not a second thing happening.
- [ ] Both records survive a restart of the server that holds them, with the ten
      minutes counted from when the request was made rather than from the restart.
- [ ] A typed address with no port reaches the peer port's default, and one with a
      port reaches that.
