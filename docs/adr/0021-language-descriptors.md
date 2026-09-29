# Language descriptors

The **Build Cache** stops being Rust's alone. What a language gets of it is said
by a **descriptor** — data, in one grammar, whether Verkstead shipped it or an
installer wrote it — and Rust becomes one descriptor among the rest.

`build_cache.rs` says of itself that *nothing here generalises over languages*
and that a sibling module is where a node or a python cache would go. That was
the right call for one language and is the wrong one for seven: a module each
is a switch each, a settings key each and a pane each, and the people this is
for are installing Verkstead themselves and building in languages its
maintainer does not. They need to be able to add one without a build.

Decided in the grilling of 2026-09-29. The roadmap that builds it is
`docs/roadmaps/language-caches/`.

## A descriptor is data, and behaviour is a capability it names

A descriptor says a label, the manifests that detect the language in a Repo,
the variables a session is given, and how its store is evicted:

```yaml
languages:
  go:
    label: Go
    detect: [go.mod]
    env:
      GOMODCACHE: "{cache}/go/mod"
      GOCACHE: "{cache}/go/build"
```

It cannot say a command to run. Anything that is behaviour — the sccache
**Compile Server**, and whatever Gradle comes to need — is a capability built
into the server, which a descriptor switches on by name. Rejected: descriptors
that name commands, which would make `config.yaml` a place programs are
started from, in a Sandbox somebody would then have to describe in YAML too.
Rejected as well: descriptors compiled into the server, and a Rust trait with a
module per language — either leaves adding a language to whoever builds
Verkstead.

The grammar shown is the shape agreed, not its final spelling; stage 01 of the
roadmap settles the key names.

## Built-ins are the same grammar, embedded

The descriptors Verkstead ships are a YAML file embedded in the binary, in the
grammar an installer writes. One grammar means the built-ins are the
documentation's worked examples and the loader has one path through it.

## One map, merged key by key

`config.yaml` holds one map keyed by language. An entry carries the descriptor
and, beside it, `enabled` and `size` — the two things the settings page
writes, and the only two it writes.

- An installer's entry under a built-in's name **merges into it key by key**.
  Rejected: replacing the built-in whole, which makes changing one variable a
  matter of copying the rest and then missing every later fix to them.
- A variable set to **`null` is taken out**.
- **Variables the Sandbox sets itself are refused by name** when the file is
  loaded — `HOME`, `PATH` and the rest of what `sandbox.rs` says. The file is
  the installer's own, and they can already open binds, but a descriptor that
  quietly replaced `PATH` would be a session that cannot find `verkstead`.
- A descriptor that does not load **falls back to the built-in of that name**,
  and turns the language off only where there is no built-in to fall back to.
  The settings page says why either way, and every other language carries on.
  An installer whose override of a built-in is refused keeps the cache they
  already had, which is what merging key by key is for in the first place:
  losing Rust's build cache to one mistyped variable is exactly the worse
  experience for not having checked that the next section refuses, and it is not
  what `config.yaml` does today, where even an unparseable file leaves the build
  cache on at its default. Rejected: refusing to start, which is every session
  down for a typo in a language nobody on the machine builds. Rejected as well:
  turning the language off whatever it is, which spends a working built-in on a
  typo; and dropping only the key at fault, which can leave a store
  half-configured — one of Go's two variables moved and the other not.
- **`rust_build_cache` is still read**, as Rust's `enabled` and `size`, so an
  install that wrote one keeps what it said.
- **The file only.** There is no editor for descriptors on the settings page;
  the page draws a checkbox for every language the server lists.

## On with nothing configured, for every session, on every platform

Every built-in is on until somebody turns it off, for the reason Rust's is: a
human should never have a worse experience for not having checked the
settings. An enabled language's variables are set for **every** session
whatever the Repo holds, as Rust's are today — a manifest is often not at the
root, and a variable nothing reads costs nothing. All three platforms from the
start.

