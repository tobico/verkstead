# 05. Retire the gates topic

## What to build

Nothing in the pipeline gates a commit any more, and the diagram convention
the gates Topic carried now lives in the committing skills — so the Topic
leaves the binary. Remove `gates` from the Guide's Topics, take its text out
of the CLI, and rewrite the core Guide's routing so it no longer sends
approval asks to a Topic that is not there. The tests that held the Topic to
its word go with it; the tests that hold the core Guide together are updated
to match what it now says.

The acceptance-gate example under `examples/` is documentation of the old
world and is referenced by the public-release roadmap — leave it where it is;
this task retires the Topic, not the example.

## Acceptance criteria

- [ ] `verkstead guide gates` no longer names a Topic, and the core Guide
      neither lists it nor routes approval asks to it.
- [ ] The CLI's test suite passes with the Topic gone.
- [ ] The acceptance-gate example and its roadmap references are untouched.
