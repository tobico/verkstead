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
Each Activity's component is checked against the `activity` world beside it,
without being started.

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

#### `activity`

An **Activity**: what a Process's state runs. A wasm component the server
starts with a Conversation's facts, which launches sessions and waits on them
through the functions the host lends it, and ends in one of the **Outcomes** it
declares here. A plugin may provide several, a block each, and each names a
component of its own.

```yaml
provides:
  - category: activity
    id: follow-up-rounds          # lowercase letters, digits and hyphens
    name: Follow-up rounds        # what is drawn
    component: follow-up-rounds.wasm
    outcomes: [committed, nothing]
    skills: [verkstead/following-up]
dependencies:
  - uuid: 15447a8d-f28d-4573-8b8b-a7226d477755   # the pack `verkstead`'s
    name: Verkstead
    author:
      name: Tobias Cohen
      email: tobi@tobico.net
    version: ^0.1
```

- **`id`** — required, and unique among the plugin's Activities: what a
  Process names it by.
- **`name`** — required. Drawn on the plugin's row.
- **`component`** — required. A `.wasm` file inside the plugin's directory,
  written against the `activity` world below. The top-level `component:` is
  the plugin's own, apart from this.
- **`outcomes`** — required, a list of words in lowercase letters, digits and
  hyphens: every way the Activity ends. A Process maps each to one of its
  states, so an Outcome never names a state. `steered` — the human steered the
  Conversation elsewhere while it ran — is every Activity's without being
  written, and writing it is refused, as is a word written twice.
- **`skills`** — optional, a list: every skill the Activity launches a session
  under, as `<pack>/<skill>`. **What an Activity names, it declares**: each
  pack is one this plugin provides, or one a plugin it lists under
  `dependencies` provides, and a skill from any other is refused, naming it.
  Where the pack is the plugin's own or a built-in's, the skill has to be one
  of its skills too; an installed dependency's pack is checked when the plugins
  load, and a skill it lacks is the plugin's trouble.

**The `activity` world.** The component exports one function, `run`, which is
handed the **start facts** and answers the Outcome as its word:

- the Worktree — the Repo's name, the Worktree's path, the branch and its
  base — and each Companion the same way, with whether it may be committed in;
- the roles' Pairings;
- the Brief, and the handoff document where there is one;
- the Timeline so far, each entry its kind and its body as the text the
  Timeline stores, which is not JSON: a brief, a handoff or a notice is its
  markdown; a move is the state's word, `follow-up` say; a steer is the
  state's word, then a newline and the instruction where the human wrote one;
  and a session's output, a question set or a commit has an empty body;
- the options the Process configured the Activity with;
- each skill it declared, with the path a session reads its `SKILL.md` at.

It imports the **host functions**, `verkstead:plugin/host`, and nothing else
of Verkstead's:

- **`launch`** a session under a skill and a role, with the prompt the Activity
  wrote — the host adds what every session is told — and the evidence
  `verkstead done` is checked against: a file landed, commits since a count,
  nothing, the Nothing-else mark, or an investigation's; and whether the session
  ends with its work or on a pull request.
- **`wait`** on it, answering what happened: it gave its Done signal, it is
  gone — with the host's sentence for how, where it ended badly — it stalled, the human steered the Conversation, or the human answered a
  round with the Nothing-else mark. Every event an Activity is handed arrives
  here.
- **`git-facts`** of the Worktree's repository or a Companion's, read-only: the
  branch, the commits past its base, and whether anything is uncommitted.
- **`notice`**, which puts a notice on the Timeline.
- **`stop`**, with a reason: the Conversation stays where it is, the reason goes
  on the Timeline, and a question set left open is closed.

**An Activity keeps nothing between starts.** A server restarted starts it
again with the same facts and the Timeline as it now stands, and the files in
the Repo are the truth it works out where it was from.

Each Activity's component is checked as the plugin loads, without being run:
one that isn't a component, imports something Verkstead does not give an
Activity, or does not export `run` is the plugin's trouble, drawn on its row
and printed by `verkstead plugin check`, naming the Activity.

