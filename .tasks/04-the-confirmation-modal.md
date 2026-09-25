# 04. The confirmation modal and its push

## What to build

B asks its human. A pending join is raised in every open workbench of B and on
B's phones, and Allow or Deny settles it. Nothing goes back to A in this task —
that is task 05 — so what this delivers is the asking and the press.

**A nudge kind for a pending join.** It carries no request id: the kinds on that
stream are a vocabulary for *what moved* and the page re-reads what moved, which
is how every other kind works, and the client's own table is what says which
queries a kind stands for. So the kind says the pending joins have moved, the
page reads the pending list back, and a page too old to know the kind falls to
reading back everything it is showing — which is already correct behaviour here.
Announce it when a request arrives, and when one is settled or expires.

**The modal, in every open workbench.** The one modal component in the app is
told whether it is up by whoever opened it, and every use of it today belongs to
a page — a Remove on a Repo, a form on a pane. This one belongs to no page: a
join has to be raised whatever the human happens to be looking at, so it is drawn
in the shell every page sits inside, beside the toast layer, which is the one
place in the tree something is already drawn over every page and is there once.
Follow the confirm pattern the Repo list's Remove uses for the card itself — the
pair of buttons, the classes every confirm pair in the app carries.

**What it says**: A's name with its OS icon, the address it was dialled from or
the first it advertised, and its certificate fingerprint — the same string A's own
pending row is drawing, so the two can be compared by eye. Allow and Deny.

**The push.** A phone gets told, titled for the device asking. This is the first
push in the tree about no Conversation at all: every one today loads a
Conversation and titles itself by its branch, and the sending underneath that is
already Conversation-free. The human settled a **second entry point beside the
Conversation one** rather than bending the existing news enum — so what is added
is a way to send a notice about something that is not a piece of work, going
through the same sending, and the rule the existing titles are all written in one
place for still holds: what a notification has to do is be told apart from every
other one at a glance. A tap opens the Remote access pane, which is where the
modal will be raised again by the nudge the page reads on arrival.

**Settled once, and seen to be settled.** Two workbenches may both be showing the
modal. The first press settles the request; the second finds it settled and the
modal goes, rather than a second Allow landing or an error being shown for having
lost a race. Expiry settles it the same way, so a modal left up for ten minutes
goes by itself.

**What Allow does here, and no more**: it settles the request as allowed and
records A as a member of B — which is task 01's table taking its first real row.
It does not dial A back and it does not tell any other member. So after this task
B's list holds A and A's row still reads *waiting*: half a link, and the next task
is what closes it. Deny settles it as denied and records nothing.

## Acceptance criteria

- [ ] A join arriving on B raises the modal in every open workbench of B,
      whatever page each is on, showing A's name, OS, address and fingerprint —
      the same fingerprint A's own pending row draws.
- [ ] Allow records A as a member of B and settles the request; Deny settles it
      and records nothing; neither can be pressed twice to any effect.
- [ ] A second workbench showing the modal sees it go when the first one presses,
      and when the ten minutes run out.
- [ ] B's phones are told, titled so the notification names the device asking,
      and a tap opens the Remote access pane.
- [ ] A page that does not know the new nudge kind still reads back what it is
      showing rather than doing nothing.
- [ ] The push failing to send costs the notification and not the request: the
      modal is up and the press works regardless.
