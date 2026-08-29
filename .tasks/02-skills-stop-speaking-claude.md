# 02. The skills stop speaking Claude

## What to build

The bundled skills are read by every backend from the stage after next, so they
stop naming mechanisms that only one agent has. Prose only: nothing about what
a session does changes here, and no per-backend advice is added — the Guide is
where that will live, tailored at print time, and the Guide is stage 02's.

**The ask advice defers to the Guide.** Every skill that asks carries a line
telling the session to run `verkstead ask` as a background command, which is
Claude Code's own mechanism for holding a shell command open for hours and is
false of backends whose shell tools yield after seconds. What each skill has to
keep saying is what is true of every backend: the ask blocks until the human
answers, that may be hours, waiting is the tool working rather than failing,
and only work the answers cannot invalidate is worth doing meanwhile. **How**
to run it comes out and is left to the Guide, which every skill already tells
the session to read before its first ask.

**`CLAUDE.md` becomes `CLAUDE.md` or `AGENTS.md`.** Nine skills tell a session
to read what the repository it is working in says about itself, and name
Claude's file for it. That is the target repository's file rather than anything
about which agent is reading, but the wording still points one way — and three
of the nine already name both, so the fix is making the other six read like
them.

The skills carry blocks word for word across several of them, and tests hold
them to each other; a sweep that edited one copy of a shared block would break
that, so every copy moves together.

## Acceptance criteria

- [ ] No bundled skill names a Claude-only mechanism for holding an ask open,
      and each still says the ask blocks, that it may take hours, that waiting
      is the tool working, and that the Guide says how to run it.
- [ ] Every skill that tells a session to read the repository's own
      instructions names `CLAUDE.md` or `AGENTS.md`, matching the three that
      already did.
- [ ] The shared blocks the skills carry word for word still match each other,
      and the tests that hold them to each other pass unchanged.
- [ ] The Guide is untouched: its Claude-worded advice stays as it is until the
      stage that tailors it per backend.