### What its wasm component may reach

**`component`** — optional. A path to a `.wasm` file inside the plugin's
directory: the wasm component that holds the plugin's code, where any of it is
code. It is a WebAssembly **component** built for WASI 0.2 — Rust's
`wasm32-wasip2` target makes one — and is written against a world of
Verkstead's WIT package, `verkstead:plugin` (`crates/server/wit/plugin.wit`):
the base world `plugin`, or the world of a Category it provides. A Category's
world is filled by the stage that gives it its grammar: the `agent` world's
exports are above, and the `activity` world's are below, where an Activity's
component is named by its block rather than here; the rest hold nothing yet.

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
- **`size`** — a size in sccache's grammar: a whole number and `K`, `M`, `G`
  or `T` for binary multiples, such as `30G`.
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

## The Categories

What a `provides:` entry holds beyond its `category`, for each Category that
has its grammar. A Category not here yet keeps the rest of its entry unread.

### Language

A `language` entry is a **descriptor**: what Verkstead gives every session for
one language's tools, as data. The seven Verkstead ships are the built-in
plugins under the repository's `plugins/` — Rust and C/C++ one plugin, because
both name the Compile Server and one mechanism is one thing to switch; Go,
Node, Python, .NET and the JVM one each — and their `plugin.yaml` files are
this grammar's worked examples, with the reasons for every variable in their
comments. [ADR-0021](adr/0021-language-descriptors.md) is why it is shaped
this way.

```yaml
provides:
  - category: language
    language: go                 # the name config.yaml's `languages:` keys by
    label: Go                    # what is drawn
    detect: [go.mod]             # for the setup card's warning, and nothing else
    env:
      GOMODCACHE: "{cache}/go/mod"
      GOCACHE: "{cache}/go/build"
    stores:
      modules:
        dir: "{cache}/go/mod"
        units:
          - named: ["*@*"]
      build:
        dir: "{cache}/go/build"
        units:
          - depth: 2
```

**A descriptor is data and cannot say a command to run.** Anything that is
behaviour is built into the server and switched on by name, under
`capabilities` — so a Language plugin has no `component` and asks for no
`capabilities` of its own at the top of the manifest: it runs nothing.

- **`language`** — required. The language's name: what an installer's
  `languages:` entry in `config.yaml` merges over, key by key. One plugin
  provides a name once.
- **`label`** — what the language is called on screen.
- **`detect`** — the manifests that say a Repo builds the language, looked for
  at the root of a checkout: a list of file names, or paths relative to the
  root, with no globs. Only the setup card's warning reads it. What a session
  is given never turns on it, because a manifest is often not at the root and a
  variable nothing reads costs nothing.
- **`env`** — the variables every session is given while the language is on,
  in the order written. A variable the sandbox sets itself — `HOME`, `PATH` and
  the rest — is refused, named, as `provides[0].env.PATH`.
- **`capabilities`** — the server behaviour the language switches on, each by
  name with the variables that are set only where this server can offer it.
  There is one: **`sccache`**, the Compile Server — one sccache server for the
  machine, in a sandbox of its own, started wherever a language naming it is on
  and there is an sccache to run. A machine with none gets the language's own
  `env` and none of the capability's. The first language naming it sizes its
  store, whether or not that one is on. A name this server has never heard of
  is a capability it cannot offer, rather than a refusal, so a descriptor
  written for a newer Verkstead still gets its variables.
- **`stores`** — the directories the language's tools fill, each by a name of
  the entry's own, with a `dir` and how it is kept under the language's size
  (below). Named rather than read back out of `env`, because a variable is not
  always a path: Maven's repository is a flag inside `MAVEN_OPTS`. A store is
  what the settings measure the disk use of, what **Clear** empties, and what is
  held to the size.

Any other key is refused, named, inside a capability, a store and a unit as
much as at the entry's top.

#### Placeholders

A value is text, and these stand for what only the server knows:

