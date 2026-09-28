# Joining the chain

A Stage whose tasks are all done **joins its roadmap's chain before its pull
request opens**. It waits until every Stage already in the chain has settled,
rebases onto the top of it, and only then is pushed, opened and wrapped up. A
new Stage is cut from the highest settled Stage in the chain rather than from
its own predecessor. The chain starts at the roadmap's own branch while that is
unmerged, so stage 01 comes off there exactly as it does today.

The point of joining at the finish rather than at the start is that it **never
rewrites a pull request somebody has started reading**: a Stage is rebased once,
before it has one. The wait on the Stage below *settling* is what makes that
true — a Stage still wrapping up is a branch still moving. So the joins are the
one thing a roadmap does in single file, and the Timeline says which Stage is
being waited on so it does not read as a stall. In a roadmap run in order —
which is every roadmap until stage 04 lands — the top of the chain is what the
Stage was cut from, the rebase moves nothing, and nobody sees a difference.

Roadmap stage: [03: Joining the chain](docs/roadmaps/parallel-stages/03-joining-the-chain.md)

## Tasks

- [x] 01: Waiting to join — [details](01-waiting-to-join.md)
- [x] 02: One at a time — [details](02-one-at-a-time.md)
- [x] 03: This repository's own block — [details](03-this-repositorys-own-block.md)
- [x] 04: The join — [details](04-the-join.md)
- [ ] 05: A conflict on the way in — [details](05-a-conflict-on-the-way-in.md)
- [ ] 06: Cut from the highest settled stage — [details](06-cut-from-the-highest-settled-stage.md)
- [ ] 07: The words — [details](07-the-words.md)
