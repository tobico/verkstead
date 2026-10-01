# 06. The spike: a Gradle daemon of Verkstead's own

What was measured on 2026-10-01, on Linux, for the [stage
brief](06-a-gradle-daemon-of-verksteads-own.md): whether a Gradle daemon run by
Verkstead can serve every session soundly, and the per-session registry beside
it. Nothing in the server, the descriptors or the dev shell was changed to do
it.

**The short answer is no, for the daemon of Verkstead's own.** A session's
client does attach to it, and builds are fast. But a client that does not
match it, or finds it busy, starts a daemon of its own inside the session's
Sandbox and registers it in the shared home. That was measured on both
Gradles, and nothing a session can be given stops it. A second session's build
then lands in that daemon and fails with *could not setcwd()*: the hazard stage
04 closed is back. Gradle also retired Verkstead's daemon by itself once a
session's compatible daemon was beside it. And every build that does reach
Verkstead's daemon runs in a Sandbox holding every Worktree, with none of its
session's binds, its `/tmp` or its Repo's `.git`. **The per-session registry
works on both Gradles** and gives most of the speed without any of that, at
the price of resting on an undocumented property. The recommendation at the
end is the per-session registry.

## The set-up

- **Gradles:** 8.14.4 (the dev shell's and CI's) and 9.4.1 (nixpkgs
  `gradle_9`, `nix build nixpkgs#gradle_9`), both run by default on OpenJDK
  21.0.12. Also fetched for the spike alone: OpenJDK 17.0.20 (`jdk17`) and
  25.0.4 (`jdk25`). The Kotlin Gradle plugin 2.2.20 was resolved from the
  Gradle Plugin Portal.
- **Sandboxes:** [`06-spike/rig.sh`](06-spike/rig.sh) is bwrap with the flags
  Verkstead renders. They were dumped from `Sandbox::command` through the
  `package_stores` fixture, and from `build_cache::compile_server`, with
  throwaway tests that were not committed. Each **session Sandbox** binds its
  own Worktree, a directory standing for its Conversation's own extra bind, and
  the Build Cache. It gets a `/tmp` and a `HOME` of its own, and
  `GRADLE_USER_HOME={cache}/gradle` as the `jvm` descriptor gives it. The
  **daemon's Sandbox** is the Compile Server's shape: the Worktrees directory
  and the Build Cache bound, its own empty `HOME` at `/verkstead/home`, its own
  `/tmp`, and `--die-with-parent` in a PID namespace of its own. Left out of
  the session Sandboxes are the binds of a Claude account, a handoff
  directory, skills and attachments, which nothing Gradle does reads.
- **The build** ([`06-spike/project.sh`](06-spike/project.sh)) has one task,
  `where`. It prints the PID it runs in, the environment's `HOME`,
  `user.home`, the working directory, `/tmp/marker`, the Worktrees it can list,
  and whether it can write the session's own bind and the session's `HOME`.
  Each session writes its own name into its own `/tmp/marker` before building,
  so `marker=` says whose Sandbox the build ran in. `marker=none` means a
  Sandbox that is neither session's: Verkstead's.
- **Verkstead's daemon** is started by
  [`06-spike/start-daemon.sh`](06-spike/start-daemon.sh), run as the daemon
  Sandbox's command. See *How a daemon can be started at all* below for why
  that, and not `gradle --foreground`.

Every script is in [`06-spike/`](06-spike/), with a header saying how to run
it. `SPIKE` names a scratch directory, and `rig.sh fresh` makes a new Data
Directory in it.

## How a daemon can be started at all

**`gradle --foreground` is not usable.** It runs the daemon inside the
client's own JVM, with no fork, and registers it in the shared home:

```
$ rig.sh daemon gradle --foreground
Daemon server started.
$ ls $GRADLE_USER_HOME/daemon/8.14.4
registry.bin  registry.bin.lock
$ cat /proc/<pid>/cmdline   # the one java there is
java -Xmx64m -Xms64m -javaagent:…gradle-instrumentation-agent-8.14.4.jar … -jar …gradle-cli-main-8.14.4.jar … --foreground
```

