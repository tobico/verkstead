# 01. The device select

## What to build

A **device select** stands left of the Repo select in the compose page's setup
row, and everything under it reads the device it names.

**Drawn only where another device exists.** With nothing linked the row is the
row it has always been — no select, no label, no gap — which is what nearly
every Verkstead is. With a member linked it lists this device first and then
each member, each wearing the OS mark a device wears wherever it is drawn, so a
WSL reads as Linux and an OS this build has no mark for still draws a row. **A
member that is not answering is listed like any other**: the list is the
cluster's membership rather than a reachability probe, and a call that cannot be
made is refused by the Relay in its own words.

**One control, exported once** beside the other setup controls, because the
composer of a saved draft draws the same one in task 04 and a later stage's
Transfer dialog draws it again. What a pick *does* is the page's own, exactly as
it is for every other control in that row.

**Remembered per browser**, the way the pane widths and the compose draft are —
this device until something is picked, which was chosen over this device every
time for the laptop-drives-desktop setup. The module that keeps what this
browser remembers is renamed to say that is what it is, the linked devices being
what the module one letter away from it is about. A remembered device that is no
longer a member reads as this device rather than leaving the page pointed at a
machine the cluster has lost.

**Everything under the select follows the pick.** The Repo dropdown lists that
device's Repos, the base picker its branches, the companion rows its registry
and the three pairing pickers its Profiles — all of it through the Relay and
nothing composed anywhere new: every one of those controls already reads which
device the page it is drawn on is about, so what the select changes is that one
reading over the composer's subtree. The pairing prefill is asked of that device
too, which is the same question about what the Repo there was last grilled with.

**And a pick drops what named the other machine.** A Repo id, a companion's Repo
id and a Pairing's Profile id are each one device's own, so picking a device
takes the repo, the base, the companions and the three pairings with it; the
Brief, the branch name and the files picked stay where they are. That is the move
a Repo switch already makes for the base and for a companion that has become the
work's own Repo, made one level up. The pairings going with it is what puts the
picked device's own remembered pairings in front of the human rather than three
empty pickers.

**Settled while a roadmap or a pull request is loaded, and loading one puts the
work back on this device.** Both of those lists are this device's own — the
roadmaps nothing is driving, and every pull request open across its registered
Repos — so a row of either names a Repo here. The select reads settled from the
moment a card is loaded, the way the Repo picker already does in that state, and
is a select again the moment it is cleared.

Nothing reaches the server here that did not before: the press under the box is
still the first thing that does, and it still creates on this device. That is
task 03.

## Acceptance criteria

- [ ] With no other device linked, the setup row is exactly as it was.
- [ ] With a member linked, the select stands left of Repo listing this device
      and each member with its OS mark, an unreachable member among them.
- [ ] Picking a member makes the Repo dropdown, the base picker, the companion
      rows and the three pairing pickers read that device's, and the prefill the
      pickers show is that device's.
- [ ] The pick survives a reload, and a device that has left the cluster reads as
      this device.
- [ ] Picking a device clears the repo, the base, the companions and the three
      pairings, and leaves the Brief, the branch name and the held files alone.
- [ ] Loading a roadmap or a pull request puts the work on this device and leaves
      the select settled; clearing the card frees it again.