- **`{cache}`** — the Build Cache: `--build-cache-dir`, or this machine's own
  cache directory where nobody has said. Where a store goes that nothing has to
  share a filesystem with.
- **`{stores}`** — a directory beside the Worktrees, for a store that does:
  pnpm, deno, bun and uv hardlink packages out of their store into the project,
  and copy the lot where the two are on different filesystems. **On Linux they
  copy anyway**: a Worktree and this directory are two bind mounts, and
  `link(2)` answers `EXDEV` across two mounts whatever is underneath. So what a
  store here saves today is the download, as `{cache}` does.
- **`{size}`** — how big this language's store may grow: its plugin's `size`
  setting (below), 10G where nobody has set one, and 30G for Rust, whose
  sccache has always been started at that.
- **`{sccache}`** — where a session reaches the sccache this server found. Only
  inside the `sccache` capability.

A placeholder's directory is made, and granted writable to a session, only
where a language that is on names it.

**Write `/` between the placeholder and what is under it, on every platform.**
A descriptor is read on three, so the grammar has one separator, and a Windows
session is handed the path its own tools would compose, `\` and all.

**A value that is a line of words is left out where a directory in it has
whitespace.** A line of flags is what a tool with no variable for a path reads
one out of, and it splits the line on whitespace: Maven 3's `mvn` splits
`MAVEN_OPTS` bare, so a Build Cache under a name with a space in it would hand
over a line that breaks the tool outright. A value that is one word is a path,
read whole, space and all.

#### Stores and how each is kept under the size

A store's `dir` is `{cache}/…` or `{stores}/…` and nowhere else, because what
is done to a store is delete what is in it. Each says one of three things:

- **`evicted: by-its-tool`** — the tool filling it evicts for itself, handed
  the size in a variable carrying `{size}`. The sweep leaves it alone. sccache
  is the one built-in that does, through `SCCACHE_CACHE_SIZE`.
- **`units:`** — what the sweep takes out of it whole, oldest first, until the
  language's units are under its size: **a package, never a file of one**,
  because a Go module or a Maven artifact with one file missing is a broken
  store rather than a smaller one. A list, each item a way of finding one,
  walking down from `under` (a path inside the store, the store itself where it
  is not said) and stopping at the first entry that is one:
  - **`depth`** — every entry that many levels down; one for what is directly
    in `under`.
  - **`named`** — an entry whose name is one of these patterns.
  - **`holding`** — a directory holding an entry whose name is one of these
    patterns.

  Where more than one is said, an entry has to be all of them. A pattern is one
  name, `*` for any run of characters and `?` for one. The walk follows no
  symlink and never takes one as a unit. **A unit may be a file** where the
  file is the whole package, or where the store is content-addressed and its
  tool checks every blob it reads.
- **neither** — the store is never swept, and the settings say so. What cannot
  be given a unit safely is left this way rather than guessed at.

Refused, naming the store: a `dir` anywhere else; a store saying both of the
first two; a unit naming none of `depth`, `named` or `holding`, which would
match anything; `depth: 0`, which is the whole store; an `under` that leaves
the store; a pattern that is a path; and a store under `.verkstead-swept`,
which is the directory the sweep moves units aside into.

**The sweep runs as the server starts and hourly after, and only while no
session and no Conversation Terminal runs.** A unit is as old as the newest
time anywhere in it — access times where the filesystem keeps them,
modification times where it does not — and is renamed into `.verkstead-swept`
at the top of `{cache}` or `{stores}` before it is deleted, so no tool ever sees
half of one. A store mounted on a filesystem of its own is never swept.
**Clear** empties every store of one language the same way, whichever of the
three it is, and is refused while anything runs.

#### What an installer writes over it

The map under `languages:` in `config.yaml` is this grammar again, keyed by
`language`, and **each entry merges over the language of its name key by key**:
changing one variable keeps every later fix to the rest. A variable, or a store,
set to `null` is taken **out**; `detect` and a store's `units` are replaced
whole. An entry naming a language no plugin provides is a language of its own.

Unlike a plugin, an entry there is never refused outright: one that will not
read falls back to the language of its name as the plugin says it, and that
plugin's row and pane carry why as its **trouble**; the language still runs.
Beside the grammar's keys it holds two of the installation's own — `enabled`,
whether sessions get the language, and `size`, how big its store may grow —
which a plugin's entry cannot hold.

**An entry naming a language no plugin provides is drawn as a Language plugin
of its own**, under **Installed**, saying it comes from this device's
`config.yaml`, so an installer who adds a language gets its row with no code
change. Its tick and its size are a cluster record keyed by a uuid derived from
the language's name: drawn only on a device whose file has the entry, and
shared by every device that does. It has no Update or Remove — the entry is
the file's to take out — and its pane has the `size` field and the build cache
below where its entry names a store. An entry of this kind that will not read
is in trouble and off.

**A Language plugin installed from a directory or from git** is a language
every session gets once it loads on a device, after the built-ins' in the
order the plugins were installed; its tick is the switch and its `size` the
size, like a built-in's, and an entry under `languages:` merges over it the
same way. Its row says which languages it provides. One providing a language
that a built-in or an earlier plugin already provides loads in trouble, and
that language stays the first plugin's.

**A built-in language's switch is its plugin's tick** on the Plugins page,
cluster-wide: unticked, none of its languages' variables, stores or grants
reach a session. Its `enabled` here was taken up into the plugin's record once,
at the first start of a release that has the ticks — on where any of its
languages was on, on any device — and is not read again. An installer's own
language's `enabled` is taken up the same way into its record, the first time
each device reads the entry.

**And its size is its plugin's `size` setting**, cluster-wide, drawn in the
plugin's pane with the default as its placeholder. Every Language plugin
declares one, of kind `size` — a plugin providing a language may call no other
kind of setting `size` — and it sizes every language the plugin provides, so
Rust & C/C++ has one size for its one Compile Server. A language's `size` here
was taken up once beside its `enabled` — the larger wins, across a plugin's
languages and across devices, and a size that will not read is none — and is
not read again. An installer's own language's `size` is taken up the same way,
beside its `enabled`.

**Under the fields, a Language plugin's pane draws its build cache**: for each
language it provides, what its stores hold on this device against the size in
force, which directories they are and how each is held to the size, when they
were last trimmed, and **Clear**, which empties that language's stores —
sccache's too, for Rust. Ticked or not, since an off language still holds what
it fetched. Clear is off, and refused by the server, while a session or a
terminal runs. The Sandbox page's **Build caches** part keeps only what is no
language's: whether the server can see an sccache.

### Agent

An entry of category **`agent`** is a harness: a program a session runs, such
as Claude Code or Codex. What about it is data is written here; the three
things about it that are code — finding and reading a session's transcript,
writing its configuration, and driving its login — are the exports of the
plugin's `component`, against the `agent` world (below). A plugin providing an
agent is refused without a `component`, and a plugin provides one agent at most.

```yaml
provides:
  - category: agent
    binary: stub
    name: Stub
    mark: stub.svg                      # optional where the plugin has one
    launch:
      model: [--model, "{model}"]
      prompt: ["{prompt}"]              # or [--prompt, "{prompt}"]
      session: [--session-id, "{session}"]
      resume:
        flag: [--resume, "{session}"]
      tail: [--yes, --settings, "{settings}", --add-dir, "{worktree}"]
      worktree: [--trust, "{worktree}"]    # optional: left off with no Worktree
      settings:
        autoUpdates: false
    account:
      - key: dir
        label: Directory
        purpose: Mounted at `~/.stub`.
        example: /home/you/accounts/work/.stub
        kind: directory
        mounted: .stub
        allowlist: [settings.json, config.toml]
        credentials: [auth.json]
        memory:
          - path: memory
            synced: everything          # or a path alone, kept on its device
    idle:
      signature: esc to interrupt
    phrases:
      exhausted: you've hit your usage limit
      signed-out:
        ends: Please run /login.
    ask:
      channel: blocking
      guide: |
        Run `verkstead ask` and wait for it: it returns with the answers.
    models:
      - id: stub-large
        name: Stub Large
        default: true
      - id: stub-small
        name: Stub Small
        for: [review]
    tools: mcp__<server>__<tool>
    install:
      instructions: https://example.com/stub/install
      commands: [npm install -g stub]
    login:
      line: [auth, login]
      status: [auth, status]
    environment:
      STUB_DISABLE_UPDATES: "1"
    instructions: STUB.md
    excluded: [.stub/]
