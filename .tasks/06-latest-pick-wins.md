# 06. Latest pick wins

## What to build

Harden the agent's discretion. A pick lets the agent proceed; it does not make
it. The agent judges from the whole Response whether everything is clear —
proceeding is producing the picked direction's artifact, and coming back is
another Question Set, with a fresh proposal if it wants the direction
reconsidered. Nothing but the picked direction's artifact moves the machine.

Concretely: a proposal may now appear on more than one Set over a grilling
(at most one in flight, which the blocking ask already guarantees), so the
one-proposal-per-grilling validation relaxes. When a later proposal Set's pick
lands, it supersedes the earlier one — the previously armed follower is
cancelled or re-pointed so exactly one watcher is live, watching for the
latest pick's artifact. On restart, Verkstead recovers the armed watcher from
the Conversation's stored latest pick.

The grilling skill states the doctrine: the choice of direction is never the
agent's — proceed only on the picked direction, and argue with a pick by
proposing again, never by producing a different artifact.

## Acceptance criteria

- [ ] After a pick, the agent can send another Set (with or without a
      proposal) instead of proceeding, and nothing fires; a later pick
      supersedes the earlier one and only the latest direction's artifact
      moves the Conversation on.
- [ ] A server restart between pick and artifact resumes watching for the
      stored latest pick.
- [ ] Validation accepts a second proposal Set from the same grilling and
      still refuses a proposal from any Conversation not Grilling.
