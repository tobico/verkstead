# 04. The to-tasks fork in-conversation

## What to build

The second direction. Choosing **task list** runs Verkstead's own fork of the
to-tasks workflow in the Conversation's Worktree, and it produces a real
`.tasks/` backlog — `TODO.md` plus numbered task files — committed in the repo.

Repo files stay the source of truth. Verkstead does not own the backlog; it runs
the workflow that writes it and reads it back afterwards.

The workflow interviews the human, and in this setting its breakdown quiz
arrives as **ordinary Question Sets** on the Timeline — the same blocking asks
grilling already uses, with the session idling until the answers come back.
Nothing new is needed for the asking; what is new is the skill.

Bundling follows the pattern grilling established: a directory under the
server's skills, embedded and written out under the State Directory at startup,
already bind-mounted read-only into every sandbox. A session enters a skill
purely by the prompt it is launched on naming the skill's mounted path — so this
needs the skill's text plus an entry point that launches a session into it, and
no new mechanism.

The fork drops what a workstation-driven flow assumes and Verkstead supplies
instead: the branch is already made, the feature is already chosen, and the
plan commit is not something to ask permission for.

## Acceptance criteria

- [ ] The to-tasks fork is bundled, installed and mounted exactly as grilling is
- [ ] Choosing task list launches a session under the implementation profile,
      inside the fork, in the Conversation's Worktree
- [ ] Its breakdown questions arrive as ordinary Question Sets on the Timeline
      and the answers reach the waiting session
- [ ] `.tasks/TODO.md` and numbered task files land committed in the Worktree
- [ ] The fork does not re-create the branch or ask for approval to commit
- [ ] The Conversation moves to Implementing
