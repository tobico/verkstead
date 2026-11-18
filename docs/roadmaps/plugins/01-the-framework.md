# 01. The framework and the Plugins page

## Goal

A directory with a `plugin.yaml` at its root is a **Plugin**, and Verkstead
loads one. The manifest's grammar is written down and read: a name, a version,
the interface version it was written against, a description, a mark as SVG, a
`provides:` list whose entries are each of one **Category**, the `capabilities:`
the plugin asks for, the `settings:` fields it wants drawn, and per-platform
overrides keyed `linux`, `macos` and `windows` over the common declaration. The
loader stands on wasmtime and the component model, with a WIT world per
Category — none of which has a Category behind it yet, so what this stage proves
is the loading, the versioning and the trouble reporting, on a manifest-only
plugin that provides nothing and a component that exports nothing. Plugins live
under `plugins/` in the Data Directory. The installed list is a **Replicated
record**, and every device clones the source for itself.

The settings page gains a **Plugins** section at the foot of the list, beside
Agent Profiles and Repos: two groups, Installed above Built-in, each row
reading the plugin's name, what it provides, where it came from, a checkbox
and — where it failed to load — its trouble. **Add a plugin…** takes a git URL
or a browsed path, clones it, reads the manifest, shows what it provides and
the capabilities it asks for, and **Install** confirms. A git row has
**Update**, a press that pulls; nothing fetches on its own. **Remove** is
refused while anything open stands on the plugin. Opening a row draws a pane
with the plugin's declared settings fields. And `verkstead plugin check <dir>`
says whether a directory is a plugin Verkstead would load, and why not.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *The shape of a plugin***: a plugin
  is declarative, with a WebAssembly component where code is needed. The
  built-ins are the same artefact a user writes, which is why the loader has to
  be real from the first stage: every later stage puts a built-in through it.
- **[ADR-0030](../../adr/0030-plugins.md), *Capabilities and trust***: a
  component reaches WASI capabilities the manifest asks for — filesystem paths,
  outbound hosts, environment — shown to the human and granted at install. The
  consent step in **Add a plugin…** is that grant, and it is the whole of it:
  there is no later prompt, and a plugin updated to ask for more is re-shown
  before the pull lands. Pure host-mediated functions and a CLI tool the host
  runs were considered and turned down (Set 1762, Q8).
- **wasmtime with the component model**, the interface written as WIT and
  bindings generated both sides (Set 1762, Q13). wasmer with a hand-rolled ABI
  and Extism were the alternatives. The cost accepted is something like 10 to
  20 MB on the single binary, which ADR-0004 already took once for the viewer
  and syntect.
- **The interface version is in the manifest**, and a plugin written for a
  newer Verkstead is refused at load with that said (Set 1762, Q14). Loading
  what parses and letting the gaps show was rejected: a gap in a plugin shows
  up as a session doing something wrong hours later.
- **A plugin that fails to load is listed with its trouble, counts as disabled,
  and nothing else is affected** (Set 1762, Q15) — a manifest that does not
  parse, a component that traps at instantiation, a pull that fails. This is
  the rule `config.yaml` is built on: nothing in it is ever an error that stops
  the server.
- **Where plugins live** ([ADR-0030](../../adr/0030-plugins.md)): a git URL or a
  local directory, under the Data Directory's `plugins/`, a git one tracking its
  default branch and pulled by a press. An archive URL and a catalogue were
  turned down (Set 1760, Q4). The installed list is a **Replicated record** —
  name, source, enabled, the revision pinned at install, and for a local
  directory the device it is on — cluster-wide like Authors and MCP servers,
  each device cloning the source itself (Set 1762, Q10). A local one is marked
  as that device's and drawn greyed elsewhere with that said.
- **One plugin may provide several things**, each of one Category, and the page
  lists the plugin once (Set 1762, Q9). The row says what it provides.
- **Settings a plugin needs are declared** — text, size, list, secret — and
  drawn in a pane when its row is opened (Set 1769, Q43). A secret goes where
  MCP header values go: `secrets.yaml`, never returned to the viewer, the record
  carrying a digest. Author tokens are not plugin settings; they stay in the Git
  section.
