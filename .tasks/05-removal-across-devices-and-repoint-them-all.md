# 05. Removal across devices and Repoint them all

## What to build

**Remove on an Author that Repos point at** checks this device's Repos and
every reachable member's, over a peer call asking a member which of its
registered Repos point at a uuid. The refusal names each Repo with its device,
and says which members were not answering and so were skipped — such a
member's Repo refuses its next Start by name, as today. With none pointing at
it anywhere reachable, the removal goes through as a deletion stamp.

The refusal offers **Repoint them all to…** over an Author picker (the live
Authors other than this one). It repoints each named Repo — this device's
directly, a member's by asking that member from here — under today's repoint
rule: a Repo with a Conversation on it, or holding it as a Companion Repo,
that is neither Done nor Closed refuses by name, and the rest are repointed.
Where every named Repo was repointed, the removal then goes through; where any
refused, what refused is named and the Author stays.

Read `docs/copy.md` for the words. Checking this device alone, and blocking on
an unreachable member, were turned down (ADR-0028).

## Acceptance criteria

- [ ] Removing an Author that a member's Repo points at is refused naming that Repo and the member's device.
- [ ] A member not answering is skipped and said, and the removal is not blocked by it.
- [ ] *Repoint them all to…* repoints this device's and the member's Repos from here, and then the Author is removed on every device.
- [ ] A Repo with a live Conversation refuses the repoint by name and the Author stays.
