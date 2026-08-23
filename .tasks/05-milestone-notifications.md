# 05. Milestones, and the needs-you that never fired

## What to build

Push notifications for the milestones the design names — a pull request opened,
a roadmap Stage complete, a Conversation Done — and for the one needs-you event
that has never fired: an Interruption opening.

Today the devices are told two things: a Question Set has arrived, and a Hold
has stood a while. An Interruption is a run stopped on a choice only the human
can make, and it is on the design's own needs-you list, but nothing tells them
about one. A run that stops unattended and says nothing is the failure this
pipeline is built to avoid, so this is the more important half of the task even
though it is not the new half.

The three milestones are the other half: the moments the work moved on without
anybody watching. Each already has one place that knows it happened — the finish
that records the pull request, the settling that ends a wrap-up, and the notice
that says which Stage started — so each sends from there, and none of them may
delay or fail the thing it is announcing. Sending goes behind the work, never in
front of it, exactly as a Set's push does: a push service that cannot be reached
costs a notification and nothing else.

What a notification carries stays as small as it is now: enough for the service
worker to draw it and to know which page to open, and nothing that would put the
substance of the work on a lock screen. Tapping one opens the Conversation it is
about. Subscriptions are pruned on the two answers that mean a device is gone,
as they already are.

## Acceptance criteria

- [ ] An Interruption opening sends one push per subscribed device, naming the
      Conversation, and tapping it opens that Conversation.
- [ ] A pull request opening, a Stage completing and a Conversation reaching
      Done each send one, and each is distinguishable on a lock screen from the
      others and from a Question Set.
- [ ] None of the five can delay, fail or alter what it announces — the record
      lands whether or not any device is reachable.