- **Disable is allowed over standing things; Remove is refused by them.**
  Disabling takes the plugin out of every picker and finishes what is running
  on it; Remove is refused while anything open stands on it, the refusal
  counting them and leading there (Set 1769, Q45; Set 1764, Q20). The two
  groups are Installed above Built-in; a built-in has no Remove.
- **The name is the manifest's**, unique among those installed, and a name a
  built-in holds is refused. Nothing about the plugin is keyed by its source.
- **The word is *Plugin*** (Set 1760, Q6). CONTEXT.md's three Avoid entries
  that name the word — MCP server, Components, Skill — stand, since each says
  not to call *that* a plugin.
- **The page sits at the foot of the settings list** beside Agent Profiles and
  Repos, the other two lists of things Verkstead runs on.
- **The built-in components are built once and embedded everywhere.** A
  component is platform-neutral bytes, so the release workflow gains one
  `wasm32-wasip2` build of every bundled component, and the musl Linux binary,
  the lipo-merged Mac binary and the Windows one each embed the same bytes the
  way the viewer is embedded today. Stage 04 is the first to put a built-in
  through it; this stage lands the job with a fixture component so the path is
  proven before a harness depends on it.

## Proposed tasks (provisional)

1. **The manifest** — a `plugin.yaml` grammar in the server with a reader that
   refuses by field, and `verkstead plugin check <dir>` over it. AC: a minimal
   manifest passes; a missing name, a Category nobody knows, a capability of an
   unknown kind, and an interface version ahead of this build each refuse
   naming the field; the check prints the refusal and exits non-zero.
2. **The loader** — wasmtime in the server, a component instantiated under the
   capabilities its manifest asks for and nothing else, WIT worlds declared per
   Category with no Category yet behind them. AC: a component exporting nothing
   loads; one asking for a path it was not granted cannot read it; one that
   traps at instantiation is a trouble and not a crash; the binary still builds
   for every release target, and the release workflow builds a fixture
   component for `wasm32-wasip2` and embeds it in each platform's binary.
3. **The record and the directory** — the installed-plugins Replicated record,
   `plugins/<name>/` under the Data Directory, a clone at install, a pull on
   Update, the local-directory case marked by device. AC: a plugin installed
   on one device is listed on another and cloned there; a local one reads as
   this device's elsewhere; a pull that fails leaves the old checkout and a
   trouble.
4. **The Plugins section** — card, pane, two groups, the row, the checkbox, the
   trouble line, Remove and its refusal. AC: a plugin with trouble shows it on
   its row; unticking a row disables it; Remove on a built-in is not drawn;
   Remove on a plugin something stands on is refused with the count.
5. **Add a plugin…** — URL or browsed path, clone, manifest read, the consent
   card listing what it provides and the capabilities, Install. AC: a URL that
   is not a repository refuses before anything is listed; a manifest asking for
   `gitlab.com` outbound shows it before Install; a name colliding with a
   built-in is refused.
6. **The settings pane** — declared fields drawn by kind, a secret written to
   `secrets.yaml` with a digest on the record. AC: a text field round-trips; a
   secret never comes back to the viewer; a size that is not a size is refused
   with the reason.

## Re-verify at start

- `wasmtime` still builds for the three release targets in `release.yml` —
  static musl on Linux, the lipo-merged universal Mac binary, Windows — and the
  size it adds is still what the ADR accepted. A target it cannot build for is
  a decision to bring back, not a workaround to find.
- Replicated records are still one SQLite table of kind, uuid, stamp and JSON
  body spread by pull (`crates/store/src/replicated.rs`,
  `crates/server/src/replicating.rs`), so an installed-plugins kind is a row
  kind added rather than a mechanism built.
- The settings page still adds a section through `WORDS` in
  `web/src/settings/openings.ts`, a card and a pane, and `Details` still
  switches on the opening.
- `Skills::installed` still wipes and rewrites `data_dir/skills` at start, so
  `plugins/` must be a sibling it never touches.
- The MCP servers section is still the model for a list with add and remove
  and for a secret kept beside a record (ADR-0021, ADR-0028).
- The three Avoid entries naming *plugin* still read as they did, and nothing
  else in CONTEXT.md has since claimed the word.
