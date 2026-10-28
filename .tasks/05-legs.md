# 05. Deal the suite over the legs the budget needs

## What to build

Legs are free on this repository and each one is a four-core machine of its
own, so the test minutes come down by dealing the suite over more of them. The
matrix grows from two partitions to the count that puts the slower leg's whole
job near seven minutes — the budget is a hard ten, and two legs of one run have
differed by two minutes of build already — up to ten. Start from the ledger's
figures after task 04: a leg's test time scales with the number of legs, its
build time does not, and the serial session-account group, about 420 seconds
of test time in all, is dealt out with everything else by nextest's `slice`
partitioning and is not a floor under any one leg.

The job's name, the partition expression and the matrix move together, and the
doc-tests stay on the first leg alone, as they are. Each leg still creates the
session account and installs nextest and sccache for itself. The account's
concurrency ceiling has room: the workflow's five other jobs and ten legs are
fifteen, and the release workflow that calls this one adds its own on top only
when a release runs.

Measure, then write the leg count and the reasoning into the ledger: the test
time per leg the count was chosen from, and what each leg came to. If the
slower leg is still over seven minutes at ten legs, say so in the ledger rather
than adding an eleventh — a larger runner was the human's fallback only after
every free lever, and tasks 06 and 07 are still to come.

## Acceptance criteria

- [ ] The Windows matrix runs the number of legs the ledger's reasoning names, no more than ten, with the doc-tests on the first leg only
- [ ] Every leg's whole job is under seven minutes on the run that landed the count, or the ledger says by how much it is not and why no more legs were added
- [ ] The ledger carries the leg count, the reasoning, and every leg's build and test time from that run
