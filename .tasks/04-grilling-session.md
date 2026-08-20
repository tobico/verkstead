# 04. Grilling session with a captured transcript

## What to build

*Start grilling* now starts a real one. The Conversation's grilling Profile's
claude runs inside the sandbox on the Conversation's worktree, primed with the
Brief, and everything it prints reaches the Timeline as it arrives.

The capture technique comes from roadrunner's `session.ts`, and only the
technique does — that is TypeScript on Bun and this is Rust, so nothing is
copied. What carries over is the shape and the reasoning behind it: the session
runs on a pseudo-terminal of its own, allocated by `script --quiet --return
--command … /dev/null`, because claude needs a terminal to behave like itself
and `script` is already fluent in the raw modes and window-size handling we
would otherwise be reimplementing. Its output is relayed, retained in full, and
summarised.

The session is interactive and never `-p` (ADR 0001 in tobico-skills): it idles
when it has nothing to do, which is what the next task's blocking asks depend
on.

This adds the design's **agent output** Event: summarised in the Timeline as a
line count plus the latest statement, with the full transcript in the details
pane. It updates live rather than appearing when the session ends — a grilling
session runs for a long time, and a Timeline that says nothing until it
finishes is a Timeline nobody can watch. The Nudge already exists for telling
an open page the world moved; use it rather than inventing a second signal.

Aborting a Conversation (task 01) now has a running session to end, and must
end it before the worktree goes.

A session that dies is not this task's subject — Interruption Events are stage
05 — but a dead session must not leave the Conversation claiming to be running.

## Acceptance criteria

- [ ] *Start grilling* launches the grilling Profile's claude in the sandbox,
      on the Conversation's worktree, with the Brief as its prompt
- [ ] The session's output appears in the Timeline while it is still running,
      summarised as line count plus latest statement
- [ ] The details pane shows the full transcript, byte for byte
- [ ] The transcript is stored, and survives the server restarting
- [ ] Aborting ends the running session before removing the worktree
- [ ] A session that exits leaves the Conversation in a state that says so
      rather than one that claims a live session
