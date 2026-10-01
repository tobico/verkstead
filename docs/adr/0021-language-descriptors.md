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
into the server, which a descriptor switches on by name. *(Amended by stage 04:
Gradle turned out to need none. Its daemon is switched off, and Maven's
cross-process locking on, with variables alone — see [Gradle's daemon is off in
a session](#gradles-daemon-is-off-in-a-session) — so `sccache` is still the one
capability there is. Stage 06 kept it so: the daemon of Verkstead's own it
measured would have been a capability, and was not built.)* Rejected: descriptors
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

**Amended by stage 04, with what was measured** — against Gradle 8.14.4, Maven
3.9.12 and OpenJDK 21, in two Sandboxes built with Verkstead's own flags:

- **The hazard is real.** With one shared home, the second session's build
  attached to the first session's daemon and failed with *could not setcwd()*
  into a Worktree that Sandbox does not bind.
- **Daemon off is a variable.** `GRADLE_OPTS=-Dorg.gradle.daemon=false` beats
  `org.gradle.daemon=true` in a Repo's `gradle.properties` and in the shared
  home's, with `org.gradle.jvmargs` beside it or not. Gradle then runs a
  single-use daemon inside the session's own Sandbox and registers nothing in
  the shared home. So the `jvm` descriptor names no capability.
- **Maven's locking is a variable too.** `MAVEN_OPTS` carries
  `-Dmaven.repo.local` and Resolver's
  `-Daether.syncContext.named.factory=file-lock` with
  `-Daether.syncContext.named.nameMapper=file-gav`, which Maven 3.9 reads and
  Maven 4 makes the default. Maven 3.8 has no named locks and ignores them.
- **An explicit `--daemon` on the command line still wins**, and registers in
  the shared home again, where another session's build that also says
  `--daemon` can attach to it. A build with the daemon off never looks there,
  so it cannot — measured, and proven beside the hole in the suite. That
  hole is **accepted and documented** rather than closed: the failure it causes
  is loud. *(Stage 06 found that a daemon of Verkstead's own would not close
  it — see below.)*
  `-Dorg.gradle.daemon.registry.base` pointed at a directory per Sandbox was
  tried and does close it, and was **turned down**: it is undocumented, so it
  may go in any release, and the obvious value — a literal `/tmp` — is only per
  Sandbox on Linux.
- **The build cache is the Repo's to switch on.** Its local directory is under
  the Gradle home, so a shared home shares it for every Repo with
  `org.gradle.caching=true`. Switching it on from `GRADLE_OPTS` was turned
  down: it would override a Repo that set `org.gradle.caching=false` on
  purpose.
- **A shared home shares its `gradle.properties` and `init.d/`**, so an init
  script one session writes runs in the others' builds — the case under *What
  is accepted* below, extended to build logic.
- **Kotlin needs nothing of its own.** Kotlin/Native's `~/.konan` is left out.

**Amended by stage 06: the daemon stays off.** The stage's spike measured a
daemon of Verkstead's own, and the per-session registry beside it, against
Gradle 8.14.4 and 9.4.1 on OpenJDK 21 (with 17 and 25 for the mismatches), in
Sandboxes rendered with Verkstead's own flags and a daemon Sandbox shaped like
the Compile Server's. The findings, the rig and every probe are in
[the spike's findings](../roadmaps/language-caches/06-spike-findings.md).

- **A daemon of Verkstead's own is unsound, and was not built.** A matching
  client does attach to it, and a warm build takes 0.2–0.3 s against 1 s. But
  a client that differs in Gradle version, JDK, `org.gradle.jvmargs`, locale
  or Gradle 9's Daemon JVM criteria — or that merely finds it busy with
  another session's build — forks a daemon of its own inside its session's
  Sandbox and registers it in the shared home, and nothing a session can be
  given stops that: a read-only registry fails every build, and Gradle has no
  attach-only client. Another session's build then lands there and fails with
  *could not setcwd()*, which is stage 04's hazard back without anyone saying
  `--daemon`. Gradle also retired Verkstead's daemon by itself once a
  compatible one was idle beside it. And a build that does reach it runs in
  its Sandbox: every Worktree writable, its Kotlin compile daemon shared, and
  none of the session's binds, `HOME`, `/tmp` or Repo `.git`. That is a wider
  crossing than the Compile Server's, because Gradle runs a Repo's code where
  sccache runs a compiler on paths it is handed. Each daemon is about 400 MiB
  after a trivial build and around a gigabyte once it has compiled Kotlin,
  idle for three hours, holding jars the Sweep would take.
- **The per-session registry works, and was not taken either.**
  `-Dorg.gradle.daemon.registry.base` pointed at the session's own `/tmp`
  kept each session's daemon to itself on both Gradles, beat a Repo's
  `gradle.properties`, closed the `--daemon` hole, and gave a Kotlin rebuild
  in 0.4 s against 3 s. The human chose against it: it rests on a property
  Gradle does not document, which is why stage 04 turned it down.
- **So the daemon stays off**, as stage 04 left it, which is safe and costs
  about 1 s on every build and about 3 s on a Kotlin rebuild. The `jvm`
  descriptor still names no capability, and **the explicit `--daemon` hole is
  still accepted** rather than closed: the per-session registry is the one
  measured way to close it, and it is the one turned down.

## Eviction is by whole units

Every store has a size — `10G` where nobody has said, Rust's sccache staying at
`30G`. Tools that evict for themselves are handed the size and left alone. For
the rest, the descriptor names the **unit** — a directory depth *(amended by stage 05:
or a marker, and a blob in a content-addressed store — see below)* — and Verkstead
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

**Amended by stage 05, with what its grilling decided (Set 996)** and what was
measured while building it:

- **A unit is named by a depth or by a marker.** A depth alone could not name
  Go's modules or Maven's artifacts, which sit at whatever depth their path or
  group id puts them. So a unit is every entry at a `depth`, the first entry
  down `named` like a pattern, or the first directory down `holding` one,
  walking from an `under` inside the store and stopping at the unit. The
  patterns are names with `*` and `?`, so the descriptor stays data.
- **A unit may be one blob where the store is content-addressed and its tool
  verifies every blob it reads** — Go's build cache, npm's `_cacache` and
  pnpm's store — because there a blob gone is a blob fetched again. It is never
  a shard, which holds a slice of every package. Never by file otherwise, as
  above.
- **The size bounds the units, not the whole store.** What a store holds beside
  its units — indexes, metadata, and most of Gradle's home — is nothing the
  sweep can take. Counted against the size, a language whose unswept part alone
  was over it would lose every unit on every pass and still be over, which is
  the clearing whole rejected above by another route. So only the units are
  held to the size, and the settings page shows the rest beside it. (Decided
  in the branch's review, Set 998.)
- **Rust's cargo half is swept**, by crate, by unpacked source, by git database
  and by checkout, and held to Rust's size separately from sccache, which holds
  the objects to the same size again. A Rust machine holds up to twice its
  size. sccache is the only tool that evicts for itself: Cargo, Go's build
  cache and Gradle clean up by age rather than to a size, and nothing else
  evicts at all.
- **Nothing runs** means no session **and no Conversation Terminal**, both of
  which reach the stores. A sweep that comes due while something runs happens
  at the first moment nothing does. **A machine that is never idle is never
  swept**, which was accepted rather than overlooked, and the page says when
  each language was last swept.
- **A unit is renamed aside onto the same filesystem**, into a directory at the
  top of `{cache}` or `{stores}`, and a launch waits for the rename in hand. A
  rename refused for crossing filesystems passes the unit over rather than
  copying it. Read-only units, Go's modules and cargo's git packs, are made
  writable first.
- ***Oldest* is the newest time anywhere in the unit**, because a Maven jar's
  own modification time is the repository's `Last-Modified`. Whether a
  filesystem keeps access times is measured with a probe file rather than read
  off the mount options. Linux's default `relatime` keeps them. By their own
  vendors' descriptions, APFS without `strictatime` and NTFS's
  system-managed default on a system volume over 128 GB do not. So on a Mac,
  and on most Windows machines, *least recently used* is *least recently
  written*, and the installer's documentation says so.
- **Clear is refused while anything runs**, rather than waiting or going
  ahead: a human who pressed it is there to be told why nothing happened, and
  the button says how many sessions and terminals it is waiting on. It empties
  every store of the language through the sweep's machinery, sccache's
  included, with the Compile Server stopped first.

## What is accepted

**One session can plant a package another installs.** A shared writable store
is that by construction, it is already true of Rust's, and the machine is one
person's. The documentation says so. A shared Gradle home extends it to init
scripts and properties, which run in every session's Gradle builds; and an
explicit `gradle --daemon` can still put one session's build in another's
Sandbox, which fails loudly rather than quietly. Stage 06 left that hole open:
a daemon of Verkstead's own would not close it, and the per-session registry
that would rests on an undocumented property.

## How it is proven

Two layers: assertions on the environment a session is given, and a real
install per tool inside a Sandbox, skipped where the tool is missing. Of the two
installs each of those does, the second is **denied its registry**, so that
using the store is what makes it succeed rather than something asserted about it
afterwards. The dev
shell and CI on Linux gain the tools. The other two platforms are proven by the
assertions alone — a real install per tool on all three was the option turned
down, for what it would do to CI.
