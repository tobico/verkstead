# 02. The Built-in rows and the tick

## What to build

**Built-ins get a record.** Today a built-in has none, so the Plugins page's
Built-in group is always empty and there is nowhere for a tick or a setting to
live. A built-in's record is created on the first write to it — a tick, a
settings save, a take-up — and **absent means enabled**, so a cluster that never
touched a built-in has it on. It is a Replicated record like an installed
plugin's (cluster-wide), but has no source to clone and no Remove.

**The rows.** The six Language plugins are listed under Built-in, each with its
display name, version, author and what it provides — the languages, by label
(Rust & C/C++ says both). The fixture stays unlisted.

**The tick is the switch.** Whether a new session gets a language is read from
its plugin's enabled flag: unticked, none of its languages' variables, stores
or grants reach the session, and the Compile Server is not wanted for it.
The Sandbox page's per-language checkboxes go (the size fields stay until task
03); a language no plugin provides — an installer's own — keeps its Sandbox
row until task 05 gives it one on the Plugins page.

**The take-up of `enabled`**, once, at the first start after this lands, the
way each Replicated record kind took up its `config.yaml` key (ADR-0028):
`languages.<name>.enabled` and `rust_build_cache.enabled` are read, and not
written again. **On wins**: a plugin is on if any of its languages was on, on
any device (Rust off and C/C++ on comes up on — Set 1784, Q4). So a device's
take-up only ever moves a record toward on, and never over a human's press: a
record a tick has written since is left alone by a device that upgrades later.

## Acceptance criteria

- [ ] The Plugins page lists the six under Built-in, each saying the languages it provides, with no Remove; the fixture is not listed.
- [ ] Unticking Go means the next session has no `GOMODCACHE`; ticking it again restores it; the Sandbox page has no language checkboxes.
- [ ] A language `enabled: false` on every device before this lands is unticked after it, and one enabled on any device (or with no entry) is ticked; a device upgrading after a human unticked a plugin does not tick it again.
