# 03. Task-list tail in-context

## What to build

A task-list pick stops ending the grilling session. The Response — the pick and
whatever the human wrote beside it — returns to the session through its
blocking ask, and that same session breaks the work into `.tasks/` and commits
the backlog, holding everything the grilling settled. The runner sees out the
grilling session exactly as it sees out a breakdown session today — backlog
commit plus quiet — and then starts the task run under the Implementation
Profile. The Conversation stays Grilling while the backlog is written and
moves to Implementing when the grilling session ends.

The grilling skill's closing section gains the task-list branch: on a
task-list pick, read the breaking-down skill and follow it from grounding
onward — the branch is made, the worktree is this one, and the human's
feedback on the pick is part of what the backlog answers to. The
breakdown-approval loop survives unchanged: the session drafts the backlog,
puts its shape to the human as ordinary Sets, and commits once approved.

The breaking-down skill is reframed to carry both entries: mid-session (the
grilling carrying on — grounding largely done, the agreement being its own
conversation) and fresh (a retried tail in a new session, primed with the
Brief and the human's retry note). The handoff keeps its current pre-proposal
timing for now — task 05 moves it — so downstream prompts are untouched here.

## Acceptance criteria

- [ ] A task-list pick leaves the grilling session running; the Response,
      including free text, reaches it as the ask result.
- [ ] The same session's backlog commit plus quiet ends it and starts the task
      run; no fresh breakdown session is ever launched on this path.
- [ ] The Conversation reads Grilling until that session ends, Implementing
      after.
- [ ] The breaking-down skill works from both entries, and its approval Sets
      still gate the commit.
