# 03. The named session's log follows

## What to build

The tail follows the log a Grok session keeps of itself, so its lines land on
the Transcript while it runs.

Grok Build takes the session id at launch, so its log is **named** rather than
found — Claude's shape, and the opposite of Codex's. What is different is the
path. Grok keeps one directory per session inside one directory per working
directory:

    ~/.grok/sessions/<the working directory, encoded>/<session id>/updates.jsonl

`updates.jsonl` is the authoritative conversation log; a `summary.json` may sit
beside it and is not it.

**The encoding of the working directory is grok's own, and is not
reimplemented.** Working out what grok would have called that directory means
reproducing somebody else's private scheme, which is the thing ADR-0006 says
not to do. What Verkstead knows is the store, and the id it named the session —
so the log is looked for by walking the store's directories for one holding a
directory of that name, exactly as Claude's log is looked for by walking the
account's project directories for a file of that name. One level of walking,
and the id is what identifies it.

Everything else about following is unchanged and none of it is written again:
lines are stored verbatim, whole lines only, polled on the cadence the byte
relay already flushes on, and a session whose log never appears has its Capture
as the whole record. Nothing here parses a line — what a line means is task
04's, and until that lands a Grok line shows on the Transcript as the JSON it
is, which is ADR-0006's rule doing its job rather than a gap.

## Acceptance criteria

- [ ] A running Grok session's `updates.jsonl` is found under the account's
      session store by the id Verkstead named it, whatever grok called the
      directory it encoded the working directory as, and its lines accumulate
      on the Transcript verbatim as they are written.
- [ ] Two Grok sessions launched at once in different Worktrees each follow
      their own log; a session whose log never appears keeps the Capture as its
      whole record, and Claude's and Codex's discovery are unchanged.
- [ ] The Timeline row for a Grok session shows a Transcript rather than
      nothing, and the details pane opens what has been stored.