**Detection is for warnings and nothing else.** It is not what starts the
**Compile Server**, and that is the one place the variables and the detection
could disagree where the disagreement is not free. A Repo whose manifest is not
at the root is handed `RUSTC_WRAPPER`, or CMake's launchers, all the same — and
with no server of Verkstead's up, the sccache client inside starts one of its
own, on the loopback every Sandbox shares, so the next such session's compiles
run inside the first one's Sandbox, where its Worktree is not bound, and the
build fails outright. That is the hazard the Compile Server exists to remove,
reached by the ordinary case the paragraph above is written for. So **the
Compile Server starts wherever a language naming the sccache capability is
enabled and there is an sccache to run**, whatever the Repo holds: it costs one
idle server on a machine that compiles neither Rust nor C++, and it cannot be
wrong about a Repo. Rejected: keeping detection and setting the launchers only
where it fired, which gives up variables-for-every-session and buys an uncached
compile where this buys a cached one.

## How deep the caching goes

Everything each ecosystem can share: package downloads everywhere, and compiled
output wherever the tool has some.

| Ecosystem | Tools |
|---|---|
| Node | npm, pnpm, yarn, deno, bun |
| Python | pip, uv, poetry, pipenv |
| JVM | Maven, Gradle |
| .NET | NuGet |
| Go | the module cache and the build cache |
| C++ | sccache, through CMake's compiler-launcher variables |
| Rust | as now |

**C++ is CMake's launchers and nothing else.** `CC` and `CXX` are left alone:
they change builds that are not C++ projects at all — a Rust crate with a C
build script, a native extension, cgo — and beside the launchers they can wrap
one compile twice. Meson finds an sccache on the `PATH` for itself. Plain
Makefiles and Bazel get nothing, and the documentation says so. The setup
card's warning that compiles are not cached extends to any language that
compiles through sccache.

## Gradle's daemon is off in a session

A Gradle daemon registers under `GRADLE_USER_HOME` and is reached over the
loopback, which every Sandbox shares — the hazard the Compile Server exists to
remove for sccache. With one shared home, a second session can attach to the
first one's daemon and have its build run inside the wrong Sandbox. So Gradle
ships with a shared home and **no daemon in a session**, which is safe and
slower.

A daemon run by Verkstead in a Sandbox of its own is what was asked for, and it
is its own stage, opening with a spike, because it is not the Compile Server
over again: a client only uses a daemon matching its Gradle version, JVM and
JVM arguments and starts its own where none matches; the build runs where the
daemon is, without the session's `HOME` or its Conversation's binds; and each
daemon holds memory for hours. That stage may end with the daemon staying off.

## Eviction is by whole units

Every store has a size — `10G` where nobody has said, Rust's sccache staying at
`30G`. Tools that evict for themselves are handed the size and left alone. For
the rest, the descriptor names the **unit** — a directory depth — and Verkstead
removes whole units, oldest first, until the store is under its size.

Never by file: a Go module or a Maven artifact with one file missing is a
broken store rather than a smaller one. Rejected: clearing a store whole when
it goes over, which makes the next build cold; and running the tool's own prune
command, which needs the tool installed where the server is and breaks the rule
that a descriptor is data.

The sweep runs on a timer and **only while no session is running**, renaming a
unit aside before deleting it. *Oldest* is access time where the filesystem
keeps one and modification time where it does not, and the documentation says
which a reader is likely to have. The settings page shows each language's disk
use beside its size, with a Clear.

## What is accepted

**One session can plant a package another installs.** A shared writable store
is that by construction, it is already true of Rust's, and the machine is one
person's. The documentation says so.

## How it is proven

Two layers: assertions on the environment a session is given, and a real
install per tool inside a Sandbox, skipped where the tool is missing. The dev
shell and CI on Linux gain the tools. The other two platforms are proven by the
assertions alone — a real install per tool on all three was the option turned
down, for what it would do to CI.
