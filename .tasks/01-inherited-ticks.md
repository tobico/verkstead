# 01. Inherited ticks

## What to build

A stage's Conversation is given the roadmap Conversation's *May be transferred
to* ticks (its **Permitted Devices**) through the one inheritance funnel that
already carries its Pairings, its Brief and its companions — the server's
`continuing::settle`, which both a settle's start and the *Continue a roadmap*
press (`adopt` and the stages started alongside it) go through. No second place
copies them.

**The roadmap's drafting device is ticked too, where it is not home** (Set
1549, Q5). The roadmap Conversation's own drafting device is permitted
implicitly by its **Birth Key** and is never a tick; a stage's birth key is home
(the device that made it), so a roadmap Conversation that had itself moved would
otherwise lose a device it may go to. The stage may go everywhere the roadmap
may. Home itself is never written as a tick (it is the stage's own birth key).

Once ticked, the stage's *Transfer to…* dialog shows them, and the agent's own
`verkstead transfer` reads them, with nothing further to build: both read the
list live.

## Acceptance criteria

- [ ] A stage started by a settle off a roadmap Conversation with nuc-win
      ticked has nuc-win ticked: its *Transfer to…* dialog shows it, and
      `store::permitted_devices` lists it.
- [ ] Every stage started by a *Continue a roadmap* press — the pressed draft's
      own and each started alongside it — carries the pressed draft's ticks.
- [ ] A roadmap Conversation drafted on device A and now on home B gives its
      stages A as a tick; one drafted on home gives no tick for home.
- [ ] A roadmap Conversation with nothing ticked gives its stages nothing ticked.
