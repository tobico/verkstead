# Every device in every prompt

A session on any Conversation of a clustered Verkstead knows what machines
exist and what it stands on. Its prompt's devices section names every member of
the cluster — name, id, operating system, and whether this Conversation may move
there — headed by which operating system this device runs, and says how to ask
for a device that is not permitted; the Guide's *Moving the work to another
device* says the same. Today a Conversation with nothing ticked is told nothing
about other devices, so an agent fixing a Windows failure cannot ask for the
Windows device it was never told exists (ADR-0029, *Consent stays off by
default, and is told*).

The skills carry the two rules ADR-0029 settled: staging and breaking-down say
when a line declares `on <os>` — only where the work can be verified nowhere
else — and implementing, next-task and addressing say that CI confirms and does
not discover, so a failing check whose job runs on another operating system is
reproduced there rather than fixed from the log.

Roadmap stage: [01: Every device in every prompt](docs/roadmaps/platform-placement/01-every-device-in-every-prompt.md)

## Tasks

- [x] 01: The section, for every member — [details](01-the-section-for-every-member.md)
- [x] 02: The Guide, the transfer help and the refusal — [details](02-the-guide-the-help-and-the-refusal.md)
- [x] 03: Declaring, in staging and breaking-down — [details](03-declaring-in-staging-and-breaking-down.md)
- [x] 04: Moving, in implementing, next-task and addressing — [details](04-moving-in-implementing-next-task-and-addressing.md)
