# 02. The run: distro packages through the elevated seam

## What to build

An install run the wizard can start, watch and cancel, covering the rows a
Linux distribution packages: the sandbox (`bubblewrap`), `git`, `gh`, and the
npm harnesses Codex and OpenCode with `nodejs` and `npm` added where they are
missing. The vendor installers, macOS and Windows are tasks 04 to 06; this
task is the run itself and the wire it is watched over, proven end to end on
Ubuntu, Fedora, Debian and Arch with stubs.

**Starting.** A POST under the onboarding endpoint takes the ticked
dependencies and returns at once. It is refused while onboarding mode is off,
and while a run is already going.

**Watching.** The onboarding reading gains what the install screen draws.
Each dependency row carries an install state beside its presence — installing,
failed with why, or nothing — and the reading carries the run as a whole:

```yaml
run:
  phase: Asking | Installing | Done      # what the status line is about
  status: Waiting for the password dialog on <hostname>
  done: 1
  total: 3                                # units finished over units ticked
  cancelling: false
```

The hostname comes from a small hostname crate. The status line's phases are
*waiting for the password dialog on <hostname>*, *installing <packages>*, and,
from task 04, *running <vendor>'s installer*.

**Running.** The run is a sequence of **units**, and a unit is one command:
the first is the elevated batch, and each vendor installer after it is one of
its own. The elevated batch is one command per distribution — `apt-get install
-y`, `dnf install -y`, `pacman -S --noconfirm` — naming every ticked package,
and where an npm row is ticked the `npm install -g` for it is joined onto the
same shell line, so that one press is one dialog. It is raised through the
`Elevate` handle the desktop app hands the server at startup, on a thread that
can block on a human reading a dialog. A server handed no handle, and a
distribution with no command — NixOS and other Linux — fail every ticked row
at once with a line saying so, and never ask. A dismissed dialog or a
non-zero exit lands on every row of that unit as failed, with the first line
of stderr the way the bwrap row carries its trouble today. After each unit the
rows are probed again, so a row goes present as soon as its unit lands.

**Cancelling.** A second POST cancels: the unit under way finishes, the units
not started are skipped, and their rows read failed with *cancelled*.

**Nothing runs while nobody is looking** stays true: a run is started by a
press and ends by itself, and the reading between presses is the probe it
always was.

The suite stands the router up over a stated machine with a stub package
manager on its `PATH` and a stub `Elevate` that records the command it was
handed, so every distribution and both arms of the handle are asked about on
the Linux runner.

## Acceptance criteria

- [ ] On a stated Ubuntu with a stub `apt-get` and a recording `Elevate`,
      ticking the sandbox, git and Codex raises exactly one command naming
      `bubblewrap`, `git`, `nodejs`, `npm` and the npm install for Codex; the
      rows read installing while it runs and present once the stub has put the
      programs on the `PATH`.
- [ ] A refused dialog leaves every ticked row failed with the refusal's line
      under it, and a router handed no `Elevate`, or a stated NixOS, fails them
      the same way without raising anything.
- [ ] Cancel during a run of three units finishes the first, skips the other
      two as *cancelled*, and the run reads done.
- [ ] The start POST is refused once the mode is off and while a run is going,
      and the golden fixtures for a run in progress and a run with failures are
      written for the viewer's tests.
