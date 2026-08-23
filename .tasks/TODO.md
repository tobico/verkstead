# manual-task

The way to move a Conversation by hand, and the check that says when one needs
moving. A **Manual Task** is a free-text instruction the human types at the end
of a Conversation's Timeline: they pick an Agent Profile, submit, and a one-off
session does what it says — outside the grilling and implementation pipeline,
changing no state. It is the escape hatch for work the pipeline is not driving.

**Stalled** is the condition that makes the hatch necessary: a Conversation in
a driven state with nothing driving it and no open Interruption, which today
offers the human nothing at all. A sweep detects it and raises an Interruption,
so the existing Remedies become the way back, and Retry relaunches whichever
driver the state calls for.

## Tasks

- [x] 01: Manual Task on the Timeline — [details](01-manual-task-event.md)
- [x] 02: The composer, and the session it starts — [details](02-composer-and-session.md)
- [x] 03: A manual task that fails — [details](03-manual-task-interruption.md)
- [x] 04: The driver registry — [details](04-driver-registry.md)
- [x] 05: The stall sweep — [details](05-stall-sweep.md)
- [ ] 06: Retrying a stall for the runner, an inline session or a wrap-up — [details](06-stall-retry-drivers.md)
- [ ] 07: Retrying a stall for a grilling — [details](07-stall-retry-grilling.md)
