# 06. The naming instruction

## What to build

When the branch name is Verkstead's, the Conversation's **first session** is
told to pick a real one. The initial prompt — the grilling session's normally,
the ungrilled build session's when grilling is skipped, and the session a
steered Draft starts — is extended with an instruction to switch the branch to
an appropriately named one (short kebab-case, chosen from what the Brief is
about) with git, early, before anything lands on it. A typed name gets no
instruction.

The **Draft** title carries beyond the draft while the name stays Verkstead's:
the sidebar and header keep reading "Draft" through the first minutes of
Grilling or Implementing, and when the rename lands — task 05 is what follows
it — the branch name becomes the title exactly as today. If the first session
ends without renaming, the title falls back to the branch name, so nothing
reads "Draft" forever; the name is then simply the Conversation's, and a later
rename is still followed as any rename is.

## Acceptance criteria

- [ ] An auto-named Conversation's first session prompt carries the naming
      instruction — grilling, ungrilled build and steered-Draft starts alike —
      and a typed-name Conversation's carries none.
- [ ] The title reads "Draft" from the start press until the rename lands,
      then the new branch name everywhere the branch is the title.
- [ ] A first session that ends without renaming drops the title to the
      branch name.
