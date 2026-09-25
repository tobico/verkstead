# 03. The flag turns the grant on

## What to build

Under `--desktop`, a refused `tailscale serve` press raises the platform's own
password dialog and then presses again — a re-try being the whole of what the
operator grant buys. Without the flag it hands back the `sudo tailscale set
--operator=…` line exactly as it always has. The onboarding wizard's install run
goes the same way, raising its one elevated command through the same handle,
because it is one dialog on one machine and takes one handle.

**The flag and a screen, both.** A sidecar started over SSH or in a container is
the app with nobody at the machine, and a dialog nobody can see is a press waiting
on a dismissal that cannot arrive — so the grant is installed only where the flag
was given *and* the probe from task 02 says there is somewhere to draw, and where
it is not installed the line is shown, which is what a machine with nobody at it
wanted said anyway. Without the flag it is never installed whatever the screen
says: a plain `serve` is a daemon, and the line is the behaviour the Remote access
pane had before there was an app at all.

**Nothing below the choice changes.** The `Elevate` seam and `Raised` stay what
they are (ADR-0020), so the pane's own code is untouched: the re-try, the reading
that makes a dismissed dialog and a failed asking one thing, and the line that
stands where the grant was refused a second time. What is new is which of two
values the server starts with, decided from the value task 01 put on the seam —
so this task is that decision and its wiring, not a change to the asking.

**What proves it is the choice where it is made, plus the suites that already
press.** The remote suite hands a stand-in dialog straight into the tailnet
reading and presses a stand-in `tailscale` whose grant is a file; the onboarding
suite does the same for the install run. Neither goes through a `Config` or a
command line, so neither can see the flag, and neither has to. The flag's own
contribution is therefore asserted as the answer itself — the way the tray app
already asserts its own screen question, which is a test that moves or goes with
the code it is about.

## Acceptance criteria

- [ ] The server answers from the flag and the screen alone: the flag with a
      screen installs the grant, the flag without one does not, and no flag never
      does whatever the screen says.
- [ ] The remote suite's stand-in `tailscale` sees the elevated re-try where a
      grant is installed and the bare refusal with its `sudo` line where it is
      not, a dismissed dialog among the latter.
- [ ] The onboarding wizard's install run raises its one elevated command through
      the same handle, so a sidecar with a screen elevates it and a plain `serve`
      is unchanged.
