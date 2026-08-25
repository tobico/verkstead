# 03. Close and the retirements

## Goal

The control surface reads as the settled model and nothing else: Abort is
**Close** everywhere, Reopen and Manual Task are gone, and Steer is the one
hand-on-the-wheel control. Demonstrable: a Conversation is closed (Worktree
deleted, branch kept) from any state, and steering it is the one way back in;
no Manual Task composer anywhere; CONTEXT.md and the design doc carry no
retired term.

## Decisions in force

All from [ADR-0010](../../adr/0010-one-stop-and-steer.md); what bears on this
stage:

- **Close is Abort renamed, not resemanticized.** Worktree deleted, branch
  kept, pressable from any state. The name distinguishes it from a force
  stop: a Conversation is closed when its Worktree is deleted, and back when
  a steer makes it a new one.
- **Reopen is retired.** Steering a closed Conversation recreates the
  Worktree on its branch; a steer into Grilling opens a new round with a new
  Brief, which is what Reopen did — so a second door would be a second thing
  to keep true.
- **Manual Task is retired.** A steer into Implementing with a hand-written
  instruction covers it, and covers it better: the instruction session drives
  the pipeline instead of leaving the Conversation stopped beside its own
  work. The folding rule simplifies with it — the never-folded-into carve-out
  for Manual Task sessions goes with the feature.
- **Old records stay readable.** Timelines holding Manual Task Events and the
  stored word `aborted` are the record; read them as they were written
  (ADR-0006's rule) rather than rewriting history to the new names.

## Proposed tasks (provisional)

1. **Close through store, server and workbench.** Rename the press, the
   refusals and the card wording; the stored state keeps reading `aborted`
   rows however the stage decides to spell new ones.
   - Closing from every state works as Abort did; old aborted Conversations
     draw as closed.
2. **Retire Reopen.** Remove the press and its endpoint; the workbench offers
   Steer on a closed or Done Conversation instead.
   - A closed Conversation steered into Grilling gets a Worktree, a new
     round and a new Brief.
3. **Retire Manual Task.** Remove `manual.rs`, the composer, and the folding
   carve-out — and the skill with them: the `manual-task` directory under
   `crates/server/skills/`, the `MANUAL_TASK` const, `skills::manual_task`
   and that skill's own tests in `skills.rs`. Stage 02's instruction skill is
   what is left standing, so check it is installed and launched before this
   one goes.
   - Nothing installs or names a manual-task skill; the instruction session
     still launches.
4. **Sweep what is left of the vocabulary.** CONTEXT.md's **Manual Task**
   entry, and the Abort and Reopen mentions woven through **Conversation**,
   **Worktree** and **Brief** — stages 01 and 02 took their own terms as they
   went, so what is left here is this stage's three. Then
   `docs/design/verkstead.md` and the UI strings across the lot.
   - No retired term survives in either document.

## Re-verify at start

- Stage 02 landed and Steer actually covers the Manual Task uses — a quiet
  moment's one-off errand included — before the composer is deleted.
- What still references `hold`, `pause`, `manual` and `abort` by then
  (`grep`, not memory): push wording, tests, the CLI's own guide — and
  `crates/server/skills/`, where the shipped skills are the one place a
  retired term is also a file that gets installed.
- Whether any Conversation in the wild sits mid-Manual-Task or held — the
  removal has to land on a quiet product, or read the old records gracefully.
