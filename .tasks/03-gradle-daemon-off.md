# 03. Gradle, daemon off

## What to build

The `jvm` descriptor gains Gradle: one shared `GRADLE_USER_HOME` under the Build
Cache, and `GRADLE_OPTS=-Dorg.gradle.daemon=false`, so every build in a session
runs in that session's own Sandbox. `detect` gains Gradle's manifests,
`build.gradle`, `build.gradle.kts`, `settings.gradle` and
`settings.gradle.kts`, for the setup card's warning.

The shared home also holds `wrapper/dists`, so a Gradle distribution that
`gradlew` downloads is fetched once for the machine. It holds `jdks/` for
toolchains, `caches/` and the rest, all shared.

What was observed at planning time, in bwrap Sandboxes with Verkstead's flags
and Gradle 8.14.4:

- **The hazard.** With one shared home and the daemon on, session A's daemon
  stays up. Session B's `gradle --daemon` attaches to it, and B's build fails
  with `Could not set process working directory to '/wb/proj': could not
  setcwd()`, because it is running inside A's Sandbox.
- **`-Dorg.gradle.daemon=false` in `GRADLE_OPTS` beats a Repo's
  `gradle.properties`**, whether that asks for `org.gradle.daemon=true` alone or
  with `org.gradle.jvmargs`. The build runs in a single-use daemon forked
  *inside the session's own Sandbox*. `registry.bin` stays empty and
  `gradle --status` says no daemons are running.
- **An explicit `--daemon` on the command line beats the variable.** This is
  accepted: the proof documents it rather than defeating it.

Gradle's user-home `gradle.properties` also outranks a project's. Establish
whether the system property still wins over a `gradle.properties` a session
writes into the shared home, and say which way it goes.

**Kotlin.** Confirm that the Kotlin compile daemon a Kotlin Gradle build starts
keeps its run files under the session's own `HOME`, not in the shared Gradle
home, so it cannot be found across Sandboxes. If it can be, that is this task's
to close with a variable, or to raise.

The store proof follows stage 02's shape. Two builds at once, then a third with
`--offline` against a shut registry. Gradle does not cache a `file://` Maven
repository, so the registry has to be served over the loopback.

## Acceptance criteria

- [ ] Two Gradle builds at once in two Sandboxes each run in their own Sandbox (a marker only that Sandbox can read proves it), and both succeed. A third with `--offline` and the registry shut succeeds. The control on an empty Build Cache fails.
- [ ] After a build, no Gradle daemon is running and none is registered in the shared home. A Repo whose `gradle.properties` asks for a daemon, with and without `org.gradle.jvmargs`, still gets none.
- [ ] A control proves the suite can see the hazard: with `GRADLE_OPTS` taken out, the second session's build lands in the first session's daemon.
- [ ] The variables are asserted on all three platforms, and a Kotlin build's compile daemon is shown not to cross Sandboxes.
