# 07. Skill packs

## Goal

The thirteen skills shipped inside the binary are one built-in **Skill Pack**
named `verkstead`, and a skill is named `<pack>/<skill>` everywhere one is
named: a session is told to read `/verkstead/skills/verkstead/grilling/SKILL.md`,
every prompt builder and every cross-reference inside the SKILL.md files says
the new path, and the mount under `/verkstead/skills` holds a directory per
pack. **Skills** is a Category: a `skills:` block in `provides:` points at a
directory of `<skill>/SKILL.md`, a plugin may carry a pack among the several
things it provides, and a pack is **attachable to a Conversation** from the
Attach menu the way an MCP server is — attached while the round drafts, and
from an Answer — so every session of that Conversation finds the pack's skills
mounted under its name and listed in its prompt beside the MCP servers. The
account's own skills still never reach a session.

## Decisions in force

- **Shared skills are a pack of their own** — the human's words on Set 1765,
  Q25: *a pack of skills becomes a module of its own, one module containing
  all the shared skills for verkstead's builtins*. Skills duplicated into every
  Process plugin, and a Process replacing a Verkstead skill by name, were the
  alternatives; the pack is what lets a Process or Activity run a skill by
  name without owning it.
- **Named `<pack>/<skill>`, mounted at `/verkstead/skills/<pack>/<skill>/`**,
  the built-in pack included ([ADR-0030](../../adr/0030-plugins.md), *Skill
  packs*). Leaving the built-in pack unprefixed was considered and turned down:
  two layouts under one mount is two things to learn, and the path a prompt
  names is where the file really is (CONTEXT.md, **Skill**).
- **A plugin may carry its own pack** (Set 1767, Q34). A Process bringing one
  skill is one plugin, not two.
- **A pack is attachable** (Set 1767, Q33). It follows the declared-MCP-server
  pattern exactly — ADR-0021's *said rather than found*, and ADR-0028's second
  moment, from an Answer — because an installed pack is a thing the human said
  and a Built Root is an allowlist. ADR-0011 stands word for word: what the
  account keeps under its own skills is still not reachable at all.
- **The prompt says so**: a neutral heading listing the packs attached, each
  with its skills and their paths, beside the `# MCP servers` list, so a session
  finds them by reading rather than by exploring the mount.
- **The Skills category is pure data.** A pack asks for no capability and
  carries no component; `verkstead plugin check` reads every `SKILL.md`'s
  frontmatter the way the skill loader does.
- **Stage 08 stands on this one**: an Activity names the skill it runs a
  session under as `<pack>/<skill>`, so the naming has to be settled before the
  first Activity crosses.

## Proposed tasks (provisional)

1. **The layout** — `data_dir/skills/<pack>/<skill>/`, the built-in thirteen
   installed under `verkstead/`, the mount unchanged at `/verkstead/skills`.
   AC: a session reads `/verkstead/skills/verkstead/grilling/SKILL.md`; the
   old path is not there; the startup rewrite still wipes and rewrites.
2. **Every path that names a skill** — the consts and builders in
   `crates/server/src/skills.rs`, the cross-references inside each SKILL.md
   (grilling to breaking-down and staging, next-task to implementing, and the
   rest), the CLI's Guide where it names one, the tests' fixtures. AC: a grep
   for `/verkstead/skills/<skill>` with no pack finds nothing outside history.
3. **The `skills` block** — a `provides:` entry pointing at a directory, read
   and checked, the built-in pack as a built-in plugin on the Plugins page. AC:
   the `verkstead` pack is a Built-in row; a plugin directory with a `skills`
   block passes the check; a `SKILL.md` without frontmatter is a refusal naming
   the skill.
4. **Attaching** — a pack on the Attach menu beside the MCP servers, kept with
   the Brief's chips, fixed at Start, attachable from an Answer; mounted into
   every session of the Conversation; listed in the prompt. AC: a Conversation
   with a pack attached has its sessions read the pack's skills at the pack's
   path; one attached from an Answer carries the waiting session on in a root
   that has it; a disabled pack is not on the menu.

## Re-verify at start

- Stage 01 landed: a Category is a `provides:` block kind and the Plugins page
  lists built-ins.
- `INSIDE` in `crates/server/src/skills.rs` is still `/verkstead/skills`, the
  read-only bind is still made in `sandbox.rs`, and `Skills::installed` still
  rewrites the directory at start.
- The thirteen skills still cross-reference each other by absolute path, and
  the human's own notes and memory files quote the old paths — worth a line in
  the pull request rather than a change.
- The Attach menu still offers files and MCP servers and nothing else
  (`web/src/Attaching.tsx`), and ADR-0028's from-an-Answer path still carries a
  waiting session on in a rewritten Built Root.
- The prompt still lists attached servers under `# MCP servers`.
