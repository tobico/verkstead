# 05. Clamped markdown cards

## What to build

Long markdown in the timeline stops pushing everything down. Three card
kinds clamp: the **frozen brief** (the editable drafting brief stays full
height — task 04), the **handoff**, and the **manual task**. Notice lines
stay whole; they are single lines already.

A clamped card shows at most **five lines**, with a gradient fading the last
of them into the card so the cut reads as a cut. The gradient appears only
when the content actually overflows the clamp — a short card shows whole,
with no fade.

Selecting the card opens the full markdown in the details pane, which makes
these three kinds openable — today none of them are. They should take the
same affordance as the other openable events (the whole surface presses, the
selection is visible), and each gets a details-pane view that renders the
complete markdown under an appropriate heading. Short cards open the pane
too: one consistent affordance, settled explicitly.

The details pane views land ahead of the pane's width cap (task 10); prose
already carries its own measure, so nothing here needs to wait on it.

## Acceptance criteria

- [ ] Frozen brief, handoff and manual-task cards clamp at five lines with a
      bottom gradient only when their content overflows
- [ ] Selecting any of the three — clamped or short — opens its full
      markdown in the details pane
- [ ] The drafting brief and notice lines are unaffected
