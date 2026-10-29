# 07. Crossing on transfer, and the words

## What to build

**Crossing.** A Set that crosses on a Transfer with pending additions carries
them with its record. On the far end each pending companion is matched to a
Repo by the rule the Companion rows already use for a Transfer, and one that
does not match is dropped and struck through, the strike saying why. A pending
server crosses by name and meets task 03's refusal on the far end if that
device does not declare it. A pending device tick crosses as it is. The set is
then answerable on the far end with the surviving picks intact, and its Submit
there applies and restarts as on the device that asked.

**The words.** CONTEXT.md gains the terms the code settled on across tasks 01
to 06, at least the pending addition with its states and the set-level
attachment, each with its *Avoid* list, and the Question Set, Attachment, MCP
server, Companion Repo, Permitted Device and Carried Conversation entries gain
the sentence that points at this. The design doc gains its section on adding
from an Answer, resting on ADR-0028, and ADR-0021 and ADR-0020 each gain the
line saying ADR-0028 amends or extends them. Every string this feature put on a
screen, the menu rows, the chips and strikes, the refusals, the Timeline lines
and the note the resumed agent is given, is checked against `docs/copy.md` and
fixed where it strays.

## Acceptance criteria

- [ ] A transferred Set with a pending companion the far end holds is answerable there with the pick intact, and its Submit applies it there.
- [ ] A pending companion whose Repo the far end does not hold arrives struck through with the reason; a pending device tick arrives intact; a Transfer with no pending additions is unchanged.
- [ ] CONTEXT.md defines the new terms and cross-references the touched ones; the design doc has its section; ADR-0021 and ADR-0020 point at ADR-0028.
- [ ] Every on-screen string of the feature follows `docs/copy.md`, and the web and server suites pass.
