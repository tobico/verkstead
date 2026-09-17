# 02. Codex's built root

## What to build

A Codex session runs in a `.codex` that is Verkstead's own, built under the
Conversation's profile (`homes/<id>/.codex`) as each session starts. Today the
whole Profile home is joined over `~/.codex`. It becomes an allowlist, the way
stage 01 did it for Claude:

- **`auth.json`, linked:** a bind on Linux, a symlink on a Mac, a hard link on
  Windows. A login or token refresh from inside lands in the account. An account
  with no `auth.json` gets none in the root, and the file a session logs in with
  is handed back as it ends.
- **The memory store, joined read-write only when the Profile's memory switch is
  on:** the `sessions/` directory, where rollouts go, and the `memories/`
  directory, which holds `MEMORY.md` and `memory_summary.md` (the human chose
  these two). Each is made in the account first where missing. When the switch is
  off, both are the root's own and start empty.
- **Not `archived_sessions/`, and none of the SQLite databases** at the top of
  `~/.codex`. Codex 0.154 keeps `state_5.sqlite`, `memories_1.sqlite`,
  `thread_history_1.sqlite`, `logs_2.sqlite`, `goals_1.sqlite` and
  `queue_1.sqlite` there. A database linked file by file loses its
  write-ahead-log siblings and will not open, so these are always the session's
  own. **Check on the real install** (`codex` 0.154 is on this machine) that a
  fresh state database next to a shared `sessions/` and `memories/` does not make
  Codex summarise all past rollouts again, or do other paid work at startup. If
  it does, stop and say so rather than working around it.
- **A `config.toml` Verkstead writes**, carrying only `model_provider` and
  `model_providers` from the account's own `config.toml` where it has them.
  Nothing else: no `mcp_servers`, `profiles`, `projects`, hooks or notify. An
  account with no file, or one that does not parse, gets an empty one. It is not
  written back.
- **Everything else is absent:** `rules/`, `skills/`, `AGENTS.md`,
  `history.jsonl`, `archived_sessions/`, logs, and whatever Codex adds next.

`cli_auth_credentials_store=file` and the Worktree trust stay on the launch line,
as ADR-0011 put them.

**This task also makes stage 01's Claude-only machinery work for any harness**,
so tasks 03 and 04 reuse it rather than repeat it:

- **Root building.** The root module is Claude's alone today. Give it a shape any
  harness's allowlist can be described in: linked files, directories joined by
  the switch, written files.
- **Linux root sharing.** A launch into a Conversation whose root something still
  runs in shares that root rather than emptying it (the `sharing` module). This is
  keyed per Conversation for Claude today, and has to hold for a Codex root too.
- **The login write-back at session end, on all three platforms.** A linked file
  the agent replaced by rename, rather than wrote through, is written back over
  the account's own. One still the same file as the account's is left alone. The
  identity check that stage 01 uses for `.credentials.json` decides. Find out how
  Codex saves `auth.json` (in place or by rename) and say which in a comment.
- **The Windows grant.** It covers the root, each joined directory, and an entry
  of its own on the linked credentials file, which is taken back with the
  Conversation's other entries. Nothing covers the whole account home. Also
  update the check for files on another volume (`across_volumes`) to name the
  hard-linked credentials.

**Discovery** follows task 01's rule. The rollout finder looks in the account's
`sessions/` when memory is on, and in the root's `sessions/` on the host when it
is off. Its match (Worktree plus launch time) is otherwise unchanged.

What a Profile stores, and how the onboarding wizard finds a Codex account
(`account_in_home`), do not change.

## Acceptance criteria

- [ ] Inside a Codex session, `~/.codex` holds `auth.json`, the written `config.toml`, and (with memory on) the account's `sessions/` and `memories/`. The account's `rules/`, `skills/`, `AGENTS.md`, `history.jsonl` and databases are not visible. The Linux, macOS and Windows boundary suites assert it.
- [ ] The written `config.toml` carries the account's `model_provider` and `model_providers` and none of its `mcp_servers`. An account with no `config.toml` still launches.
- [ ] With memory off, `sessions/` and `memories/` inside are empty and the account's are untouched. The session's rollout is found under the root on the host.
- [ ] With memory on, the rollout is found in the account's `sessions/`, as today.
- [ ] A login written or replaced inside reaches the account's `auth.json` at session end on all three platforms. A file still linked to the account's is left alone.
- [ ] On Linux, a Conversation Terminal opened while a Codex session runs shares its root rather than emptying it.
- [ ] Whether a fresh state database triggers re-summarising on Codex 0.154 was checked, and the result is written down in a comment beside the allowlist.
