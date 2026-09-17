# 03. Grok Build's built root

## What to build

A Grok Build session runs in a `.grok` that is Verkstead's own
(`homes/<id>/.grok`), built from an allowlist with the machinery task 02 made
general, in place of the whole Profile home joined over `~/.grok`:

- **`auth.json`, linked**, and handed back at session end where the session
  replaced it, on all three platforms. An account with no file (an API key
  arriving by environment) gets none.
- **The memory store, shared only when the Profile's memory switch is on:**
  `sessions/` joined read-write and `MEMORY.md` linked, each made in the account
  first where missing. When the switch is off, both are the root's own and empty.
  `MEMORY.md` is a linked file, so it is handed back like the credentials where
  Grok replaces it by rename.
- **`index.sqlite`: decide in this task, after checking a real install.** The
  human chose to leave this open. Find out whether Grok Build writes it in
  write-ahead-log mode (`-wal` and `-shm` siblings, or `PRAGMA journal_mode`).
  If it does, a file-by-file link breaks it, so keep it the session's own, and
  check that Grok still finds and resumes the named session without the
  account's index. If it does not, link it with the rest of the store. Write down
  what was found and what was chosen in a comment beside the allowlist.
- **A `config.toml` Verkstead writes**, carrying only what names a model provider
  in Grok's own `config.toml`, where Grok has any such keys. Read Grok Build's
  documentation and the real install for what those keys are. If there are none,
  write no file, or an empty one if Grok needs it there.
- **Everything else is absent:** `skills/`, the human's other settings, history,
  and whatever Grok adds next. Grok's global instructions file is stage 03's
  business, not this task's.

**Needs xAI's Grok Build installed on this machine.** The human agreed to
install it before this task starts. The `grok` found on the PATH during planning
was `@vibe-kit/grok-cli` 0.0.33, a different program (it keeps an API key in
`~/.grok/user-settings.json`). Check `grok --version` against ADR-0011's
measured 1.0.13 before relying on it. If Grok Build is still not installed, stop
and say so rather than building from documentation alone.

**Discovery** follows task 01's rule. The named session's `updates.jsonl` is
looked for in the account's `sessions/` when memory is on, and in the root's on
the host when it is off. The lookup (beside the store, or one encoded directory
down) is otherwise unchanged.

## Acceptance criteria

- [ ] Inside a Grok session, `~/.grok` holds `auth.json`, whatever config Verkstead wrote, and (with memory on) `sessions/` and `MEMORY.md`, and nothing else of the account's. The three boundary suites assert it.
- [ ] With memory off, the named session's `updates.jsonl` is found under the root on the host and draws on the Timeline. The account's `sessions/` and `MEMORY.md` are untouched.
- [ ] With memory on, the log is found in the account's `sessions/`, as today.
- [ ] A login, or a `MEMORY.md` replaced inside, reaches the account at session end on all three platforms.
- [ ] How `index.sqlite` is treated was decided against a real Grok Build install, and the finding is written down.