```

**Who it is.** `binary` is the program a session runs, found on PATH by that
name — a name and never a path, which is refused. `name` is what a sentence calls it. `mark` is its mark, read as the
top-level `mark` is; where it has none, the plugin's is drawn, and an agent
with neither is refused.

**`launch`** — required: its launch line, in parts, each a list of the words
on the line. The words may hold **placeholders** the host fills: `{model}`,
`{prompt}`, `{session}`, `{worktree}` and `{settings}`. Any other bare
`{name}` is refused; braces around anything that isn't a bare name are the
word's own, so an inline table such as `projects={"{worktree}"={trust="yes"}}`
is written as it is and its `{worktree}` still filled.

- **`model`** — required, holding `{model}`: how the model is said.
- **`prompt`** — required, holding `{prompt}`: how the Brief is given, a flag
  and it or it alone.
- **`session`** — optional, holding `{session}`: how the session id Verkstead
  picked is named. A harness that takes none leaves it out: it **names
  itself**, its transcript is found by the Worktree and the launch rather than
  named, and the id `parse-transcript` answers with is what it is called from
  then on.
- **`resume`** — optional: how a conversation is carried on, under the id the
  session that was having it was named at launch, or named itself. Either a
  **`flag`** holding `{session}`, written where `session` would have been, or
  a **`subcommand`**, written straight after the binary, with **`also`** the
  words a resume needs on the end that a launch does not. A subcommand
  holding no `{session}` — `[resume]`, as codex's is — has the id written as
  a word of its own after the model, in front of the prompt. Without a
  `resume`, a resumed Conversation opens a session of its own.
- **`tail`** — optional: what goes last. `{worktree}` is the Worktree the
  session works in.
- **`worktree`** — optional, holding `{worktree}`: words about the Worktree
  that are written after the `tail` only where the session has one, and are
  left off whole where it has none — a flag and the value it takes, such as
  codex's `[-c, 'projects={"{worktree}"={trust_level="trusted"}}']`, rather
  than a flag left with nothing after it.
- **`settings`** — optional: settings for this session alone, as a map, which
  the host writes as JSON to a file and names on the line as `{settings}`, in
  whichever part the harness takes it, the resume's among them. One without
  the other is refused. The file is `verkstead-settings.json` in the
  session's root, beside the configuration, and where `write-configuration`
  answers settings of its own those are written instead.

Each placeholder is filled where it stands in a word, and once: a Brief that
happens to say `{session}` reaches the harness saying it.

The approval bypass belongs in `tail`: Verkstead runs every session
unattended, inside its Sandbox.

**`account`** — required, at least one: the paths an account is made of, in
the order the Profile form draws a field apiece for them. **The first is a
directory**, and is the one the account is said from: a session is given a
root built in its place at `mounted` — its `credentials`, which here are one
file, joined in as the login, its `memory` joined where the Profile shares memory,
and what `write-configuration` answers written beside them. The rest of its
`allowlist` is the account's configuration, handed to that export as found.

The paths after the first are given to a session too, each at its `mounted`.
**A file is joined as it is**, and a session writing to it writes the
account's. **A directory is built in the root as the first is**: its
`credentials` joined, its `memory` joined where the Profile shares memory and
made empty of the session's own where it does not, and its `allowlist` copied
in as found with the account's own MCP servers taken out. `write-configuration`
composes the first path's configuration alone. A Profile launched on a device
that holds no binding of it is given the first path's files, which are what its
mirror carries: the login as it is, and each configuration file as
`write-configuration` answers it handed the account's and no servers — what a
root built from the mirror would hold, and nothing more of how the human works.
Its memory is synced, a directory at a time, as the next list says.

- **`key`** — what the Profile holds the path under, unique in the list.
- **`label`** — what the form labels the field. **`purpose`**, optional, is
  said under it, and **`example`**, optional, is what it shows before anything
  is typed.
- **`kind`** — `directory` or `file`.
- **`mounted`** — where it is joined into a session's HOME, said from there.
- **`allowlist`**, **`credentials`** and **`memory`** — optional, a
  directory's only, each a list of paths said from it. `allowlist` is the
  account's configuration files, each read whole and handed to
  `write-configuration` as found, or as none where the account has no such
  file; a path written as a directory (ending `/`) or inside `memory` is
  refused. `credentials` are the files that hold the login, and `memory` the
  directories that are the memory store. Nothing else of the directory reaches
  a session: with no `allowlist`, the export is handed no configuration and
  writes what it will from nothing.

  **A `memory` entry is a path, or a map of its `path` and how it is
  `synced`.** A path alone is memory that stays on the device it was written
  on: a session away from where the Profile is bound starts it empty. The
  first path's memory may say instead which of its files a session on another
  device is given, and sends back when it ends — one of the host's own rules:

  - `everything` — every file under it.
  - `worktree` — the codex-shaped rollouts under it, `rollout-*.jsonl` opening
    with a `session_meta` line, whose session worked in a Worktree of this
    Conversation's name.
  - `sessions` — the directories in it, or one level down, called by one of
    the Conversation's session ids.
  - `database` — the SQLite file named by **`database`**, directly in it, with
    its `-wal` and `-shm` beside it, and nothing else there.

  ```yaml
  memory:
    - path: sessions
      synced: worktree
    - path: memories
      synced: everything
  ```

  A word that is none of these is refused naming the field, and so is
  `synced` on a later path: what crosses is said from the account's directory.
  Nothing crosses where the Profile's memory switch is off.

Every path written relatively here, and `instructions` below, is refused where
it would climb out of where it is said from.

**`idle`** — required: how a session is told to be idle. `silence` is a
session that has written nothing for a while. A map holding a **`signature`**
is one whose Screen holds that text while it works and not while it waits.

**`phrases`** — optional. **`exhausted`** is the stable start of what a
session prints when its account is out of window. **`signed-out`** is what it
says when its account has signed out: the sentence a line **`ends`** with, and
**`then`**, optional, the line drawn next, where the sentence alone is also
said by a session signed in. A `signed-out` needs a `login`, which is what puts
it right.

**`ask`** — required: how a session puts a Question Set to the human.
**`channel`** is `blocking`, for a harness that can hold a shell command open
for hours, or `store-and-nudge`, for one whose shell tool gives up after
seconds: the ask returns at once, the session ends its turn, and Verkstead
types into its terminal when the answer lands. **`guide`** is the *Running the
ask* section of the Guide a session reads, in markdown: `verkstead guide` inside
the harness's sandbox asks the server for it and prints it where every
harness's goes, and the channel decides which kinds of ask the Guide describes.

**`models`** — required: the models a Profile of it offers, each an **`id`**,
what the harness is told, and a **`name`**, what is drawn. Exactly one is
**`default: true`**, which a Pairing runs where nothing else says; **`for`**
lists the roles — `grilling`, `implementation`, `review` — whose default is
that model instead, each role on one model at most. The Profile form offers the
list in the order written; a fresh Repo's default Pairing reaches a plugin
harness only where no built-in harness has a Profile.

**`tools`** — required: how the harness names an MCP server's tools to the
model, with `<server>` and `<tool>` where the two names go.

**`install`** — optional. **`instructions`** is a link to the harness's own
install instructions, an `http://` or `https://` address; **`commands`** are
lines that install its binary, run only when the human presses for it. Write a
platform's own under that platform's block.

