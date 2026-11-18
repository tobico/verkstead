# Verkstead is made of plugins

Amends [ADR-0011](0011-agent-backends.md), whose *`AgentType` stays a closed
word list* no longer holds; [ADR-0020](0020-a-conversation-has-a-process.md),
whose five Processes become five plugins over one engine rather than five
entry paths into one shared ladder; and [ADR-0021](0021-declared-mcp-servers.md)
in one respect, said under *Skill packs*. ADR-0004's one binary stands: the
binary grows a runtime and the built-ins ride inside it.

Every harness Verkstead can run is an arm of a closed Rust enum, matched on in
some forty-five places across twenty files. Every Process is a variant beside
four others, and all five run through one `Lifecycle` of eight states that
thirty-three files branch on. Everything Verkstead knows about a pull request
goes through the host's `gh`, in one three-thousand-line module with no second
implementation behind it. Each of those was the right shape when there was one
of the thing; none of them is a place a human can add one of their own.

So Verkstead becomes **plugins over a core**: a harness, a Process, a Forge, a
Language, an Activity and a pack of Skills are each a **Plugin**, the ones
Verkstead ships are plugins of the same shape bundled inside the binary, and a
human installs, disables, removes and writes their own from a **Plugins**
section of the settings page. Decided in the grilling of 2026-11-17 and 18,
Question Sets 1760 to 1770. The roadmap that builds it is
`docs/roadmaps/plugins/`. The first thing written against it from outside is a
GitLab Forge, as a demonstration rather than a daily tool.

## The shape of a plugin

**A plugin is a directory with a `plugin.yaml` at its root**, declarative
first, and a **WebAssembly component** where a piece of it is code. The
manifest carries its **metadata** — a **uuid**, a display name, the author's
name and email, a **semantic version** and, optionally, the URL of its
repository — the **interface version** it was written against, a
description, a mark as SVG, the **dependencies** it needs, what it
**provides** — a list, each entry of one **category** — the
**capabilities** it asks for, **settings** fields it wants drawn, and
per-platform overrides keyed `linux`, `macos` and `windows` over the common
declaration. The uuid is the plugin's identity: the installed record, a
dependency and everything stored about a plugin key by it, and the display
name is only what is drawn. The built-ins carry fixed uuids, and the words
stored before there were plugins — `claude`, `github` — are read as theirs. The runtime is **wasmtime with the
component model**, the interface written as WIT and bindings generated on both
sides, so a plugin is written in whatever compiles to a component. A plugin
written against a newer interface than the server knows is refused at load
with that said.

**Six categories.** *Agent*, a harness; *Process*, what kind of work a
Conversation is for and the states it runs through; *Forge*, where pull
requests live; *Language*, a descriptor of the kind `languages.yaml` already
holds; *Activity*, one of the things a Process's state runs; *Skills*, a pack
of the workflow documents sessions are put inside. The first three were the
Brief's. Languages were added because a descriptor already behaves like a
plugin half-way — an installer adds one in `config.yaml` with no code — and
deserved the whole way. Activities and Skills fell out of the Process branch:
a Process declared as data needs something to run in each state, and a simple
Process should be able to compose what Verkstead ships while a complex one
brings its own; and the skills those Activities run are shared across
Processes, so they are a thing of their own rather than a thing of any one
Process. A plugin may provide several things across several categories, and
the Plugins page lists it once.

**Every built-in is the artefact a user writes.** The five Processes and the
six Languages are data in the same grammar a user's manifest uses, embedded in
the binary. The five harnesses, GitHub and the six Activities are components,
embedded the same way. This was decided twice: first as the cheaper reading —
built-ins as in-process Rust behind the same trait, the boundary proved by the
first third-party plugin — and then, branch by branch, as the honest one: the
Activities first, *to prove out the boundary*, then GitHub, then the harnesses.
What the cheaper reading saved it would have cost in a boundary nobody had
walked across before a stranger did. The in-process Rust trait stays as the
thing both the embedded components and an installed one are run through; what
goes is every implementation of it that is not the wasm-backed one.

**No viewer code.** A plugin declares data — names, descriptions, an SVG mark,
state words, a noun — and the viewer draws it generically. The panes stay
Verkstead's and generic over a category: the pull request pane draws a merge
request. A plugin carrying JavaScript was rejected as a second extension
surface with its own security and its own breakage, for which no category had
a need.

## Capabilities and trust

**WASI, with the capabilities the manifest asks for, shown to the human and
granted at install.** A component runs with nothing until its manifest names
what it wants — filesystem paths, outbound hosts, environment — and the
**Install** step shows that list beside what the plugin provides before
anything is written. This is the same move as ADR-0011's built root and
ADR-0021's declared servers: a capability is something **said** rather than
something found, and the saying is the human's. Two narrower shapes were
considered and turned down. Pure functions with one host-mediated `fetch` that
attached the credential itself would have kept every secret out of every
component, and could not express a Forge against an API the host had never
heard of or an Activity that reads a Worktree. A command-line tool the host
runs on the component's behalf, the way `gh` runs today, needs the tool on
every device and puts the token in its environment anyway.

