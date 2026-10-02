# 06. A Gradle daemon of Verkstead's own

## Goal

Either a Gradle build in a session runs through a daemon Verkstead started, in
a Sandbox of Verkstead's own, with the speed a daemon gives and no session's
build ever running inside another's Sandbox — or this stage ends with a
written finding that it cannot be done soundly, and the daemon stays off as
stage 04 left it. Both are this stage finished.

## Decisions in force

From [ADR-0021](../../adr/0021-language-descriptors.md).

- **This is what was asked for.** Offered four ways to handle Gradle's daemon,
  the human picked the daemon run by Verkstead, as the **Compile Server** is
  for sccache — and then agreed to ship Gradle first with the daemon off and
  make this a later stage.
- **It opens with a spike, and a grilling of its own.** Nothing here is
  designed. The differences from the Compile Server that the grilling laid
  out, each reasoned rather than observed:
  - **There is no one daemon.** A client only uses a daemon matching its
    Gradle version, its JVM and its JVM arguments, and starts its own where
    none matches. Nothing known lets Verkstead take that spawn over. So a
    mismatch puts a daemon back inside a session's Sandbox, registered where
    every other session finds it — the hazard stage 04 closed.
  - **The build runs where the daemon is.** A build script is code, so it
    would run in a Sandbox holding every Worktree, without the session's
    `HOME` or the binds its Conversation was given.
  - **Each daemon holds memory**, on the order of a gigabyte, and idles for
    hours.
- **The Compile Server is the pattern for what can be borrowed:** a Sandbox of
  its own holding the Worktrees directory and the cache and nothing else of
  the Data Directory; started as the session account on Windows; held to the
  server's life by each platform's own means.
- **A daemon registry per session was the other option on the table**, and
  was turned down for resting on a property Gradle does not document. If the
  spike finds this stage's design unsound, that option is the next thing to
  weigh rather than something already ruled out for good.
- **Data only still holds.** Whatever is built is a capability the Gradle
  descriptor names.

## Proposed tasks (provisional)

1. **The spike.** Two Sandboxes, one shared Gradle home, a daemon started
   outside both. AC: a written answer to each of — does a session's client
   attach to it; what makes it refuse and start its own; where the build's
   file access really happens; what it costs in memory per distinct build.
2. **The grilling.** The findings put to the human, with a recommendation.
   AC: a decision to build, to take the per-session registry instead, or to
   leave the daemon off.
3. **Whatever that decides.** Provisional in the strongest sense: not
   sketched until the spike is read.
4. **The docs.** The ADR amended with what was found, whichever way it went.

## Re-verify at start

- Assumes stage 04 landed with Gradle's daemon off and a proof that two
  sessions do not meet.
- Check whether Gradle has changed how daemons are found or started since
  this was written.
- Check how the Compile Server is started and held now, since it is what this
  would be modelled on and stages 01 and 03 will both have touched it.
