# 04. Coming home

## What to build

A stage moved back to its home lands on its tombstone as a transfer back does
today — written over wholesale under the id it already has — and the roadmap
reads the same afterwards.

**What a round trip disturbs today:** the landing renumbers every Event and
issues a fresh `stage_joinings` place behind whatever is waiting. The chain is
ordered by the pull request's Event id and the queue by its place, so a stage
that came home would move up the chain and to the back of the queue. Keep both:
its place in the chain and its place in the queue are what home already held
for the tombstone, and a return keeps them. A stage moved away and back with no
join in between is unaffected either way.

Once home, it is no longer a copy away from home: it reports nothing and asks
home nothing, and the scheduler drives its roadmap on its settle as usual.

**The glossary.** Rewrite `CONTEXT.md`'s **Transfer** entry where it says it is
*being revised by ADR-0029* for this stage's half — a Stage moved away reports
its lifecycle to its home device, whose tombstone follows it, and finishes there
by asking home to join — leaving the half that stages 03 and 06 make true.

## Acceptance criteria

- [ ] A stage moved away and back reads the same standing at home before and after.
- [ ] Chain order and queue order are unchanged by a round trip made after the stage joined.
- [ ] A stage that has come home sends no reports, and its settle starts what stood on it locally.
- [ ] The Birth Key and Transfer glossary entries say what is now true.
