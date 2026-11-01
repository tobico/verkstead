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

- [ ] A Profile of type `agy` saves over the form and the API, pairs, and launches; the Capture shows agy's own sign-in screen drawn inline, no alternate screen taken.
- [ ] The root under `homes/<id>` holds exactly the allowlist above and nothing else of the account; with memory on, the three joined entries are the account's and with it off they are the root's own and empty.
- [ ] The written `settings.json` carries only the provider keys the account has, the Instructions text lands where agy reads a global rule, and an attached MCP server appears in `config/mcp_config.json`.
- [ ] The launch line and the `--settings` JSON are as above, with one `--add-dir` per companion, and `.gemini/` is in the Worktree's `info/exclude` after a launch.
- [ ] The onboarding probe offers a Profile for a `~/.gemini` holding `antigravity-cli/` and not for a bare one; the pairing defaults name a placeholder Pro and Flash for task 03 to correct.
