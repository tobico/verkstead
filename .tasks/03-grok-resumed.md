# 03. Grok, and the directory its store is keyed by

## What to build

The second harness, and the first one whose log has to be moved as well as
carried.

**Its id is Verkstead's own**, as Claude's is: Grok Build takes the session id
on the launch line, insists on a valid UUID and refuses one it already has a
session for, so the name is on the record already and task 01's lookup finds
it with nothing added. Its resume line is the same shape as Claude's — a flag
and the id, the prompt staying positional — and the same exclusion holds: a
resumed session is not also a named one, and the record of what this session
was called keeps the old name so the log stays findable.

**What differs is where the log sits.** Grok files a session's directory under
its own encoding of the working directory the session ran in, and the memory
sync carries that directory verbatim: what it picks out of the store is the
session directories called one of the ids Verkstead named, and what it sends is
each one under its path relative to the store. So the log arrives on this
device still filed under the *sending* device's encoding of the *sending*
device's Worktree path — which is why the Transcript search walks the store
rather than reproducing the encoding, and why following the log works while
resuming it would not. Grok resolves a resume against the directory it is
started in.

So the arriving session directory is **relocated to this device's encoding of
this device's Worktree path** as the sync lands it. Which encoding that is, is
grok's own arithmetic over a path and the store is walked rather than the
arithmetic reproduced — so the honest way to name the destination is to find
what this device's store already calls this Worktree, and to fall back cleanly
where it has never run grok here before. Getting that wrong is a resume that
refuses, which is the fallback and not a broken session.

Nothing here changes what Claude does, and nothing the two share is generalised
before there is a third: task 04 is what says whether the relocation is one
mechanism or two.

## Acceptance criteria

- [ ] A grok session transferred mid-turn resumes into the same conversation on
      the receiving device, primed with the note and no re-prime.
- [ ] The carried session directory is under this device's encoding of this
      device's Worktree path, not the sending device's.
- [ ] The Transcript of the resumed session opens where the carried log ended,
      as Claude's does.
- [ ] A store this device has never run grok in, or a directory the relocation
      cannot name, falls through to Verkstead's Resume.
