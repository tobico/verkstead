# 01. Verkstead asks about an unticked device

## What to build

The first Question Set Verkstead authors itself, end to end, through the one
case that needs nothing else from this stage: work that wants a platform whose
devices are linked but none permitted.

Today stage 03's placement (the `placing` module in the server) answers that
case — its `Unplaced::NonePermitted` — with a Notice and runs the work here.
Replace that Notice with a Set:

- **Stored and hung on the Timeline as a session's Set is** (the store's `ask`),
  and **marked as Verkstead's**. Nothing records who asked a Set today; record it
  *beside* the Set rather than in its body, the way the deferrals table records
  how a Set was asked (that module's header says why a fact learned about a Set
  hangs off it rather than becoming a column).
- **Drawn with its asker named**: the Timeline card and the Set's sheet read as
  asked by Verkstead, not by the agent.
- **The phone is told**, as for any open Set, the notification naming the branch
  and the device (`docs/copy.md`).
- **The words are ADR-0029's table** read through `docs/copy.md`, with the work
  named as placement already names it — *Task 03 wants Windows. nuc-win is linked
  but not permitted.* or *The inline run wants Windows. …* (stages are placed at
  stage 06, not here). A verb on each button.
- **Options**: one *Move to <device>* per linked unticked device of that
  platform, in placement's order, then *Run it here* (Set 1547, Q2).
- **Settled by a Response handler of Verkstead's own** that runs the picked
  Option, rather than the Response going to an agent: *Move to* ticks the device
  under *May be transferred to* and moves the Conversation by the ordinary
  Transfer on Verkstead's own account (as placement already asks it, `AskedBy`
  Task or Inline), and the far end's arrival launches the work there; *Run it
  here* launches the task — or the inline run — here. An answer naming no Option
  (a comment only, all unanswered) is the human not deciding: say how the work
  stands and leave it waiting, rather than guessing.
- **No device of the platform linked stays a Notice** (`Unplaced::NoneLinked`),
  and the work runs here, said once. `NoneCanTake` is untouched by this task.

The waiting itself — holding a place, surviving a restart — is task 02; here the
runner may simply stand down after putting the Set, as it does after asking for a
move.

## Acceptance criteria

- [ ] A backlog task declaring `on windows`, launched on Linux with a Windows
      device linked but not ticked, puts a Set asked by Verkstead naming the task,
      the platform and the device, with *Move to <device>* · *Run it here*; it is
      drawn as Verkstead's on the Timeline and its sheet, and it notifies.
- [ ] *Move to* ticks the device and moves the Conversation, and the task's
      session starts on that device; *Run it here* starts it here. Both are
      proven in the server suite, the inline run's case included.
- [ ] Two unticked Windows devices give two *Move to* Options; no Windows device
      linked is still the Notice, and the task runs here.
