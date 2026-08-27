# 03. A block per repo under the Set

## What to build

The Diff under a Question Set covers every repository the Conversation may write
to, each labeled and read on its own.

**Composed per repo, at Set arrival**: the Conversation's own worktree first,
then each read-write companion's in the order the Conversation was configured
with them. Read-only companions contribute nothing — their checkouts are
detached and bound read-only, so there is nothing uncommitted to find.
Uncommitted-only throughout, exactly as the main repo's is: committed work is on
the Timeline as Events already. **A clean worktree contributes no block, and all
of them clean means no Diff at all**, as today.

**A Set carries them as a list of repo-and-patch.** The single Diff field stays
where it is and keeps its meaning for the Sets already stored, which must go on
rendering as they always did; the per-repo list is a new field beside it, and a
Set that carries the list is drawn from the list. Each entry names its repository
by the Repo's registered name.

**One Diff section on the page**, with a labeled block per repo inside it and the
per-file folds inside those. The table of contents keeps its one *Diff* entry and
groups each repo's files under that repo's name, so a jump still lands on the
fold it names. **A Set with only the main repo's block draws it unlabeled**,
exactly as today: the label earns its place when repos mix, which is the rule the
commit cards follow.

## Acceptance criteria

- [ ] A Set asked while the main worktree and a read-write companion's are both
      dirty draws two labeled blocks in that order, and the table of contents
      lists each repo's files under its name.
- [ ] A companion whose worktree is clean contributes no block; every worktree
      clean means no Diff section at all; and only the main repo dirty draws one
      unlabeled block as it does today.
- [ ] A Set stored before this task renders exactly as it did.
