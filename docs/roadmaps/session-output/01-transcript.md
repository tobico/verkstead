# 01. The Transcript

## Goal

A session's details pane shows the conversation, not the bytes: the agent's
prose rendered as markdown, tool calls collapsed to one-line summaries, user
turns, and thinking collapsed — live while the session runs. The Event
summary line and Interruption evidence show real assistant prose. Sessions
that leave no session log (test stubs, other backends) still show the raw
view exactly as today.

## Decisions in force

- **Content comes from the agent's session log, never from un-rendering the
  TUI stream** — [ADR 0006](../../adr/0006-transcript-from-session-log.md)
  has the full rationale and the containment choices (verbatim storage,
  render-time parsing, Capture as fallback record).
- **The rename comes first and goes all the way down.** *Transcript* now
  means the readable record and *Capture* the raw bytes
  ([CONTEXT.md](../../../CONTEXT.md)). The `transcripts`/`transcript_chunks`
  tables, the `/transcript/{event}` endpoint, wire types, `Output.tsx`, and
  tests all currently say "transcript" meaning bytes; they are renamed with
  a table migration, not left contradicting the glossary. Doing it as the
  first task keeps the mechanical diff out of the substantive ones.
- **Session identity is a fact, not a guess**: Verkstead generates the UUID
  and passes `--session-id` at spawn, then finds the log by globbing the
  profile's `projects/` dir for `<uuid>.jsonl` — deliberately *not*
  computing Claude's cwd-slug, which is a private algorithm that can change.
- **Tail on the existing 500 ms cadence** (same as the relay's flush), plain
  polling, no inotify. Appended lines are stored as verbatim rows keyed
  `(event_id, seq)` — the same append-only shape as the capture chunks —
  and each append nudges the viewer.
- **One AgentOutput Event per session**, as today. The Timeline stays a list
  of things that happened, not a chat log.
- **Rendering is the server's** (`crates/render`, per the rule restated in
  Output.tsx and ADR 0003's direction): known message kinds become sanitized
  HTML; **unknown kinds render as collapsed raw JSON** — nothing hidden, so
  a format change shows itself instead of silently emptying the pane.
- **Summaries and Interruption evidence switch to the latest assistant
  text**, with the existing escape-stripper
  (`crates/server/src/transcript.rs`) kept as the fallback when no
  Transcript rows exist. That fallback is also the whole details-pane story
  for log-less sessions — which is what keeps the stub-agent test suite
  passing.
- Quiet-detection stays keyed on PTY output (the Capture), untouched by any
  of this.
- TS wire types are generated from Rust via ts-rs (`web/src/api/types.ts` is
  generated — never hand-edited).

## Proposed tasks (provisional)

1. **Rename the byte capture** — migration renaming
   `transcripts`/`transcript_chunks` to capture tables; endpoint, wire
   types, store/render/server code, `Output.tsx`, CSS class names, and
   tests follow. AC: grep for "transcript" finds only the new meaning's
   plumbing (or nothing yet); existing byte-for-byte UI behavior unchanged;
   old DB upgrades cleanly.
2. **Name the session at spawn** — generate the UUID, pass `--session-id`,
   record it with the session. AC: spawned argv carries the flag; a real
   session's log appears under the profile dir at that UUID; stub sessions
   (no flag support) still start.
3. **Tail and store the Transcript** — glob-and-poll tailer following the
   log, verbatim rows appended with nudges. AC: rows accumulate while a
   session runs; a mid-line partial read never stores a torn line; a
   session with no log stores nothing and errors nowhere.
4. **Render the conversation** — `crates/render` parses known kinds
   (assistant text, tool use/result, user, thinking) into the wire type;
   details pane renders them (tool calls and thinking collapsed); unknown
   kinds as collapsed raw JSON; fallback to the raw Capture view when no
   rows exist. AC: fixture log renders all four kinds; an unrecognized line
   shows as JSON, not nothing; stub-agent flow still shows raw bytes.
5. **Switch summaries and evidence** — Event `latest` and the Interruption
   tail prefer the latest assistant text, stripper fallback intact. AC:
   summary shows prose for a real session; stub session summaries unchanged;
   Interruption evidence reads as prose when rows exist.

## Re-verify at start

- `claude` CLI still accepts `--session-id` and writes
  `~/.claude/projects/<slug>/<uuid>.jsonl`; spot-check the current JSONL
  line shapes against the render task's assumptions.
- Spawn still happens in `Sessions::start` via `Agents::argv`
  (`crates/server/src/sessions.rs`) with `script` providing the PTY —
  stage 02 changes this, so confirm which landed first.
- The relay's 500 ms flush and `store::append_transcript` shape are as
  described in ADR 0006's context.
- The nudge model is still invalidate-everything (ADR 0005) — the tailer
  needs no finer signal.
