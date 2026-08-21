# 04. Adopting — every refusal by name

## What to build

The Adopt press refuses by name, and the page says which. A human is present and
pressing the button, so each refusal is answered to them directly — this is not
a decision taken while nobody was watching, and so it is not a **Notice**.

Every one of these is something different for the human to go and do, so none of
them collapses into a single *cannot adopt*:

- the Conversation is not drafting — it has been adopted already, or aborted;
- no grilling Profile is chosen, or no implementation Profile is;
- a chosen Profile's pair is not where it was left, so there is no account to
  run under;
- the stage is **no longer startable at the base**, which is three answers and
  not one: every stage of the roadmap is now complete, the brief it names cannot
  be read there, or an in-progress annotation on it names a branch that still
  exists;
- the stage's slug branch is already taken in the Repo;
- nothing in the repository answers to what the base commit resolves to;
- git would not make the Worktree — the one refusal with nothing for the human
  to correct, whose reason is in the server log.

They are **checked cheap-first**: the record's own state, then the Profiles,
then the answers that cost a git call. The startability answers are the same
rule task 01 reads for the notice, asked again here at the base commit rather
than at the default branch tip — which is why *gone complete* is among them at
all: between the notice being drawn and the button being pressed, somebody may
have ticked the last box.

A branch-exists read that **fails** counts as **taken**. That is the safe way
round: what is on the other side of it is making a branch and letting an agent
loose on it.

Nothing is created and nothing is checked out for any refusal.

## Acceptance criteria

- [ ] Each refusal above comes back by its own name, and the page draws a
      different line for each.
- [ ] A refused press leaves no Conversation moved, no branch made and no
      Worktree on disk.
- [ ] A branch-exists read that fails is treated as taken, and the cheap checks
      are answered before anything that costs a git call.
