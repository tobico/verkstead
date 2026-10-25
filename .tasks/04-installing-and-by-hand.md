# 04. Installing and by hand

## What to build

The first step's other two screens, as `01-installing` and `01-by-hand` draw
them. Nothing on the wire changes: the per-row install state and the failure's
own words are already there.

**Installing.** The title reads *Installing* and the standing line names the
machine. Under it the count *N of M installed* over the ember progress bar from
the run's done and total; then the status line, which sits in an ember callout
with the server's words and *Answer it on the machine Verkstead runs on, not
here.* while the phase is Asking, and as a plain line otherwise; then a card
of the ticked rows, each with a ring and its name, the row the run is on
showing **Installing** in ember and a row still waiting showing nothing (Question
Set 1382 settled this: no queued label, the bar says how far the run has got).
The foot holds Cancel alone, reading *Cancelling…* once pressed; there is no
primary press during a run. The screen still moves on by itself when the run
ends, to the next step or to the by-hand screen.

**By hand.** The title is *Install the Rest by Hand*, the standing line *Run
these on <machine>. Each one ticks itself off here as it lands.* Then one line
with *N still missing* on the left and **Commands for** beside a `select` on
the right, one option per OS guide in the order the tabs stand today, the
detected one chosen and the human's choice kept through re-reads; the Mac's
Homebrew line and the Intel Mac's note draw under the select for the tab that
has them. The nine tab buttons go, and the tests that press them press the
select instead.

Under that, one card holding a block per row: the waiting ring, the name and
its why, and on the right a **Failed ▾** pill where the run failed it. The pill
is a disclosure, closed by default, opening *What the machine said* over the
failure's full text in a red wash (Question Set 1379's decision). Then the
command in a block with a `$` prompt and the copy button, the vendor note
under it, and the alternative behind an **Or from npm ›** link. The probe's
details move here from the choosing screen: the resolved path and link target
under a row that has since landed, the seen-elsewhere note, and the sandbox's
own stderr, each in the quiet form it has today. The PATH box, labelled PATH
with the directories and the read-once sentence, moves under the card.

**The press.** Back opens the choosing screen. The count *N of M found* stands
beside the primary button, M being the rows listed. The button is released
**once a sandbox, git, and one agent are present**, one rule whichever way the
screen was reached; holding it until every listed row landed was rejected
because the screen now lists gh and every harness. It reads **Next**, except
after a run where ticked rows the run failed are still missing, when it reads
**Skip**, so the press says that something ticked is being left behind. From
the link there was no run and nothing was skipped, so it is Next.

## Acceptance criteria

- [ ] The install screen draws the count, the bar, the status in the callout
      during Asking and as a line otherwise, and the ticked rows with Installing
      on the running one; Cancel is the only press and Next is absent.
- [ ] The by-hand screen draws the select with every OS guide reachable, both
      Macs among them, the block per row with the closed-by-default Failed
      disclosure, the command with copy, the alternative, and the PATH box
      under the list, and the probe's details are drawn here and nowhere else.
- [ ] Next is released once the step's objective is met whichever way the
      screen was reached, reads Skip after a run while failed ticked rows are
      still missing, and both rules have tests.
