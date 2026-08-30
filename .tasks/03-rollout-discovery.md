# 03. Rollout discovery

## What to build

Finding the rollout log a Codex session keeps of itself, and handing it to the
tail that already follows Claude's.

Codex takes no session id at launch, so its log cannot be named the way
Claude's is — it is **found**. Codex writes it under the account home's session
store, in a directory per day, as `rollout-<timestamp>-<uuid>.jsonl`, and its
first line is a `session_meta` whose payload carries the session's `cwd`. So
the session's log is the one whose meta names this session's Worktree and which
appeared after this session was launched. The tail today is told a name and
looks for a file called that; for Codex it is told a Worktree and a moment, and
looks for the log that matches both.

Everything else about following is unchanged and stays unchanged: lines stored
verbatim, nothing parsed on the way in, one Nudge per batch, and a session
whose log never appears staying Capture-only with nothing logged about it —
ADR-0006's rules, which this does not touch.

Two things the current codex is doing that the finder has to be indifferent to:
it is moving its session store into SQLite alongside the rollouts, and it
compresses older rollouts. The live log is still plain JSONL in the day's
directory, so what this needs is to ignore whatever in there is not one rather
than to keep up with either.

**Until task 04 lands, the rollout draws as bookkeeping.** Codex's line kinds
are not Claude's, so the renderer folds each of them away under its own name —
which is ADR-0006's fall-back rule working, not a gap. The lines are on the
Transcript verbatim from this task on, and task 04 is what makes them a
conversation.

## Acceptance criteria

- [ ] Two Codex sessions launched near-together in different Worktrees each
      follow their own rollout, matched on the Worktree its meta names and the
      moment it appeared.
- [ ] A session whose log never appears stays Capture-only and says nothing
      about it, and a Claude session's log is still found by name exactly as
      today.
- [ ] The rollout's lines reach the Transcript verbatim, in order, with a
      partial line held rather than stored torn.
