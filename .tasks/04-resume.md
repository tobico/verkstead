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

## What task 01 left here

Two end-to-end tests went when the stall stopped raising Interruptions, because
nothing could arrange what they were about any more. Resume is what they become:

- `retrying_a_stalled_backlog_run_takes_the_next_task_off_the_repository` — the
  next task read off `.tasks/` after a session died and the human took over.
- `retrying_a_stalled_grilling_starts_a_fresh_one_told_what_was_already_settled`
  — a fresh grilling on the Brief and the digest, with the Set the dead session
  left open archived on the way past. It used two Question Set constants,
  `ASKED_ALREADY` and `LEFT_HANGING`, which went with it.

The halts of all three lifecycles are covered already, so what these need is the
press and what it starts.

## What task 02 left here

Six end-to-end tests went when the last raising sites moved over to halts:
nothing arranges an Interruption any more, so nothing could press a Remedy on
one. Resume is what each of them becomes.

Off the Retry remedy, which Resume replaces:

- `retrying_runs_the_step_again_in_a_session_told_what_the_human_wrote` — the
  step run again in a fresh session. Resume carries no note, so what survives
  is the fresh session on the right step, under the right Pairing, primed with
  the Brief.
- `retrying_a_red_check_watches_it_again_from_no_attempts_spent` — the fix
  counters forgotten before the watchers go back on, which is this task's
  Wrapping branch. A `gh` red until a file appears drove it; the helper it used,
  `gh_checking_until`, went with it.
- `retrying_a_manual_task_runs_the_instruction_again_under_the_implementation_profile`
  — not Resume's: a manual task is outside the pipeline, and its instruction is
  submitted again rather than resumed. Nothing replaces it.
- `a_driver_waits_for_a_manual_session_rather_than_ending_it` — a driver
  starting while a Manual Task holds the Turn waits for the Worktree instead of
  killing the session. Resume is the press that starts the driver now.

Off the other two remedies, which task 07's Stop and Abort take over:

- `taking_over_and_aborting_both_leave_the_repo_as_the_session_left_it` — the
  Worktree kept and nothing launched behind the stop. The halt tests assert the
  Worktree half already; what is left is Abort from a halt keeping it.
- `a_remedy_pressed_twice_is_the_first_choice_arriving_again` — this task's
  *pressed twice, the second press refuses as already driven*.

The settle endpoint has no test of its own from here on. It stays for the
Interruptions already stored, and task 08's migration is where that is proved.
