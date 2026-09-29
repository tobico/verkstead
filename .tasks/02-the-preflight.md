# 02. The preflight, and the dialog that refuses

## What to build

*Transfer to…* on the Conversation actions menu — both the pane's menu and the
sidebar card's right-click, which are the same rows drawn twice. Drawn from
every state but Draft and Closed: a Draft is moved by its composer's device
select, which stage 07 built, and a Closed Conversation has no work to move.

The row opens a dialog, on the pattern the close's confirm set: a card over the
page rather than a page of its own. Inside it, the device select stage 07
exports, listing every Member and not this device. Picking one sets a
**preflight** going, and what it found is drawn under the select.

The preflight is a reading of what the far end lacks, and it is asked of that
device rather than worked out here — with one exception. Whether one of that
device's Repos **is** this repository is settled by the rule stage 08 wrote,
which runs on the end that is going to act on the answer: the far end sends its
registry and this end applies it. Everything else only the far end can say, so
it answers about itself: whether it has a Repo for each Companion, whether the
harness each of the Conversation's Pairings names is on its PATH, and whether
it is answering at all.

Go is refused while anything is missing, and the refusal is by name rather than
a count. A Repo with no match names the repository and points at **Open repo**
on that device. A harness that is not there is named against the Pairing that
wants it, in the words the onboarding probe uses. A device that is not
answering is refused as unreachable and named — never as *no match*, those
being two different things to say: no match sends somebody to open a repository
on a machine that may already have it, and a machine that is asleep will answer
perfectly well tomorrow.

Go does nothing yet beyond being drawn enabled or refused. What moves the work
is the next task.

## Acceptance criteria

- [ ] *Transfer to…* is on both menus for a Conversation in any state but Draft
      and Closed, and on neither for those two.
- [ ] The dialog's select lists every Member with its OS mark and not this
      device, and picking one draws that device's preflight.
- [ ] A Repo the far end has no match for is named in the dialog, points at
      **Open repo** on that device, and leaves Go refused.
- [ ] A Pairing whose harness the far end has not got is named with that
      Pairing, and a device that answers nothing is refused as unreachable by
      its own name.
