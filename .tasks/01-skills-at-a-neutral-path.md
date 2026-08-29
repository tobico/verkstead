# 01. The skills mount at `/verkstead/skills`

## What to build

The bundled skills stop landing inside the Profile's home and start landing at
a path no backend owns, and the hiding the old mount did is kept by an empty
directory put where they used to be.

**The mount moves to `/verkstead/skills`.** A path of Verkstead's own, beside
`/verkstead/bin`, which is already how the executable and the sccache reach a
sandbox — the directory is made by the bind itself and holds what this binary
ships and nothing the host put there. Read-only, for the reason it always was:
what a session is grilled by is the product's, not a file the session can
rewrite mid-run. Nothing about how the skills are installed under the Data
Directory changes; what changes is where a sandbox binds them.

**An empty directory is bound read-only over `~/.claude/skills`.** The old
mount hid whatever the account kept there, and that hiding was as much the
point as the mounting was: a Profile is an account to run as rather than a
second opinion about how to work, and the case it guards is an older fork of
Verkstead's own skills sitting in the account directory. A mount that has moved
away covers nothing, so the empty bind takes its place. It goes **after** the
bind of the Profile's pair over `~/.claude`, because bwrap applies binds in the
order they are given and the one that lands second is the one that wins — the
ordering comment that governs the mount today governs this instead. Read-only,
so a session cannot fill it in and then read from it.

**Every prompt names the new path.** The constants that send a session into a
skill are what a session actually reads — the prompt names the skill above the
Brief, because a sandbox has no global instruction file to say what the session
is for. All of them move, including the one held only by a test. They stop
being written with a tilde: the tilde was there because the path was inside
whatever HOME a sandbox has, and this one is absolute and the same for every
session.

**And the skills' own cross-references.** The grilling skill reads on into two
others by naming their paths; both move with the mount. After this, no prompt
and no bundled skill names `~/.claude/skills` as somewhere a skill is read from.

CONTEXT.md's **Skill** and **Sandbox** entries say where the skills mount, so
both are corrected here.

## Acceptance criteria

- [ ] A session reads the grilling skill from `/verkstead/skills`, and cannot
      write to it; the whole installed set is there and nothing of the host's
      own checkout is reachable.
- [ ] A skill the Profile's account directory keeps under `~/.claude/skills` is
      not visible to the session: the directory is there, empty and read-only,
      while the rest of the Profile's pair is writable as before.
- [ ] No prompt constant and no bundled skill names `~/.claude/skills` as
      somewhere to read a skill from, and a grilling session started from the
      button still reads its skill and runs exactly as it did.
- [ ] CONTEXT.md's Skill and Sandbox entries name the new path and the empty
      bind that keeps the hiding.
