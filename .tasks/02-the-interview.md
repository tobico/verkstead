# 02. The interview

## What to build

**The interview**, a component in the built-in Activities plugin: it launches
the grilling session on `verkstead/grilling` under the Grilling role, and ends
when that session's `verkstead done` is accepted with the picked Direction's
artefact checked — the handoff at `/tmp/verkstead/handoff.md` for inline, the
plan commit for a task list, the roadmap commit and pull request for a roadmap
(ADR-0008). Its outcomes are `direction:inline`, `direction:task-list` and
`direction:roadmap`, plus `steered`.

Breaking down and staging go on inside the same session, as the grilling skill
says, so the interview launches one session and never a second.

The proposals the session puts are honoured by the host as today
(`conversations.rs` `settle_a_proposal`, `write_the_artifact`) — they are not
the component's. `start_grilling` and the relaunch (`grillings.rs` `again`,
with the Sets a relaunch holds open, the settled digest) call the Activity by
name; stage 11 replaces that choice with the Process's first state. The
component is stateless across restarts: a relaunched grilling is the Activity
started again over the Timeline, as Follow-up rounds is.

Where an outcome lands the Conversation (Implementing over a handoff, the
backlog runner, the stage runner) stays the host's mapping in this stage.

The chooser drawn from an offered list is task 03; here every proposal still
offers all three Directions.

## Acceptance criteria

- [ ] A grilling ends in each of the three Directions with the right artefact checked and the outcome fired, the grilling tests passing unchanged
- [ ] The interview's own Sets still carry the Diff, and a proposal is still settled and its artefact written by the host
- [ ] A grilling picked up again (Resume, or a restart) relaunches under the component with its held Sets and digest as before
- [ ] The Plugins page's Activities row names *Interview*
