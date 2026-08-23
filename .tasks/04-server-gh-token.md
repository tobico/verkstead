# 04. The server's own gh uses the token

## What to build

Every host-side `gh` invocation the server makes — pull request views, checks,
comments — runs with `GH_TOKEN` from `secrets.yaml`, so the one configured token
authenticates everything. The file is read at call time, not held from startup,
so a token saved or rotated through the UI applies to the next `gh` call without
a restart.

With no token in the file, the invocation runs as it does today and falls back
to whatever login the host's `gh` has — the existing not-logged-in Trouble
reporting already says what to do, and its message should now point at the
settings page rather than at `gh auth login`.

## Acceptance criteria

- [ ] With only the token configured — no gh login in the service home — PR
      views, checks and comments all work
- [ ] The stub-gh tests cover both token present (the variable reaches the
      child) and absent (no variable set)
- [ ] The not-logged-in message names the settings page as the fix
