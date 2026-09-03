# 06. The startup probe

## What to build

At startup the server probes the image it is about to equip sessions with:
run it once with `guide`, in the environment a session would get rather than
the server's own — an AppImage points its loader at bundled libraries with
`LD_LIBRARY_PATH`, and a probe inheriting that would pass on exactly the
machine where sessions fail. A failed probe is handled the way a missing image
already is: no session starts, the startup log says why, and each refused
session is named as it is refused.

## Acceptance criteria

- [ ] A server whose image exits non-zero on `guide` under the session's
      environment starts no session, and the log names which session that
      cost.
- [ ] A healthy image is probed once at startup rather than at every spawn.
- [ ] A test proves the probe does not inherit the server's own environment.
