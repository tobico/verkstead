# 01. What a declared roadmap has ready

## What to build

One reading that answers, for a roadmap in a Repo, **which of its stages may
start now**. It takes three things that already exist: what each stage's line
declares it stands on, Verkstead's own record of what each stage of that roadmap
has got to, and the boxes the roadmap keeps. A stage is **ready** when every
stage it stands on has **settled** by the record — not merged — and it is itself
neither done nor in flight. Ready stages come back **lowest number first**, the
roadmap's order still being the roadmap's own.

**An undeclared roadmap is read as each stage standing on the one before it.**
That is what *in order* means to the scheduler, and it is what keeps this one
reading rather than two: every roadmap written before any of this runs exactly as
it did, and a stage in flight now holds up what stands on it instead of being
skipped over — declared and undeclared alike.

Both readings of a roadmap go through it: the carry-on that runs when a stage
settles, and the adoption's, which answers the roadmap notice, the compose page
and the press off one reading. **Neither starts more than one stage yet.** The
carry-on takes the lowest ready stage and the adoption offers it, so what changes
here is *which* stage that is rather than how many — the starting of several is
task 03's.

And **nothing ready stops meaning the roadmap is complete**: a roadmap whose only
unfinished stages are in flight has nothing to start and is not finished. That is
a third answer beside the stage and the roadmap complete, and the Timeline says
so in its own words.

## Acceptance criteria

- [ ] A declared roadmap whose 03 stands on 02 starts 03 when 02 settles, with 01
      still in flight — rather than the lowest unticked box.
- [ ] A stage whose dependency is in flight is not ready and nothing is started
      for it, in a declared roadmap and in an undeclared one alike.
- [ ] A roadmap with nothing ready and a stage still in flight says there is
      nothing to start; one with nothing ready and nothing in flight is complete.
