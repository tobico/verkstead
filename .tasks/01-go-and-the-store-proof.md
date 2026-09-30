# 01. Go, and the proof a store was read

## What to build

Go's built-in **Descriptor** — the first of this stage — together with the
harness every later task adds a row to, and `go` in the dev shell and in CI.

Go goes first because its store is the simplest and because it has both halves
the roadmap cares about: a module cache of downloads and a build cache of
compiled output that is nothing but a directory. The entry is keyed `go`, with a
label, the manifest that detects it, and the two variables that move those two
directories under `{cache}` — both checked against Go's current documentation as
this task starts, not assumed. Detection is for the composer's warning and
nothing else: the variables are every session's whatever the Repo holds.

**The harness is the deliverable that matters.** A new Linux-only suite that, per
tool, stands up two Sandboxes against one Build Cache and runs a real install in
each **at the same time**, then a third in a Sandbox of its own **denied its
registry** — `GOPROXY=off` for Go. A third build that succeeds with nothing to
fetch from succeeded out of the store, and the two that raced are what say they
did not damage it. *Found the store populated* is deliberately not the
assertion: the store is populated because the first installs populated it,
whether or not the third read a byte of it.

A proof whose tool is not installed is **skipped with a line naming the tool**,
so a checkout run on a machine that builds only Rust stays green — and **a skip
fails the build in CI**, so a tool cannot quietly leave the list. CI gets a job
of its own on Linux rather than more steps in the Rust job, installing what the
runner image lacks; the runner already carries `go`.

The environment assertions come with it, on all three platforms, beside where
Rust's are asserted today. Two existing tests assume Rust is the only built-in
and will need putting right: the one that switches Rust off and asserts the
Build Cache bind has gone — with Go in the file it has not — and the unit test
that splits a rendered environment at the built-ins' length.

No `size` key and no `{size}` placeholder anywhere in this stage: the settings
page draws a size field only for a language whose store an sccache bounds, and
bounding a package store is stage 05's.

## Acceptance criteria

- [ ] Go's two directories are set for every session on all three platforms,
      asserted where Rust's are, and the Build Cache comes in writable for them
- [ ] Two Sandboxes build one module at once against one store, and a third
      Sandbox then builds it with `GOPROXY=off` and succeeds
- [ ] A proof whose tool is missing is skipped in a line naming it, and the same
      skip fails the build in CI
- [ ] `go` is in the dev shell; the new CI job runs the suite on Linux, and the
      pull request records CI's wall clock before and after
