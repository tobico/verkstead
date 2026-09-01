# 02. Launch on Startup on macOS

## What to build

The second arm of `startup`, beside the XDG one. Everything above that module
already works the way this stage needs and does not change: the registration is
the state rather than a copy of it, the box on the tray menu is drawn from
reading it, checking writes it and unchecking removes it, and every launch
rewrites it with the path of the executable that is running while it is there.
What is missing is the macOS half of the platform question.

A **LaunchAgents plist**, written into the user's own agents directory and named
for the app id the way the XDG entry is — one file that is there or is not,
which is the same shape the Linux arm already has, and which works for a binary
run from anywhere rather than only for one inside its bundle. What it starts is
an ordinary launch with the browser left alone, because a login is not a moment
to be handed a browser window — the same rule the XDG entry follows and for the
same reason.

A machine with nowhere to keep one is not a failure: it is the greyed menu item
the crate already draws.

## Acceptance criteria

- [ ] Checking the box registers the running executable and unchecking removes
      it, with the registration as the only state kept anywhere
- [ ] A launch rewrites a registration that names somewhere else, and registers
      nothing that was not registered already
- [ ] Unit tests over a temporary directory, as `startup`'s Linux ones are, so
      the arm a Linux runner will never run is still an arm its tests call
