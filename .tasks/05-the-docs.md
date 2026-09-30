# 05. The docs

## What to build

Say what the JVM gets, where the other languages' docs already say theirs.

- **`docs/adoption.md`**: what the `jvm` entry shares. That is Maven's local
  repository and the `mvnw` distributions, and Gradle's whole home: its
  dependency caches, the wrapper distributions, toolchain JDKs and the local
  build cache.
  - Gradle's daemon is **off in a session**, and why: one shared home plus the
    shared loopback means a second session's build would run in the first one's
    Sandbox. It is slower per invocation. Stage 06 is where that may change.
  - An explicit `gradle --daemon` still gets a daemon, and what goes wrong when
    it does.
  - The build cache is shared only for Repos that switch it on.
  - A shared Gradle home shares `gradle.properties` and `init.d/`, so an init
    script one session writes runs in others' builds. This is the "plant a
    package" acceptance, extended.
  - Kotlin needs nothing of its own. Kotlin/Native's `~/.konan` is not shared.
- **ADR-0021**: amend the Gradle section and the "whatever Gradle comes to
  need" line with what was found. Daemon off and Maven's locking both turned out
  to be variables, so no capability was needed. Record the `--daemon` hole and
  why `org.gradle.daemon.registry.base` was turned down.
- **`CONTEXT.md`**: the language list and anything else that names the
  built-ins.
- **`docs/development.md`**: the new tools in the dev shell and CI, where
  stage 02's are listed.

## Acceptance criteria

- [ ] The adoption doc lists what the `jvm` entry shares, says the daemon is off and why, and points to stage 06.
- [ ] ADR-0021 records that Gradle and Maven needed no capability, and records the `--daemon` hole as accepted.
- [ ] `CONTEXT.md` and `docs/development.md` name the new descriptor and tools.
