# 02. Settings-held watched paths widen the boundary

## What to build

Let `config.yaml` carry Watched Paths, and have every admission decision
check the union of the CLI/env set and the settings set — then stop requiring
the flag, so a standalone install can start with nothing configured and be
set up entirely from the workbench.

The file takes a `watched_paths` key, a flat list of absolute paths:

```yaml
watched_paths:
  - /home/you/src
```

Semantics, all settled in the grilling:

- **Union, re-read per use.** Repo registration and Agent Profile admission
  check CLI/env paths plus whatever `config.yaml` holds at that moment, so a
  watched path added at runtime admits from the next request on. The CLI/env
  set keeps its startup resolution and fail-fast checks.
- **A settings path that does not resolve admits nothing and refuses nothing
  else.** Missing directory, relative path, not a directory — the entry
  simply covers nothing (logged), the boundary stays closed, and nothing
  about startup or other entries fails. The existing fail-closed rule holds
  throughout: no watched path anywhere means every path is outside.
- **`--watched-path` stops being required.** The server starts with none
  configured anywhere — fail-closed, registering nothing — because a fresh
  standalone install has to be able to reach the settings page before it has
  any configuration. The startup log should still say what is watched,
  including that nothing is.
- **Removal is free.** Admission is checked at registration and never again,
  so taking a watched path out of settings breaks nothing already registered;
  it only stops future registrations there. Nothing needs building for this
  beyond not building anything — no cascade, no refusal.

The nix module is untouched here: it keeps passing flags and keeps its
build-time assertion that `watchedPaths` is non-empty (settings-only paths
cannot function under the hardened unit, whose namespace is built from the
option — task 06 records that).

## Acceptance criteria

- [ ] A repo registers from inside a watched path that was added to
      `config.yaml` after the server started
- [ ] The server starts with no `--watched-path`, no env var and no settings
      entry, and admits nothing until a watched path is configured
- [ ] A settings watched path that does not resolve covers nothing, is
      logged, and neither startup nor any other entry is affected
- [ ] Removing a settings watched path leaves already-registered Repos
      working and stops new registrations under it
