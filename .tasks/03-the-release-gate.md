# 03. The release gate

## What to build

A tag whose version the workspace manifest disagrees with fails the release run
before anything is built. `v0.1.1` shipped a binary reporting `0.1.0`, so a
fresh install offered to update itself to the version it already was; nothing in
the pipeline had anything to say about it, and `releasing.md` says as much in so
many words.

**The check is the run's first job**, and every job that builds or publishes sits
behind it — today that is the one job the rest already need, so putting the gate
in front of that one puts it in front of all of them. What it compares is the
tag without its `v` against the version the workspace manifest reports, and what
it says when they differ names both, because a line saying only *mismatch* sends
somebody to look up the two numbers the run already had.

**The hyphen rule is the one `releasing.md` already states.** A tag with a hyphen
in it is semver's spelling of a pre-release, so `v0.1.0-rc.1` and `v0.1.0-rc.2`
are both a manifest reading `0.1.0` — which is the same rule the Windows
Installer version already forces from the other end. A run started by hand is a
rehearsal and may be started against a branch, which has no version to disagree
with; there the gate has nothing to compare and stops nothing.

**The manifest is not bumped here.** `v0.1.1` stays as it shipped — a tag cannot
be re-released — and the next is `v0.1.2`.

**And the procedure becomes a step rather than a check.** `releasing.md`'s
*Before you tag* says today that the version has to match and that nothing
checks it. Something checks it now, and what the human does about it is bump the
version and commit it before tagging, which is an action in the procedure rather
than a thing to remember to verify.

## Acceptance criteria

- [ ] A `v*` tag whose version disagrees with the workspace manifest's fails the
      run at its first job, with a line naming what the tag said and what the
      manifest says; nothing is built.
- [ ] `v0.1.0-rc.1` against a manifest reading `0.1.0` passes, and a run started
      by hand against a branch is not stopped.
- [ ] `releasing.md` no longer says nothing checks this, and its procedure bumps
      the version as a step before tagging.
- [ ] The workspace manifest's version is unchanged by this task.
