# 04. The JVM

## Goal

A session building with Maven or Gradle resolves each dependency once for the
machine, and Gradle's build cache is shared between Conversations. Two
sessions building at once neither damage the store nor reach into each other's
Sandbox. A Gradle build in a session runs with no daemon, which is slower per
invocation and is the price of that.

## Decisions in force

From [ADR-0021](../../adr/0021-language-descriptors.md).

- **Maven is here rather than with the package stores**, because sharing its
  local repository between concurrent writers is more than pointing a
  variable: the grilling took it that Maven's cross-process locking has to be
  switched on. That is a claim to verify, not a fact recorded.
- **Gradle gets a shared home, and no daemon in a session.** A daemon
  registers under the Gradle home and is reached over the loopback, which
  every Sandbox shares — so with one shared home a second session can attach
  to the first one's daemon and have its build run inside the wrong Sandbox,
  where its Worktree is not bound. This is the hazard the **Compile Server**
  exists to remove for sccache. Gradle has no way to move only its caches out
  of its home.
- **Rejected for this stage:** a daemon registry per session, which rests on
  a property Gradle does not document; a home per session reading a shared
  read-only dependency cache, which nothing fills; and the daemon of
  Verkstead's own, which is stage 06.
- **Everything the ecosystem can cache** includes Gradle's build cache.
  Switching it on changes how a project's build behaves, not only where it
  writes, and whether Verkstead switches it on for a project that did not ask
  was not settled. The stage's own grilling settles it.
- **"Daemon off" has to hold against the project.** A Repo's own
  `gradle.properties` may ask for a daemon. What wins is this stage's to
  establish and prove.
- **Data only still holds.** If switching the daemon off or the locking on
  cannot be said with variables, it is a built-in capability the descriptor
  names — not a command in YAML.
- **Proven by real builds** in two Sandboxes on Linux, including two at once.

## Proposed tasks (provisional)

1. **A JDK, Maven and Gradle in the dev shell and CI.** AC: each answers
   inside a Sandbox; the proofs skip where they are missing.
2. **Maven.** AC: a second Sandbox's build downloads nothing; two builds at
   once leave a repository a third can use; a project's own settings file is
   still read.
3. **Gradle, daemon off.** AC: a build in a session leaves no daemon running
   and registers none in the shared home; two sessions building at once each
   run in their own Sandbox; a Repo asking for a daemon still gets none.
4. **Gradle's build cache**, as the grilling settles it. AC: a task output
   produced in one Conversation is reused in another.
5. **The docs.** What is shared, that the daemon is off and why, and that
   stage 06 is where that may change.

## Re-verify at start

- Assumes stage 01 landed.
- **Verify the daemon hazard itself first**, with two Sandboxes and one shared
  home, before building against it. The whole shape of this stage rests on it
  and it was reasoned rather than observed.
- Check how Gradle is told to run no daemon from the environment, and what
  precedence that has over a project's `gradle.properties`.
- Check what else lives under the Gradle home besides caches — properties,
  init scripts, credentials — since a shared home shares those too.
- Check Maven's current documentation for how its local repository is moved
  and how cross-process locking is switched on, and from which version.
- Check that Kotlin needs nothing of its own beyond Gradle's and Maven's
  stores.
