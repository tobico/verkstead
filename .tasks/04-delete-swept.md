# 04. Delete, swept

## What to build

Permanent deletion — the first anywhere in Verkstead — of a Conversation the
Cleanup's delete clock has run out on: every row it owns, removed in
dependency order, and nothing outside the store touched.

**The walk.** Reach the event-keyed bulk through the timeline events (capture
summaries, chunks and turns; transcript lines; session names, pairings and
agents), the Set rows through the set-event pairings (the sets themselves,
responses, Set locks, deferrals, endings), then the conversation-keyed
sidecars — commits and their summaries, pull requests with their checks,
merges and standings, shares and share comments, placements, the archive row
and the trim mark, and the rest — then the timeline events, then the
Conversation row itself. Around twenty tables key on the Conversation
directly; enumerate them from the schema when building rather than trusting a
list written here, and leave behind a test that fails when a future table
references the Conversation without joining the walk.

**What it never touches.** The git branch — a branch is the repository's, and
closing already chose to keep it — and any published share, which was
published deliberately. No git operation belongs anywhere in this path.

**The sweep.** Extend task 01's cleanup pass: where the delete cleanup is
*enabled* and `archived_at` is older than the delete duration, delete. Off by
default, so nothing is ever deleted until the human turns it on; when they do,
the existing backlog goes on the next pass, as settled. One log line per
delete, nothing else said anywhere.

**Afterwards.** A deleted Conversation is gone from the sidebar even with
Show archived on — its rows are gone, so nothing lists it — and opening its
URL answers plainly that there is no such conversation, a clean not-found
rather than a server error.

## Acceptance criteria

- [ ] After a store-level delete, no table anywhere in the store holds a row
      naming the Conversation — verified by a test that enumerates the
      references rather than repeating a list.
- [ ] At test pace, with delete disabled an old archive is only ever trimmed;
      enabled, it is gone on the next pass; a fresh archive and an unarchived
      Conversation are untouched either way.
- [ ] The deleted Conversation's URL answers not-found cleanly, and no git
      operation occurs anywhere in the delete path.
