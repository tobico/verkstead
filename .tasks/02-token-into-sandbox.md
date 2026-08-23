# 02. The token reaches the sandbox

## What to build

A settings-file layer that reads `secrets.yaml` from the Data Directory, and the
first consumer of it: at session spawn, a present GitHub token becomes
`--setenv GH_TOKEN` on the sandbox command, and the read-only bind of the host's
gh config (`~/.config/gh`) is removed — along with the machinery that located it
via `XDG_CONFIG_HOME`. `gh` honors `GH_TOKEN` natively, so no gh files exist
inside the sandbox at all.

The schema is flat:

```yaml
# secrets.yaml
github_token: ghp_...
```

The file is read at each session spawn, so a rotated token applies from the next
session — running sessions keep the environment they started with, which the
grilling accepted. There is no environment-variable override on the server side:
the file is the single source. A missing or malformed `secrets.yaml` must never
stop a session starting — no token simply means no `GH_TOKEN` set, and `gh`
inside says it is not logged in.

## Acceptance criteria

- [ ] With a token in `secrets.yaml`, a session sees `GH_TOKEN` and has no
      `~/.config/gh`
- [ ] With no file, an empty file, or unreadable YAML, sessions start with no
      `GH_TOKEN` and nothing else breaks; the malformed case is logged
- [ ] The gh-config bind and its host-side path resolution are gone from the
      sandbox surface, its tests, and the vm test's expectations
