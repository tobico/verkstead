# Skill packs

The thirteen skills Verkstead ships become one built-in **Skill Pack** named
`verkstead`, and a skill is named `<pack>/<skill>` everywhere one is named — a
session is told to read `/verkstead/skills/verkstead/grilling/SKILL.md`, and the
mount under `/verkstead/skills` holds a directory per pack. **Skills** becomes a
real Category: a plugin's `provides:` entry of `category: skills` names a pack
and the directory of `<skill>/SKILL.md` it is, and the `verkstead` pack is the
Plugins page's first Built-in row.

A pack an installed plugin provides is **attachable to a Conversation** from the
Attach menu the way an MCP server is — while the round drafts, and from an
Answer — so every session of that Conversation finds it mounted under its name
and listed in its prompt beside the MCP servers. The account's own skills still
never reach a session (ADR-0011, as ADR-0030 *Skill packs* amends it). Stage 08
stands on this: an Activity names the skill it runs as `<pack>/<skill>`.

Roadmap stage: [07: Skill packs](docs/roadmaps/plugins/07-skill-packs.md)

## Tasks

- [x] 01: One pack named `verkstead` — [details](01-one-pack-named-verkstead.md)
- [x] 02: The Skills category — [details](02-the-skills-category.md)
- [ ] 03: Attaching a pack while the round drafts — [details](03-attaching-a-pack-while-drafting.md)
- [ ] 04: Attaching a pack from an Answer — [details](04-attaching-a-pack-from-an-answer.md)
