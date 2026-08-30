# 01. Settings-held sandbox binds reach sessions

## What to build

Let `config.yaml` carry Sandbox Configuration binds, and have every session
spawn compose them with the CLI/env set.

The file takes a `sandbox_binds` key holding a flat list of strings in exactly
the grammar `--sandbox-bind` takes — `/abs/path` for a bind every Sandbox
gets, `name=/abs/path` for one only sessions in the Repo registered under
that name get:

```yaml
sandbox_binds:
  - /var/cache/verkstead-node
  - verkstead=/var/cache/something
```

One grammar across flag, env and file — reuse the existing bind parser rather
than writing a second one.

Semantics, all settled in the grilling:

- **Union.** The CLI/env binds and the settings binds compose; neither
  overrides the other. The CLI set keeps today's behaviour exactly: resolved
  once at startup, a missing path refuses to start.
- **Re-read per spawn.** The settings set takes the settings side of the
  line: read at the moment a session spawns, like the git author and the
  build-cache switch, so a change applies to the next session without a
  restart.
- **Never fatal.** A settings bind whose path does not resolve at spawn — the
  directory is missing, or the hardened unit cannot see it — is skipped with
  a logged warning naming the entry, and the session launches without it. A
  malformed entry (relative path, empty repo name) is treated the same way:
  logged, skipped, never a session that will not start. This mirrors how the
  settings files already treat everything else.

Follow the config-file idioms already in place: unknown keys ignored, blank
entries dropped, absent key means nothing configured.

## Acceptance criteria

- [ ] A bind added to `config.yaml` is mounted read-write in the next
      session's sandbox, with no server restart
- [ ] A `name=/path` settings bind reaches only sessions of the Repo
      registered under that name, composed after the global ones
- [ ] A settings bind whose directory is missing is skipped with a logged
      warning and the session launches; a malformed entry likewise
- [ ] CLI/env binds behave exactly as before: fail-fast at startup, composed
      into every sandbox alongside the settings-held set
