# 03. Stage backlogs, the join and placement

## What to build

The backlog runner from task 02 takes over a **stage's** backlog too, which
brings three things the plain task list never needed. Each is the host's
decision, so each is a host function the component calls explicitly, not one
hidden inside `launch` (Set 1839, Q3).

- **Planning a stage.** A stage Conversation with no backlog yet launches a
  session on `verkstead/next-stage` expecting `landed(arrived(".tasks/TODO.md"))`,
  primed with the stage's Brief and where its branch was cut from, as
  `plan_stage` and `skills::next_stage` do now. The host still records the
  backlog landing (`conversations::backlog_landed`). A stage still waiting on
  the human's answer about where it starts (`placing::stage_waiting`) is not
  planned again. Whether a Conversation is a stage, and what it was cut from,
  reach the component as start facts or options.
- **The chain join.** Before the finish, a host function waits on the chain
  as `joins::let_in` does now, and answers the branches this one goes on top
  of. The component carries them in the finish's prompt.
- **Placement.** A task line declaring `on <platform>` (read off `TODO.md` the
  way `declarations::task_wants` reads it) is placed by a host function before
  its session is launched, as `placing::task` does now. It answers either that
  the work stays here, or that the Conversation moved or is waiting on the
  human. In those two cases the component ends without an outcome, because the
  Activity is started again wherever the Conversation lands.

ADR-0029's placement stays the host's. The component names the platform, and
never a device.

## Acceptance criteria

- [ ] A stage Conversation plans its backlog, works its tasks and joins the
      chain before its finish opens a pull request, all under the component
- [ ] A task line `on windows` on a Linux server moves the Conversation, and
      the backlog runner starts again over there at the same box
- [ ] Resume on a stopped stage takes it up where it was: planning, a task, or
      the join
- [ ] The sessions suite's stage and join tests pass without being rewritten
