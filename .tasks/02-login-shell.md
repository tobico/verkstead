# 02. The login shell

## What to build

A terminal runs the shell its human would get at the machine, and the packaged
install has one to give.

The server reads its own user's login shell from the passwd database and runs
that where it is usable: an absolute path to a file that exists and is neither
`nologin` nor `false`. Anything else — a missing entry, a shell that is not
there, a system user's `nologin` — falls back to `/bin/sh`, the one path every
Sandbox is certain to have a shell at. There is no setting for it. The Sandbox
reaches whichever was chosen because `/nix`, `/usr`, `/bin` and
`/run/current-system` are bound in on every platform.

Inside, `SHELL` names the shell that was chosen, where a session's Sandbox says
`/bin/sh`. The shell is started interactive and not as a login shell: a login
shell reads the system's profile, which on NixOS rebuilds `PATH`, and the
invariant that the running server's own `verkstead` is first on a Sandbox's
`PATH` has to stand in a terminal as it does in a session.

The resolution is a function of the passwd answer and whether the file is
there, so that it is tested with those as values rather than by changing the
account the tests run under.

On the nix module, `services.verkstead.shell` — a package, `pkgs.bash` by
default — sets the `verkstead` service user's shell, so a packaged install's
terminals come up in bash rather than falling back. Switching it to fish is one
line of configuration, and the option's description says so.

## Acceptance criteria

- [ ] A checkout run under a user whose passwd shell is, say, zsh opens zsh in
      the terminal, and `echo $SHELL` inside names it.
- [ ] A user whose passwd shell is `nologin`, `false`, missing from the
      Sandbox or absent from passwd gets `/bin/sh`, and `SHELL` says so.
- [ ] `verkstead` is first on `PATH` inside the terminal, as it is in a
      session.
- [ ] Unit tests cover each passwd answer above without depending on the
      account the suite runs under.
- [ ] The nix module's `services.verkstead.shell` defaults to bash, sets the
      service user's shell, and the module evaluates with it set to fish.
