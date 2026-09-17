# 04. OpenCode's built root

## What to build

An OpenCode session is given its two XDG directories built rather than joined
whole, using the machinery task 02 made general. Today the Profile home's
`.config/opencode` and `.local/share/opencode` are both joined into the fresh
HOME.

- **The config directory is built either way**, and holds only an `opencode.json`
  Verkstead writes. It carries the account's `provider` key and nothing else: no
  `mcp`, `plugin`, `agent`, `command`, `instructions` or themes. Read it from
  whichever config file the account has (`opencode.json` or `opencode.jsonc`, the
  latter with comments). An account with none, or one that does not parse, gets
  a file with no provider. The account's `node_modules/`, `package.json`, agents
  and commands are not there.
- **The data directory, when the Profile's memory switch is on, is joined whole.**
  The database runs in write-ahead-log mode with `opencode.db-wal` and
  `opencode.db-shm` beside it (checked on opencode 1.18.30 during planning), and
  a database linked file by file will not open. The credentials, `mcp-auth.json`,
  snapshots and logs travel with it.
- **When the switch is off**, the data directory is the root's own. `auth.json`
  alone is linked into it and handed back at session end where the session
  replaced it, on all three platforms. The database is the session's own and
  starts empty.

`OPENCODE_DB` and the shell-timeout variable are set as today. A relative
`OPENCODE_DB` resolves against whichever data directory the session has.

**Discovery** follows task 01's rule. The records reader opens the pinned
`opencode.db` in the account's data directory when memory is on, and in the
root's on the host when it is off. The rest of the reader is unchanged.

The onboarding wizard still finds an OpenCode account by both directories
existing under a home.

## Acceptance criteria

- [ ] Inside an OpenCode session, the config directory holds only the written `opencode.json`, with the account's `provider` and nothing else of the account's config. The three boundary suites assert it.
- [ ] With memory on, the data directory inside is the account's, and the session's records are found in the account's `opencode.db`.
- [ ] With memory off, only `auth.json` of the account's data directory is visible inside. The session's records are found in the root's `opencode.db` on the host, and the account's database is unchanged by the session.
- [ ] With memory off, a login made or replaced inside reaches the account's `auth.json` at session end on all three platforms.
- [ ] An account whose config is `opencode.jsonc` with comments still has its `provider` carried over.
