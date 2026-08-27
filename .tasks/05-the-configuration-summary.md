# 05. What the Conversation was configured with

## What to build

The Brief's details pane gains a summary of the whole of a Conversation's
configuration, read where the frozen Brief is read.

**Why it belongs here at all.** The setup rows go when the card freezes, so
without this a read-only companion would leave no trace anywhere for the rest of
the Conversation's life — a read-write one surfaces later through its commits and
its pull request, and a read-only one never does. And the companions are not the
only thing: the worktree directories and the picked Pairings are shown nowhere
today either, and they belong in the same place.

**What it says**, under the Brief's own markdown:

- the Repo, the branch the work is on, and the base commit it came off
- every worktree directory — the Conversation's own and each companion's
- both Pairings, grilling and implementation, profile and model
- each companion, with its mode, its branch and its directory

**The base is the commit, not the picked branch.** The pick is a branch name
while the Conversation drafts and is replaced by the commit it resolved to when
grilling starts, which is the honest thing to report about work already under
way.

**A Conversation with no companions still gets the summary** — the worktree
directory and the Pairings are as unfindable today as a companion is.

**Read-only throughout.** The pane reports the configuration; the setup card is
still the only place any of it is changed. Nothing here is a control.

**The three documents share one plain renderer today**, so the Brief needs one of
its own to carry this. The handoff and the instruction panes are not changed by
it.

## Acceptance criteria

- [ ] Opening the Brief shows, under the Brief itself, the repo, the branch, the
      base commit the work came off, every worktree directory, and both
      Pairings.
- [ ] Each companion is listed with its mode, its branch and its directory.
- [ ] A Conversation with no companions still gets the summary, and the handoff
      and instruction panes are unchanged.
- [ ] Nothing on the pane changes anything.
