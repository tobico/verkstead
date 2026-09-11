# 03. The three screens

## What to build

The dependencies step becomes three screens over the reading task 02 put on
the wire, and every Continue in the wizard becomes **Next**.

**The checkbox screen.** A present row keeps its tick and a not-applicable row
its dash; an absent row draws a checkbox where the mark was, with the row's
name as its label. Only the gating rows — the sandbox and git — start ticked;
the harnesses and gh start unticked, and nothing says in advance which rows
cannot be installed here. The tabs, the PATH list, the restart note and every
instruction leave this screen. Next is pressable when the present rows plus
the ticked rows would meet the objective — a sandbox, git and one harness —
and otherwise disabled with the note the step has today. Pressing it starts
the run and opens the install screen.

**The install screen.** A progress bar of one unit per ticked row, filled as
each lands, and the status line from the wire under it. A Cancel button, which
posts the cancel and reads *cancelling* until the unit under way is over. The
page polls every two seconds while a run is going, and ten seconds otherwise.
When the run ends the screen moves on by itself: to the next step when every
ticked row is present, and to the hint screen otherwise.

**The hint screen.** Only the ticked rows still absent, each with the failure
or refusal under it and the instruction the old screen drew — the eight OS tabs
above them, the PATH list and the restart note where they were. Next is
disabled with a counter beside it, *n/m detected*, over the ticked rows, and
releases when every one of them is present; the poll keeps running until it
does. Back returns to the checkbox screen, which is how a row nobody wants
after all is unticked.

Which screen is open is a fact about this device, kept beside the open step
and never on the wire, the way the step is.

The viewer's tests are drawn over the golden fixtures task 02 writes — a run in
progress, a run with failures — beside the fresh and part-way readings they
have today. The adoption doc's paragraph on the first step says what the step
now does.

## Acceptance criteria

- [ ] On the fresh reading, the sandbox and git rows are ticked, the harnesses
      and gh are not, and Next is disabled until a harness is ticked.
- [ ] Pressing Next posts the ticked rows, the install screen shows the bar at
      0/3 and the status line, and a reading with every row present moves the
      step on without a further press.
- [ ] A reading with one failed row opens the hint screen with that row alone,
      its stderr under it, the counter reading 0/1, and Next disabled until a
      later reading shows it present.
- [ ] No button in the wizard reads Continue, and the tests and the adoption doc
      say Next.
