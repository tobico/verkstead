# 04. Resume

## What to build

One standing way to start driving again: **Resume**, in the start-work menu
beside the manual task, shown whenever the Conversation is in a driven state
with nothing driving it — halted or merely undriven. Pressing it recomputes
what should be running from the lifecycle and the repository, clears the
halt, and takes the driver registration:

- **Grilling** — a fresh grilling session from the Brief and the digest of
  what was already asked and answered.
- **Implementing** — the next step read off the branch: the task list's
  lowest task, the finish step, or a fresh inline implementation from the
  Brief and handoff.
- **Wrapping** — the wrap-up watchers, with the checks fix-attempt counters
  forgotten first, as Retry does today.

Resume is never silent. It either starts something — a Notice is not needed,
the session shows up — or it refuses with a named reason the viewer shows,
the way a manual task refuses: nothing left to start and why, no
implementation pairing chosen, already driven, and so on. The silent bails in
the driving recompute (no direction, no worktree, an empty task list) become
those refusals. Carries no note field.

The old Retry remedy keeps working for already-stored Interruptions until
task 08.

## Acceptance criteria

- [ ] Resume on a halted Conversation in each of the three lifecycles starts
      the right thing, clears the halt and drops the badge.
- [ ] Resume with nothing startable returns a named refusal the viewer
      shows, and leaves the Conversation as it found it — pressed twice, the
      second press refuses as already driven.
- [ ] The menu shows Resume exactly when the Conversation is in a driven
      state and undriven.
