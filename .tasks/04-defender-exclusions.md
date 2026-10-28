# 04. Defender exclusions on the runner

## What to build

The Windows runner image scans every file written with Defender's real-time
protection, which taxes a compile that writes thousands of object files and a
suite that spawns processes and writes temporary directories by the hundred.
The runner is a throwaway VM whose only purpose is this run, the way the Linux
job's runner is when that job relaxes AppArmor for the sandbox suite.

A step at the head of the Windows job, before the checkout writes anything,
excludes from real-time scanning the directories the build and the tests write
to: the workspace, the cargo target directory under it, the runner's temp
directory, and the cargo and rustup homes under the runner's user profile.
PowerShell's `Add-MpPreference -ExclusionPath` is the verb, one call per
directory, and the step's comment says why the exclusions are safe here and
nowhere else. Where the image turns out to carry no Defender, or to refuse the
preference, the step says so plainly and fails rather than passing quietly: a
measurement against a step that did nothing is no measurement.

The tests themselves are the measurement. Record both legs' build and test
times in the ledger from the run that carried this commit, beside the row
before it, and say in the ledger what the exclusions bought. If they bought
nothing measurable, the step still stays unless it cost time: the ledger says
which.

## Acceptance criteria

- [ ] The Windows job's first step excludes the workspace, the target directory, the runner temp and the cargo and rustup homes from real-time scanning, and fails aloud where it cannot
- [ ] Both legs are green with the step in place
- [ ] The ledger carries both legs' build and test times from the run that measured this commit, and a line on what the exclusions changed