While the plugin is enabled, its harness is a row of the setup wizard's first
step, after the built-ins: present once `binary` is on a session's PATH, the
same walk the built-ins' rows are. Ticking the row and pressing Next runs the
`commands` in order, each a step of its own and the first that fails stopping
the rest, as the user that runs Verkstead — PowerShell behind one elevation
dialog on Windows, where every install is — in the same install run as
everything else on the step. A command may end in a comment, or run over
several lines, without reaching the next. The plugin's row on the Plugins
page offers the same press, **Install**, where the binary is missing. Nothing
else runs them. The by-hand screen shows the `commands`, a line apiece, and
the `instructions` link; a harness with neither says so.

**Disabling** the plugin keeps its Profiles: they are offered on no picker, a
remembered Pairing on one is not applied, and a session already running under
it finishes. Its row says how many Profiles use it, and **Remove** is refused
while any do.

**A Profile of it waits for the whole cluster.** A Profile replicates to every
member, and a release from before plugin harnesses reads one naming a harness
it does not know as an error that takes its whole Profile list with it. So a
Profile of a plugin's harness is refused while any member is on a release older
than the device saving it, the refusal naming the member to update. A built-in's
is written as the word every release reads, and never waits.

**`login`** — optional: how Verkstead logs a Profile of it in. **`line`** is the
words after the binary that log in, **`status`** the words that ask whether
the account is logged in — both run as they are written, so a placeholder in
either is refused — and **`terminal`**, `false` unless it says
otherwise, whether the harness reads the code it is given off a terminal
rather than a pipe. An agent with no `login` is one Verkstead does not log in.

