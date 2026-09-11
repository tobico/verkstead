# Windows CI speedup

The `Windows` CI job takes ~36 minutes against ~11 for every other job, and
the whole gap is the test step. Investigation traced it to a single cause: when
a session's sandbox boundary is set up, it writes a "step" (traverse +
read-attributes) access-control entry on every ancestor of every path the
session may reach — including the drive root `C:\`. On the elevated GitHub
runner that call *succeeds* and makes Windows re-propagate inheritance across
the whole C: volume, costing ~340s per boundary — 96% of a boundary's setup and
the bulk of the 30-minute test step. On a normal (unelevated) machine the same
call is denied in microseconds, so this is chiefly a CI artifact, but the entry
is redundant everywhere: the session account is a member of Users and can
already traverse those public directories.

This backlog removes that redundant work at the source, takes Windows Defender
out of the CI build tree, and runs the Windows suite through cargo-nextest so
its ~45 test binaries run in parallel instead of serially — then measures the
result against the Linux job and shards only if the numbers demand it. The
target is Linux-parity (~10 min). All four tasks land in one pull request; the
sandbox change (task 01) is security-sensitive boundary code (ADR-0014) and
belongs in its own reviewable commit.

## Tasks

- [ ] 01: Skip a boundary step the account can already make — [details](01-skip-redundant-step.md)
- [ ] 02: Keep Defender out of the CI build tree — [details](02-defender-exclusions.md)
- [ ] 03: Run the Windows suite through cargo-nextest — [details](03-nextest.md)
- [ ] 04: Measure against Linux parity, shard only if needed — [details](04-measure-and-shard.md)
