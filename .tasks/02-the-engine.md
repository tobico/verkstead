# 02. The engine, end to end on a fixture Process

## What to build

Verkstead runs a Conversation on a third-party Process by reading its
manifest. Proven end to end on a fixture plugin of two states over the
built-in One session Activity, installed, picked and run.

- **Which Process a Conversation runs** is stored in the `processes` side table
  as the providing plugin's uuid and the Process id (Set 1877, Q6). The five
  bare words already stored — `develop`, `investigate`, `review`, `tinker`,
  `fix-merge-issues` — are read as aliases of the built-in plugins' Processes
  (task 03 and 05 write those plugins; until then the aliases resolve to the
  code paths that run them today). A Conversation with no row still reads
  Develop, as now.
- **The picker's rows come from the server**: every Process from an enabled,
  loaded plugin, with its name and description; one in trouble or disabled is
  not offered. The five built-ins can still be drawn from their present
  definitions until task 07 retires the viewer's hand-kept table — this task
  adds the server-sent list and the third-party rows on it.
- **Start enters the first declared state**: the state word is written to the
  `conversations.state` column, the state's Activity started with the
  Conversation's facts and the state's options.
- **An Outcome is mapped by the manifest**: to the next state (entered the same
  way), to Done or Closed, or to `stopped`, which leaves the Conversation
  Stopped where it stands with the Activity's stop Notice, waiting on Resume.
  The human's Close reaches Closed from any state as now.
- **A restart or a Resume re-enters the current state's Activity**: the
  Activity works out where it was from its facts (the restart rule). A
  Conversation whose Process is in trouble stops where it stands with its card
  saying which dependency is wanted, and carries on from its state once that
  dependency is back.
- **Disabling a Process plugin** takes its row off the picker; Conversations
  already on it run to the end (Set 1764, Q20).

The built-in Processes keep running on their present code paths here; this
task is the engine and a third-party Process beside them.

## Acceptance criteria

- [ ] An installed fixture Process appears on the picker with its name and
      description, and is gone from it once its plugin is disabled
- [ ] A Conversation on the fixture runs Draft → first state → second state →
      Done over One session, each state's word in the `conversations.state` column
- [ ] A server restart mid-state carries the Conversation on from that state
- [ ] An Outcome mapped to `stopped` leaves it Stopped with its Notice, and
      Resume re-enters the state
