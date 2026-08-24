# 02. Every stop is a halt

## What to build

Every remaining site that raises an Interruption records a halt with a Notice
instead, through the same path task 01 built. Each names its own cause the
way its Interruption did, and each is marked deliberate or circumstance:

| Site | Kind |
|---|---|
| A session ended without its step landing | deliberate |
| An inline implementation ended badly or committed nothing | deliberate |
| Checks still red after every fix attempt | deliberate |
| A review session ended without putting anything to the human | deliberate |
| A manual task's session exited badly | deliberate |
| The finish step's pull request cannot be found | deliberate |
| A restart took away the session a pick was waiting on | circumstance |

Deliberate means Verkstead chose to stop and the human must choose to go
again; circumstance means nothing chose anything, and task 06 will let
restarts resume these on their own. After this task nothing in the codebase
raises an Interruption; the type, the table and the settle endpoint remain
only for events already stored, until task 08 removes them.

## Acceptance criteria

- [ ] No code path constructs a new Interruption; each site above writes a
      halt and a Notice carrying the cause it names today.
- [ ] The deliberate-or-circumstance mark is stored per the table and the
      server tests for each site assert it.
- [ ] A Conversation already halted is not halted twice — one open halt per
      Conversation, as one open Interruption was.