**A plugin's code runs on the host, beside the server, and never inside a
session's sandbox.** ADR-0021 refused stdio MCP servers because a child process
the agent starts inside its sandbox is a hole in the boundary that makes an
unattended session safe. A plugin is on the other side of that boundary: it
extends Verkstead, not the agent, and it reaches what the human granted it and
nothing else.

**Secrets a plugin declares go where secrets go.** A `secret` settings field
is written to `secrets.yaml` and never returned to the viewer, the way an
Author's token and an MCP header are. A Forge's token is not the plugin's
setting at all but the Author's — see *Forge*.

## Where plugins live

**Installed from a git URL or a local directory**, kept under `plugins/` in
the Data Directory beside `worktrees/` and `skills/`. A git plugin tracks its
default branch and has an **Update** press that pulls; nothing fetches on its
own. An archive, a release asset and a catalogue Verkstead hosts were each
considered: the first two are a second install path for the same bytes a
repository already holds, and a catalogue is a service to run for a product
whose every other decision avoids one. The **Build a plugin** button means a
plugin is developed in a Repo of its own, so the install takes the repository
it was written in, and a local directory is the development loop.

**The installed list is a Replicated record** (ADR-0028) — uuid, source,
enabled, pinned revision — so a plugin installed on one device is installed on
the cluster and each device clones the source itself. One installed from a
local directory is marked as that device's, and the other devices list it
greyed with that said. Per-device in `config.yaml` was the alternative, and
would have had a Process exist on one device of a cluster and not on the next,
which is the one thing the record kinds were made to stop.

**A plugin that fails to load** — a manifest that does not parse, a component
that traps on instantiation, a pull that fails, an interface version too new —
is listed on the Plugins page with its trouble, counts as disabled, and
nothing else is affected. The rule `config.yaml` is built on, applied to the
next thing read from disk. Refusing to start until it is fixed was rejected as
one plugin holding every Conversation hostage.

**A plugin declares what it depends on.** A Process names Activities, and a
Process or an Activity names Skills as `<pack>/<skill>`, any of which may be
another plugin's. So the manifest lists its **dependencies**, each carrying
what Verkstead needs to say and fix one without the plugin it names being
there: the uuid, the display name, the author's name and email, the URL of its
repository where it has one, and a **version specifier** over the semantic
version. A plugin whose dependency is missing, disabled or outside the
specifier is in trouble — listed with which one and why, and counting as
disabled — and where the dependency carries a repository URL its row offers
the install or the update that would put it right, through the same consent
card as **Add a plugin…**. Disabling a plugin that others depend on says
before it is done which ones it takes down with it, and **Remove** is refused
while any installed plugin depends on it, the refusal naming them. A Process
naming an Activity or a skill it declares no dependency for is refused by
`verkstead plugin check` and the loader alike — the built-ins included, the
five Processes depending on the plugins that provide the six Activities and
the `verkstead` pack, so unticking one of those says that Develop goes with
it. Leaving the reference to be found missing at run time was the
alternative, and is a Conversation standing in a state whose Activity is not
there.

**Authoring.** A template repository per category, each a minimal plugin that
passes the check, and `verkstead plugin check <dir>` in the CLI, so the session
the Build a plugin button primes can close its own loop before an install is
tried. A `verkstead plugin new` that writes a skeleton was turned down as a
template the binary has to carry; the repositories are the templates.

## Agent

A harness's pieces split along one line: what is **measured** and what is
**parsed**. The launch line with its placeholders, the account as a list of
labelled paths, the allowlist, the credential files and memory directories,
the idle signature or byte-silence, the usage-limit and signed-out phrases,
the ask channel and its *Running the ask* section of the Guide, the model list
with a default, the binary's name, a link to its install instructions and
install commands per platform run with the human's say-so — all data, in the
manifest. **Three exports** are code: `parse-transcript`, the harness's record
into the kinds of line the Transcript pane draws; `write-configuration`, the
account's configuration with declared MCP servers merged into the harness's
own format and the session-only settings beside it; and `login`, the dialogue
with the harness's sign-in — what it prints, what to type, where the address
is. Each of the three was offered as data with a wasm override for the hard
case and decided as an export every time: a declarative JSONL mapping covers
Codex and Claude and not OpenCode's SQLite or agy's brain, a JSON-merge
declaration covers one config format and not TOML, and a login table is a
regular expression away from being code already. So every Agent plugin
carries a component.

