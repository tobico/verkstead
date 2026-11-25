# 01. Carry an inline run and a roadmap's writing on

## What this is

`resume::recompute` reads whether the session that stopped was **cut off** — a
session stopped part way through its step rather than finishing it — and writes
a `continued_sessions` row for the launch after it to spend, so that what starts
is that session resumed with the harness's own resume rather than a fresh one
primed from the record. ADR-0030 is the whole of the reasoning.

Four states reach that reading: Grilling, Implementing, Follow-up and
Investigating. Inside **Implementing** it is the direction's own answer, and only
one of the three directions answers it:

```rust
// crates/server/src/resume.rs
async fn mid_turn(direction: Direction, worktree: &Path) -> MidTurn {
    let part_way = match direction {
        Direction::TaskList => crate::runner::mid_step(worktree).await,
        Direction::Inline | Direction::Roadmap => false,
    };
    ...
}
```

So a session cut off part way through an **inline** implementation, or part way
through a **roadmap**'s own writing, is replaced rather than carried on: whatever
the agent had worked out goes with the process, and the human gets a fresh
session re-reading the Brief. Which is the loss the whole occasion exists to
stop — a task session cut off mid-task is carried on today, and an inline one
doing the same work is not.

## Why it was left, and what that means for the fix

The row the reading writes is **spent by the launch after it**, and the two
drivers these directions reach — `runner::inline_again` and
`runner::roadmap_again` — may launch no session at all. What each of them asks
is whether the branch is already on a pull request, which goes over the network
to `gh`, and the answer may be *wrap it up*, which starts the wrap-up's watchers
and no session. A row written in the recompute in front of that is a row nothing
spends: it sits there until the next session of the work takes it up, and that
session is told to resume a conversation that had nothing to do with it.

So this cannot be another arm of `mid_turn`. **The row has to be written where
the session is launched**, after whatever decides there is a session to launch at
all. `resume::cut_off` is the writing, and it is `pub`-less today — it will need
to be reachable from `crate::runner`, or its guts moved somewhere both can call.
Note what it already does that must not be lost: it refuses to write over a row
an arrival or an answer pre-wrote, it says nothing where no session ever ran
(`store::newest_agent`), and it writes a Notice naming *why not* where a session
ran that Verkstead cannot name.

## And what mid-turn means here

The second half of the work, and the part that needs deciding rather than
plumbing. A task-list run reads it off the Worktree — every step of a run ends in
a commit, so work left standing is a step left part way through, which is
`runner::mid_step`. Neither of these two directions has a backlog, so there is no
step to have been part way through, and the two are not the same shape as each
other:

- An **inline** implementation is one session doing the whole of the work, and
  its one step ends in the commit it pushes. Uncommitted work in the Worktree
  reads as mid-turn by `mid_step`'s own logic, and an inline run that has
  committed and pushed is one whose turn was over — but an inline run that
  committed and was cut off before its push is mid-turn with a clean Worktree,
  which is the case `mid_step` would get wrong. What the branch holds against
  what the remote holds may be the better reading here.
- A **roadmap**'s own writing is a session writing the roadmap document, and its
  step ends in that document being committed. Whether an uncommitted or absent
  roadmap reads as mid-turn is the same question asked of a different artifact —
  `runner::roadmap_again`'s own reading of what it finds is where to start.

Decide each with the human if the reading is not obvious from the code. Getting
it wrong in the *mid-turn* direction is the cost ADR-0030 names: an agent told to
resume a conversation whose turn was already over, and sent to pick up work it
had just finished.

## Done looks like

- A session cut off part way through an inline implementation is carried on by
  every way back in — the human's Resume, a login's carry-on after a sign-out, a
  restart's startup pass, and an arrival — and the Timeline says which of the two
  resumes it was, in the cut-off's own clause.
- The same for a roadmap's own writing.
- **No row is left standing where no session is launched.** A test that takes one
  of these to the reading that wraps up with no session in it, and asserts that
  the session after that is an ordinary session of the work rather than one told
  to resume a stranger's conversation.
- Tests for each direction's mid-turn reading, both ways round: cut off part way
  through, and stopped with its step done.
- CONTEXT.md's **Carried Conversation** entry and ADR-0030 both say an inline
  implementation and a roadmap's writing read as *between steps* and were left
  for whoever wanted them. Both need rewriting to say what they read now.

## Where to read

- `crates/server/src/resume.rs` — `mid_turn`, `mid_round`, `cut_off`, and the
  Implementing arm of `recompute`.
- `crates/server/src/runner.rs` — `inline_again`, `roadmap_again`,
  `backlog_again` (whose `Step::Nothing` correction is the job-must-not-change
  guard on the run's side, and the shape a launch-site reading has to fit
  beside), `mid_step`, `carrying_on`.
- `crates/store/src/continuations.rs` — the row, and
  `carrying_a_conversation`'s read-without-spending.
- ADR-0030, especially *An inline implementation and a roadmap's own writing read
  as between steps*, which is the decision this task reverses, and *The job must
  not change under a carry-on*.
- CONTEXT.md: **Carried Conversation**, **Resume**, **Stopped**.

## Careful on this machine

This work runs on the Verkstead the change is about. Do not restart, stop or
replace `verkstead.service` on this annex to test a restart — it would end the
session doing the work. Test a shutdown or a force stop against a second
Verkstead with its own Data Directory and ports, as docs/development.md
describes.

Both `env -u` are needed to reach the server suite, or `cargo test` stops at the
cli suite's three `guide::` contract tests:

    env -u VERKSTEAD_SERVER nix develop . --command \
      env -u VERKSTEAD_SERVER cargo test -p verkstead-server --no-fail-fast

The server suite's baseline on this machine is nine failures that are the machine
rather than any branch: six `sessions::` terminal and preview ones (a Sandbox
with no `/home` never reaches a prompt) and three `package_stores::` dotnet ones.
A full parallel run fails a handful more under load, and the set is not fixed —
subtract the nine, rerun whatever is left alone with `--test-threads 1`, and only
chase what fails twice. A run also rewrites `web/tests/fixtures/repos.json`;
`git checkout --` it before committing.