**`environment`** — optional: a map of each variable a session is told beyond
what every session is told, to its value, which is text.

**`instructions`** — required: the file the harness reads as its global
instructions, said from where the account is mounted.

**`excluded`** — optional: what the harness writes inside the Worktree it
runs in, each a line of git's `info/exclude`.

**Any of it may differ by platform.** An agent's own `linux:`, `macos:` or
`windows:` holds keys of the entry, and reading for that platform takes each
key present there in place of the common one, whole, as the top-level blocks
do:

```yaml
    binary: stub
    windows:
      binary: stub.exe
```

#### The `agent` world

The component of a plugin providing an agent is written against the `agent`
world of `crates/server/wit/plugin.wit`, and is refused at load where it does
not export all three of:

- **`parse-transcript(account, looking)`** — the record of a session, found
  under the account's directory and read into what the Transcript pane draws:
  something said, a tool call, its result, a notice of the harness's own
  bookkeeping, and a line it does not know, which folds away under its own
  name. None where there is no record of it yet. `account` is always
  `/account`: the account's directory, granted read-only under that name on
  every platform whatever its path on the host, so the record is found by
  joining `/`-separated paths onto it. `looking` holds the session's id where
  there is one, the Worktree it runs in and when it was launched, and the
  cursor the last answer gave. The answer holds only what is past that cursor,
  a cursor to carry on from — the export's own, which the host hands back and
  never reads — and the id the record was found under. A harness that names
  itself is handed no id until an answer gives one: the host writes it down as
  the session's name, asks by it from then on, and a resume names it.
- **`write-configuration(found, servers)`** — what a session's root holds of
  the account's configuration, given each configuration file as the account
  holds it, with the account's own MCP servers already taken out, and the
  servers the Conversation attached. It answers with the files and with the
  session-only settings `{settings}` names. The host takes the servers out by
  the keys the harnesses met so far keep them under — a JSON object's
  `mcpServers`, `mcp_servers` and `mcp`, a TOML table's `mcp_servers` — and
  hands any other file over as it is. A file it answers is written into the
  root where its path stays inside it, and an export that fails is a session
  whose root holds none of the configuration, said in the server's log.
- **`login(written)`** — the next step of a login, given what the harness has
  written since the last call: the address to show the human, that the harness
  is waiting for the code they bring back, words to say to them, or done. The
  host runs the harness and owns its console; the export only reads it.

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
