# 02. The phone

## What to build

Below 60rem the rail folds into the header the two phone boards draw
(`phone-01-choosing` and `phone-02-steps-open`): the brand on the left, a
**Steps** button on the right, and under them a strip of three dots, the open
one filled, with the open step's name beside them. The paper under it is the
same paper as the wide layout at the phone's width, the four harness tiles
staying a 2x2 grid there too once task 03 draws them.

**Steps opens a sheet over a veil.** It is the app's one Modal, a native
dialog, with a class of its own placing the card at the top of the window
rather than rising from the bottom, as the board draws it. Inside: the brand
and a Close button, the SETUP label, and the three steps listed as the rail
lists them, with the summary under a met one and an **Open** link on the right
of a met step that is not open. Open opens that step and closes the sheet; an
unmet step has no Open, for the rail's reason. Escape and a press on the veil
close it. The Menu was considered for this and rejected: the sheet is a card
over the page, not rows dropped from a button.

**The foot is sticky** on a phone: the primary button at full width, and the
quiet alternative (today only step 1's *Install by hand instead*, which task
03 adds) as a centred ember link under it. Back stands in the sticky foot too
where a step has one, drawn as the board's quiet button.

## Acceptance criteria

- [ ] At 390px the header shows the brand, Steps and the three dots with the
      open step's name; from 60rem none of it is drawn and the rail is.
- [ ] Steps opens the Modal at the top of the window listing the three steps;
      Open is drawn only on met steps that are not open, and pressing it opens
      the step and closes the sheet.
- [ ] The foot is sticky at the bottom with a full-width primary button, and
      the frame suite covers the sheet's opening and the Open rule.
