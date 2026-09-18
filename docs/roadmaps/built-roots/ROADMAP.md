# Built roots roadmap

A session runs in an account directory of Verkstead's own rather than in the
human's. Today the Agent Profile's account is joined into the session's profile
whole — `~/.claude` by junction on Windows, by symlink on a Mac, by a read-write
bind on Linux — so a session inherits every plugin, hook and setting the human
has, writes into their plugin registry, and leaves its own configuration changes
in their real files. This roadmap replaces the whole-account join with a **built
root** per harness, seeded by an allowlist: the credentials, the memory store
when the Profile says so, a configuration file Verkstead writes, and an
instructions file from the settings page. Alongside it are the smaller things a
first Windows install turned up: what the reporter could not see while two
sessions failed, and the hygiene of a first run.

What it realizes is the promise [the adoption doc](../../adoption.md) already
makes — *a session's behaviour is the product's* — and the decisions are in
[ADR-0011](../../adr/0011-agent-backends.md) (*Amended: the account is built
rather than joined*) and [ADR-0014](../../adr/0014-windows-sessions.md) (the
amendment of the same name). The terms are in [CONTEXT.md](../../../CONTEXT.md),
which each stage updates as its piece lands.

## The findings

Four reports from an independent Windows 11 install of `v0.1.1`, 16 September
2026, with Claude Code installed as the desktop app and then from npm. The
numbers below are the reports' own and the briefs cite them.

- **1** — the desktop app's `claude.exe` is an Electron launcher; a session
  starts it, it prints nothing and exits. *Stage 05.*
- **2** — a first boundary took 112 s to write and nothing said so. The reports
  call it a race against the stall watchdog; it is not — the watchdog has no
  clock — but the silence is real. *Stage 04, and stage 01 shrinks the walk.*
- **3** — `v0.1.1` shipped reporting `0.1.0`; a fresh install offers to update
  itself. *Stage 05.*
- **4** — the workbench key is in the desktop's log file, which the tray opens.
  *Stage 05.*
- **5** — a per-user install registers its uninstall entry in HKLM. *Parked.*
- **6** — the log file reads as mojibake in a code-page viewer. *Stage 05.*
- **7** — `msiexec` returns 1603 under a restricted token. *Not Verkstead's.*
- **8** — the Conversation Terminal ignores synthetic Enter. *Out of scope.*
- **9** — the agent parks for ever on the bypass-permissions consent prompt,
  which a fresh account has never accepted. *Stage 01.*
- **9a–9d** — the `containers\standing` warning; a mixed-separator prompt path;
  a doubled separator that is npm's own shim; the `.claude.json` copy diverging.
  *Stage 05, stage 05, nothing, stage 01.*
- **9e / 9f** — a session's logins and settings land in the human's real files;
  every repository's transcripts are readable. *Stage 01.*
- **10** — the human's plugins and hooks run in every session and fail on every
  tool call; no user-side workaround exists. *Stage 01.*
- **11** — a push refused for a token scope is a question rather than a
  diagnosis. *Stage 05 names the scopes.*
- **12** — a session writes into the human's plugin registry a path inside the
  sandbox's ephemeral storage. *Stage 01.*

Not Windows-only: this repository's own Linux sessions inherit the maintainer's
enabled plugin and global `CLAUDE.md`, and rewrite their `known_marketplaces.json`.
Linux never parked on the consent prompt only because that account had accepted
it once by hand.

## What was decided, in one place

- **A built root, not launch flags and not `--safe-mode`.** Flags close the
  inheritance and not the write-through; `--safe-mode` takes the Repo's own
  `CLAUDE.md`, the skills and MCP with it.
- **The same rule for every harness**, for consistency — rejected: Claude alone.
- **The allowlist:** credentials, the memory store, a configuration file
  Verkstead writes carrying only what names a model provider, an instructions
  file. Not the human's global instructions file, not plugins, hooks, history,
  commands or agents.
- **The `.claude.json` write-back stays**, because a re-login from inside a run
  has to land in the account; the pre-seeded trust entries land with it.
- **A memory switch on every Profile**, on by default.
- **One instructions text for every Profile**, on the settings page, written as
  each harness's file and appended to the prompt where a harness has no file.
- **The agent stays interactive on a pseudoconsole.** `--print` was suggested and
  is off the table: the Screen, the Hold, quiet detection and the Rescue all
  stand on the terminal (ADR-0006, 0007, 0011).

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

01 and 02 are strictly ordered, and 03 follows 02: the roots have to exist
before the other harnesses' are built by the same rule, and before there is
anywhere to write an instructions file. 04 and 05 depend on nothing and may go
first; 05 carries the release gate, which matters before `v0.1.2` is tagged.

## Stages

- [x] 01: Claude's built root — [brief](01-claude-root.md)
- [x] 02: The other three roots and the memory switch — [brief](02-three-roots-and-memory.md)
- [x] 03: The instructions text — [brief](03-instructions-text.md)
- [ ] 04: What the reporter could not see — [brief](04-what-the-reporter-could-not-see.md) *(in progress: `built-roots/04-what-the-reporter-could-not-see`)*
- [ ] 05: First-run hygiene — [brief](05-first-run-hygiene.md)
