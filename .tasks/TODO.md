# Carrying a cut-off session on

When a session ends before its work did, whatever starts the run again resumes
**that session's own conversation** with the harness's own resume (`claude
--resume <id>` and its siblings), instead of Verkstead's own Resume re-priming a
fresh session from the record. A cut-off session today loses everything the agent
had in its head, and the recompute can even start a different job altogether.

This extends the **Carried Conversation** machinery — which already carries a
conversation on for a Transfer, a sandbox-widening answer (ADR-0028) and *Keep it
here* (ADR-0029) — to every way a session is cut off mid-turn.

**The settled rule.** A session cut off *mid-turn* — one that did not finish its
step — is carried on. One that finished its step, or a run stopped between tasks,
stays Verkstead's own Resume, because its next step is a fresh session anyway. The
decision is read **at launch** (in `resume::recompute`, reached by the human's
press, `at_startup`, and the sign-out carry-ons alike) and written as a
`continued_sessions` row the existing `sessions::continuing` machinery takes up —
so a crash and a restart fall out for free, nothing having run at the crash to
write anything.

## Tasks

- [x] 01: The occasion and the launch-time carry-on — [details](01-occasion-and-detection.md)
- [x] 02: The Implementing mid-turn signal — [details](02-implementing-signal.md)
- [ ] 03: The job-must-not-change short-circuit — [details](03-job-short-circuit.md)
- [ ] 04: The remaining stop-ways — [details](04-remaining-stop-ways.md)
- [ ] 05: Restart and shutdown — [details](05-restart-and-shutdown.md)
- [ ] 06: The sign-out and usage-window reader preservation — [details](06-reader-preservation.md)
- [ ] 07: CONTEXT.md and the ADR — [details](07-context-and-adr.md)