**Profiles go generic.** A Profile's account shape is its type's today, and
Claude's dir-and-file pair is special-cased in the form and in three store
columns. The form draws one field per path the plugin declares, Claude's pair
becomes two declared paths, and the Claude-only columns move into the per-path
table that every other harness's home already lives in. `AgentType` stops
being a closed enum anywhere — store, render, viewer — and becomes the
plugin's uuid, `claude` and the other four stored words read as the
built-ins'; the ten hand-written viewer tables keyed by it go, and the mark
comes from the manifest.

**The CLI fetches the tailored Guide from the server.** `verkstead guide`
splices a per-harness section in from a constant today, read off
`VERKSTEAD_AGENT`; the section is in the manifest now, on the server the CLI
already talks to, so it asks, and prints the core alone where the server
cannot be reached.

**Disabling an Agent plugin is allowed over standing Profiles.** They stay but
are unpickable, a remembered Pairing on one is not applied — the rule for a
Profile that has broken — running sessions finish, and the page says how many
Profiles the plugin has. Refusing while any Profile exists would make the
checkbox a lie on every harness ever used.

**The harnesses cross one at a time**, Codex first because it has no login and
a plain record, Claude last because it is the one in daily use.

## Activity

**An Activity is a component the host drives**: started with the
Conversation's facts and the Timeline so far, handed events, calling host
functions, and ending in one of the **outcomes** its manifest declares. The
host lends it sessions — launch one under a named skill and role with a
prompt and the Done-signal evidence it expects, wait on its signal — the
Forge, a Question Set of Verkstead's own, a Timeline notice, the Worktree's
git facts, and whatever WASI capabilities its manifest asked for. The narrower
shapes — sessions and outcomes alone, or sessions plus a read-only view of
everything else — were offered and the widest taken, for the same reason the
capability model was: an Activity nobody here has imagined yet is the point of
the category.

**Stateless across a restart.** The host restarts an Activity with the same
facts and the Timeline, and the Activity works out where it was from them and
from the files in the Repo, which stay the truth where they exist —
`.tasks/TODO.md`, the roadmap. A key-value store per Activity per
Conversation was offered and declined: state nothing else can read is state
nothing else can repair.

**One wrap-up, with declared watchers.** The wrap-up watches checks, comments,
the review and mergeability at once; Fix Merge Issues is the same Activity with
two watchers off. A watcher per Activity, several running in one state, was
rejected: a state runs one Activity, and concurrency inside it is the
Activity's own.

**The six built-in Activities become components**: follow-up rounds first,
being the smallest that still launches, waits, asks and reads git; then the
interview, one session and the wrap-up; then the backlog runner and the stage
runner, which spawn Conversations and so give the interface that host function.

## Process

**A Process is states over Activities.** The manifest declares its states —
each with its word on screen, the Activity it runs and that Activity's
options, whether it takes a Steer and with what brief, and whether it is a
**detour** that returns to the state it was steered from — and its
**transitions**, mapping each outcome of each state's Activity to a state.
`verkstead plugin check` and the loader refuse a Process with an outcome
unmapped, because a Conversation with nowhere to go is the one failure a
state machine must not have at run time. **Draft, Done and Closed are
Verkstead's**, unnamed in any manifest: the three states nothing returns to,
each with a way in of its own.

The alternatives were a machine of sessions only, where the composite
Activities stayed hard-coded behind Develop, and a fully general one of guards
and actions in wasm. The first could not write Develop; the second could write
anything and nothing a human could read. States over Activities is the shape
an author writes in YAML that still runs the ladder.

**Roles are the plugin's.** It declares them by name and label, one marked as
the **building role** the Done signal, memos and previews assume; the three
known names — Grilling, Implementation, Review — keep their per-Repo memory
and platform defaults, and the Agent control's shape follows the count, which
retires ADR-0020's table in favour of the manifest.

**Directions are mapped, not assumed.** The interview ends in a pick; a
Process maps the directions it offers to states, the chooser draws only
those, and one direction is no chooser at all.

**Every Steer target is a declared state, Investigating included.** A universal
detour Verkstead offered on every Process was considered and rejected: a
Process that does not want an investigation in the middle of it should not
have one, and one that does declares the state. Develop declares its own,
marked as a detour; Investigate is the Process that is nothing but one. A
steer still records the state it came from, as ADR-0020 settled.

**The words come from the manifest** — the Process's name and description on
the picker, each state's word on the card, the steer rows — and the viewer's
hand-kept tables go. **The five built-ins keep their names and their state
words**, so nothing stored migrates; disabling one takes its row off the
picker and Conversations already on it run to the end.

## Skill packs

