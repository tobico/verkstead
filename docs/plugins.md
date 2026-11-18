# Plugins

A **Plugin** is a directory with a `plugin.yaml` at its root. This is the
grammar of that file, written for the person writing one: what each key holds,
what Verkstead refuses, and how to find out before installing whether a
directory is a plugin Verkstead would load. Why plugins have the shape they do
is [ADR-0030](adr/0030-plugins.md)'s; the words are
[CONTEXT.md](../CONTEXT.md)'s **Plugin** and **Category**.

This document is the one place the grammar is written down. Each Category's own
grammar — what a `provides:` entry of that Category holds beyond its
`category` — is added here by the stage that puts the Category behind it.

## Checking a directory

    verkstead plugin check <dir>

reads `<dir>/plugin.yaml` with the same reader the server loads plugins with,
as each of Linux, macOS and Windows would read it, and prints either what the
plugin is or why Verkstead would not load it. A refusal names the field at
fault — `dependencies[0].version`, `capabilities[2]` — in the sentence the
Plugins page draws beside the plugin, says which platforms it was met on where
it is not all three, and exits non-zero, so the check can stand in a plugin's
own CI. No server is needed.

Where the manifest names a `component`, the check then starts it, the way the
server would and under the capabilities the manifest asks for: a file that is
not a WebAssembly component, one that imports something Verkstead does not
provide, one that traps while it starts, and one still starting after ten
seconds, which is stopped there, are each refused the same way, with the
sentence the Plugins page would draw on the plugin's row.

## Installing one from a directory

**Add a plugin…**, at the foot of the **Plugins** section of the settings,
takes the path of the plugin's directory and reads its manifest into a consent
card: who the plugin is, what it provides, and every capability it asks for.
**Install** grants those capabilities, and is the only time they are asked
about. A uuid already installed, or one a built-in plugin holds, is refused at
the card.

A plugin installed from a directory is **read where it is**, never copied, and
belongs to the device it was installed on: every other device of the cluster
lists it greyed, naming that device, and loads nothing of it. Its checkbox is
the cluster's.

**Update** on its row reads the manifest again, which is the loop for writing
one: edit `plugin.yaml` or rebuild the component, then press it. A manifest
that no longer reads is the plugin's trouble, drawn on its row, until an Update
finds it fixed; and one that now asks for more capabilities than were granted
shows the consent card again before anything changes. Until then it is loaded
by nobody.

## Installing one from git

**Add a plugin…** takes a git URL in the same field — `https://`, `ssh://`,
`file://` or `git@host:path`. The server clones it the way it clones a Repo,
as the one Author where there is one, reads the manifest into the same consent
card, and takes the clone back: a URL that isn't a repository is refused with
nothing listed or kept. **Install** clones it again into the Data Directory's
`plugins/<uuid>/`, and the record pins the revision it is at.

A git plugin is **every device's**: each device of the cluster clones it for
itself and checks out the pinned revision, and is in trouble until it can. It
tracks its repository's default branch, and nothing fetches on its own.
**Update** on its row is the pull: the default branch is fetched and read at its
new revision, and the record re-pinned, which every other device follows. A
pull that fails, or a manifest there that no longer reads, leaves the old
checkout loaded and says why on the row; one asking for more capabilities shows
the consent card again before the new revision lands.

## The smallest plugin

```yaml
uuid: 0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90
name: Nothing at all
author:
  name: Ada Lovelace
  email: ada@example.com
version: 0.1.0
interface: 1
provides: []
```

A plugin that provides nothing is a plugin: the first stage of the plugins
roadmap loads exactly this, and every later key is optional.

## The keys

**A key Verkstead does not know is refused**, at the top and inside every entry
below, naming it. A misspelt `capabilites:` read as no capabilities would be a
plugin quietly doing less than its author wrote, and a refusal at check time is
cheaper than a session doing the wrong thing hours later. The one exception is
the body of a `provides:` entry, which is its Category's to define.

### Who it is

- **`uuid`** — required. The plugin's identity: what its installed record, a
  dependency on it and everything stored about it are keyed by. Hyphenated,
  `8-4-4-4-12` hexadecimal digits; it is read in lowercase. Generate one once
  and never change it: a plugin with a new uuid is a different plugin.
