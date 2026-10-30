# Platform placement roadmap

Work that wants an operating system gets there without the human pressing
anything: a stage or task line's `on windows` places it on a ticked device of
that platform, the closing proposal says which platforms the work wants so the
human ticks devices at the moment they pick a direction, every session is told
every device of the cluster so it can ask for one, and where the device wanted
is asleep or not yet permitted, Verkstead asks with a Question Set of its own
rather than refusing. What forced it: agents fixing Windows failures from Linux
by pushing and reading CI, with a Windows device linked the whole time. The
decisions and their why are in [ADR-0029](../../adr/0029-work-is-placed-by-platform.md);
[CONTEXT.md](../../../CONTEXT.md) carries a pointer on each of the terms it
changes — **Permitted Device**, **Proposal**, **Stage**, **Transfer** and
**Notice** — and the stage that makes each true rewrites the term.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Three stages are roots. 01 is the prompt and the skills, useful the day it
lands with nothing else built. 02 is the proposal's `wants` and the chooser's
ticks. 05 fixes something true today: a stage moved by hand already goes dark
to its roadmap's scheduler, so driving a roadmap across devices is its own root
rather than a tail on placement. 03 stands on 02 for the placing of an inline
run by what its proposal wants. 04 stands on 03, placement being what fires the
gate it builds. 06 stands on 04 for the gate and on 05 because a stage placed
elsewhere is only safe once home can see it settle; placing a *task* never
leaves its Conversation's device dark, which is why tasks and stages are two
stages here.

## Stages

- [x] 01: Every device in every prompt — [brief](01-every-device-in-every-prompt.md) — no dependencies
- [ ] 02: What the work wants, on the chooser — [brief](02-what-the-work-wants-on-the-chooser.md) — no dependencies *(in progress: `roadmaps/platform-placement/02-what-the-work-wants-on-the-chooser`)*
- [ ] 03: Tasks placed by platform — [brief](03-tasks-placed-by-platform.md) — after 02
- [ ] 04: Verkstead's own Question Set — [brief](04-verksteads-own-question-set.md) — after 03
- [ ] 05: Home drives a roadmap across devices — [brief](05-home-drives-a-roadmap-across-devices.md) — no dependencies
- [ ] 06: Stages placed by platform — [brief](06-stages-placed-by-platform.md) — after 04, 05
