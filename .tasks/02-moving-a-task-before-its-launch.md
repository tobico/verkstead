# 02. Moving a task before its launch

## What to build

Verkstead moves declared work; the agent never calls `verkstead transfer` for
it (ADR-0029, *Placing declared work*). When the driver picks the next task
(`carry_on` → `next_step`), and before it launches that task's session, it
reads the task's platform (task 01). If the task wants a platform and this
device's OS, read through `platform::os_mark` (never matched as a string),
isn't that platform, the driver **places** the task:

1. **Candidates** are the Conversation's *permitted* devices of that platform,
   read with the same rule the agent's own call and `transfers::cluster` use.
   That is the drafting device plus the ticks, in that order, never this
   device (Set 1543, Q2). Each member's free-text `os` word goes through
   `os_mark`, so *Linux (WSL)* is a Linux device: `on linux` may pick a Windows
   machine's WSL, and `on windows` never does.
2. **The first one whose preflight is clean** is chosen (`preflight::of` with
   no `lacks`). A device that is unreachable, or that answers but lacks
   something (no Repo, no harness, another Author), is passed over (Q3).
3. **The move is the ordinary Transfer**, requested on Verkstead's own account
   for this task. With nothing running it runs where it stands, so the driver
   asks for it and stands down. The far end's arrival presses its own Resume.
   That Resume reaches `carry_on` over there, finds the task already on its
   platform, and launches it. Both ends' Timelines say the move was made for
   the task, naming the task and the device, in `docs/copy.md`'s voice. This
   means a third reason beside `store::AskedBy::{Human, Session}` and
   `verkstead_render::MovedFor`. Mind the wire: `CameFrom.asked` crosses
   between Verksteads, so an older far end must still read the arrival.

**A placement is asked at every launch of a declared task, Resume included**
(Q5a). The declaration is what the task wants, wherever the Conversation
happens to stand when it is launched. **The work stays where it is**
afterwards: an undeclared task, the finish and the wrap-up run wherever the
last move left the work, and nothing ever moves it home.

Where no candidate is found, this task launches the task here and says nothing
beyond a log line. Task 03 adds the Notice and the hold for a roadmap stage's
tasks.

Also make the words true where they now are not:

- In the breaking-down skill, replace the sentence *"A task line's platform is
  recorded rather than acted on: nothing places a task by it yet."* with what
  happens now (Verkstead moves the task to a permitted device of that platform
  before its session starts). Keep the `crates/server/src/skills.rs` tests in
  step.
- CONTEXT.md's **Transfer** term: write the task half of the *Being revised by
  ADR-0029* pointer into the term. Leave the pointer standing for the inline
  run (task 04) and for stages (stage 06).

## Acceptance criteria

- [ ] A two-device suite: a backlog of `01` (undeclared), `02 — on windows`, `03` (undeclared), driven on a device whose OS word is Linux with a member whose word is Windows ticked. Task 01 runs here, the Conversation moves before 02's session starts, 02 and then 03 run on the far device, and nothing moves it back.
- [ ] Both Timelines name task 02 and the device the move was for; the arrival's Notice reads correctly to a far end that predates the new reason.
- [ ] Candidate choice is unit-tested without a cluster: the drafting device counts as permitted, a *Linux (WSL)* member is a Linux candidate and never a Windows one, a ticked device that is unreachable or lacking is passed over for the next, and a device already of the platform is never moved off.
- [ ] A Resume of a declared task that was moved back by hand moves it again; an undeclared task never asks for a move.
- [ ] The breaking-down skill and CONTEXT.md's **Transfer** say what is now true.
