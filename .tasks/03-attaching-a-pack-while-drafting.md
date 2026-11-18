# 03. Attaching a pack while the round drafts

## What to build

An installed pack is attachable to a Conversation from the Attach menu,
following the declared-MCP-server pattern exactly (ADR-0021, ADR-0030 *Skill
packs*) — an installed pack is a thing the human said, and a Built Root is an
allowlist.

**The menu and chips.** The composer's Attach menu gains a section of packs
beside its MCP servers: every enabled, loaded installed pack not already
attached. The built-in `verkstead` pack is never offered (Q8a) — it is always
there. A pick is kept as a chip read back from the Conversation, removable with
× while the round drafts and fixed once it starts (refused as not drafting, the
way `attach_mcp_server_to_draft` is). The compose page replays its picks after
the Conversation is created, and a draft moving device replays them, as
servers do. A frozen Brief shows them read-only. Mirror the MCP-server path:
a store table beside `conversation_mcp_servers`, loaded onto the Conversation,
carried as a slice on a Transfer and swept by cleanup; attach/remove routes and
render types beside `AttachedServerView`.

**The mount.** Today the sandbox binds the whole skills directory at
`/verkstead/skills`. Instead it binds `verkstead/` and each attached pack at
`/verkstead/skills/<pack>`, read-only, so a session has the packs its
Conversation attached and no others. On a Mac the seatbelt reaches the same
per-pack real paths, and on Windows the grant does.

**The prompt** (Q6). Beside `# MCP servers`, and appended where every session
is launched rather than in each builder (as `skills::served` is), a neutral
`# Skill packs` heading lists each attached pack with its skills and the path
of each `SKILL.md`, so a session finds them by reading rather than exploring.
The built-in pack is not listed (Q8c). No attached pack, no heading.

**Gone at launch** (Q7). A pack attached but disabled or removed since — or a
local plugin's pack on a device that doesn't have it after a Transfer — keeps
its chip, marked gone, and is neither bound nor listed.

## Acceptance criteria

- [ ] A Conversation with a pack attached has its sessions read the pack's
      skills at `/verkstead/skills/<pack>/<skill>/SKILL.md`, listed under
      `# Skill packs` in the prompt; a Conversation without it cannot reach
      that path
- [ ] A disabled pack is not on the menu, and one disabled after attaching is
      a gone chip, neither bound nor listed
- [ ] A pack chip can be removed while drafting and is refused once the round
      has started; the attached packs survive a Transfer
