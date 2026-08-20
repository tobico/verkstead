# 06. The auto-advancing task runner

## What to build

The backlog works itself. Once `.tasks/` exists, Verkstead launches a fresh
session for the lowest-numbered task, waits for it to finish, and launches the
next — unattended, one task per session, with no gate between them and no
approval asked. The pinned task-list Event ticks along as tasks complete.

The decision core and the done-signal are roadrunner's, ported rather than
reinvented — the semantics are still the right ones:

- **What is next** is read from the repo alone: the lowest-numbered task file
  left, or a finish step when only `TODO.md` remains. Task files are
  `NN-<slug>.md`; `TODO.md` never matches.
- **A task is done** when its file is gone from the Worktree *and* the commit
  removing it has landed — a file deleted but not committed is a session still
  mid-task. The finish step's signal is `TODO.md` going the same way.
- **The poll must not take `index.lock`.** It is watching a repo a session is
  committing in, and a watcher that trips the session's own `git add` breaks the
  step it is waiting for.
- **A session ends on done plus quiet**, not on done alone. Work does not always
  stop at the commit, so the session is ended only after it has printed nothing
  for a grace period, and output arriving puts the whole grace back on the
  clock. A session that keeps talking is never killed blind.

The fresh session per task runs the bundled next-task fork, which — like the
to-tasks fork — drops the approval gate, the context-clear prompt and the finish
gate that a workstation flow needs and Verkstead supplies instead.

Nothing here handles a session that crashes or hangs; that is task 07.

## Acceptance criteria

- [ ] The next step is decided from `.tasks/` alone — lowest-numbered task file,
      or the finish step when only `TODO.md` is left
- [ ] The done-signal is the task file gone **and** committed
- [ ] Polling uses git in a way that never contends for `index.lock`
- [ ] A session is ended only once done and quiet for the grace period, with
      fresh output restarting the grace
- [ ] Completing one task launches a fresh session for the next with no gate
- [ ] The pinned task-list Event updates as tasks complete
- [ ] An empty backlog leaves the runner idle rather than looping
