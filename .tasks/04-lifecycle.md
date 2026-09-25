# 04. Lifecycle

## What to build

The four ways this app starts and stops: a second launch that is the first window
brought forward, an address somebody else holds that is a dialog and an exit, a
sidecar that ends taking the app with it, and a window that quits when it is
closed.

**The port is fixed and a foreign listener is fatal** (ADR-0012, carried into
ADR-0020 for the same reasons). `127.0.0.1:8422` is what the CLI defaults to and
what `tailscale serve` is set up against, so fronting whatever is already there
would conflate two Verksteads over possibly different Data Directories, and
picking a free port would leave the human's bookmark pointing at the wrong one.
Both were rejected twice. What is left is saying so and stopping: a dialog naming
the address, in the app's own words — the Rust app's wording is worth reading —
and a non-zero exit, with no sidecar started and nothing made.

**The single-instance lock comes first, and it is what makes the probe
unambiguous.** Electron's own lock is what a second launch of the *app* meets:
it hands off to the first instance, which shows and focuses its window, and the
second exits without starting anything. So by the time the address is probed, a
listener on it cannot be a second copy of this app — it is a daemon the machine
starts for itself, a `verkstead serve` in a terminal, or something else entirely,
and all three are the same dialog. Probe by asking whether anything answers there
rather than by taking the address and letting go of it again: the sidecar is what
binds it, moments later.

**The server ending is the app quitting**, carried over from the tray app. The
child exiting at all — cleanly, killed, or crashed — quits the app, and what it
exited with is reported before the app goes.

**And closing the window quits, for now.** Stage 03 makes that a choice on the
Desktop page and a tray to keep running in; here it is quit on every platform,
including the Mac, where the Dock behaviour arrives with the rest of 03.

## Acceptance criteria

- [ ] A second `pnpm start` brings the first window forward and exits without
      starting a sidecar of its own.
- [ ] A listener planted on `127.0.0.1:8422` before launch is a dialog naming the
      address and a non-zero exit, with no sidecar started.
- [ ] Killing the sidecar quits the app, and the app's log says what it exited
      with.
- [ ] Closing the window quits the app and leaves no `verkstead` process behind.
