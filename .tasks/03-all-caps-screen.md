# 03. Fix the all-caps screen

## What to build

The agent-output Screen sometimes renders all its terminal text in capitals.
Diagnosed: the timeline's liveness badge is styled by a bare `.live` class
rule carrying `text-transform: uppercase`, and the live Screen marks itself
with a `live` modifier class on the `.screen` element — the badge rule
matches it, and the transform inherits into xterm's DOM-rendered text.

Scope the badge rule to where badges live (or rename one of the two classes)
so the terminal renders text in its own case. Sweep for the same collision
pattern on the other bare state classes while there.

## Acceptance criteria

- [ ] A live session's Screen shows terminal text in its original case
- [ ] The liveness badge on timeline rows is unchanged
