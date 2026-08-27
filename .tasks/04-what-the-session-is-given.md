# 04. What the session is given

## What to build

The companions reach the agent: bound into every session's sandbox by mode, and
named in every session prompt.

**The sandbox binds each companion's worktree and its repo's git common dir**,
both of them, read-only or read-write with the companion's mode. Both are needed
for the same reason the main repo needs both: a worktree's own git directory
lives inside the repository's, so a checkout bound without it has no object
database behind it. The extra binds a sandbox already takes are read-write only,
so a bind now has to carry which it is.

**A read-only git common dir is the thing to actually verify rather than
assume.** Git writes inside that directory for more than committing — the index
and its lock live there — so the task is done when reading from inside a
read-only companion genuinely works and writing to it genuinely does not, tested
rather than reasoned about.

**That companion repo's own configured sandbox binds are composed in too**,
because a companion's build needs its build caches like any other repository's.
They are bound **read-write as configured, whatever the companion's mode**: they
are the installer's own holes, they sit outside the repository, and a build
writes to them — a read-only companion whose cache was read-only would fail on a
cold cache for no gain.

**The dev shell stays the main worktree's alone.** A session is launched under
`nix develop` for the Conversation's own repo where its flake provides a shell,
and nothing here wraps a second one. A companion with a flake of its own is
entered by the agent, `nix` being on the sandbox `PATH` — the binds are half of
what a companion's build needs and this is the other half. Nothing to build:
worth saying so the session does not invent a second wrapper.

**The prompt carries one neutral listing and no instructions.** A
`# Companion repositories` section naming each companion, its worktree path, its
branch and its write status, on **every** session prompt of the Conversation, the
grilling one included. Neutral by design: the agent decides from the Brief what
to use, and the listing says nothing about what to do with them — which is also
why it says nothing about dev shells.

**Appended once, where every session is launched from.** That one place already
holds both the Conversation and the prompt and already composes the sandbox
binds, so the grilling prompt gets the listing free rather than as a special
case, and a prompt builder added later cannot forget it. The words themselves
stay with the other prompt blocks and their tests, as one block builder the
launch calls.

## Acceptance criteria

- [ ] From inside a session, a read-only companion's worktree can be read and
      its history shown, and a commit or push from it is refused.
- [ ] A read-write companion takes a commit on its own branch from inside the
      session.
- [ ] A companion repo's own configured sandbox binds are composed into the
      sandbox read-write, whatever the companion's mode.
- [ ] Every session prompt of the Conversation, the grilling one included,
      carries one `# Companion repositories` section naming each companion, its
      path, its branch and its write status — and no instruction about what to
      do with them.
