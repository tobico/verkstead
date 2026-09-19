# 02. The other three roots and the memory switch

## Goal

Codex, Grok Build and OpenCode sessions run in built roots by the rule stage
[01](01-claude-root.md) set for Claude: credentials linked, the session store
shared, a configuration file Verkstead writes, nothing else of the human's. And
every Profile carries a **memory** switch, on by default, that says whether the
account's store is shared into the root or the session starts with one of its
own — on every harness, Claude's included. Transcript discovery finds every
log where it always did.

## Decisions in force

- **The same rule for all four harnesses.** The human chose consistency over a
  Claude-only rendering: a built root is what a session's account *is*, whatever
  the harness. ADR-0011's *A Profile is one home directory, except Claude's
  pair* still describes what a Profile *stores*; what a session is given is the
  amended paragraph beside it.
- **The allowlist per harness**, from the vendors' documentation and Verkstead's
  own transcript readers:

  | Harness | Credentials, linked | Memory store, shared by the switch | Verkstead writes |
  |---|---|---|---|
  | Codex | `~/.codex/auth.json` | `~/.codex/sessions/` | `config.toml` |
  | Grok Build | `~/.grok/auth.json` | `~/.grok/sessions/`, `MEMORY.md`, `index.sqlite` | `config.toml` |
  | OpenCode | data `auth.json` | the data directory's database | config `opencode.json` |

  Codex's `cli_auth_credentials_store=file` and its Worktree trust stay on the
  launch line, where ADR-0011 put them.
- **The written configuration carries only what names a model provider** —
  Codex's `model_provider` and `model_providers`, OpenCode's `provider`, and
  Grok's equivalent where it has one — the way Claude's carries `apiKeyHelper`
  and `env`. Nothing else of the human's file: MCP servers, rules, themes,
  hooks and skills are theirs, and a session's behaviour is the product's.
  Rejected: carrying nothing, which logs a custom-provider account out; and
  carrying the file whole, which is the join by another name.
- **OpenCode's data directory is shared whole when memory is on.** Its
  database runs in write-ahead-log mode with two sibling files, and a database
  linked file by file is a database that will not open; the credentials sit in
  the same directory. With memory off, `auth.json` alone is linked and the
  database is the session's own. Its config directory is built either way.
- **The switch is a fact about the Profile**, stored beside its account, on by
  default and migrated on for every Profile that exists — the memory is what
  the human has been getting, and the switch is a way to stop rather than a
  way to start. Off means the store is the session's own and empty: fresh
  memory, no transcript of the human's reachable. On the form it is one
  checkbox, drawn for every harness.
- **Discovery reads the built root.** Every transcript reader — Claude's glob
  of `projects/`, Codex's rollout match, Grok's named session, OpenCode's
  database — looks in the session's root rather than in the account's
  directory. With memory on the two are one directory; with it off only the
  root has the log.
- **Grok's global instructions file is unnamed** in its documentation as of
  this roadmap; stage 03 decides what that means. Its memory files are named
  from the same documentation, and the stage checks them on a real install.

## Proposed tasks (provisional)

1. **The memory column and the checkbox.** A store migration in the rebuild
   pattern the profiles table already uses, the Profile form's checkbox, the
   generated wire types. AC: an existing Profile reads as on; the form saves
   off; the picker and the card are unchanged.
2. **Codex's built root.** AC: `auth.json` linked, `sessions/` shared by the
   switch, a `config.toml` carrying only provider keys; the human's `rules/` and
   `skills/` are not visible; a rollout for the session is found either way.
3. **Grok's built root.** AC: the same shape; the named session's
   `updates.jsonl` is found with the switch off.
4. **OpenCode's built root.** AC: the data directory shared whole when on, the
   credentials alone when off; the pinned database is found either way; the
   config directory holds only what Verkstead wrote.
5. **Claude's switch.** The unconditional join from stage 01 goes behind the
   column. AC: off leaves `projects/` empty inside and the glob still finds the
   session's log there.

## Re-verify at start

- Assumes stage 01 landed: `homes/<id>` on Linux and the built-root rendering
  on all three platforms.
- Assumes the three vendors still keep credentials, store and configuration at
  the paths in the table — check each against the shipped version, OpenCode's
  write-ahead-log siblings especially.
- Assumes the profiles table is still rebuilt rather than altered in place by
  its migrations.