- **`name`** — required. The display name, and only ever drawn.
- **`author`** — required. A map of `name` and `email`.
- **`version`** — required. A [semantic version](https://semver.org), such as
  `1.4.0`. `1.0` is not one, and is refused.
- **`repository`** — optional. The URL of the plugin's repository.
- **`interface`** — required. A whole number from 1: the interface version the
  plugin was written against. This Verkstead reads up to **1**, and a plugin
  naming a higher one is refused as being for a newer Verkstead, rather than
  loaded for whatever parses.

### What it looks like

- **`description`** — optional text.
- **`mark`** — optional. The plugin's mark, as SVG: either the SVG itself,
  inline, or a path to an `.svg` file inside the plugin's directory. Making it
  safe to draw is the viewer's job.

### What it needs

**`dependencies`** — optional, a list. Each entry says enough to name the
plugin it depends on and fetch it without it being installed:

```yaml
dependencies:
  - uuid: 7c1d2a43-0f5e-4b9a-8c6d-3e2f1a0b9c8d
    name: Shared skills
    author:
      name: Grace Hopper
      email: grace@example.com
    repository: https://example.com/shared-skills.git   # optional
    version: ^2
```

`version` here is a **version specifier** — `^2`, `>=1.4, <3`, `~1.2.3` —
over the dependency's semantic version, in Cargo's syntax. It is required: a
dependency on any version at all is one that breaks the day its author ships a
2.0.

A dependency is resolved whenever the installed plugins load — at the start, at
every press on the Plugins section, and when another device's change arrives —
against the installed plugins that loaded on this device and the built-ins.
Where one is **missing**, **disabled**, installed outside its specifier, a
directory on another device, or in trouble itself, the plugin depending on it
is in trouble, counts as disabled, and its row says which by the `name` written
here. The row offers the press that puts it right: **Install** from
`repository` through the consent card where the dependency isn't installed and
names one, **Update** on the dependency's own row where it is outdated. A
disabled one is ticked on its own row. Either way, once it is put right, the
plugin depending on it loads.

### What it provides

**`provides`** — a list, each entry of one **Category**: `agent`, `process`,
`forge`, `language`, `activity` or `skills`. One plugin may provide several, of
several Categories, and is listed once.

```yaml
provides:
  - category: forge
    # the rest is the forge Category's grammar
```

An empty list, or none, is a plugin that provides nothing.

#### `skills`

A **Skill Pack**: a directory of skills, each a `<skill>/SKILL.md`, that a
session reads at `/verkstead/skills/<pack>/<skill>/SKILL.md` and names
`<pack>/<skill>`.

```yaml
provides:
  - category: skills
    name: rust-review      # the pack: lowercase letters, digits and hyphens
    directory: skills      # relative to the plugin; holds <skill>/SKILL.md
```

Each directory at the top of `directory` is a skill, and its `SKILL.md` opens
with frontmatter — a `---` line, the skill's `name`, which is its directory's,
and a non-empty `description`, then another `---` — the way an agent's skill
loader reads one. A skill with no `SKILL.md`, no frontmatter, another `name`
or no `description` is refused, naming the skill. Files beside the skills are
the pack's own and carried with it; every file under a skill's directory goes
where its `SKILL.md` does.

`verkstead` is the name of the pack Verkstead ships, and refused for any other.
A pack name is one plugin's: a plugin whose pack has the name of one already
loaded loads in trouble, saying whose it is. A pack asks for no capability
and needs no component.

A loaded pack is offered on a Brief's Attach menu, under **Skill packs**
beside the MCP servers. Attached, every session of that Conversation has it
mounted read-only at `/verkstead/skills/<pack>/` and listed in its prompt with
each skill's path; a Conversation that did not attach it cannot reach it. A
pack disabled or removed after it was attached keeps its chip, marked *no
longer installed*, and is neither mounted nor listed until it is back.

A pack is offered on a question set's Attach menus too, at every answer field
and the comment box, the way a server is ([ADR-0028](adr/0028-adding-from-an-answer.md)):
picked, it waits on the set, and the Submit puts it on the Conversation. A
session waiting on that set is carried on in a sandbox with it mounted; a pack
disabled or removed between the pick and the Submit refuses the Submit,
naming it.

### What its wasm component may reach

**`component`** — optional. A path to a `.wasm` file inside the plugin's
directory: the wasm component that holds the plugin's code, where any of it is
code. It is a WebAssembly **component** built for WASI 0.2 — Rust's
`wasm32-wasip2` target makes one — and is written against a world of
Verkstead's WIT package, `verkstead:plugin` (`crates/server/wit/plugin.wit`):
the base world `plugin`, or the world of a Category it provides. None of those
worlds holds anything yet; each Category's stage fills its own.

The component may import WASI 0.2 and WASI HTTP, and nothing else. What any of
it reaches is what `capabilities` grants, below.

**`capabilities`** — optional, a list. A wasm component reaches nothing until
its manifest asks, and what it asks for is shown to the human at **Install**
and granted there, once. Each entry is one capability, keyed by its kind:

```yaml
capabilities:
  - filesystem: ~/.config/glab        # read-only unless it says otherwise
  - filesystem: ~/.cache/glab
    access: read-write
  - outbound: gitlab.com              # a host it may connect to
  - environment: GITLAB_HOST          # a variable it may read
```

- **`filesystem`** — a directory on the host, with `access` either
  `read-only` (the default) or `read-write`. The component is handed it as a
  WASI preopen named by the path, with a leading `~` spelt out as the home
  directory, and it is the only filesystem the component has: a path outside
  every granted directory cannot be opened at all. A granted directory that
  does not exist yet is not handed over; a granted path that is a file is
  refused when the component starts.
- **`outbound`** — a host it may send HTTP requests to, through WASI HTTP. A
  request to any other host is refused before a connection is made. Raw
  sockets and name lookups are not granted, whatever is asked.
- **`environment`** — an environment variable it may read, with the value
  Verkstead's own process has. No other variable is set.

Any other kind is refused, and so is an entry naming two.

### What it wants set

**`settings`** — optional, a list of the fields drawn in the plugin's pane on
the Plugins page:

```yaml
settings:
  - key: host
    label: Host
    kind: text
    description: The GitLab instance, where it isn't gitlab.com.   # optional
```

`key` is what the value is kept and handed over under, unique among the
plugin's settings; `label` is what is drawn. `kind` is one of:

- **`text`** — a line of text.
- **`size`** — a size as `languages.yaml` writes one: a whole number and `K`,
  `M`, `G` or `T`, such as `30G`.
- **`list`** — a list of lines.
- **`secret`** — kept in `secrets.yaml` and never shown again.

**Settings** on the plugin's row opens its pane, which draws these fields, and
draws them whether or not the plugin loaded, so a setting it needs can be put
right there. The values are the cluster's, kept under the plugin's uuid and
each field's `key`: a `text`, `size` or `list` value on the plugin's installed
record, which every device reads, and a `secret` in each device's
`secrets.yaml`, the record carrying only a digest of it, by which every other
device fetches its own copy from one that holds it. A secret is never shown
again: the pane says only whether one is set. A value the field's kind cannot
hold, such as a size that isn't one, is refused at the field with the reason,
and nothing else in that save is written.

## Per-platform overrides

A top-level `linux:`, `macos:` or `windows:` holds keys of the common
declaration, and reading for that platform takes **each key present there in
place of the common one, whole**:

```yaml
capabilities:
  - environment: HOME
windows:
  capabilities:
    - environment: USERPROFILE
```

On Windows this plugin may read `USERPROFILE` and not `HOME`: the block's list
replaces the common list rather than adding to it, so what a platform runs with
is what one block says. Every other platform reads the common list.

Only `description`, `mark`, `dependencies`, `provides`, `capabilities`,
`settings` and `component` may differ by platform. Who the plugin is — its
uuid, name, author, version, repository and interface — is the same
everywhere, and writing one of those under a platform is refused. A platform's
block is checked whichever platform is reading, so a mistake in `windows:` is
found on Linux too.
