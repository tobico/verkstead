# 05. macOS

## What to build

The run on a Mac, where every install is Homebrew's and Homebrew refuses to
run as root. The sandbox row is nothing to install and never ticks.

**With brew present**, the ticked rows are one unit each, run as the user:
`brew install git`, `brew install --cask claude-code`, `brew install --cask
codex`, `brew install opencode`, `brew install gh`. Homebrew's prefix is on the
Apple floor already, so nothing is written to `session_path`. Grok is xAI's
installer from task 04, the same on a Mac.

**Without brew**, two units come first. An elevated one, through the
`osascript` arm, makes Homebrew's prefix — `/opt/homebrew` on Apple silicon,
`/usr/local` on Intel — and hands it to the user, which is the whole of what
the installer would have called `sudo` for. Then Homebrew's own install script
runs as the user with `NONINTERACTIVE=1`, finds its prefix writable, and asks
for nothing. A prefix the elevated step could not make, or an installer that
exits non-zero, fails every brew row at once with the line it printed, and the
hint screen shows the Homebrew install line.

The suite asks about this on a stated Mac with stub `brew`, a stub installer
and a recording `Elevate`: which arm is a value rather than a `cfg`, so the
Linux runner can prove the order of the units and the command each is.

## Acceptance criteria

- [ ] On a stated Mac with `brew` present, ticking git and Claude runs two brew
      units as the user, raises nothing, and the rows read Present under the
      Homebrew prefix.
- [ ] On a stated Mac without `brew`, the run is the elevated prefix step, the
      installer as the user, and then the brew units, in that order; a refused
      prefix step fails every brew row and runs no installer.
