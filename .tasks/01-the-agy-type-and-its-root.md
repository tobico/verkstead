# 01. The `agy` type and its Built Root

## What to build

A fifth agent type, written `agy` and shown as **Antigravity CLI**, everywhere a
type is named: the store's enum and its word, the render crate's type and
account shape, the viewer's Profile form with one field for the account's
`~/.gemini`, the harness mark, the Guide's word, the program list a session is
followed into (`agy`), the pairing defaults, the carrying and transfer wording,
the onboarding account probe. A `~/.gemini` is an account only where it holds
`antigravity-cli/`; a bare one is what Gemini CLI left behind.

The Built Root for this type, by the one rule every harness fills (ADR-0011):

- **Built**: `antigravity-cli/` and `config/`. Nothing of the account's
  plugins, hooks, skills, rules or MCP servers.
- **Joined with memory on, as one**: `antigravity-cli/conversations/`,
  `brain/` and `conversation_summaries.db`, made in the account first where
  missing; with memory off each is the root's own and empty.
- **The login**: whatever file agy falls to where its keyring is unreachable —
  task 02 measures the path; this task leaves the link's place ready and links
  nothing it has not seen.
- **Written**: `antigravity-cli/settings.json` carrying only what names a
  provider out of the account's own — `modelProvider`, `customModelsConfig`,
  `gcp` — and the Instructions text as the global rules file agy reads under
  `config/` (measure which file agy honours there: a `GEMINI.md`, an
  `AGENTS.md`, or `rules/*.md`). A declared MCP server goes into
  `config/mcp_config.json` in agy's own schema (`mcpServers`, with `url` for a
  remote one and `headers` where the attachment carries them).

The launch line: `--model <id>`, the Brief by `--prompt-interactive`,
`--dangerously-skip-permissions`, an `--add-dir` per Companion Repo, and a
session-only `--settings` JSON carrying `toolPermission: always-proceed`,
`allowNonWorkspaceAccess: true` (the skills and the handoff are outside the
Worktree), `trustedWorkspaces` naming the Worktree and its companions,
`altScreenMode: never`, `runningLightSpeed: off` and `enableTerminalSandbox:
false`. The mode stays agy's default: `auto` turns its own sandbox on, and
bwrap is the boundary. Effort and telemetry are not touched. The resume line
is `--conversation <id>` in place of the Brief, for task 04 to fill.

At launch, `.gemini/` is written into the Worktree's `.git/info/exclude`, once
and idempotently, so the directory agy writes inside the Worktree reaches
neither the Diff nor a commit.

A fresh account has post-onboarding steps agy walks a human through; a headless
run marks them done in `antigravity-cli/jetski_state.pbtxt`. Measure what a
fresh root shows a TUI session and keep it from parking there, seeding that
file the way Claude's bypass consent is seeded if that is what it takes.

Use agy 1.4.12, installed at `~/.local/bin/agy` on this machine, signed out:
everything here is provable at its sign-in screen.

## Acceptance criteria

- [x] A Profile of type `agy` saves over the form and the API, pairs, and launches; the Capture shows agy's own sign-in screen drawn inline, no alternate screen taken.
- [x] The root under `homes/<id>` holds exactly the allowlist above and nothing else of the account; with memory on, the three joined entries are the account's and with it off they are the root's own and empty.
- [x] The written `settings.json` carries only the provider keys the account has, the Instructions text lands where agy reads a global rule, and an attached MCP server appears in `config/mcp_config.json`.
- [x] The launch line and the `--settings` JSON are as above, with one `--add-dir` per companion, and `.gemini/` is in the Worktree's `info/exclude` after a launch.
- [x] The onboarding probe offers a Profile for a `~/.gemini` holding `antigravity-cli/` and not for a bare one; the pairing defaults name a placeholder Pro and Flash for task 03 to correct.

## What was measured, and what is still waiting

**agy 1.4.12**, signed out, at `~/.local/bin/agy` on this machine.

- **The launch, on the real thing.** The `bwrap` line `Sandbox::command` renders
  for an agy Profile, with the line `line()` writes, run under a pty: agy drew
  its own *Welcome to the Antigravity CLI. You are currently not signed in.*
  login picker inline, with no `?1049h` anywhere in what it sent. The account
  was left with nothing but what the joins put there; the root on the host held
  `antigravity-cli/` and `config/`, agy making its own `builtin/`, `log/`,
  `cache/`, `jetski_state.pbtxt` and `config/projects/` there for itself.
- **Nothing to seed for onboarding, signed out.** A run in a fresh root writes
  all four `POST_ONBOARDING_STEP_TYPE_*` steps into `jetski_state.pbtxt` as
  completed before it draws the sign-in screen, so a fresh root parks on
  nothing but the sign-in itself. What a signed-in fresh account shows after
  the sign-in (the billing choice, the telemetry opt-in the changelog names) is
  only measurable signed in.
- **The summaries database runs in write-ahead-log mode**, so it is joined
  with its `-wal` and `-shm`, each made empty in the account where missing.
  Proved on the real binary: started on three empty joined files, agy built its
  `conversation_summaries` table and its log landed in the account.
- **The global rule is `~/.gemini/GEMINI.md`, not a file under `config/`** —
  read off the binary (`/.gemini/GEMINI.md`, `/.gemini/AGENTS.md`) and agy's
  own `rules.md`, which documents only directory rules. Not yet confirmed by a
  signed-in session that a rule there reaches the model.
- **The login is not linked yet.** `AGY_LOGIN` names `authstore.enc` under
  `antigravity-cli/`, a name the binary carries, and nothing has been seen
  written there. Signed out inside the sandbox, agy's log says *Using
  file-based secret storage at `$HOME/.config/antigravity/plugin-secrets.json`*
  for plugin secrets — outside `~/.gemini` — so the login's file may be beside
  it rather than in the account; task 02 measures it.
- **Placeholders for later tasks**: the channel is blocking and the Guide
  reads opencode's *Running the ask*; no idle signature (silence judges it);
  the MCP tool naming is `mcp_<server>_<tool>`, unread off a run; the pairing
  defaults and the viewer's known models are `gemini-pro` and `gemini-flash`;
  there is no resume line and no Transcript search; the onboarding probe offers
  the account without a wizard row, its tick asked of the machine directly.
