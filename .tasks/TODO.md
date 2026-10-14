# Memos

A building session can leave a **Memo** for the sessions after it on the same
Conversation: working knowledge no artifact holds — a flaky test, a build trap,
a decision taken mid-task and why. `verkstead memo` takes markdown on stdin and
puts it on the Timeline as an Event of its own, stamped with the session that
left it; every later session of the Conversation is told the unretired memos in
its prompt, oldest first, until a later session or the human retires one. What
was settled, and why, is ADR-0023 and the **Memo** entry in `CONTEXT.md`, both
committed beside this plan.

## Tasks

- [x] 01: A memo on the record — [details](01-a-memo-on-the-record.md)
- [x] 02: `verkstead memo` leaves one — [details](02-verkstead-memo-leaves-one.md)
- [x] 03: Shown to every later session — [details](03-shown-to-every-later-session.md)
- [x] 04: Retiring — [details](04-retiring.md)
- [x] 05: An unknown Event kind no longer breaks a Timeline — [details](05-tolerant-timeline-read.md)
- [ ] 06: The Guide and the skills — [details](06-the-guide-and-the-skills.md)
