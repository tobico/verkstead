# 01. Mirror rows and the cluster-wide picker

## What to build

Each device keeps a **mirror row** per member Profile it has heard of, marked
with its home device and the id it has there. That is the whole of why mirrors
exist: a pairing, a Repo's memory of what it was last grilled with, and every
Conversation go on holding a **local** Profile id, and nothing that reads one
changes.

**Fetched device to device, not through the browser.** The whole viewer
namespace is already served to a member over the Peer Listener behind the Member
Gate, so the Profiles a member lists are readable as they stand and there is no
new route in this task. What is new is this device *asking* — the way it already
holds a Nudge stream to each member rather than making the browser hold one.
The rows are refreshed when a member says its Profiles moved, when a device is
linked, and at startup; a member that is not answering keeps the rows it last
gave, as its Conversations are kept.

**A mirror carries what a row is drawn and picked by, and no account of this
device's own**: the name, the agent type, the models, the memory switch, and
what the home device says about the account. The account paths on it are the
home machine's and belong to no filesystem here — what a session away from home
is actually given is task 04.

**The Profiles reading answers this device's own rows and every mirror
together**, each saying which device it is at home on, and the pickers and the
Profiles section draw that device with the mark it wears on a sidebar row. One
list rather than a section per device: a Profile is a Profile, and which machine
its account sits on is a fact on the row.

**A Profile removed at home leaves every device.** The next refresh finds it
gone, the mirror goes with it, and every Conversation here that had picked it is
nulled out of — both halves of the Pairing and the Repo's memory of it — exactly
as a local removal does. A removal *pressed* here is task 02.

**And a mirror cannot be started under yet.** Nothing has fetched the account,
so a session launched under one would run logged out. It reads as such on the
row and Start is refused; task 04 is what lifts that, and task 07 is where the
three lasting broken states are said.

## Acceptance criteria

- [ ] With A linked, B's Profiles section and all three pairing pickers list A's
      Profiles beside B's own, each remote row carrying A's name and its OS mark.
- [ ] A Profile saved, renamed or removed on A reaches B on the refresh; a
      removal there takes B's mirror with it and nulls the pairings that named
      it.
- [ ] A mirror keeps one local id across refreshes, so a pairing made against it
      survives them, and a member that stops answering keeps its rows.
- [ ] Start under a mirror is refused, naming that its account is not on this
      device.
