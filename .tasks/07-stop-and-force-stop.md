# 07. Stop and Force stop

## What to build

The conversation header menu that holds Abort grows the human's two ways of
stopping, worded as settled:

- **Stop** — *Pause after the current task until you resume.* Nothing new
  starts; a running session runs to its natural end, and the Conversation
  then halts, deliberately, with a Notice saying the human asked.
- **Force stop** — *Halt any running tasks and stop immediately.* Every
  running session of the Conversation's — grillings included — is ended now,
  and the halt is written at once. An unanswered Question Set is left
  standing.
- **Abort** — *Permanently end the conversation and delete the worktree.*
  Already built; it keeps its place and gains the description.

Both stops are the human's own act, so neither pushes. Both are undone by
Resume. A Stop pressed while nothing is running halts immediately; either
pressed on a Conversation already halted refuses as such.

## Acceptance criteria

- [ ] The menu shows the three actions with their descriptions, each
      offered only when it applies.
- [ ] Stop lets a live session finish its step, then halts before the next
      launch; Force stop ends the live session at once.
- [ ] Both write a deliberate halt with a Notice naming the human's press,
      fire no push, and Resume brings either back.
