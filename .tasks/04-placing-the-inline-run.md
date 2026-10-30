# 04. Placing the inline run

## What to build

An inline run whose proposal wants exactly one platform starts there (ADR-0029,
*Placing declared work*). Stage 02 gave the closing `proposal` a `wants` list
(`verkstead_schema::Want`, `WantedPlatform`), but it lives only on the
proposal Set's YAML: nothing on the server reads it yet. Read the `wants` of
the proposal the human chose a direction on (the Conversation's grilling
proposal Set, which `store` already finds proposals by).

When the grilling hands off (`follow_handoff`) and Inline was chosen, the
implementation session is placed **before it launches**, the way task 02
places a task: same candidates, same preflight, same Transfer on Verkstead's
own account, and the far end's Resume (`inline_again`) launches the
implementation there. The handoff is recorded in the store before the move, so
it crosses with the record. The Timeline names the inline run, rather than a
task, as what the move was for. Where it cannot be placed, task 03's Notices
apply, naming the work rather than a task.

- **Wanting exactly one platform**: placed.
- **Wanting several, or none**: starts where it is, and the agent moves itself
  as the work needs.
- **Placed only where it starts** (Set 1543, Q5b): a Resume of an inline run
  (`inline_again`) never asks for a placement. Once the run has started, the
  agent and the human decide where it is.

Write the inline half of CONTEXT.md's **Transfer** *Being revised by ADR-0029*
pointer into the term, leaving the stage half pointing at stage 06.

## Acceptance criteria

- [ ] A proposal wanting `windows`, chosen Inline on a Linux device with a Windows device ticked, has its implementation session start on the Windows device, and the Timeline says it moved for the inline run.
- [ ] A proposal wanting `linux` and `windows`, or nothing, starts where it is.
- [ ] A Resume of an inline run on a device of the wrong platform launches there without moving.
- [ ] With no permitted device of the wanted platform, the inline run gets task 03's Notice and starts here.
