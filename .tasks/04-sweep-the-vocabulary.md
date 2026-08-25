# 04. Sweep the vocabulary

## What to build

The written record catches up with the product. Tasks 01 to 03 each carried the
words closest to the code they changed; what is left is the documents that teach
the vocabulary, where a retired term is woven through entries about other
things.

**CONTEXT.md** loses its **Manual Task** entry outright, and the Abort and
Reopen mentions threaded through **Conversation**, **Worktree** and **Brief**
become Close and Steer. Watch for the ones that are not about the press at all:
the folding rule names a Manual Task's session as the one never folded into, the
stall rule names the states nothing is supposed to drive, and the *Avoid* lists
carry the retired words as words to avoid — which is right, but they now avoid
them as retired terms rather than as near-misses for something current. Earlier
stages of this roadmap took their own terms as they went, so what is left here
is this stage's three and no others.

**`docs/design/verkstead.md`** carries the same three in its record of what was
decided and when. It is a design document rather than a history, so the retired
terms go — but the corrections already stamped in it stay stamped.

**`docs/adoption.md`** sends the reader to a Manual Task for steering work,
which is the sentence this stage makes wrong. It points at Steer instead.

The roadmap briefs and the ADRs under `docs/adr/` are **not** swept. They are
the record of decisions taken at a time, and ADR-0010 retiring a term is
something it has to be able to say.

Finish by grepping rather than remembering: whatever still names Abort, Reopen
or Manual Task outside the old-record read paths and the historical record is
either a miss or wants a comment saying why it stayed.

## Acceptance criteria

- [ ] No retired term survives in CONTEXT.md or `docs/design/verkstead.md`, and
      CONTEXT.md has no Manual Task entry.
- [ ] `docs/adoption.md` points the reader at Steer for work that needs steering.
- [ ] A grep for the three terms across the source, the workbench and the shipped
      skills turns up only the paths that read old records, and the roadmap and
      ADR history.
