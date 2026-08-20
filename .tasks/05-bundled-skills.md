# 05. Bundled skills

## What to build

Verkstead ships its own skills and installs them into every sandbox, so a
session's behaviour is the product's rather than whatever happens to be on the
host. `~/src/tobico-skills` stops being bound in — the previous task's surface
never had it, and this is what replaces what it would have supplied.

Only the grilling skill is forked now. The design's full set, gates removed and
wrap-up added, arrives with the stages that need it: stage 03 for the
implementation skills, stage 04 for wrap-up. Forking them now would be writing
against requirements that do not exist yet.

The fork is mostly new writing rather than a copy. The upstream grilling skill
is twelve lines of generic "interview me relentlessly" and never mentions
askance or Question Sets at all: what actually makes an agent ask is the host's
global `CLAUDE.md`, and the sandbox does not have one. So the bundled fork has
to carry the ask instruction itself — either in the skill or in the Profile's
`~/.claude/CLAUDE.md`, but somewhere inside the sandbox, and said explicitly.

Whatever it says has to name the CLI as `verkstead`, not `askance`: the real
askance stays installed on the host and the agent-facing surface was renamed
deliberately.

The skills ride inside the binary, as the viewer already does, so there is
nothing beside the executable that can go missing.

## Acceptance criteria

- [ ] A session in the sandbox finds the bundled skills installed, and finds no
      `~/src/tobico-skills`
- [ ] The bundled grilling skill instructs the agent to ask through the
      `verkstead` CLI, without relying on any host configuration
- [ ] The skills are embedded in the binary rather than read from beside it
- [ ] A grilling session started with no host `CLAUDE.md` reachable still asks
      through Question Sets