But a client matches a daemon on its JVM arguments, and the foreground one
records whatever it was started with. Three tries, each followed by a session
build with the daemon on:

1. **Started bare.** It recorded `daemonOpts=-Xms64m,-Xmx64m,…`. The
   client wanted the defaults, `-XX:MaxMetaspaceSize=384m,
   -XX:+HeapDumpOnOutOfMemoryError, -Xms256m, -Xmx512m`, and said *"At least
   one daemon option is different"*. It then started its own daemon, in the
   session.
2. **Started with `GRADLE_OPTS` set to the defaults.** This time the client
   attached, and every build failed in the daemon:
   *"net.rubygrapefruit.platform.NativeException: Unable to get mutable
   environment variable map. … module java.base does not "opens java.util" to
   unnamed module"*. A forked daemon is given ten `--add-opens`/`--add-exports`
   flags ([`06-spike/daemon-opts.sh`](06-spike/daemon-opts.sh)), and a
   foreground one is not.
3. **Started with the opens added**, in `GRADLE_OPTS` or in
   `JDK_JAVA_OPTIONS`. The daemon recorded the opens among its `daemonOpts`,
   and the client refused it as different again.

So a foreground daemon matches a client only with arguments it cannot build
with. This was measured on 8.14.4. The opens are the same on 9.4.1 (its fork
line carries the same ten flags), so it was not taken further there.

**What works is the ordinary daemon, forked by a client inside Verkstead's
Sandbox.** `start-daemon.sh` runs `gradle --daemon -q help` in an empty build
under the daemon's own `HOME`, so a daemon is forked with exactly the arguments
a default client would give it. Then it `exec`s a `sleep` that holds the
Sandbox open. The daemon is not a child of anything Verkstead starts. It is
reparented to the Sandbox's PID 1, beside the `sleep`:

```
bwrap (28404) ─ bwrap ─┬─ sleep 2147483647
                       └─ java … GradleDaemon 8.14.4     (28470)
                            └─ java … KotlinCompileDaemon (28875, once Kotlin has built)
```

## 1. Does a session's client attach?

**Yes, once its `GRADLE_OPTS` no longer says `-Dorg.gradle.daemon=false`, and
only when everything it matches on is the same.** Nothing else in the
session's environment has to change: the client finds the daemon through the
registry in the shared `GRADLE_USER_HOME`, and reaches it over the loopback
every Sandbox shares.

On 8.14.4, with Verkstead's daemon up and the session's `GRADLE_OPTS` empty:

```
$ refuse.sh <gradle 8> "same everything" "" <gradle 8>
gradle=8.14.4
marker=none
home.write=FAILED java.io.FileNotFoundException: …/home-s0/probe-home (No such file or directory)
BUILD SUCCESSFUL in 768ms
--- registry, from the daemon's Sandbox
    44 IDLE     8.14.4
```

On 9.4.1, the same: `marker=none`, then `BUILD SUCCESSFUL in 570ms` and
`297ms` on the second build, with `45 IDLE 9.4.1` in the registry.

The same trivial build with the daemon off, as sessions run today, takes 1 s
every time on both. A warm daemon takes 240–300 ms.

## 2. What makes a client refuse it and start its own, and where that one registers

**A client compares the daemon's Gradle version, Java home, JVM arguments and
locale with its own.** The version is compared by registry directory: one
`daemon/<version>/` per Gradle. For the rest, `--info` prints `Wanted:` beside
`Actual:`:

```
Wanted: DaemonRequestContext{jvmCriteria=…openjdk-21… (no JDK specified, using current Java home),
        daemonOpts=[-XX:MaxMetaspaceSize=384m, -XX:+HeapDumpOnOutOfMemoryError, -Xms256m, -Xmx512m,
        -Dfile.encoding=UTF-8, -Duser.country=US, -Duser.language=en, -Duser.variant],
        applyInstrumentationAgent=true, nativeServicesMode=ENABLED, priority=NORMAL}
```

