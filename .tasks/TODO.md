# The JVM

A session building with Maven or Gradle resolves each dependency once for the
machine, and Gradle's build cache is shared between Conversations for any Repo
that switches it on. Two sessions building at once neither damage the store nor
reach into each other's Sandbox. A Gradle build in a session runs with no
daemon, which is slower per invocation and is the price of that.

The grounding at the start of the stage found that all of this is **data**:
one built-in `jvm` descriptor setting variables, with no capability of the
server's own. It also observed the daemon hazard for real, before anything was
built against it: two Sandboxes sharing one Gradle home, and the second
session's build attached to the first one's daemon and failed with
`could not setcwd()` into a Worktree that Sandbox does not bind.

Roadmap stage: [04: The JVM](docs/roadmaps/language-caches/04-the-jvm.md)

## Settled when the stage started

The grilling of 2026-09-30 (Question Set 988). Everything below was observed in
two bwrap Sandboxes built with Verkstead's own flags, against Gradle 8.14.4,
Maven 3.9.12 and OpenJDK 21 from nixpkgs.

- **One `jvm` descriptor** covers Maven and Gradle, with one box on the
  **Language support** pane, the way `node` covers six tools.
- **Daemon off is `GRADLE_OPTS=-Dorg.gradle.daemon=false`.** It beats a
  Repo's `gradle.properties` asking for `org.gradle.daemon=true`, even with
  `org.gradle.jvmargs` beside it. Gradle then runs a *single-use* daemon inside
  the session's own Sandbox, and nothing is registered in the shared home.
- **An explicit `--daemon` on the command line still wins**, and registers in
  the shared home again. This is **accepted and documented**, not worked around.
  The failure it causes is loud: the other session's build cannot reach its
  Worktree. `-Dorg.gradle.daemon.registry.base` was tried and does close the
  hole, but it was turned down: it is undocumented, and a literal `/tmp` is only
  per-Sandbox on Linux.
- **The build cache is the Repo's choice.** Its local directory is under the
  Gradle home, so a shared home shares it for every Repo that switches caching
  on. Switching it on from `GRADLE_OPTS` was turned down, because it would
  override a Repo that set `org.gradle.caching=false` on purpose.
- **Maven is `MAVEN_OPTS`**: `-Dmaven.repo.local` and Resolver's
  `-Daether.syncContext.named.factory=file-lock` with
  `-Daether.syncContext.named.nameMapper=file-gav`. 3.9.12's debug log confirms
  it reads both.
- **Accepted: a shared Gradle home shares its `gradle.properties` and
  `init.d/`.** An init script one session writes runs in every other session's
  builds. This is the "one session can plant a package another installs" case,
  and it is documented.
- **Kotlin needs nothing of its own.** Kotlin/Native's `~/.konan` is left out
  of this stage.

## Tasks

- [x] 01: A JDK, Maven and Gradle in the dev shell and CI — [details](01-jdk-maven-gradle-in-the-shell-and-ci.md)
- [x] 02: Maven — [details](02-maven.md)
- [x] 03: Gradle, daemon off — [details](03-gradle-daemon-off.md)
- [x] 04: Gradle's build cache, where a Repo turns it on — [details](04-gradles-build-cache.md)
- [ ] 05: The docs — [details](05-the-docs.md)
