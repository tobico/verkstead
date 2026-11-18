# 02. The Skills category

## What to build

A plugin's `provides:` entry with `category: skills` becomes a real thing, and
the `verkstead` pack becomes the built-in plugin that provides one.

**The grammar** (settled in Set 1785):

```yaml
provides:
  - category: skills
    name: rust-review      # the <pack>: lowercase letters, digits and hyphens
    directory: skills      # relative to the plugin; holds <skill>/SKILL.md
```

- `name` is the pack's name (Q3). `verkstead` is reserved for the built-in; an
  installed plugin whose pack name is already taken — by the built-in or by
  another loaded plugin — loads **in trouble**, the trouble naming the clash.
- Each `<skill>/SKILL.md` under the directory must open with frontmatter
  carrying a `name` equal to its directory's name and a non-empty
  `description` (Q5) — how all thirteen are written today. Anything else is a
  refusal naming the skill, from the manifest reader, so `verkstead plugin
  check` and loading say the same sentence. A pack asks for no capability and
  carries no component: the Category is pure data.

**The built-in pack** (Q2): the thirteen skills move from
`crates/server/skills/` to `plugins/verkstead/skills/`, beside a `plugin.yaml`
(a fresh uuid, `provides:` one `skills` entry named `verkstead`). `build.rs`
learns to embed a built-in's whole directory rather than only its manifest and
wasm, so the pack is read through the same manifest reader an installed
plugin's is. Debug builds still read the skills off disk, so editing one is
visible to a running `cargo run -p verkstead-cli -- serve` without a recompile.
Update the memory/docs pointer that says the skills are under
`crates/server/skills/` (docs/development.md and anything else naming it).

**The Plugins page**: the `verkstead` built-in is the first row of the
Built-in group (which is drawn only once it has a row). A built-in has no
Remove and, per Q8b, no enable switch. The fixture built-in stays unlisted.

**Installed packs reach the disk** (Q4): each enabled, loaded installed
plugin's pack is copied into `data_dir/skills/<pack>/` when it loads and again
whenever the plugins record moves (an Update, a disable, a remove, a merge from
a member), through the same `said_at` rewrite the built-in pack gets. A
disabled, removed or troubled plugin's pack is not there. Nothing mounts or
lists these packs yet — that is task 03.

## Acceptance criteria

- [ ] `verkstead` is a Built-in row on the Plugins page, providing `skills`,
      with no Remove and no enable switch
- [ ] `verkstead plugin check` passes a plugin directory with a `skills` entry;
      a `SKILL.md` without frontmatter, or whose `name` is not its directory's,
      is a refusal naming the skill; a pack named `verkstead` is refused
- [ ] An enabled installed plugin's pack lands under `data_dir/skills/<pack>/`;
      disabling it removes it, and a second plugin claiming the same pack name
      loads in trouble
- [ ] Sessions still read `/verkstead/skills/verkstead/<skill>/SKILL.md`
      exactly as after task 01
