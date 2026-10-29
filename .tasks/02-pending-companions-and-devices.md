# 02. Pending companions and device ticks

## What to build

The plus-menu from task 01 grows three kinds of row, each recorded as a pending
addition on the Set under the question's label and drawn as a chip beside the
Answer, with × and the settled-Set refusals exactly as the pending server has.

**A companion.** Each registered Repo that is not the Conversation's own and
not already a Companion is offered by name. Picking one opens a card, not a
bare pick: prefilled read-only off the Repo's default branch, with the three
fields the Steer's companion rows ask and reusing those rows (the read-write
toggle, the base picker, the read-write branch name). The menu lists registered
Repos only; the Repo dropdown's *Open repo…* and *Create repo…* rows are not
drawn here, because registering is the trust boundary and is done at a desk.

**Opening a companion up.** Each Companion the Conversation holds read-only is
offered as a row to open it up to read-write, through the same card with the
mode fixed and the base and branch name to fill. Nothing offers to remove a
Companion or put one back to read-only.

**A device tick.** Under a *May be transferred to* group, each other device of
the cluster that is not already a Permitted Device of the Conversation is
offered; picking one records it with no card. Where there is no cluster or no
other device the group is not drawn.

The chip says what was picked in words: the repo's name with its mode and, for
read-write, its branch; the device's name. Nothing is applied to the
Conversation in this task; the next launch binds what it bound before.

Read ADR-0028, the companion-repos roadmap's stage on the Steer, and the
CONTEXT.md entries for Companion Repo, Permitted Device, Repo and Steer.

## Acceptance criteria

- [ ] The menu offers registered Repos not on the Conversation and not its own, each read-only Companion to open up, and under *May be transferred to* each other device not already permitted; a Repo already on, the Conversation's own Repo, and a device already ticked are not offered.
- [ ] Picking a Repo opens a card prefilled read-only off the default branch whose three fields are the Steer's rows reused; the recorded addition holds repo, mode, base and branch, and the chip names the repo and mode.
- [ ] Picking a device records a tick with no card and draws a chip with the device's name.
- [ ] Every pending kind survives a reload, shows on another device, comes off with ×, and is refused on a settled Set.
- [ ] Nothing is applied: the Conversation's companions and permitted devices are unchanged by any pick.
