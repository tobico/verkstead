# 01. The spike

## What to build

A measurement, not a feature: real Gradle builds in Sandboxes built with
Verkstead's own flags, on Linux, answering whether a Gradle daemon run by
Verkstead can serve every session soundly. Nothing goes into the server or the
`jvm` descriptor. What is delivered is a findings file committed beside the
stage brief, `docs/roadmaps/language-caches/06-spike-findings.md`, plus
whatever scripts reproduce it (kept beside it, or quoted in it in full).

Read first: the stage brief (`06-a-gradle-daemon-of-verksteads-own.md`),
ADR-0021's *Gradle's daemon is off in a session* section and what stage 04
measured there, the `jvm` entry in `crates/server/languages.yaml`, the Gradle
proofs in `crates/server/tests/package_stores.rs` (its
`one_session_beside_anothers_daemon` helper is the two-Sandbox harness to
borrow), and the Compile Server in `crates/server/src/build_cache.rs`
(`BuildCache::compiling` and `compile_server`), whose shape is the one to copy:
a Sandbox of its own holding the Worktrees directory and the Build Cache and
nothing else of the Data Directory, run in the foreground as a child, and held
to the server's life.

**The set-up.** Two session Sandboxes with one shared `GRADLE_USER_HOME` under
the Build Cache, and a daemon started outside both, in a third Sandbox like the
Compile Server's. `gradle --foreground` is the candidate for starting a daemon
as a foreground process with no build, the way `SCCACHE_NO_DAEMON` makes
sccache's server a child. Find out whether it is, or what is instead.

**On two Gradles**, decided by the human: 8.14.4 (the dev shell's and CI's) and
a 9.x from nixpkgs (`gradle_9`, 9.4.1 in nixos-26.05), fetched for the spike
alone with `nix shell` or a store path. The dev shell and CI stay as they are.
Gradle 9 needs Java 17 or later to run a daemon, and adds *Daemon JVM criteria*
(`gradle/gradle-daemon-jvm.properties`), which can pick, and auto-provision, the
daemon's JDK per Repo. Measure what that does to matching.

**And the per-session registry beside it**, also decided by the human:
`-Dorg.gradle.daemon.registry.base` pointed at a directory per Sandbox. Stage
04 found it closes the `--daemon` hole and turned it down as undocumented, and
the brief names it as the next thing to weigh. Measure it again on both
Gradles, so the grilling compares two measured options rather than one measured
option and one remembered one.

Questions to answer in the findings, each with what was run and what it said:

1. **Does a session's client attach** to the daemon Verkstead started? What
   does the session's environment have to say for that to happen, in place of
   today's `GRADLE_OPTS=-Dorg.gradle.daemon=false`?
2. **What makes a client refuse it and start its own**: Gradle version (a
   Repo's `gradlew` pins its own), Java home, JVM args, Daemon JVM criteria, the
   daemon being busy with another session's build. And where a client starts
   its own, **where it registers**. If it is the shared home, that is the hazard
   stage 04 closed coming back. Can the session be kept from ever spawning one?
3. **Where the build's file access really happens**: whose `HOME`, whose binds,
   whose `/tmp`, what a build script can read and write, and what becomes of a
   Conversation's own binds and of a build directory outside the Worktree.
4. **What it costs in memory** per distinct (Gradle version × JDK × jvmargs)
   build, and how long a daemon idles (3 hours by default).
5. **Kotlin's compile daemon**: started from inside Verkstead's daemon, where do
   its run files go, and do two sessions' Kotlin builds now meet?
6. **Holding and stopping it**: can it be held as a child and killed with the
   server, and what it keeps open in the Gradle home that a stage-05 Clear or
   Sweep of the JVM's stores would have to stop it for.

Close the findings with a **recommendation**: build the daemon of Verkstead's
own, take the per-session registry, or leave the daemon off. Give the reasoning
for each.

macOS and Windows are not run, as the human decided. Say what of each answer is
expected to carry over from the Compile Server's way of starting and holding a
process on those platforms, and what would need proving there.

## Acceptance criteria

- [ ] `06-spike-findings.md` answers all six questions above, each with what was run and the output that answered it, on Gradle 8.14.4 and on 9.x
- [ ] The per-session registry is measured on both Gradles alongside the daemon of Verkstead's own
- [ ] The findings end in a recommendation with its reasoning, and nothing in the server, the descriptors or the dev shell has changed
