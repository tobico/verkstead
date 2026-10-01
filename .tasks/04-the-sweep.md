# 04. The sweep

## What to build

A sweep that brings every store over its size back under it by removing **whole
units, oldest first** — the units task 03 names, the sizes task 01 parses, the
walk task 02 built. Stores bounded by their tool (sccache) and stores naming no
unit are left alone. Rust's cargo half is held to Rust's size separately from
sccache. Sized per language, over all of that language's swept stores together.

**When.** On a timer the shape of the Cleanup's (`cleanup.rs`: a loop spawned at
startup, never on a server that runs no sessions, its pace in `Pace`, settings
read on every pass) — hourly. And **only while nothing runs**: no session
(`Sessions::working`) **and no Conversation Terminal** — both are sandboxed with
the stores (`sessions.rs` and `terminals.rs` are the two callers of
`sandboxed`). A sweep that comes due while something runs happens **at the first
moment after that when nothing does**, not at the next tick; a machine that is
never idle is never swept, which is accepted (Set 996) and is what the pane's
*last swept …* and the docs say.

**Never half a unit.** A unit is **renamed aside** before it is deleted, into a
directory of Verkstead's own at the top of its placeholder's directory
(`{cache}` or `{stores}`), so the rename stays on one filesystem; a rename that
fails with `EXDEV` (a store an installer mounted separately) skips that unit
and logs it, never falls back to a copy. Whatever is left aside from a pass that
died is deleted at the start of the next. **A session or terminal launch waits
for the unit being moved**, so nothing launches into a half-moved store and no
running thing ever sees one vanish; the sweep stops between units the moment
anything starts. **Read-only units** — Go's module cache (0555 directories,
0444 files), cargo's git packs — are made writable before the rename (a
directory's own write bit is needed to move it) and the delete. **Never follow
a symlink.**

**Oldest** is a unit's newest **access time** where the filesystem keeps one,
and its newest **modification time** where it does not (`noatime`); Linux's
default `relatime` updates atime about daily, which is fine for this. Take the
newest time **anywhere in the unit** or of the unit directory itself — a Maven
jar's own mtime is the server's Last-Modified date, so a fresh download of an
old artifact must not look old. Decide how the server tells whether a store's
filesystem keeps access times, so the docs (task 06) can say it.

All three platforms. On Windows, sessions run as a local account of Verkstead's
own; check the server can rename and delete what that account wrote in the
store (the cfg(windows) suite runs under Wine here).

Afterwards the disk use from task 02 is refreshed, and the pane shows when
the store was last swept.

## Acceptance criteria

- [ ] A store over its size is brought under it by removing whole units, oldest first by atime (mtime where none is kept, newest time in the unit); a store under its size, one bounded by its tool and one naming no unit are untouched.
- [ ] Nothing is removed while a session or a Conversation Terminal runs, a launch waits for the unit being moved, a due sweep runs at the first idle moment, a unit is never seen half-deleted (rename aside on the same filesystem, leftovers cleared next pass), and read-only Go module directories are swept.
- [ ] The real-install proofs from stages 02–04 (`tests/package_stores.rs`) pass against a store that has just been swept down to a size below what it held.