**Skills are grouped in packs**, named `<pack>/<skill>` and mounted at
`/verkstead/skills/<pack>/<skill>/`, the thirteen Verkstead ships as one pack
named `verkstead`. A Process or Activity names the skill it runs by that
path, and a plugin providing several things may provide a pack among them,
so a plugin bringing one skill is one plugin. Duplicating the shared skills
into every Process that runs them was the alternative, and would have had
five copies of `implementing` drifting apart.

**A pack is attachable to a Conversation from the Attach menu**, the way an
MCP server is (ADR-0021, ADR-0028), its skills mounted into every session of
that Conversation and listed in the prompt beside the servers. This amends
ADR-0011 in one respect only: *what the account keeps under its own skills is
not reachable at all* stands, and what is new is a pack the human installed
and attached — said, as a server is said, rather than found.

## Forge

**A Forge declares capabilities** — checks, review, publish, stacks, create —
and the core settles what an absent one would have watched: a wrap-up watcher
over a capability the Forge has not got is settled at once, a Repo on a Forge
without `publish` has no Publish on its Share pane and the pane says so, and a
chain on a Forge without `stacks` is left to rebases, each stage's pull
request opened against the branch below as it always was underneath. Requiring
every capability of every Forge would have made the first one impossible to
write.

**A Repo's Forge is read off its remote host** against the hosts each enabled
Forge declares, plus the hosts the human adds to a Forge for a self-hosted
instance; a Repo matching none has no Forge and no wrap-up. Picking it on the
Repo's row was the alternative, one more thing to set that the remote already
says.

**One token per Forge per Author**, keyed by the plugin's uuid in
`secrets.yaml`, the Git settings section asking for one per enabled Forge and
verifying it through the plugin's declared scopes. The plugin declares the
credential helper's username and the hosts it covers for git inside the
sandbox, and **the noun** — pull request, merge request — which the viewer
uses wherever a Repo on that Forge is drawn.

**A session opens its pull request with `verkstead pull-request open`**, and
the skills become forge-neutral: the server opens it through the Repo's Forge
as the Author, and the lines that said `gh pr create --draft` go. The plugin
may still name a CLI tool and its token variable to put into the sandbox for
the agent's own use — `gh` and `GH_TOKEN` — and syncing a stack stays the
session's job there with `gh stack`, where the skills already run it. A forge
section per plugin spliced into the skills was considered: a marker per step
in ten documents, against one subcommand.

**GitHub becomes a component over WASI HTTP**, REST and GraphQL as the Author,
and `gh` leaves the server: walking a chain is read off the API, and the
Mac-only lookup of `gh` on the machine's path goes with it. This reversed the
earlier in-process reading for the reason the Activities did — GitLab proves
the boundary once, and GitHub proves it every day.

## Language

**One Language plugin per language, Rust and C++ sharing one** because they
share the sccache Compile Server, whose behaviour stays the server's and is
switched on by name as the descriptor grammar always said. **One switch**: the
Sandbox page's per-language checkbox goes, and the Plugins page tick is
whether sessions get the language. The store size becomes a declared settings
field. A Language plugin is data and nothing else.

## The Plugins page

A section at the foot of the settings list beside Agent Profiles and Repos.
**Installed above Built-in**, each row the plugin's display name, version and
author, what it provides,
its source, a checkbox and its trouble where it failed to load. **Add a
plugin…** takes a git URL or a browsed path, clones it, reads the manifest,
shows what it provides and the capabilities it asks for, and **Install**
confirms. **Update** on git rows. **Remove** is refused while anything open
stands on the plugin — Profiles on an Agent, Conversations on a Process, Repos
on a Forge, and any installed plugin that depends on it — the refusal counting
them and leading there, disable being the way to stop new use meanwhile. A
row in trouble over a dependency says which, and offers its install or update
where the dependency names a repository. A plugin's declared settings fields are drawn
in a pane when its row is opened.

**Build a plugin** asks the category from a small menu, then opens the compose
page with a Brief naming the category, its template repository and
`verkstead plugin check`, the Process set to Develop and the Repo left to the
human — a plugin being its own repository, Create repo is the likely next
press. A Brief already on the compose page is saved as a Draft first. This is
the compose page's first prefill of any kind.

## Considered and rejected

- **A Rust crate compiled in**, the plugin architecture internal and a user
  plugin a fork and a rebuild. Nobody installs a fork.
- **Declarative only**, with anything needing code added to Verkstead itself.
  It writes a Process and not a Forge.
- **An adapter program on the host speaking JSON over stdio**, in any
  language. It runs with the server's whole reach, and the human can grant it
  nothing narrower.
- **Three categories, closed**, with a fourth a Rust-side addition. Languages
  were already half a plugin; Activities and Skills were needed to make a
  Process one.
- **A universal Investigating detour** on every Process. A Process declares
  what it has.
- **Built-ins as in-process Rust behind the trait.** Decided and then undone,
  category by category, as the boundary's first crossing was moved from a
  stranger's plugin to Verkstead's own.
