# 01. The member store and the gate

## What to build

A **Member** becomes a row that is really kept, and the **Member Gate** stops
answering *no* to everybody (ADR-0020, *A cluster is a membership*). Nothing in
this task links anything — the join is task 03 — so what a row comes from here is
a fixture. What it delivers is the thing every task after it writes into, and the
first honest answers to the three questions the membership is asked.

**The members table.** Id, name, OS, addresses, fingerprint, last seen, as the
brief's decisions say — the **Device Id** as the key, because that is what every
record in a cluster names a device by, and the fingerprint beside it because that
is what a caller *is* on the **Peer Listener**. Follow how the store's other
tables are declared and brought up: a module of its own with its own schema step,
`STRICT`, and every write through the store's own immediate-transaction helper.
The addresses are a list, in the order a peer should try them — how a list is
stored is yours to settle, but a device advertises all of them on every exchange
and the order is load-bearing, so it has to come back out in the order it went
in.

**The membership handle stops being a stub.** Today it is a count with *does this
device hold that fingerprint* hardcoded to false and *how many are owed an
announcement* answering *all of them*. Three callers ask it three questions —
the gate asks whether a caller is a member, the **Changeover** asks how many have
yet to acknowledge a new fingerprint, and the Devices reading asks how many there
are — and all three now read rows. Keep the fixture constructor the suite leans
on: a stated membership is what lets a test stand where the product cannot yet,
and stage 01 put it there deliberately.

**Which reorders the start.** The membership is built before the database is
opened today, and the device's identity is read after it and consults it, because
a certificate near its expiry is re-issued at a start and whether the new one can
be presented straight away depends on whether anybody is owed an announcement of
it. A membership that reads rows needs the store open first. Move what has to
move and leave the reasoning intact: the order the start does these things in is
an argument written down in the comments there, and it should still read as one
afterwards.

**The Devices list draws a row per member.** The section holds this device alone
today, and the Remote access card carries a count. Now the list is this device's
row and then one per member — the OS icon, the name, and the addresses under
them, the same shape this device's own row already has — with *this device* still
marking which one is this machine. The card's count comes off the rows rather
than being answered separately. No Unlink yet: that is task 07, and a row with
nothing to press on it is what this task draws.

**Proving the gate.** The stage's own member-only routes arrive later — the
announcement in task 06, the Unlink broadcast in 07, the renewal in 08 — so there
is no product route here to prove the gate against. The suite already has what it
needs: stage 01 stood a one-line gated route up in the peer listener's tests for
exactly this question, and what changes now is that a caller presenting a
recorded certificate gets through it. Do not invent a product route to have
something to check.

## Acceptance criteria

- [ ] A call presenting a member's certificate reaches a gated route on the peer
      listener; a call presenting an unknown one, or none at all, is refused
      there and still reaches the identity endpoint.
- [ ] A member removed from the table is refused on the next call, with nothing
      cached from before it went.
- [ ] The Devices list draws a row per member beside this device's own, with the
      OS icon, the name and the addresses, and no Unlink on any of them.
- [ ] The Remote access card's count comes off the rows, and still reads right in
      each of the Tailscale states it already covers.
- [ ] A changeover's *how many have yet to acknowledge this fingerprint* is
      answered from rows: with members recorded against the old fingerprint it is
      their number, and with none it is nought and the changeover completes at the
      start that began it, as it does today.
- [ ] The addresses come back out of the table in the order they went in.
