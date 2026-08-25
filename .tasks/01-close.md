# 01. Close, wherever the press is named

## What to build

Abort is renamed **Close** end to end. Not resemanticized: the press does
exactly what it did — the Worktree is deleted, the branch is kept, and it is
pressable from any state — and a Conversation that has been closed is off the
ladder the way an aborted one was. Only the name changes, and it changes
everywhere at once.

The rename is one slice rather than four because the state is carried through
every layer as one word: the store's lifecycle, the render crate's mirror of
it, the TypeScript that mirror generates, the fixtures the workbench tests read,
and the strings on the page. Splitting it by layer leaves a tree that does not
build in between.

**The rename reaches the wire.** The endpoint path, the outcome type the browser
reads, and the state value in the generated TypeScript all say Close. The
workbench is bundled with the server, so there is no other client to keep in
step.

**The stored word moves too, and old rows move with it.** The lifecycle is
stored as text, and every `moved` Timeline Event carries that same text as its
body. New rows say `closed`, and a migration rewrites the ones that say
`aborted` — the state column and the Event bodies alike — so a Timeline reads as
one vocabulary rather than two. Reading `aborted` still works for anything a
migration never reached, the way the store already reads the shapes that came
before it.

Everything the human sees says Close: the menu row, the button, the note under
it, the refusals, the card in the sidebar, and the Steer modal's line about
where a Worktree gets recreated.

## Acceptance criteria

- [ ] Closing a Conversation from every state does what Abort did — the Worktree
      is deleted, the branch is left where it is, and closing one that is closed
      already is not an error.
- [ ] A Conversation that was aborted before this landed draws as closed, and its
      Timeline reads as one vocabulary — the migration has moved its stored state
      and its move Event alike.
- [ ] No user-facing string, endpoint path or exported type says Abort, and the
      workbench builds against the regenerated types.
