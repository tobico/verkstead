# 06. git on a Mac counts only with the command line tools

## What to build

Every Mac has `/usr/bin/git`. Without Xcode's command line tools it is a stub
that opens Apple's install dialog and exits non-zero, and the wizard's probe —
a `PATH` walk — counts it present, so a Mac with no tools ticks git and hands
each session a `git` that fails. ADR-0016 (*Macs*) settles that **git on a
Mac counts only with the command line tools**.

The git row gets what the Linux sandbox row already has: a `PATH` walk plus a
run. On a Mac, where the resolved `git` is Apple's stub — the one under
`/usr/bin` — the row is present only where `xcode-select -p` exits zero,
naming the tools' directory; where it exits non-zero, the row is absent with
the words it printed under it, the way a `bwrap` that would not run leaves its
stderr under the sandbox row. A `git` that resolves anywhere else — Homebrew's,
nix's — is present as it always was. The probe never runs the stub itself:
that is what opens the dialog, on a machine nobody is looking at, every ten
seconds. Linux and Windows are unchanged.

Both Mac tabs' git rows lead with `xcode-select --install`; the Apple-silicon
tab keeps `brew install git` as the alternative under it, since Homebrew's git
is the one a developer's Mac usually runs. The Apple-silicon planner's git
unit stays `brew install git` — a Mac with `brew` has the tools, Homebrew
needing them — and on a Mac with neither the row goes to the hint screen,
where it says to run Apple's dialog by hand.

Stub `xcode-select` on the stated machine's `PATH` the way `bwrap` is stubbed:
one that answers `-p` with a path and exits zero, one that exits non-zero with
a line. Gate a real-Mac assertion on `target_os = "macos"` beside the stubbed
ones, as the sandbox row's test does, so the argv the stub agrees with is
checked against a program the runner has.

## Acceptance criteria

- [ ] A stated Mac whose only `git` is under `/usr/bin` shows the row absent
      with `xcode-select`'s words where the tools are missing, and present
      with the resolved path where `xcode-select -p` succeeds; a `git`
      resolved elsewhere is present without the run.
- [ ] The probe never executes the resolved `git`; the stubbed `git` in the
      test fails loudly if run.
- [ ] Both Mac tabs' git rows show `xcode-select --install`, the Apple-silicon
      one with `brew install git` beneath it; the web tests say so.
