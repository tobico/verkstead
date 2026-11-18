# 10. The backlog and stage runners across

## Goal

The last two built-in Activities cross: the **backlog runner**, which works
`.tasks/TODO.md` box by box with a `next-task` session per task and the finish
step after the last, and the **stage runner**, which works a roadmap under
`docs/roadmaps/<name>/` by starting a Conversation of its own per stage on a
branch of its own, as many at once as the declarations and the limits allow,
each joining the chain as it finishes. With them, every Activity Verkstead runs
is a component in a built-in plugin, the runner is host functions and nothing
else, and stage 11 can write all five Processes as data. The Activity interface
gains the one host function the stage runner needs and no other Activity has:
starting a Conversation.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Activity*.** Built-ins cross to
  prove the boundary; these two are the ones with the most host behaviour
  behind them, so they cross last among the Activities and with the interface
  already proven by four.
- **Both are stateless across restarts, and already were.** The backlog
  runner reads `.tasks/TODO.md` at HEAD to find the next unticked box, and the
  stage runner reads `ROADMAP.md`'s checkbox list and the declarations after
  each link; neither keeps a table of where it got to. That is the rule Set
  1767 Q31 settled for every Activity — files in the Repo are the truth — and
  these two are its origin. Crossing them must not add a store the originals
  never had.
- **The backlog runner's outcomes** are `finished`, when the finish step's
  pull request is open and its Done signal accepted, `abandoned`, when a task
  session was stopped and the human did not resume it, and `steered`. Each
  task runs as one session on `verkstead/next-task` with the evidence
  `Committed` and the box ticked in the commit — the finish being its own
  session, which `verkstead done` reads the task's box out of `.tasks/TODO.md`
  for. Memos left per task are the host's record and reach every later session
  as they do now.
- **The stage runner starts Conversations**, which no other Activity does, so
  the Activity interface gains a host function here: *start a Conversation* on
  a Repo with a Brief, a Process and a branch cut from a named base, and *read
  a Conversation's state and settledness*. It is a capability like the others,
  named in the manifest and shown at install, because an Activity that can
  start work is one worth knowing about. The stage runner's own outcomes are
  `finished`, when every box is ticked and settled, `abandoned`, and
  `steered`.
- **The scheduler stays what [ADR-0021](../../adr/0021-parallel-stages.md)
  made it**: a stage starts when the stages its line names are settled, up to
  the roadmap's limit and the server-wide one from `at_once`, and joins the
  chain as it finishes before its pull request opens. The limits are the
  host's — `at_once` is this device's `config.yaml` — so the host refuses a
  *start a Conversation* over the limit and the Activity waits, rather than
  the Activity reading the limits.
- **Placement by platform stays the host's too**
  ([ADR-0029](../../adr/0029-work-is-placed-by-platform.md)): a stage's line
  may say `on windows`, and which device the Conversation starts on is decided
  where *start a Conversation* is implemented, not in the component.
- **The `next-stage` annotation** — the installed server writing over the tail
  of a stage's line to mark it in progress — is written by the host when it
  starts the Conversation, since the Activity cannot write the Repo's files
  without a capability it has not asked for and should not need.
- **The stage runner reads the finish sequence from
  `docs/agents/git-workflow.md`** of the Repo it runs in, as `stages.rs` does,
  and hands it to the stage's Conversation in its Brief; that file is the
  Repo's and the Activity reads it through the Worktree capability.

## Proposed tasks (provisional)

1. **Start a Conversation** — the host function and its capability: a Repo, a
   Brief, a Process, a branch and base, a platform where the line names one;
   refused over the limits and the Activity told to wait; a Conversation's
   state and settledness readable back. AC: a fixture Activity starts a
   Conversation and reads it to Done; one over the limit waits and starts when
   a slot frees; a stage marked `on windows` is placed as ADR-0029 says.
2. **The backlog runner as a component** — `tasks.rs`'s box reading, the
   `next-task` session per task with `Committed` evidence, the finish step,
   the memo flow; outcomes `finished`, `abandoned`, `steered`. AC: a task list
   runs to its pull request under the component; an interrupted run restarts
   at the right box with nothing stored; the finish refuses a task session that
   also deleted the backlog, as today.
3. **The stage runner as a component** — `stages.rs`'s index reading, the
   declarations, the chain join, `next-stage` annotation requested of the
   host, and the scheduler's waits. AC: a roadmap with declarations runs
   stages side by side up to the limit; a stage joins the chain before its
   pull request opens; a roadmap declaring on some lines and not others is
   refused as today.
4. **The runner emptied** — `tasks.rs`, `stages.rs`, `units.rs`,
   `continuing.rs` and `carrying.rs` reduced to host functions; the Plugins
   page lists *Backlog runner* and *Stage runner* under Built-in. AC: no
   Activity logic remains in the server outside the components; the suite's
   backlog and roadmap tests pass unchanged.

## Re-verify at start

- Stages 08 and 09 landed, and what the `activity` world looks like after
  them — in particular whether a host function already lets an Activity read a
  Repo's files, which the stage runner needs for `git-workflow.md`.
- `stages.rs` still reads `docs/agents/git-workflow.md` near line 388 and
  still writes the `next-stage` annotation over the tail of a stage's line.
- `at_once` still carries `roadmap_stages` and `conversations` in this
  device's `config.yaml`, and `placing.rs` still decides a stage's device.
- The finish step's refusal — a task session that also deletes the backlog —
  still lives in `done.rs`'s reading of `.tasks/TODO.md` at HEAD.
- Memos are still kept per Conversation in the store and shown to every later
  session; nothing about them should move into the component.
