# 05. Sizes and eviction

## Goal

Every language's store has a size, and stays under it. The settings page shows
what each store holds on disk beside the size it is allowed, with a field to
change the size and a Clear. A machine that has run Verkstead for a year has
not had its disk filled by packages for projects nobody has opened since.

## Decisions in force

From [ADR-0021](../../adr/0021-language-descriptors.md).

- **A size per language.** `10G` where nobody has said; Rust's sccache stays
  at `30G`.
- **Tools that evict for themselves are handed the size and left alone.**
  sccache is the one already wired. Which others do, and how each is told a
  size, is checked per tool at the start.
- **For the rest, the descriptor names the unit** — a directory depth — and
  whole units go, oldest first, until the store is under its size. **Never by
  file:** a Go module or a Maven artifact with one file missing is a broken
  store rather than a smaller one.
- **Rejected:** clearing a store whole when it goes over, which makes the next
  build cold; and running the tool's own prune command, which needs the tool
  installed where the server is and breaks the rule that a descriptor is data.
- **On a timer, and only while no session is running.** A unit is renamed
  aside before it is deleted, so a tool never sees half of one.
- **Oldest is access time where the filesystem keeps one, and modification
  time where it does not.** The docs say which a reader likely has, because on
  a filesystem keeping no access times *least recently used* quietly becomes
  *least recently written*.
- **Disk use and a Clear per language on the page**, beside the size.
- **Rust's cargo half has no size today** and says so in `build_cache.rs`.
  Whether it gains one here, as every other store does, was not asked
  separately; the stage's own grilling confirms it.
- **Clear was not given a rule about running sessions.** The sweep waits for
  none to be running; whether a Clear pressed by a human waits, refuses or
  goes ahead is the stage's to settle.

## Proposed tasks (provisional)

1. **Size in the descriptor and on the page.** AC: every language draws a size
   field with its default as the placeholder; the size reaches a
   self-evicting tool as its own variable.
2. **Disk use.** AC: the page shows each store's size on disk; measuring a
   large store does not hold up the settings read.
3. **The unit, in the grammar and the built-ins.** AC: each built-in without
   an evictor of its own names a unit; a descriptor naming none is never swept
   and the page says so.
4. **The sweep.** AC: a store over its size is brought under it by removing
   whole units oldest first; nothing is removed while a session runs; a unit
   is never seen half-deleted; the real-install proofs from stages 02 to 04
   pass against a store that has just been swept.
5. **Clear.** AC: a cleared store is empty and the next install refills it.
6. **The docs.** What is evicted, when, and what *oldest* means on the
   reader's filesystem.

## Re-verify at start

- Assumes stage 01 landed. Says nothing useful about stores whose stages have
  not: check which of 02, 03 and 04 are in, and name units only for what is
  there.
- Check `cleanup.rs` and its pace for the timer this sweep should share a
  shape with, and how the server knows no session is running.
- Check, per tool, which evict for themselves and how each is given a size.
- Check whether any store holds read-only directories, which a rename or a
  delete has to get past — Go's module cache was named in the grilling.
- Check that a rename aside stays inside the store's own filesystem.
