# 04. Grok Build

## Goal

xAI's `grok` is a backend. Demonstrable: a Conversation grilled and built
under a Grok Build Profile — its session log *named* at launch the way
Claude's is, store-and-nudge asking, screen-signature idling, its JSONL
Transcript on the Timeline, and the form offering the type. The usage-limit
stop ships with whatever wording has been observed by then, possibly none.

## Decisions in force

All from [ADR-0011](../../adr/0011-agent-backends.md); what bears on this
stage, with the research facts that shaped them (v1.0, August 2026 — worth
re-checking):

- **Home.** One directory, bound at `~/.grok`. Subscription OAuth tokens
  land in its `auth.json`; an API key would come by environment, but the
  Profile's directory is the account either way.
- **Launch line.** Prompt positional, model by `-m`, and — alone among the
  new backends — **`--session-id`** naming the session at launch, so the log
  under the home's `sessions/` store is a fact rather than a find, exactly
  Claude's shape. Verkstead passes `--always-approve` and `--sandbox off`:
  Grok Build's own sandbox refuses to start where bwrap is unavailable
  inside, and bwrap is already the boundary.
- **The log.** JSONL per session (`updates.jsonl` the authoritative
  conversation log), organized by encoded working directory and session id —
  the tail follows it as it follows Claude's, lines verbatim, parsed at
  render time.
- **Skills need nothing special.** Grok Build discovers Claude-style skill
  directories natively and reads arbitrary paths; the prompt naming
  `/verkstead/skills/...` is enough.
- **Usage-limit phrase as observed.** The free-tier wording is known, a paid
  account's is not, and the mechanism ships with the constant empty until a
  real stop is seen — until then it lands as an ordinary stall.
- **Ask and idle are stage 02's mechanisms**; this stage contributes Grok
  Build's idle signature and the proof against the real thing.

## Proposed tasks (provisional)

1. **The `grok` type launches.** Variant, home bind, argv mapping
   (`--session-id`, the two bypass flags, model, prompt), form row.
   - A Grok Build Profile saves, pairs and launches; the named session's log
     directory appears under the home.
2. **The log follows.** Point the tail at the named session's JSONL under
   `~/.grok/sessions`; render its line kinds as turns and bookkeeping.
   - A real session's Transcript draws on the Timeline; unknown kinds fold
     under their own names.
3. **Grok Build's idle signature, and the end-to-end proof.** The constant,
   and a grilling asked, nudged, answered and built under the type.
   - The real TUI reads idle at its prompt and busy while its spinner runs.

## Re-verify at start

- What stage 03 taught: the store-and-nudge round trip and the signature
  approach against a real TUI — apply the corrections before repeating the
  shape here.
- Grok Build's flags and session layout against its current release; v1.0
  was weeks old when this was planned. `--session-id` semantics especially:
  it creates only, never resumes.
- Whether its TUI needs `--no-alt-screen` (it has one) for the Capture to
  stay a readable record, per stage 03's finding.
- Whether a paid account's limit wording has been observed anywhere yet.
