# 03. The skills that keep the declaration

## What to build

Three skills write the end of a stage line, and all three have to leave the
declaration on it. Two of them would write one away today without meaning to,
and a roadmap that has lost one line's declaration is exactly the mixed
roadmap task 02 refuses.

**`staging` writes the line in the first place.** Its stage-defining step and
its roadmap template ask for a declaration on **every** stage line, in the
wording the reading expects: `after 01, 03` naming other stages by their
labels as written, `no dependencies` for a stage that stands on nothing, and
`on linux`, `on macos` or `on windows` where a stage wants a platform. All or
nothing, said plainly, because a roadmap declaring on some lines and not
others is refused at that same session's `verkstead done`. The template's
prose dependency notes above the list **stay where they are** — they are the
*why* behind the declarations rather than a substitute for them.

**`next-stage` appends the in-progress annotation** when it plans a stage, and
**`next-task` takes it off again** when it ticks the stage off. Each spells a
whole stage line out in an example, and each example as written would replace
a declaring tail with a bare one. Both say the declaration stays where it is,
and **every example stage line in all three skills carries one** — an example
is what an agent copies.

The text assertions in `skills.rs` cover the new wording, the way they already
cover the `## Stages` list and the brief's own sections: these are forks
Verkstead ships, and what keeps a fork saying what Verkstead needs is a test
that reads it.

One thing to know about the timing. The change lands in the skills this
repository ships, and the sessions that plan the remaining stages of the
`parallel-stages` roadmap run against the **installed** server until this
branch is released — so those sessions still write the annotation over
whatever the tail held. The roadmap's own lines are the ones to watch: each
stage session puts its declaration back by hand, and the roadmap's preamble
already says so.

## Acceptance criteria

- [ ] `staging`'s stage-defining step and its roadmap template ask for a
      declaration on every stage line, in the wording the reading expects, and
      the template's prose dependency notes are kept as the why.
- [ ] `next-stage`'s score step and `next-task`'s tick step both say the
      declaration stays where it is, and every example stage line in the three
      skills carries one.
- [ ] `skills.rs` asserts the wording in each of the three, and following
      those skills against a declaring line — annotating the stage in
      progress, then ticking it off — leaves the line's declaration untouched.
