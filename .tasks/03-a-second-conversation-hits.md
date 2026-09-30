# 03. A second Conversation's build hits the cache

## What to build

sccache hashes a C/C++ compile's absolute paths: the source file, the `-I`
directories, and the line markers in the preprocessed output. Every
Conversation has its own Worktree (`<data>/worktrees/<repo>-<branch>…`), so the
same project in a second Conversation misses every time. This was measured
against sccache 0.15 while planning: two directories holding one source gave 0
hits.

sccache's **`SCCACHE_BASEDIRS`** fixes this. It takes a list of absolute
directories and strips them from what is hashed, and with both directories
named, the same measurement hit, `-g` and `-I` included. **It is read by the
server, not the client** (setting it on the client did nothing), **only when
the server starts**, and it takes exact paths, not a pattern. A parent
directory holding both Worktrees does not work, because the Worktree's own name
is still left in the path.

So the Compile Server is started with `SCCACHE_BASEDIRS` naming every Worktree
that exists, and it is **restarted to take in a new one only while no other
session is running**. That was decided when the plan was put to the human. A
restart under a session mid-compile can leave that session's client starting a
server of its own inside its Sandbox, which is the hazard the Compile Server
exists to remove. On a busy machine, then, a new Conversation's compiles miss
until the next quiet moment. A miss is a slow build, never a failed one.

`BuildCache::compiling` already runs at every session spawn and already
restarts the server when the wanted size changes. The set of Worktrees becomes
a second thing it compares, with the restart held back while any other session
runs. A Worktree that no longer exists is simply dropped from the list at the
next restart. Separate the list with the separator sccache expects on each
platform (`:` on the Unixes; check Windows).

Check the lowest sccache version that honours `SCCACHE_BASEDIRS`, and what an
older one does with it (probably ignores it, and so misses). Record that for
task 05.

Extend task 02's proof. A second Conversation, meaning a second Worktree at a
different path, builds the same CMake project after the first. The Compile
Server is restarted between the two because no session is running then.

## Acceptance criteria

- [ ] The same CMake project built in a second Conversation's Worktree is served
      entirely from the cache, asserted from the Compile Server's stats.
- [ ] While another session is running, spawning a session for a new Worktree
      does not restart the Compile Server. Once no session is running, the next
      spawn does, and its `SCCACHE_BASEDIRS` then names the new Worktree. This
      is proven by a test of the decision, not of timing.
- [ ] A restart for a changed size still happens as before, and Rust sessions
      behave as before: their proofs pass unchanged.
