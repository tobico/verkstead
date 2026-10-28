# 08. The hard timeout, the final ledger and the Linux issue

## What to build

The budget is kept by a check that goes red when it is missed, and this task
turns that check on once the figures say it will hold. Read the ledger's last
row: if every leg's whole job is under seven minutes, the Windows job gets
`timeout-minutes: 10` and the ledger says which run was the last to prove it.
If a leg is still between seven and ten, the timeout still goes on — the
human chose a hard ten over a margin — and the ledger says what the risk is
and which lever is left. The leg count is trimmed where the final figures
leave room: a leg under four minutes is a runner doing little, and the count
task 05 chose was for the suite before tasks 06 and 07.

Then the words catch up with the work. The Windows job's comment is read from
the top and brought in line: what it says about two legs, about nextest's
partitioning, about the build step and about sccache has to describe the job
as it now is, with the ledger's final table and the budget at its head.
`docs/development.md` says the same where it describes the Windows job and how
the suites are run. The pull request's description says what the branch did
and carries the final figures.

And the Linux `Rust` job, 28 minutes on the same run that measured the
baseline against the 14 or so its own comment records, gets a GitHub issue:
the measurements from this work, what transfers for free — nextest in place of
`cargo test` running one binary after another, the merged test binaries and
the line tables it already has, the template repositories — and what does not,
the Linux job having two cores and building the workspace alongside the run.
The issue is the follow-up the human asked for; it proposes, and this branch
does nothing to the Linux job. Link it from the pull request description.

## Acceptance criteria

- [ ] The Windows job carries `timeout-minutes: 10`, and the ledger names the last run before it in which every leg passed under seven minutes, or says by how much one did not
- [ ] The Windows job's comment and `docs/development.md` describe the job as it now runs, and the ledger's final table is at the comment's head
- [ ] A GitHub issue for the Linux job exists with the measurements and the transferable levers, and the pull request description links it
