# 03. What the rest of the workbench says about it

## What to build

The presses and marks around a pending steer, none of which the form itself
owns.

**Resume** is the opposite decision, so a Resume that starts something deletes
the pending steer first; a Resume refused by name leaves it, nothing having
changed. **Close** deletes it in the transaction that closes the Conversation,
the way closing shuts every open Question Set. **Stop** and **Force stop** are
about the run rather than the move, and leave it alone. The **interrupt tick**
is drawn from whether a session is running now rather than from what the press
found, since the item may sit open for hours and the session may have been
seen out meanwhile.

A pending steer **waits on the human**: it joins the one rule the sidebar disc
and the status button's *Waiting on you* are both read from, beside an open
Set and a stop nobody chose. It raises no notification, the press being the
human's own. The status word reads *Waiting on you* rather than *Stopped*
while one is pending.

Steer pressed from the **sidebar's right-click menu** on a Conversation that is
not open navigates to that Conversation's pending steer, pushing rather than
replacing, since the human is going somewhere. And the Timeline **scrolls the
pending steer into view** when the press selects it, whether or not the human
had scrolled up to read history; a card selected by a press is the one thing
they want to see.

A pending steer never boards a **Share**: it is not an Event, so nothing has to
be done, and a test says so.

## Acceptance criteria

- [ ] Resume deletes the pending steer and resumes; a refused Resume leaves it;
      Close deletes it; Stop and Force stop leave it; the interrupt tick follows
      the live Conversation.
- [ ] A Conversation with a pending steer reads *Waiting on you* on the status
      button and carries the sidebar disc, and nothing is pushed to a phone for
      it.
- [ ] Steer from the sidebar menu on another Conversation lands on its pending
      steer with the item scrolled into view; a share of a Conversation with a
      pending steer carries no trace of it.
