# 03. Codex

## Goal

The first backend whole. Demonstrable: a Conversation grilled, built and
wrapped under a Codex Profile — launched from its one home directory, asking
by store-and-nudge, judged idle by its screen signature, its Transcript
readable on the Timeline from its own rollout log, and a usage-limit stop
naming the account when the phrase is seen. The Profile form offers the
`codex` type from this stage on.

## Decisions in force

All from [ADR-0011](../../adr/0011-agent-backends.md); what bears on this
stage, with the research facts that shaped them (as of codex late-2026,
worth re-checking):

- **Home.** One directory, bound at `~/.codex` inside the sandbox. The
  account's `auth.json` and `config.toml` live in it; the credential store
  must be file-backed (`cli_auth_credentials_store = "file"`) since there is
  no keyring inside. Subscription (ChatGPT) and API-key logins both land in
  `auth.json`, so the Profile does not care which.
- **Launch line.** Interactive TUI, prompt positional, model by `-m`.
  Verkstead passes `--dangerously-bypass-approvals-and-sandbox` (its own
  sandbox breaks inside bwrap and bwrap is the boundary) — and pre-seeds
  directory trust, since some versions still show a trust prompt despite the
  bypass. Codex takes `-c key=value` config overrides on the command line,
  which is the tool for that without writing into the human's home.
- **Session identity is found, not named.** Codex takes no session id at
  launch. The rollout log appears under the home's `sessions/YYYY/MM/DD/` as
  `rollout-<timestamp>-<uuid>.jsonl`, its first line carrying the session's
  cwd — so the session's log is the one whose meta matches the Worktree,
  appearing after launch. ADR-0006 otherwise unchanged: lines verbatim,
  parsed at render time, Capture the record until the log is found.
- **The rollout renderer.** `session_meta`, `event_msg`, `response_item`,
  `turn_context`, `compacted` — rendered the way the Claude renderer
  renders: turns as the conversation, the rest folded into bookkeeping under
  the names the log gives, nothing dropped.
- **Usage-limit phrase.** Confirmed wording opens with "You've hit your
  usage limit" — plan-dependent decoration after it, so the stable prefix is
  what `says_so` gets, read off Capture and Transcript both as today.
- **Ask and idle are stage 02's mechanisms**; this stage contributes Codex's
  idle signature constant and proves both against the real thing.

## Proposed tasks (provisional)

1. **The `codex` type launches.** Variant, home bind, argv mapping (flags,
   trust pre-seed, model, prompt), form row.
   - A Codex Profile saves, pairs and launches into a sandbox; the Capture
     shows it at its prompt.
2. **Rollout discovery.** Find the session's log by worktree and start time
   under the home's sessions store; hand it to the tail that follows
   Claude's today.
   - Two sessions launched near-together in different Worktrees each find
     their own log; a session whose log never appears stays Capture-only.
3. **The rollout renderer.** Codex's line kinds into the Transcript's turns
   and bookkeeping.
   - A real rollout draws as a conversation; an unknown line kind folds into
     bookkeeping under its own name, ADR-0006's rule.
4. **Codex's limit phrase and idle signature.** The two constants, and the
   end-to-end proof: a grilling asked, nudged, answered and built under a
   Codex Profile.
   - The signature judges the real TUI idle at its prompt; a limit line
     stops the run naming the Profile.

## Re-verify at start

- Codex's current flag set and rollout format — the research facts above are
  from late 2026 and Codex moves fast; re-confirm `--yolo`'s behavior, the
  trust prompt, the rollout filename shape and the meta line's cwd field.
- Stage 02's nudge line wording against what the Codex composer does with a
  typed line and Enter.
- Whether the alternate screen needs `--no-alt-screen` for the Capture and
  Screen to stay readable records, or the emulator handles it — decide on
  the real thing, it changes only the launch line.
- That the sandbox's PATH reaches a host-installed `codex` (host provides
  binaries, per the ADR).