Each variation below was one session build against Verkstead's daemon, run
with [`06-spike/refuse.sh`](06-spike/refuse.sh). Each one that did not match
said *"Starting a Gradle Daemon, 1 incompatible … Daemon could not be
reused"*, and printed `marker=s0`: it built in a daemon it had started
**inside the session's own Sandbox**.

| Session differs by | 8.14.4 | 9.4.1 |
|---|---|---|
| nothing | attached | attached |
| `LANG=C.UTF-8` in its environment (`user.country`/`user.language` change) | own daemon | own daemon |
| `JAVA_HOME` = JDK 17 | own daemon | own daemon |
| `JAVA_HOME` = JDK 25 | — | own daemon |
| Repo `gradle.properties`: `org.gradle.jvmargs=-Xmx1g` | own daemon | own daemon |
| the client is the other Gradle (a Repo's `gradlew` pinning its own) | own daemon, in `daemon/9.4.1/` | — |
| Daemon JVM criteria `toolchainVersion=21` (the daemon's own JDK) | — | attached |
| Daemon JVM criteria `toolchainVersion=17`, JDK 17 in `org.gradle.java.installations.paths` | — | own daemon, on JDK 17 |
| Daemon JVM criteria `toolchainVersion=17`, nothing to find one with | — | fails: *"Unable to download toolchain matching the requirements ({languageVersion=17, …}) from 'null', due to: No defined toolchain download url for LINUX on x86_64 architecture."* |
| criteria `toolchainVendor=ADOPTIUM` | — | fails, the same way |
| Verkstead's daemon busy with another session's build | own daemon | own daemon |

Three things in that table matter beyond the obvious.

- **The locale is part of the match.** On JDK 17, `file.encoding` also follows
  the locale (`-Dfile.encoding=US-ASCII` with no `LANG`). JDK 18 and later
  default it to UTF-8. Verkstead would have to start its daemon with exactly
  the environment every session has, and a human's `export LANG` in a Terminal
  would be enough to miss it.
- **Gradle 9's Daemon JVM criteria take the daemon's JDK out of `JAVA_HOME`.**
  A Repo with `gradle/gradle-daemon-jvm.properties` names the JDK the daemon
  must run on. A criteria file written by `gradle updateDaemonJvm` carries
  `toolchainUrl.*` lines, and with those Gradle downloads that JDK into
  `$GRADLE_USER_HOME/jdks`. Downloading was not run, because the spike's Repo
  had no such lines. Either way the daemon it then forks is forked **by the
  session's client, in the session's Sandbox**. A criteria file that matches
  the daemon's JDK changes nothing: the client attached.
- **Busy is not a mismatch, and it is the common case.** A daemon runs one
  build at a time, so two sessions building at once always means one of them
  starts a daemon of its own.

**Where the client's own daemon registers: in the shared home.** Every one of
them appeared in `daemon/<version>/registry.bin` under `{cache}/gradle`, read
from the daemon's Sandbox:

```
    44 IDLE     8.14.4          ← Verkstead's
    51 IDLE     8.14.4          ← session 1's, started while Verkstead's was busy
    45 STOPPED  (by user or operating system)
```

It lives as long as the session's Sandbox, and while it does, **another
session's build reaches it**. [`06-spike/busy-lands.sh`](06-spike/busy-lands.sh):
session 0 holds Verkstead's daemon busy, and session 1 builds meanwhile and
stays open. Then session 0 holds Verkstead's busy again, and builds once more:

```
[s1 while busy] Starting a Gradle Daemon, 1 busy and … Daemons could not be reused, use --status for details
[s1 while busy] marker=s1
[s0 second build] FAILURE: Build failed with an exception.
[s0 second build] Could not set process working directory to '…/worktrees/s0': could not setcwd() (errno 2: No such file or directory)
```

That was 8.14.4. 9.4.1 printed the same lines. **This is the hazard stage 04
closed, back again**, without anybody asking for `--daemon`.

**And Gradle stopped Verkstead's daemon by itself.** After the busy runs, the
registry said of it:

```
    44 STOPPED  (other compatible daemons were started and after being idle for 0 minutes and not recently used)
```

8.14.4 said that of Verkstead's daemon, and 9.4.1 said it word for word of its
own (`45`). Gradle expires an idle daemon when another compatible one is
beside it. The `sleep` holding the Sandbox stayed up, so a server watching its
child would not have noticed. That stretch was observed, not traced: the order
in which the two daemons were last used decides which one goes.

**Can a session be kept from ever starting one? Nothing found does it.**

- `-Dorg.gradle.daemon=false` is the only way a client is kept from forking a
  reusable daemon, and with it a client never looks for one either. That is
  today's setting.
- **A read-only registry** in the session, a `--ro-bind` of
  `{cache}/gradle/daemon` over itself, fails every build, matching or not.
  Reading the registry takes a lock file:

  ```
  FAILURE: Build failed with an exception.
  java.io.FileNotFoundException: …/gradle/daemon/8.14.4/registry.bin.lock (Read-only file system)
  ```

- There is no documented *attach only* mode, and none was found among the
  launcher's properties.

Also seen: the registry records PIDs from every Sandbox's own PID namespace,
so they collide. `45` was both Verkstead's daemon and a session's. A daemon's
log is `daemon-<pid>.out.log` beside the registry, so one Sandbox's daemon log
overwrites another's.

## 3. Where the build's file access really happens

**In the daemon's Sandbox, with the session's environment variables.** From
the attached builds above, and from [`06-spike/q3.sh`](06-spike/q3.sh), where
session 0's Worktree is a real git worktree of a Repo outside the Data
Directory. The session is bound that Repo's `.git`, the way Verkstead binds
it, and the daemon's Sandbox is not.

```
[s0] shell.git=f1885ec                      ← git in the session's own shell
[s0] git=FAILED fatal: not a git repository: (null)
[s0] child.HOME=…/home-s0 child.marker=none
[s0] tmp=left-by-s0,left-by-s1
[s1] git=FAILED fatal: not a git repository (or any parent up to mount point …/data)
[s1] child.HOME=…/home-s1 child.marker=none
[s1] tmp=left-by-s0,left-by-s1
```

That is 8.14.4. 9.4.1 gave the same lines, its first run listing only
`left-by-s0`. The control, with no daemon of Verkstead's up and the same
script, printed `git=27e4314` and `child.marker=s0`.

- **`HOME`**: the build is *told* the session's `HOME`, because the client
  forwards its environment (`env.HOME=…/home-s0`, and `child.HOME` in a
  process it starts), but that directory does not exist where the build runs.
  Writing there failed with *No such file or directory*. `user.home` is the
  account's home from the password database, `/home/infi`, in every Sandbox,
  and no Sandbox binds it.
- **Binds**: none of the session's own. Its Conversation's extra bind failed
  the same way (`outside.write=FAILED`). The Repo's `.git` is not there, so
  anything a build does with git fails: a version from `git describe`, a
  commit id baked into a jar. Verkstead's daemon cannot be bound every
  Conversation's binds: it is one process, and the binds differ by
  Conversation.
- **Every Worktree, read and write**: `worktrees.visible=s0,s1`. A build
  script in one Conversation can read and change another Conversation's
  checkout. In a session's own Sandbox it lists only its own
  (`worktrees.visible=s0`).
- **`/tmp`**: the daemon's, shared by every session's build. Each build saw
  the file the other session's build left there. A build directory or any
  other output outside the Worktree (`/tmp`, a Conversation's bind, the
  session's `HOME`) is either missing or shared with every other session.
- **Processes the build starts**, such as `exec`, a test JVM or `git`, start in
  the daemon's Sandbox too (`child.marker=none`).

The brief's reasoning is borne out. The build runs where the daemon is, and a
daemon serving every session has every session's reach and none of any one
session's.

## 4. What it costs in memory

**One daemon per distinct (Gradle version × JDK × JVM arguments × locale)**,
because each distinct one is refused by the daemons that already exist. Four
distinct builds in one session left four daemons
([`06-spike/memory.sh`](06-spike/memory.sh)), each measured resident after one
trivial build:

```
Gradle 8.14.4, JDK 21, default jvmargs: rss=421 MiB
Gradle 9.4.1, JDK 21, default jvmargs: rss=411 MiB
Gradle 9.4.1, JDK 17, default jvmargs: rss=413 MiB
Gradle 9.4.1, JDK 21, -Xmx1g: rss=379 MiB
```

They grow with work.

- After a session's Kotlin builds, a Gradle daemon held **553 MiB** and the
  Kotlin compile daemon beside it **474–497 MiB**
  ([`06-spike/kotlin-timing.sh`](06-spike/kotlin-timing.sh)).
- Verkstead's daemon, after compiling both sessions' Kotlin build scripts and
  sources, held **1104 MiB**, with its Kotlin daemon at **518 MiB**.

The ceiling is the heap (`-Xmx512m` by default; Repos commonly ask for 2–4 GB)
plus 384 MiB of metaspace plus native memory. So it is about a gigabyte per
daemon by default, and more for Repos that raise the heap, as the brief
guessed.

**Idle timeout: 3 hours.** Every daemon context read in the spike says
`idleTimeout=10800000`. `org.gradle.daemon.idletimeout`, which is documented,
changes it for daemons a client forks.

**Speed**, the same Kotlin source changed and recompiled three times in one
session:

| | 8.14.4 | 9.4.1 |
|---|---|---|
| daemon off (today) | 17 s, 3 s, 3 s | 14 s, 3 s, 3 s |
| a daemon kept per session | 3 s, 442 ms, 424 ms | 3 s, 411 ms, 380 ms |

The first daemon-off build includes resolving the Kotlin plugin into an empty
home.

## 5. Kotlin's compile daemon

**Started from Verkstead's daemon, it lives in Verkstead's Sandbox, and two
sessions' Kotlin builds meet in it.** Its run files are under the JVM's
`user.home`, `/home/infi/.kotlin/daemon`, which is a directory of the daemon
Sandbox's own root. A real Kotlin/JVM build with the Kotlin Gradle plugin
2.2.20 ([`06-spike/kotlin-project.sh`](06-spike/kotlin-project.sh)), first in
session 0, then in session 1, then in session 0 again, all on Verkstead's
8.14.4 daemon:

```
[s1] kotlin.runs=/home/infi/.kotlin/daemon kotlin-daemon.2026-10-01T11-55-02.219Z.….run
[s1] kotlin.daemon pid=397 parent=45 runFiles=/home/infi/.kotlin/daemon
[s1] gradle.pid=45
[s0] kotlin.daemon pid=397 parent=45 runFiles=/home/infi/.kotlin/daemon
[s0] Options for KOTLIN DAEMON: IncrementalCompilationOptions(… workingDir=…/worktrees/s0/k/build/kotlin/compileKotlin/cacheable …)
```

One Kotlin compile daemon (397, a child of Gradle daemon 45) compiled both
Worktrees. It works, because that Sandbox binds both, but it is the same
crossing as question 3. A compiler plugin or annotation processor in one
Conversation runs in a process that writes another's checkout.

In a session that started its own Gradle daemon, whether because it missed
Verkstead's or because it has a per-session registry, the Kotlin daemon is
that session's. Its run files are in the session's own root, as stage 04
measured.

## 6. Holding and stopping it

**Held: yes, but by its Sandbox rather than by itself.** The daemon is not a
child of anything the server starts. A client forks it and it is reparented to
the Sandbox's PID 1. What the server can hold is the Sandbox, and on Linux
that is enough. Killing the holding `bwrap` took the namespace down:

```
$ kill 28404     # the daemon Sandbox's outer bwrap
28470 gone       # the Gradle daemon
28875 gone       # its Kotlin compile daemon
```

And each time the shell that started a daemon Sandbox exited,
`--die-with-parent` took that Sandbox down with it, which is exactly the
Compile Server's holding. **But being alive is not the same as being up.**
Gradle retires the daemon itself (question 2), and also after the 3-hour idle
timeout unless that is raised, while the holder lives on. So a server would
have to watch the registry, or the daemon's PID inside the namespace, rather
than its child.

**What it keeps open in the Gradle home**, read off `/proc/<pid>/fd` after the
Kotlin builds: 55 files.

```
     33 caches/modules-2/files-2.1     ← jars: the Sweep's units
      3 caches/modules-2/metadata-2.107
      1 caches/modules-2/modules-2.lock
     10 caches/8.14.4/transforms
      3 caches/8.14.4/fileHashes
      1 caches/8.14.4/fileContent
      1 caches/8.14.4/generated-gradle-jars
      1 caches/journal-1/file-access.bin
      1 caches/journal-1/journal-1.lock
      1 daemon/8.14.4/daemon-45.out.log
```

The Kotlin daemon held 7 more jars under `files-2.1`. A daemon keeps the jars
of plugins and build logic loaded in its classloaders between builds. The
Sweep renames units aside and only runs while no session runs. A daemon of
Verkstead's own would be up while nothing runs, holding those jars and
Gradle's own locks, and the Sweep takes no Gradle lock. **So a Sweep or a
Clear of the JVM's stores would have to stop it first**, the way a Clear stops
the Compile Server now, and count it as something running rather than as the
server's own.

**The per-session registry needs none of that.** A per-session daemon is in the
session's PID namespace and goes when the session's Sandbox does. The
registries above show it: every session daemon's entry turned `STOPPED (by
user or operating system)` as its Sandbox ended. The Sweep and a Clear run
only while no session runs, so by then there is no daemon to stop.

## The per-session registry, measured again

`GRADLE_OPTS=-Dorg.gradle.daemon.registry.base=/tmp/gradle-daemons` in place
of `-Dorg.gradle.daemon=false`, with the daemon left on and no daemon of
Verkstead's ([`06-spike/registry.sh`](06-spike/registry.sh)). 8.14.4 and 9.4.1
gave the same results.

- **Each session reuses its own daemon.** Three builds in one session took
  1 s, then 239 ms, then 241 ms on 8.14.4, and 1 s, 222 ms, 216 ms on 9.4.1.
  The registry and the daemon's log are in the session's own `/tmp`
  (`daemon-46.out.log registry.bin registry.bin.lock`), and nothing new is in
  the shared home's.
- **Two sessions never meet, even when both ask for `--daemon`.** Session 0
  held an idle daemon open, and session 1 built with `--daemon`: *"Starting a
  Gradle Daemon"*, `marker=s1`.
- **A Repo cannot point it back at the shared home.** `gradle.properties`
  saying `org.gradle.daemon.registry.base=$GRADLE_USER_HOME/daemon`, or
  `systemProp.org.gradle.daemon.registry.base=…`, still started a daemon
  registered in the session's own `/tmp`. The `-D` from `GRADLE_OPTS` wins, as
  `org.gradle.daemon=false` does.
- **The build runs in the session's own Sandbox**, with its `HOME`, binds,
  `/tmp`, `.git` and nothing else, as it does today.
- **The Kotlin daemon is the session's too** (question 5).
- **Cost:** one daemon per session per distinct build, about 400 MiB idle and
  around a gigabyte with Kotlin beside it (question 4), for as long as the
  session runs or 3 hours idle. That is where the memory goes. It is bounded by
  the number of sessions building JVM projects at once, not by the number of
  Repos on the machine.
- **Still undocumented.** Neither the 9.4.1 nor the current
  `build_environment` and `gradle_daemon` pages mention it. They name only
  `org.gradle.daemon` and `org.gradle.daemon.idletimeout`. The 9.4.1
  launcher still reads it (it is in `gradle-launcher-9.4.1.jar`). If a release
  drops it, the failure is the one stage 04 accepted: one session's
  `--daemon`-equivalent reaching another's daemon in the shared home, loudly.
  Mismatched clients do not make it worse, because with this setting the only
  daemons there are the sessions' own.
- **`/tmp` is per Sandbox only on Linux.** On a Mac or Windows the directory
  has to be one Verkstead makes per Conversation, as stage 04 noted.

## macOS and Windows

Not run, as decided. What would carry over from the Compile Server's way of
starting and holding a process, and what would need proving:

- **Matching (questions 1, 2 and 4)** is Gradle's, and the same on every
  platform, so the refusals and the spawn into the shared registry are
  expected exactly as measured. So is the busy case. On Windows every session
  runs as the one session account, so a session's daemon registered in the
  shared home is reachable by every other session as it is here.
- **Where the build runs (question 3)** follows from a daemon in a Sandbox of
  its own on every platform: on a Mac, a `sandbox-exec` profile granting the
  Worktrees directory and the cache; on Windows, the Compile Server's grants
  for the session account. A build would see every Worktree and none of its
  session's grants there too.
- **Holding (question 6)** is where the platforms differ, and is what would
  need proving. The Compile Server is a foreground child, kept by a process
  group on a Mac and a Job on Windows. A Gradle daemon is forked by a client
  and outlives it. Linux's PID namespace catches it regardless. Whether a
  Mac's process-group keeper or a Windows Job catches a process that detaches
  itself is unmeasured: Gradle may start it in a session of its own, or break
  away from the Job. A per-session daemon has the same question, about the
  session's own keeper rather than the server's.
- **The per-session registry** has no `/tmp` of its own to point at off Linux.
  The directory has to be per Conversation and made by Verkstead, and only
  environment assertions would prove it there, as the rest of the roadmap is
  proven.

## Recommendation: take the per-session registry

**Build the daemon of Verkstead's own? No.** Each reason was measured, on
both Gradles:

- A session cannot be kept from starting a daemon of its own and registering
  it in the shared home. Every mismatch does it: version, JDK, `jvmargs`,
  locale, and Daemon JVM criteria. Busy does it too, which is any two sessions
  building at once. A second session's build then fails in the first one's
  Sandbox: stage 04's hazard, back without anyone asking for `--daemon`.
- Gradle retires Verkstead's daemon by itself once a compatible one appears
  beside it, so Verkstead cannot even count on its daemon being the one there.
- When it does work, the build runs where no session's binds, `HOME`, `/tmp`
  or `.git` are, and where every Worktree is writable. That breaks builds
  that use git, and lets one Conversation's build logic and Kotlin compiler
  plugins write another Conversation's checkout. This is a wider crossing than
  the Compile Server's: sccache runs a compiler on paths it is handed, and
  Gradle runs a Repo's code.
- It costs a daemon per distinct combination, around a gigabyte each, up while
  nothing runs, holding the jars the Sweep would take.

None of that is fixable from Verkstead's side without Gradle growing an
*attach-only* client, which it does not have.

**Take the per-session registry? Yes.** It gives most of the speed: a warm
build in a fifth of the time, and a Kotlin rebuild in 0.4 s instead of 3 s.
Every build stays in its own session's Sandbox, and it closes the `--daemon`
hole stage 04 left open. The per-session daemon dies with its session, so the
Sweep and a Clear have nothing to stop. Its cost is a daemon per session that
builds on the JVM, held for the life of that session. Its risk is the one it
was turned down for: an undocumented property, still read by 9.4.1 and still
missing from the docs. That risk is bounded. If the property goes, the result
is a loud failure in the case stage 04 already accepted, not a quiet one, and
a proof in the suite asserting that two sessions do not meet would catch it
on the next Gradle bump. The open questions it leaves are for the grilling:
where the directory is off Linux, whether the per-session daemons get a
shorter idle timeout than 3 hours to bound the memory, and whether the
Terminal of the same Conversation shares its session's daemon (on Linux it
does when they share a root, and they share every bind besides).

**Leave the daemon off? Only if the undocumented property is judged not worth
its risk.** It is safe and measured. The cost is about 1 s on every trivial
build and about 3 s on every Kotlin rebuild, against 0.2 s and 0.4 s, and the
`--daemon` hole stays as it is, accepted and documented.
