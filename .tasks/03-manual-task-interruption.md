# 03. A manual task that fails

## What to build

What happens when a manual session ends badly: the human is told, and can have
another go.

A bad exit raises an ordinary Interruption with the usual three Remedies, so the
Conversation carries *blocked on you* and the push fires. That is the whole
reason it is an Interruption rather than a message: the human submits from a
phone and walks away, and resubmitting by hand is not something they are there
to do.

**Retry re-runs the same instruction**, with whatever they wrote alongside
appended under it, in the same place and style a retried step's note goes. The
instruction is read back off the newest Manual Task Event on the Conversation's
Timeline: a Retry is dispatched off the step the evidence names, and a step is a
bare word with no room for the text that was typed. That needs a step word of
its own for a manual task, so the retry can tell it from a backlog step.

The Profile a retry runs under is the Conversation's implementation Profile. The
one-off pick belonged to the submission and is not kept.

**A clean exit that commits nothing raises nothing.** A manual task may
legitimately change nothing — that is not a failure and there is nothing to ask
about.

*Take over manually* and *Abort* keep their meanings exactly, as they do
everywhere: nothing reverts, resets or stashes anything, and the Worktree is
left as the session left it.

## Acceptance criteria

- [ ] A manual session that exits badly raises one Interruption carrying the
      evidence and the three Remedies, and the Conversation is blocked on the
      human until one is chosen
- [ ] Retry launches a fresh manual session on the same instruction, read off
      the Timeline, with the human's note under it
- [ ] A manual session that exits cleanly having committed nothing raises
      nothing
