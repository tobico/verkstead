# 06. Memory sync

## What to build

The Profile's **memory switch holds away from home**. Before launch, what the
home account remembers of this Repo comes over the link and is written into the
account mirror with its paths rewritten for **this** machine; as the session
ends, what the session wrote goes back rewritten for the **home** machine. That
is what makes the transcript of a session run on B readable on A, and it is what
a harness's own resume will later stand on.

**The unit is the smallest one each harness's store has**, because only one of
the four keys its store by path:

- **Claude** — the two entries a Built Root already names: the Repo's main
  checkout, which holds Claude's memory of the Repo, and the Worktree, where the
  session's transcript is written.
- **Codex and Grok Build** — this Conversation's own logs out of the one flat
  directory they keep for every directory they have ever run in, with the memory
  files beside them. Their whole store is every repository the human has ever
  worked on and is not this Repo's anything.
- **OpenCode** — its data directory whole. The database in it runs in
  write-ahead-log mode and one copied a file at a time will not open.

**The path rewrite is the entry name**, for the one harness whose entries are
named after paths, and it has to match that harness's own encoding exactly — a
name computed any other way is a second entry rather than the same memory.

**Which path it is rewritten to** is two different answers:

- The **Repo's** entry goes through the match of task 03 — the home device's
  path for this repository on the way over, this device's on the way back. No
  match means there is nothing of that Repo to pull; the session starts without
  it and the Timeline says so, rather than the launch being refused.
- The **Worktree's** entry goes to **the path the home device would have used**:
  its own worktrees directory under its Data Directory, plus the same stem this
  Worktree was named with. A Worktree lives under the Data Directory rather than
  under the Repo, so there is no match to ask for — and naming it the way that
  machine would name it is what leaves the transcript findable when the work
  comes home.

**Switched off, nothing syncs.** A Profile whose memory switch is off starts on
an empty store away from home exactly as it does at home, and neither direction
carries anything.

## Acceptance criteria

- [ ] A session on B under A's Profile starts with what A's account remembered
      of the Repo, and a memory it writes is in A's account afterwards.
- [ ] Its transcript is readable on A once the session ends, under the entry A's
      own Worktree for that branch would carry.
- [ ] A Codex or Grok Build session carries its own logs and memory files both
      ways and leaves the rest of the home store untouched; an OpenCode one
      carries its data directory and opens against it.
- [ ] Memory switched off syncs nothing in either direction and the session
      starts empty.
- [ ] No Repo match pulls nothing, says so on the Timeline, and still starts the
      session.
