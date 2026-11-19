# Plugins roadmap

Verkstead becomes **plugins over a core**. A harness, a Process, a Forge, a
Language, an Activity and a pack of Skills are each a **Plugin** — a directory
with a `plugin.yaml` at its root, declarative first and a WebAssembly component
where a piece is code — the ones Verkstead ships are Plugins of the same shape
bundled inside the binary, and a human installs, disables, removes and writes
their own from a **Plugins** section of the settings page, whose **Build a
plugin** button primes a Conversation to write one. The decisions and their
why are in [ADR-0030](../../adr/0030-plugins.md); the terms are in
[CONTEXT.md](../../../CONTEXT.md) under **Plugin**, **Category**, **Activity**,
**Outcome**, **Forge** and **Skill Pack**, and the revised **Skill** and
**Process** entries, which gain and revise further as each stage lands. The
briefs reference both rather than restating them.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

**Three threads hang off the framework**, and the declarations below are
their dependencies. Stage 01 is the manifest, the runtime, the record and the
page, and everything stands on it. **Agents** run 03 to 06: the interface and
the generic Profile first, then the five harnesses crossing to components one
at a time — Codex first for having no login and a plain record, then the three
with the harder records and the first real login dialogue, then Claude last as
the one in daily use. **Activities and the Process engine** run 07 to 11: the
Skill pack is first because an Activity names its skill by pack, the interface
lands with the smallest Activity, the other five cross in two stages that are
reorderable with each other, and the Process engine comes last because it can
only write the five Processes as data once every Activity they run is a
component. **The Forge** runs 12 and 13, standing on 09 because the wrap-up
Activity is the Forge's main caller and the two should not cross in one stage.
Languages (02) stand on 01 alone and are the page's first real rows, so 02 is
reorderable with 03 and 07. Build a plugin (14) stands on one landed stage per
Category, because its templates describe interfaces that have to exist first,
and on 13, whose GitLab repository is its Forge template.

**Stage 05 wants Windows**: the Antigravity login path runs on a pseudoconsole
the launcher makes as the session account, under `cfg(windows)`, and can be
verified nowhere else. Every other stage is shared code with a cross-platform
suite and declares no platform; where a Mac matters — the seatbelt a crossed
harness's login runs under — the brief says to re-verify there by hand.

## Stages

- [x] 01: The framework and the Plugins page — [brief](01-the-framework.md) — no dependencies
- [x] 02: Languages as plugins — [brief](02-languages.md) — after 01
- [x] 03: The Agent interface — [brief](03-the-agent-interface.md) — after 01
- [x] 04: Codex across — [brief](04-codex-across.md) — after 03
- [ ] 05: Grok, OpenCode and Antigravity across — [brief](05-grok-opencode-antigravity-across.md) — after 04 — on windows
- [ ] 06: Claude across — [brief](06-claude-across.md) — after 05
- [x] 07: Skill packs — [brief](07-skill-packs.md) — after 01
- [x] 08: The Activity interface and follow-up rounds across — [brief](08-the-activity-interface.md) — after 07
- [ ] 09: The interview, one session and the wrap-up across — [brief](09-interview-session-wrap-up-across.md) — after 08
- [ ] 10: The backlog and stage runners across — [brief](10-runners-across.md) — after 08 *(in progress: `roadmaps/plugins/10-runners-across`)*
- [ ] 11: The Process engine — [brief](11-the-process-engine.md) — after 09, 10
- [ ] 12: The Forge interface and GitHub across — [brief](12-the-forge-interface.md) — after 09
- [ ] 13: GitLab — [brief](13-gitlab.md) — after 12
- [ ] 14: Build a plugin — [brief](14-build-a-plugin.md) — after 02, 06, 11, 12, 13
