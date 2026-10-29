# 04. Submit applies companions

## What to build

Submit applies the Set's pending companions and opened-up companions through
the Steer's own plan and make for adding a Companion and upgrading one, reached
from the Response's transaction rather than the Steer form. The order is the
Steer's: the record and git are asked everything a checkout turns on before the
Response is stored, and the checkouts are made after. A fetch that fails, a
base that does not resolve, or a read-write branch already taken refuses the
Submit by name with nothing made and nothing stored, the Answers kept on the
sheet and the chip removable; a Repo whose registration has gone since the
pick refuses the same way.

An opened-up companion gets its read-write branch cut from the chosen base and
its detached read-only checkout replaced, as the Steer does. The record line
under the Set's Event names the companion with its mode, and for read-write its
branch; the Response's `added` entry names the companion's directory under the
Conversation's root and its mode, so the agent knows where to look.

Nothing is ended in this task. A running session keeps the bindings it started
with and the companion reaches the next launch, bound at its mode and listed in
the prompt the way a Steer-added companion is. Where the Steer's make would
replace a checkout under a session, follow what the Steer does for that case.
Task 05 adds the restart for the session waiting on the Set.

Read ADR-0028 and the companion-repos roadmap, in particular its stage on the
Steer and stages, before starting.

## Acceptance criteria

- [ ] Submit with a pending companion fetches, resolves and cuts through the Steer's plan and make; the next session of the Conversation has it bound at its mode and listed in its prompt.
- [ ] Opening a read-only Companion up from an Answer cuts the branch and replaces the detached checkout as the Steer's upgrade does.
- [ ] A fetch that fails, a branch already taken, or a Repo no longer registered refuses the Submit by name, with nothing made, nothing stored, and the Answers kept.
- [ ] The Timeline line names the mode or the branch; the Response's `added` entry names the companion's directory and mode.
- [ ] Every git question is asked before the Response is stored, and the making happens after; the Steer's existing tests for add and upgrade still pass.
