# 05. Add a plugin from a local directory, and the Plugins section

## What to build

The first end-to-end install, from a local directory, and the page that lists
it.

**The record.** A new Replicated record kind for installed plugins (ADR-0028):
one row per plugin, keyed by its **uuid**, with the source, enabled, and for a
local one the path and the **device** it is on. Task 06 adds the pinned
revision. It is added as a row kind in the existing replicated table, the way
Authors and MCP servers are (see `replicating/` in the server). Nothing about a
plugin is keyed by its source.

**A local plugin is read in place** (Set 1772, Q2). The record holds its path
and nothing is copied. **Update** on a local row re-reads the manifest, and
re-shows the consent card first where it now asks for more capabilities, so
editing then pressing Update is the development loop. Git plugins go under
`plugins/<uuid>/` in the Data Directory in task 06. `plugins/` is a sibling of
`skills/`, which `Skills::installed` wipes at start, so nothing here may live
under `skills/`.

**Loading.** At start, and whenever the record changes (including a merge from
another device), the server reads every enabled plugin this device can reach
through task 01's reader and task 03's loader. A failure is **trouble** on that
plugin, which then counts as disabled, and nothing else is affected.

**Add a plugin…** In this task it takes a browsed path, using the path field and
browsing dropdown that Agent Profiles use. It reads the manifest and draws the
**consent card**: the plugin's name, version and author, what it provides, and
every capability it asks for. **Install** is the grant (ADR-0030, *Capabilities
and trust*). There is no later prompt. A uuid already installed, or held by a
built-in, is refused with that said.

**The section.** A `plugins` word in `WORDS` (`web/src/settings/openings.ts`), a
card at the **foot of the settings list after Repos**, and a pane listing
**Installed above Built-in**. The Built-in group is drawn only when it has a row
(Set 1772, Q4), so it is absent in this stage. Each row shows the display name,
version and author, what it provides (by Category), where it came from, a
**checkbox**, and a **trouble line** where there is one. Unticking disables the
plugin cluster-wide. A local plugin is drawn on other devices **greyed, saying
whose it is** ("on wolf"), and is not loaded there. Read `CONTEXT.md` and
`docs/copy.md` before naming anything on screen. Follow the MCP servers section
for the shape of a list with add.

## Acceptance criteria

- [ ] A local fixture plugin is added through the browsed path and consent card,
      installed, and listed on its row with name, version, author, provides and
      source.
- [ ] Breaking its manifest on disk, then pressing Update, shows its trouble on
      the row and counts it as disabled. Other plugins and the server are
      unaffected, and a plugin that is fine again loads at the next Update.
- [ ] Unticking a row disables the plugin, and the record says so on every
      device.
- [ ] On a second device the local plugin is listed greyed, naming the device
      it is on (a two-device suite test).
- [ ] Adding a uuid that is already installed, or a built-in's, is refused.
