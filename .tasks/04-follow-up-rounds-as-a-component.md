# 04. Follow-up rounds as a component

## What to build

Rewrite follow-up rounds as the first built-in Activity component.
`runner::following_up`, `over`, `back_to_the_wrap_up` and `finished` move into
it, with `follow_ups.rs`'s reading of the brief and the rounds behind them. A
Conversation steered into Follow-up, and a Tinker started from a Draft, run
their rounds under it through the seam task 03 made.

**The plugin.** Make one built-in **Activities** plugin, with a block per
Activity (Set 1803, Q3). *Follow-up rounds* is its first block, with its own
component.
- The plugin declares a **dependency** on the `verkstead` pack's plugin, and its
  block names `verkstead/following-up`.
- It is listed under Built-in, and its row names *Follow-up rounds*.
- Stages 09 and 10 add their Activities to the same plugin as blocks of their
  own.

**The component's rounds.**
- It launches one session under `verkstead/following-up` with `NothingElse`
  evidence, ending with its work. It writes its own prompt from the start facts,
  with the same words `skills::following_up` writes today: the Brief, the
  handoff, the steer's brief or a Tinker's, and the rounds already settled.
- **Ending.** On *done* or *marked*, it reads the git facts of the branch and of
  every read-write Companion past their bases. It ends `committed` where any of
  them holds commits, and `nothing` where none does.
- **A gone session.** It re-checks the mark and the open Set, as today. A
  session that finished on a marked round ends as above. Otherwise it calls
  *stop* with today's sentence (*The follow-up session finished. Nobody is left
  to ask you anything…*), which closes the Set left open.
- **A steer** ends it `steered`.

**Options.** The skill, the evidence and the ending's reading are options
defaulting to the follow-up's values. Stage 09 then runs investigating rounds
as this Activity with `verkstead/investigating`, `Investigated`, and a check
that nothing was committed. Do not wire that here.

**The host's mapping until stage 11** (Set 1803, Q7):
- On a pull request, either Outcome goes back to the wrap-up as
  `back_to_the_wrap_up` does. `pushed` — commits landed past where they stood
  at this round's launch — stays the host's reading, and re-arms the checks.
- `committed` with no pull request sends the `submitting` session, as
  `asked_for_a_pull_request` does.
- `nothing` lands Done, as `finished` does.

Keep the two readings apart: the Tinker ending reads the branch past its base,
and never `pushed`.

## Acceptance criteria

- [ ] A steered Follow-up and a Tinker run their rounds under the component, and every follow-up and Tinker test in the sessions suite passes unchanged.
- [ ] A Tinker that committed only in a Companion still lands in Wrapping on a pull request it sent for, and one that built nothing lands Done.
- [ ] The Plugins page lists the Activities plugin under Built-in, naming *Follow-up rounds*, and `verkstead plugin check plugins/<its dir>` passes.
