# 05. Handoff after the pick

## What to build

The handoff moves to the far side of the choice, and becomes inline's alone.
The grilling skill stops requiring a handoff before the closing Set — a
refused proposal no longer costs a rewrite, and the closing Set is cheap to
send. On an inline pick, the session's one remaining job is the handoff:
written after the Response arrives, shaped by whatever the human said beside
the pick, and then the session goes quiet.

Verkstead ends the inline tail the way it ends every tail — artifact plus
quiet, where the artifact is the handoff document present in the
Conversation's handoff directory — then takes the handoff onto the Timeline at
session end (the one moment it is certainly finished), and starts the
implementation session primed with Brief and handoff as today. A session that
goes quiet without writing one is the stalled handler's to catch.

Task-list and roadmap picks write no handoff at all: the committed backlog or
roadmap is the plan and the record, written by the context that settled it.
Downstream prompts — per-task, reviewing, addressing — simply have no handoff
to fold in on those paths, and their builders already tolerate its absence;
the breaking-down and staging skills' fresh entries drop the
handoff-as-agreement framing and ground from the Brief, the repo, and the
retry note.

## Acceptance criteria

- [ ] The grilling skill neither writes nor mentions a pre-proposal handoff;
      on an inline pick it writes the handoff (reflecting any pick feedback)
      and stops.
- [ ] Handoff present plus quiet ends the inline tail; the handoff lands on
      the Timeline at session end and primes the implementation session.
- [ ] A task-list or roadmap Conversation records no handoff anywhere — no
      Timeline Event, nothing folded into downstream prompts — and those
      pipelines run to completion regardless.
