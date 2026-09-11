# 04. Measure against Linux parity, shard only if needed

## What to build

With the boundary fix (01), Defender exclusions (02), and nextest (03) in
place, measure the `Windows` job's end-to-end wall-clock and compare it to the
`Rust` (Linux) job, whose ~10-minute run is the parity target.

If Windows is at or near parity, leave it as a single job — no sharding. If it
is still well above the target, split the test run into a 2-way partition using
nextest's own partitioning across two matrix legs of the job. The repository is
public, so extra standard runners are free; the only cost of a shard is a
second cached build (~4.5 min) and a less legible workflow, so add one only if
the measured time earns it. Record the decision and the numbers behind it.

This task is where the "parity even at some cost to test realism" goal
(settled during grilling) is checked against reality — it is deliberately last,
because its input is the measured effect of the three changes before it.

## Acceptance criteria

- [ ] The `Windows` job's end-to-end time with 01–03 applied is measured and recorded against the Linux job's
- [ ] A 2-way shard is added only if the measured time is above the ~10-minute target, and the reasoning is written down where a reader will find it (PR description or a workflow comment)
- [ ] If no shard is added, that decision and the number that justifies it are recorded too
- [ ] Whatever the outcome, the Windows job remains legible — no sharding kept that isn't earning its place
